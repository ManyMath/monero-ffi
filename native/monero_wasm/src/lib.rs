//! WASM entry point for the Monero wallet extension.

mod actors;
pub mod abstractions;
mod core_rpc;
mod error_codes;
#[cfg(target_arch = "wasm32")]
pub mod browser_impl;
pub mod ffi_web;
pub(crate) mod logging;
mod messages;
mod seed_format;
#[cfg(target_arch = "wasm32")]
pub mod rpc_adapter;
#[allow(dead_code)]
pub(crate) mod signal_ids;
mod signals;
#[cfg(target_arch = "wasm32")]
mod wasm_fetch;

#[cfg(target_arch = "wasm32")]
pub use browser_impl::{BrowserStorage, CallbackRpcClient, JsTimeProvider, WasmRpcClient};

#[cfg(not(target_arch = "wasm32"))]
pub mod ffi_native;

#[cfg(target_arch = "wasm32")]
pub mod test_api;
