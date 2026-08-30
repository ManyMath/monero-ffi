//! Stateful wallet2-style facade over `monero-rust` domain APIs.
//!
//! The envelope and body formats contain no Rust layout or language-specific
//! representation. C, C++, Java, Dart, and TypeScript adapters can all issue
//! the same commands through their native byte transport.

use std::collections::BTreeMap;

use minicbor::{Decoder, Encoder};
use monero_ffi_protocol::{
    HandlerResponse, ProtocolHandler, Version, WALLET2_PROTOCOL, WireEnvelope,
};

pub const OPEN_SEED: &str = "wallet.open_seed";
pub const GET_PRIMARY_ADDRESS: &str = "wallet.get_primary_address";
pub const GET_PUBLIC_KEYS: &str = "wallet.get_public_keys";
pub const CLOSE: &str = "wallet.close";

pub struct Wallet2Facade {
    next_wallet: u64,
    wallets: BTreeMap<u64, monero_rust::DerivedKeys>,
}

impl Default for Wallet2Facade {
    fn default() -> Self {
        Self {
            next_wallet: 1,
            wallets: BTreeMap::new(),
        }
    }
}

impl Wallet2Facade {
    pub fn new() -> Self {
        Self::default()
    }

    fn open_seed(&mut self, command: &WireEnvelope) -> Result<HandlerResponse, String> {
        if command.wallet.is_some() {
            return Err("wallet.open_seed must not include a wallet handle".to_string());
        }
        let request = OpenSeedRequest::decode(&command.body)?;
        let keys = monero_rust::derive_keys(&request.seed, &request.network, &request.passphrase)?;
        let address = keys.address.to_string();
        let wallet = self.next_wallet;
        self.next_wallet = self
            .next_wallet
            .checked_add(1)
            .ok_or_else(|| "wallet handle space exhausted".to_string())?;
        self.wallets.insert(wallet, keys);
        Ok(HandlerResponse {
            wallet: Some(wallet),
            body: encode_address(&address)?,
            extensions: BTreeMap::new(),
        })
    }

    fn wallet(&self, command: &WireEnvelope) -> Result<&monero_rust::DerivedKeys, String> {
        let handle = command
            .wallet
            .ok_or_else(|| format!("{} requires a wallet handle", command.operation))?;
        self.wallets
            .get(&handle)
            .ok_or_else(|| format!("unknown wallet handle {handle}"))
    }
}

impl ProtocolHandler for Wallet2Facade {
    fn protocol(&self) -> &'static str {
        WALLET2_PROTOCOL
    }

    fn version(&self) -> Version {
        Version::new(1, 0)
    }

    fn handle(&mut self, command: &WireEnvelope) -> Result<HandlerResponse, String> {
        match command.operation.as_str() {
            OPEN_SEED => self.open_seed(command),
            GET_PRIMARY_ADDRESS => {
                let address = self.wallet(command)?.address.to_string();
                Ok(HandlerResponse::body(encode_address(&address)?))
            }
            GET_PUBLIC_KEYS => {
                let keys = self.wallet(command)?;
                Ok(HandlerResponse::body(encode_public_keys(
                    &keys.public_spend_key,
                    &keys.public_view_key,
                )?))
            }
            CLOSE => {
                let handle = command
                    .wallet
                    .ok_or_else(|| "wallet.close requires a wallet handle".to_string())?;
                self.wallets
                    .remove(&handle)
                    .ok_or_else(|| format!("unknown wallet handle {handle}"))?;
                Ok(HandlerResponse::default())
            }
            operation => Err(format!("unsupported wallet2 operation {operation}")),
        }
    }
}

struct OpenSeedRequest {
    seed: String,
    network: String,
    passphrase: String,
}

impl OpenSeedRequest {
    fn decode(body: &[u8]) -> Result<Self, String> {
        let mut decoder = Decoder::new(body);
        let fields = decoder
            .map()
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "open-seed body must use a definite map".to_string())?;
        let mut seed = None;
        let mut network = None;
        let mut passphrase = String::new();
        for _ in 0..fields {
            match decoder.u8().map_err(|error| error.to_string())? {
                0 => seed = Some(decoder.str().map_err(|error| error.to_string())?.to_owned()),
                1 => network = Some(decoder.str().map_err(|error| error.to_string())?.to_owned()),
                2 => passphrase = decoder.str().map_err(|error| error.to_string())?.to_owned(),
                _ => decoder.skip().map_err(|error| error.to_string())?,
            }
        }
        if decoder.position() != body.len() {
            return Err("open-seed body contains trailing data".to_string());
        }
        Ok(Self {
            seed: seed.ok_or_else(|| "open-seed body is missing seed".to_string())?,
            network: network.ok_or_else(|| "open-seed body is missing network".to_string())?,
            passphrase,
        })
    }
}

/// Encode the stable wallet2 `wallet.open_seed` request body.
pub fn encode_open_seed(seed: &str, network: &str, passphrase: &str) -> Result<Vec<u8>, String> {
    let mut body = Vec::new();
    let mut encoder = Encoder::new(&mut body);
    encoder.map(3).map_err(|error| error.to_string())?;
    encoder.u8(0).map_err(|error| error.to_string())?;
    encoder.str(seed).map_err(|error| error.to_string())?;
    encoder.u8(1).map_err(|error| error.to_string())?;
    encoder.str(network).map_err(|error| error.to_string())?;
    encoder.u8(2).map_err(|error| error.to_string())?;
    encoder.str(passphrase).map_err(|error| error.to_string())?;
    Ok(body)
}

fn encode_address(address: &str) -> Result<Vec<u8>, String> {
    let mut body = Vec::new();
    let mut encoder = Encoder::new(&mut body);
    encoder.map(1).map_err(|error| error.to_string())?;
    encoder.u8(0).map_err(|error| error.to_string())?;
    encoder.str(address).map_err(|error| error.to_string())?;
    Ok(body)
}

fn encode_public_keys(spend: &[u8; 32], view: &[u8; 32]) -> Result<Vec<u8>, String> {
    let mut body = Vec::new();
    let mut encoder = Encoder::new(&mut body);
    encoder.map(2).map_err(|error| error.to_string())?;
    encoder.u8(0).map_err(|error| error.to_string())?;
    encoder.bytes(spend).map_err(|error| error.to_string())?;
    encoder.u8(1).map_err(|error| error.to_string())?;
    encoder.bytes(view).map_err(|error| error.to_string())?;
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use monero_ffi_protocol::{FrameKind, RuntimeDispatcher, WireEnvelope};

    const SEED: &str = "honked bagpipe alpine juicy faked afoot jostle claim cowl tunnel orphans negative pheasants feast jetting quote frown teeming cycling tribal womanly hills cottage daytime daytime";

    #[test]
    fn wallet2_commands_share_state_behind_the_dispatcher() {
        let mut dispatcher = RuntimeDispatcher::new();
        dispatcher.mount(Wallet2Facade::new()).unwrap();

        let open = WireEnvelope::command(
            1,
            None,
            WALLET2_PROTOCOL,
            OPEN_SEED,
            encode_open_seed(SEED, "stagenet", "").unwrap(),
        );
        let opened = dispatcher.dispatch_frame(open).unwrap();
        assert_eq!(opened.kind, FrameKind::Completion);
        let wallet = opened.wallet.expect("open returns a wallet handle");

        let address = WireEnvelope::command(
            2,
            Some(wallet),
            WALLET2_PROTOCOL,
            GET_PRIMARY_ADDRESS,
            Vec::new(),
        );
        let address = dispatcher.dispatch_frame(address).unwrap();
        let mut decoder = Decoder::new(&address.body);
        assert_eq!(decoder.map().unwrap(), Some(1));
        assert_eq!(decoder.u8().unwrap(), 0);
        assert_eq!(
            decoder.str().unwrap(),
            "58aWiYGUeqZc5idYcx31rYR58K1EVsCYkN6thrZppU1MGqMowPh1BYy4frVWH5RjGLPWthZy9sRGm5ZC4fgX44HUCmqtGUf"
        );

        let close = WireEnvelope::command(3, Some(wallet), WALLET2_PROTOCOL, CLOSE, Vec::new());
        dispatcher.dispatch_frame(close).unwrap();
        let missing = WireEnvelope::command(
            4,
            Some(wallet),
            WALLET2_PROTOCOL,
            GET_PRIMARY_ADDRESS,
            Vec::new(),
        );
        assert!(dispatcher.dispatch_frame(missing).is_err());
    }
}
