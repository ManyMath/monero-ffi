//! Platform traits and RPC wire types used by the FFI browser adapters.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub type AbResult<T> = Result<T, AbError>;

#[derive(Debug)]
pub enum AbError {
    Storage(String),
    Rpc(String),
    Serialization(String),
    Network(String),
    NotFound(String),
    InvalidData(String),
}

impl std::fmt::Display for AbError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (kind, message) = match self {
            Self::Storage(message) => ("Storage error", message),
            Self::Rpc(message) => ("RPC error", message),
            Self::Serialization(message) => ("Serialization error", message),
            Self::Network(message) => ("Network error", message),
            Self::NotFound(message) => ("Not found", message),
            Self::InvalidData(message) => ("Invalid data", message),
        };
        write!(formatter, "{kind}: {message}")
    }
}

impl std::error::Error for AbError {}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
pub trait WalletStorage {
    async fn save(&self, key: &str, data: &[u8]) -> AbResult<()>;
    async fn load(&self, key: &str) -> AbResult<Vec<u8>>;
    async fn delete(&self, key: &str) -> AbResult<()>;
    async fn list_keys(&self) -> AbResult<Vec<String>>;
    async fn exists(&self, key: &str) -> AbResult<bool>;
}

pub trait TimeProvider {
    fn now(&self) -> u64;
    fn now_ms(&self) -> u64;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeightResponse {
    pub height: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockResponse {
    pub blocks: Vec<BlockData>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockData {
    pub block_header: BlockHeader,
    pub txs: Vec<TransactionData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeader {
    pub height: u64,
    pub timestamp: u64,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionData {
    pub tx_hash: String,
    pub tx_blob: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutsResponse {
    pub outs: Vec<OutEntry>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutEntry {
    pub height: u64,
    pub key: String,
    pub mask: String,
    pub txid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxSubmitResponse {
    pub status: String,
    pub tx_hash: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetOutsParams {
    pub outputs: Vec<OutputIndex>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputIndex {
    pub amount: u64,
    pub index: u64,
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
pub trait RpcClient {
    async fn call<T>(&self, method: &str, params: Value) -> AbResult<T>
    where
        T: for<'de> Deserialize<'de>;

    async fn get_height(&self) -> AbResult<u64>;
    async fn get_blocks(&self, start_height: u64, count: u64) -> AbResult<BlockResponse>;
    async fn get_outs(&self, params: &GetOutsParams) -> AbResult<OutsResponse>;
    async fn submit_transaction(&self, tx_blob: &str) -> AbResult<TxSubmitResponse>;
    async fn get_fee_estimate(&self) -> AbResult<u64>;
}
