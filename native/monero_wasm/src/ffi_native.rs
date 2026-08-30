//! C FFI entry points for native (non-WASM) platforms.
//!
//! This module is only compiled for native targets (`not(target_arch = "wasm32")`).
//! It provides an opaque byte-buffer transport identical in semantics to the
//! web worker transport, but using C calling conventions so that Dart's `dart:ffi`
//! (via ffigen-generated bindings) can call into Rust directly.

use std::ffi::c_void;
use std::sync::Mutex;

/// Opaque handle returned by [`hub_init`]. Holds the tokio runtime and
/// a join handle for the background thread driving the event loop.
pub struct HubHandle {
    /// The tokio runtime handle, kept alive so spawned tasks continue running.
    #[allow(dead_code)]
    runtime_handle: tokio::runtime::Handle,
    /// Sender to signal the background event loop to shut down.
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    /// Background thread running `runtime.block_on(...)`. Joined on shutdown.
    runtime_thread: Option<std::thread::JoinHandle<()>>,
    dispatcher: Mutex<monero_ffi_protocol::RuntimeDispatcher>,
}

// Wrapper to make raw pointers Send+Sync for the global Mutex.
struct CallbackState {
    callback: RustSignalCallback,
    user_data: *mut c_void,
}

// Safety: The callback and user_data are provided by the Dart FFI caller
// who guarantees the function pointer is valid for the lifetime of the handle
// and that user_data (if non-null) remains valid. The Mutex serializes all access.
unsafe impl Send for CallbackState {}
unsafe impl Sync for CallbackState {}

/// Function pointer type for Rust -> Dart signal delivery.
///
/// # Safety contract (caller of `hub_init` must guarantee)
///
/// * `callback` must be safe to invoke from any thread
/// * `user_data` must remain valid until [`hub_shutdown`] is called
pub type RustSignalCallback =
    extern "C" fn(signal_id: u32, data_ptr: *const u8, data_len: usize, user_data: *mut c_void);

/// Global callback stored after `hub_init` so that actor code can send
/// RustSignals back to Dart.
static GLOBAL_CALLBACK: Mutex<Option<CallbackState>> = Mutex::new(None);

/// Send a RustSignal to Dart from anywhere in the Rust runtime.
///
/// Allocates a persistent copy of the data via [`alloc_rust_bytes`] so that
/// the Dart-side async callback can safely read the bytes after Rust returns.
/// The Dart side MUST call [`hub_free_bytes`] after copying the data.
#[allow(dead_code)]
pub(crate) fn send_rust_signal(signal_id: u32, data: &[u8]) {
    let cb = {
        let Ok(guard) = GLOBAL_CALLBACK.lock() else {
            return;
        };
        guard.as_ref().map(|s| (s.callback, s.user_data))
    };
    if let Some((callback, user_data)) = cb {
        let (ptr, len) = alloc_rust_bytes(data.to_vec());
        callback(signal_id, ptr, len, user_data);
    }
}

/// Initialize the hub runtime, register the Dart callback, and spawn actors.
///
/// Creates a single-threaded tokio runtime on a dedicated background thread,
/// then spawns the actor system (`create_actors`) inside it. The background
/// thread runs the event loop until [`hub_shutdown`] drops the handle.
///
/// Returns an opaque [`HubHandle`] pointer that must be passed to
/// [`hub_send_dart_signal`] and eventually freed with [`hub_shutdown`].
/// Returns null on failure.
#[unsafe(no_mangle)]
pub extern "C" fn hub_init(callback: RustSignalCallback, user_data: *mut c_void) -> *mut HubHandle {
    if let Ok(mut guard) = GLOBAL_CALLBACK.lock() {
        *guard = Some(CallbackState {
            callback,
            user_data,
        });
    }

    let dispatcher = match crate::new_protocol_dispatcher() {
        Ok(dispatcher) => dispatcher,
        Err(_) => return std::ptr::null_mut(),
    };

    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(_) => return std::ptr::null_mut(),
    };

    let runtime_handle = runtime.handle().clone();
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    // Run the tokio event loop on a background thread so `hub_init` returns
    // immediately and the actors can process signals asynchronously.
    let runtime_thread = std::thread::spawn(move || {
        runtime.block_on(async {
            // Spawn actors; they run as independent tasks listening on channels.
            crate::actors::create_actors().await;
            // Block until hub_shutdown sends the shutdown signal.
            let _ = shutdown_rx.await;
        });
    });

    let handle = Box::new(HubHandle {
        runtime_handle,
        shutdown_tx: Some(shutdown_tx),
        runtime_thread: Some(runtime_thread),
        dispatcher: Mutex::new(dispatcher),
    });
    Box::into_raw(handle)
}

/// Send a DartSignal (Dart -> Rust) into the hub runtime.
///
/// # Safety
///
/// `handle` must be a valid pointer returned by `hub_init`.
/// `data_ptr` must point to `data_len` readable bytes, or be non-null
/// with `data_len == 0`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hub_send_dart_signal(
    handle: *mut HubHandle,
    signal_id: u32,
    data_ptr: *const u8,
    data_len: usize,
) {
    if handle.is_null() {
        return;
    }

    let data = if data_len == 0 {
        Vec::new()
    } else if data_ptr.is_null() {
        return;
    } else {
        unsafe { std::slice::from_raw_parts(data_ptr, data_len) }.to_vec()
    };

    crate::signal_ids::route_dart_signal(signal_id, data);
}

/// Result of dispatching a language-neutral protocol envelope.
///
/// `status == 0` means the buffer contains an encoded completion envelope.
/// Any other status means the buffer contains a UTF-8 error message. In both
/// cases the caller owns the buffer and must release it with `hub_free_bytes`.
#[repr(C)]
pub struct HubDispatchResult {
    pub data_ptr: *mut u8,
    pub data_len: usize,
    pub status: i32,
}

/// Dispatch a versioned protocol envelope through a mounted facade.
///
/// # Safety
///
/// `handle` must be a live pointer returned by [`hub_init`]. `data_ptr` must
/// identify `data_len` readable bytes when `data_len` is non-zero.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hub_dispatch_envelope(
    handle: *mut HubHandle,
    data_ptr: *const u8,
    data_len: usize,
) -> HubDispatchResult {
    let error = |message: String| {
        let (data_ptr, data_len) = alloc_rust_bytes(message.into_bytes());
        HubDispatchResult {
            data_ptr,
            data_len,
            status: 1,
        }
    };

    if handle.is_null() {
        return error("null hub handle".to_string());
    }
    let data = if data_len == 0 {
        &[]
    } else if data_ptr.is_null() {
        return error("null envelope pointer".to_string());
    } else {
        unsafe { std::slice::from_raw_parts(data_ptr, data_len) }
    };

    let handle = unsafe { &*handle };
    let mut dispatcher = match handle.dispatcher.lock() {
        Ok(dispatcher) => dispatcher,
        Err(_) => return error("protocol dispatcher lock poisoned".to_string()),
    };
    match dispatcher.dispatch(data) {
        Ok(completion) => {
            let (data_ptr, data_len) = alloc_rust_bytes(completion);
            HubDispatchResult {
                data_ptr,
                data_len,
                status: 0,
            }
        }
        Err(dispatch_error) => error(dispatch_error.to_string()),
    }
}

/// Shut down the hub runtime and free the handle.
///
/// Sends a shutdown signal to the background event loop, waits for the
/// thread to finish, then frees the handle.
///
/// # Safety
///
/// `handle` must be a valid pointer returned by `hub_init`, and must not
/// be used after this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hub_shutdown(handle: *mut HubHandle) {
    if handle.is_null() {
        return;
    }

    if let Ok(mut guard) = GLOBAL_CALLBACK.lock() {
        *guard = None;
    }

    let mut hub = unsafe { Box::from_raw(handle) };

    // Signal the event loop to stop.
    if let Some(tx) = hub.shutdown_tx.take() {
        let _ = tx.send(());
    }

    // Wait for the background thread to finish.
    if let Some(thread) = hub.runtime_thread.take() {
        let _ = thread.join();
    }
}

/// Free a byte buffer that was allocated by Rust via [`alloc_rust_bytes`]
/// and passed to Dart.
///
/// # Safety
///
/// `ptr` must have been allocated by [`alloc_rust_bytes`] (which uses
/// `Box<[u8]>`), and `len` must match the original allocation length.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hub_free_bytes(ptr: *mut u8, len: usize) {
    if ptr.is_null() {
        return;
    }
    // Reconstruct the Box<[u8]> that was leaked by alloc_rust_bytes.
    let _ = unsafe { Box::from_raw(std::slice::from_raw_parts_mut(ptr, len)) };
}

/// Allocate a byte buffer from a Vec and return (ptr, len) for FFI.
///
/// The returned pointer must be freed with [`hub_free_bytes`] using the
/// returned length. Uses `into_boxed_slice()` to ensure capacity == length,
/// avoiding the capacity mismatch issue with `Vec::from_raw_parts`.
#[allow(dead_code)]
pub(crate) fn alloc_rust_bytes(data: Vec<u8>) -> (*mut u8, usize) {
    let boxed = data.into_boxed_slice();
    let len = boxed.len();
    let ptr = Box::into_raw(boxed) as *mut u8;
    (ptr, len)
}

#[cfg(test)]
mod tests {
    use super::*;
    use monero_ffi_protocol::{FrameKind, WALLET2_PROTOCOL, WireEnvelope};

    const SEED: &str = "honked bagpipe alpine juicy faked afoot jostle claim cowl tunnel orphans negative pheasants feast jetting quote frown teeming cycling tribal womanly hills cottage daytime daytime";

    extern "C" fn discard_signal(_: u32, _: *const u8, _: usize, _: *mut c_void) {}

    #[test]
    fn native_byte_transport_dispatches_wallet2_commands() {
        let handle = hub_init(discard_signal, std::ptr::null_mut());
        assert!(!handle.is_null());

        let command = WireEnvelope::command(
            7,
            None,
            WALLET2_PROTOCOL,
            crate::wallet2_facade::OPEN_SEED,
            crate::wallet2_facade::encode_open_seed(SEED, "stagenet", "").unwrap(),
        )
        .encode()
        .unwrap();
        let result = unsafe { hub_dispatch_envelope(handle, command.as_ptr(), command.len()) };
        assert_eq!(result.status, 0);
        let completion_bytes =
            unsafe { std::slice::from_raw_parts(result.data_ptr, result.data_len).to_vec() };
        unsafe { hub_free_bytes(result.data_ptr, result.data_len) };

        let completion = WireEnvelope::decode(&completion_bytes).unwrap();
        assert_eq!(completion.kind, FrameKind::Completion);
        assert_eq!(completion.id, 7);
        assert!(completion.wallet.is_some());

        unsafe { hub_shutdown(handle) };
    }
}
