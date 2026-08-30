//! WebAssembly byte transport for the language-neutral protocol dispatcher.

use std::cell::RefCell;

use wasm_bindgen::prelude::*;

thread_local! {
    static DISPATCHER: RefCell<Option<monero_ffi_protocol::RuntimeDispatcher>> =
        RefCell::new(crate::new_protocol_dispatcher().ok());
}

/// Dispatch one encoded protocol command and return its encoded completion.
#[wasm_bindgen]
pub fn dispatch_envelope(command: &[u8]) -> Result<Vec<u8>, JsValue> {
    DISPATCHER.with(|slot| {
        let mut slot = slot.borrow_mut();
        let dispatcher = slot
            .as_mut()
            .ok_or_else(|| JsValue::from_str("protocol dispatcher failed to initialize"))?;
        dispatcher
            .dispatch(command)
            .map_err(|error| JsValue::from_str(&error.to_string()))
    })
}
