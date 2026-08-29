//! Test API for WASM
//!
//! Bindings to test Monero functions from JS.

use monero_rust::{
    SeedFormat, derive_address, derive_keys, generate_seed, seed_birthday, validate_seed,
};
use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;

#[derive(serde::Serialize)]
struct DerivedKeysJson {
    address: String,
    secret_spend_key: String,
    secret_view_key: String,
    public_spend_key: String,
    public_view_key: String,
}

impl From<monero_rust::DerivedKeys> for DerivedKeysJson {
    fn from(keys: monero_rust::DerivedKeys) -> Self {
        Self {
            address: keys.address.to_string(),
            secret_spend_key: keys
                .secret_spend_key
                .as_deref()
                .map(hex::encode)
                .unwrap_or_default(),
            secret_view_key: hex::encode(&*keys.secret_view_key),
            public_spend_key: hex::encode(keys.public_spend_key),
            public_view_key: hex::encode(keys.public_view_key),
        }
    }
}

#[wasm_bindgen]
pub struct TestApi;

#[wasm_bindgen]
impl TestApi {
    /// Generate seed (classic 25-word by default)
    #[wasm_bindgen]
    pub fn generate_seed() -> Result<String, JsValue> {
        generate_seed(SeedFormat::Classic)
            .map_err(|e| JsValue::from_str(&format!("seed gen failed: {}", e)))
    }

    /// Generate seed with specified type ("classic" or "polyseed")
    #[wasm_bindgen]
    pub fn generate_seed_typed(seed_type: &str) -> Result<String, JsValue> {
        let format = crate::seed_format::parse_seed_format(seed_type)
            .map_err(|e| JsValue::from_str(&format!("seed gen failed: {e}")))?;
        generate_seed(format).map_err(|e| JsValue::from_str(&format!("seed gen failed: {}", e)))
    }

    // Derive address from seed
    #[wasm_bindgen]
    pub fn derive_address(seed: &str, network: &str) -> Result<String, JsValue> {
        derive_address(seed, network, "")
            .map_err(|e| JsValue::from_str(&format!("addr failed: {}", e)))
    }

    #[wasm_bindgen]
    pub fn derive_keys(seed: &str, network: &str) -> Result<JsValue, JsValue> {
        let keys = derive_keys(seed, network, "")
            .map_err(|e| JsValue::from_str(&format!("keys failed: {}", e)))?;

        to_value(&DerivedKeysJson::from(keys))
            .map_err(|e| JsValue::from_str(&format!("serialize failed: {}", e)))
    }

    #[wasm_bindgen]
    pub fn validate_seed(seed: &str) -> Result<(), JsValue> {
        validate_seed(seed).map_err(|e| JsValue::from_str(&e))
    }

    #[wasm_bindgen]
    pub fn test_wasm() -> String {
        "WASM OK".to_string()
    }

    /// Test polyseed generation directly (for debugging)
    #[wasm_bindgen]
    pub fn test_polyseed() -> Result<String, JsValue> {
        generate_seed(SeedFormat::Polyseed)
            .map_err(|e| JsValue::from_str(&format!("polyseed test failed: {}", e)))
    }

    /// Get polyseed birthday (restoration timestamp) for a given seed
    /// Returns null for classic seeds, the Unix timestamp (seconds) for polyseeds
    #[wasm_bindgen]
    pub fn get_seed_birthday(seed: &str) -> Option<u64> {
        seed_birthday(seed)
    }
}
