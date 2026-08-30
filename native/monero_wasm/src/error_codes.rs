//! Compatibility error metadata for the existing Dart/Flutter signal protocol.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Network {
    Mainnet,
    Testnet,
    Stagenet,
}

impl Network {
    pub(crate) fn parse(value: &str) -> Result<Self, ErrorResponse> {
        match value.to_lowercase().as_str() {
            "mainnet" => Ok(Self::Mainnet),
            "testnet" => Ok(Self::Testnet),
            "stagenet" => Ok(Self::Stagenet),
            _ => Err(ErrorResponse::new(
                ERR_INVALID_NETWORK,
                format!(
                    "Invalid network: '{}'. Expected mainnet, testnet, or stagenet",
                    value
                ),
            )),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Mainnet => "mainnet",
            Self::Testnet => "testnet",
            Self::Stagenet => "stagenet",
        }
    }
}

impl From<Network> for monero_rust::Network {
    fn from(network: Network) -> Self {
        match network {
            Network::Mainnet => Self::Mainnet,
            Network::Testnet => Self::Testnet,
            Network::Stagenet => Self::Stagenet,
        }
    }
}

pub(crate) fn parse_core_network(value: &str) -> Result<monero_rust::Network, String> {
    Network::parse(value)
        .map(Into::into)
        .map_err(|error| error.message)
}

pub(crate) fn validate_network(value: &str) -> Result<(), ErrorResponse> {
    Network::parse(value).map(|_| ())
}

pub(crate) fn validate_node_url(url: &str) -> Result<(), ErrorResponse> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err(ErrorResponse::new(ERR_INVALID_URL, "Node URL is empty")
            .with_hint("Provide a URL like http://127.0.0.1:18081"));
    }
    if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
        return Err(ErrorResponse::new(
            ERR_INVALID_URL,
            format!("Node URL has invalid scheme: '{}'", trimmed),
        )
        .with_hint("URL must start with http:// or https://"));
    }
    Ok(())
}

pub(crate) fn validate_node_url_for_network(
    url: &str,
    network: Network,
    allow_http: bool,
) -> Result<(), ErrorResponse> {
    validate_node_url(url)?;
    let is_http = url.trim().starts_with("http://");
    if is_http && network == Network::Mainnet && !allow_http {
        return Err(ErrorResponse::new(
            ERR_RPC_MAINNET_HTTP_NOT_ALLOWED,
            "Mainnet RPC connections require HTTPS",
        )
        .with_hint(
            "Use an https:// URL, or set allow_insecure_http: true for development testing",
        ));
    }
    if is_http {
        log::warn!(
            "Insecure HTTP connection for {} RPC ({})",
            network.as_str(),
            url.trim()
        );
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ErrorResponse {
    pub(crate) code: u32,
    pub(crate) message: String,
    pub(crate) hint: Option<String>,
    pub(crate) transient: bool,
}

impl ErrorResponse {
    pub(crate) fn new(code: u32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            hint: None,
            transient: false,
        }
    }

    pub(crate) fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub(crate) fn transient(mut self) -> Self {
        self.transient = true;
        self
    }

    /// Classify legacy string errors for the existing numeric signal protocol.
    pub(crate) fn from_string(message: &str) -> Self {
        let lower = message.to_lowercase();
        if lower.contains("invalid seed")
            || lower.contains("failed to parse seed")
            || lower.contains("invalid bip39")
            || lower.contains("failed to parse derived legacy seed")
        {
            return Self::new(ERR_SEED_INVALID, message)
                .with_hint("Double-check the seed phrase for typos");
        }
        if lower.contains("seed phrase is empty") {
            return Self::new(ERR_EMPTY_FIELD, message)
                .with_hint("Provide a non-empty seed phrase");
        }
        if lower.contains("unknown language") {
            return Self::new(ERR_SEED_UNKNOWN_LANGUAGE, message);
        }
        if lower.contains("invalid checksum") {
            return Self::new(ERR_SEED_INVALID_CHECKSUM, message)
                .with_hint("Double-check the seed phrase for typos");
        }
        if lower.contains("invalid seed length") || lower.contains("invalid word count") {
            return Self::new(ERR_SEED_INVALID_LENGTH, message)
                .with_hint("A classic seed is 25 words; a polyseed is 16 words");
        }
        if lower.contains("invalid network") {
            return Self::new(ERR_INVALID_NETWORK, message)
                .with_hint("Expected mainnet, testnet, or stagenet");
        }
        if lower.contains("address") && lower.contains("invalid") {
            return Self::new(ERR_INVALID_ADDRESS, message);
        }
        if lower.contains("different network") {
            return Self::new(ERR_ADDRESS_WRONG_NETWORK, message);
        }
        if lower.contains("connection")
            || lower.contains("fetch failed")
            || lower.contains("failed to get height")
            || lower.contains("failed to create rpc")
        {
            return Self::new(ERR_RPC_CONNECTION, message)
                .with_hint("Check your network connection and node URL")
                .transient();
        }
        if lower.contains("no spendable outputs")
            || lower.contains("no selected outputs")
            || lower.contains("no confirmed outputs")
        {
            return Self::new(ERR_NO_OUTPUTS, message);
        }
        if lower.contains("insufficient") {
            return Self::new(ERR_INSUFFICIENT_FUNDS, message);
        }
        if lower.contains("transaction") && (lower.contains("failed") || lower.contains("error")) {
            return Self::new(ERR_TX_BUILD_FAILED, message);
        }
        Self::new(ERR_INTERNAL, message)
    }
}

pub(crate) const ERR_EMPTY_FIELD: u32 = 1000;
pub(crate) const ERR_INVALID_NETWORK: u32 = 1001;
pub(crate) const ERR_INVALID_ADDRESS: u32 = 1002;
pub(crate) const ERR_ADDRESS_WRONG_NETWORK: u32 = 1003;
pub(crate) const ERR_INVALID_URL: u32 = 1004;
pub(crate) const ERR_NO_OUTPUTS: u32 = 1101;
pub(crate) const ERR_INSUFFICIENT_FUNDS: u32 = 1102;
pub(crate) const ERR_ENCRYPTION: u32 = 1103;
pub(crate) const ERR_DECRYPTION: u32 = 1104;
pub(crate) const ERR_TX_BUILD_FAILED: u32 = 1200;
pub(crate) const ERR_TX_DOUBLE_SPEND: u32 = 1202;
pub(crate) const ERR_RPC_CONNECTION: u32 = 2000;
pub(crate) const ERR_RPC_MAINNET_HTTP_NOT_ALLOWED: u32 = 2008;
pub(crate) const ERR_SEED_INVALID_LENGTH: u32 = 3000;
pub(crate) const ERR_SEED_INVALID_CHECKSUM: u32 = 3001;
pub(crate) const ERR_SEED_UNKNOWN_LANGUAGE: u32 = 3002;
pub(crate) const ERR_SEED_INVALID: u32 = 3003;
pub(crate) const ERR_INVALID_KEY: u32 = 3005;
pub(crate) const ERR_INTERNAL: u32 = 9000;
pub(crate) const ERR_SERIALIZATION: u32 = 9001;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_mainnet_http_policy_code() {
        let error = validate_node_url_for_network("http://node.example", Network::Mainnet, false)
            .unwrap_err();
        assert_eq!(error.code, ERR_RPC_MAINNET_HTTP_NOT_ALLOWED);
    }

    #[test]
    fn classifies_connection_errors_as_transient() {
        let error = ErrorResponse::from_string("connection refused");
        assert_eq!(error.code, ERR_RPC_CONNECTION);
        assert!(error.transient);
    }
}
