//! WASM entry point for the Monero wallet extension.

pub mod abstractions;
mod actors;
#[cfg(target_arch = "wasm32")]
pub mod browser_impl;
mod core_rpc;
mod error_codes;
pub mod ffi_web;
pub(crate) mod logging;
mod messages;
#[cfg(target_arch = "wasm32")]
mod protocol_transport;
#[cfg(target_arch = "wasm32")]
pub mod rpc_adapter;
mod seed_format;
#[allow(dead_code)]
pub(crate) mod signal_ids;
mod signals;
pub mod wallet2_facade;
#[cfg(target_arch = "wasm32")]
mod wasm_fetch;

#[cfg(target_arch = "wasm32")]
pub use browser_impl::{BrowserStorage, CallbackRpcClient, JsTimeProvider, WasmRpcClient};

#[cfg(not(target_arch = "wasm32"))]
pub mod ffi_native;

#[cfg(target_arch = "wasm32")]
pub mod test_api;

pub fn new_protocol_dispatcher()
-> Result<monero_ffi_protocol::RuntimeDispatcher, monero_ffi_protocol::ProtocolError> {
    let mut dispatcher = monero_ffi_protocol::RuntimeDispatcher::new();
    dispatcher.mount(wallet2_facade::Wallet2Facade::new())?;
    Ok(dispatcher)
}
