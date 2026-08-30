//! Compatibility facade for the original Dart/Flutter signal API.
//!
//! The facade preserves the existing numeric signal IDs and JSON payloads,
//! while carrying them through the versioned, language-neutral envelope.
//! Responses continue to use the registered legacy callback.

use minicbor::{Decoder, Encoder};
use monero_ffi_protocol::{
    HandlerResponse, LEGACY_SIGNAL_PROTOCOL, ProtocolHandler, Version, WireEnvelope,
};

pub const SEND: &str = "signal.send";

#[derive(Default)]
pub struct LegacySignalFacade;

impl LegacySignalFacade {
    pub const fn new() -> Self {
        Self
    }
}

impl ProtocolHandler for LegacySignalFacade {
    fn protocol(&self) -> &'static str {
        LEGACY_SIGNAL_PROTOCOL
    }

    fn version(&self) -> Version {
        Version::new(1, 0)
    }

    fn handle(&mut self, command: &WireEnvelope) -> Result<HandlerResponse, String> {
        if command.wallet.is_some() {
            return Err("legacy signal commands must not include a wallet handle".to_string());
        }
        if command.operation != SEND {
            return Err(format!(
                "unsupported legacy signal operation {}",
                command.operation
            ));
        }

        let (signal_id, json) = decode_send(&command.body)?;
        crate::signal_ids::route_dart_signal(signal_id, json)?;
        Ok(HandlerResponse::default())
    }
}

/// Encode a stable `signal.send` body containing a numeric ID and JSON bytes.
pub fn encode_send(signal_id: u32, json: &[u8]) -> Result<Vec<u8>, String> {
    let mut body = Vec::new();
    let mut encoder = Encoder::new(&mut body);
    encoder.map(2).map_err(|error| error.to_string())?;
    encoder.u8(0).map_err(|error| error.to_string())?;
    encoder.u32(signal_id).map_err(|error| error.to_string())?;
    encoder.u8(1).map_err(|error| error.to_string())?;
    encoder.bytes(json).map_err(|error| error.to_string())?;
    Ok(body)
}

fn decode_send(body: &[u8]) -> Result<(u32, Vec<u8>), String> {
    let mut decoder = Decoder::new(body);
    let fields = decoder
        .map()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "legacy signal body must use a definite map".to_string())?;
    let mut signal_id = None;
    let mut json = None;
    for _ in 0..fields {
        match decoder.u8().map_err(|error| error.to_string())? {
            0 => {
                if signal_id
                    .replace(decoder.u32().map_err(|error| error.to_string())?)
                    .is_some()
                {
                    return Err("legacy signal body repeats signal ID".to_string());
                }
            }
            1 => {
                if json
                    .replace(decoder.bytes().map_err(|error| error.to_string())?.to_vec())
                    .is_some()
                {
                    return Err("legacy signal body repeats JSON payload".to_string());
                }
            }
            _ => decoder.skip().map_err(|error| error.to_string())?,
        }
    }
    if decoder.position() != body.len() {
        return Err("legacy signal body contains trailing data".to_string());
    }
    Ok((
        signal_id.ok_or_else(|| "legacy signal body is missing signal ID".to_string())?,
        json.ok_or_else(|| "legacy signal body is missing JSON payload".to_string())?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use monero_ffi_protocol::{RuntimeDispatcher, WireEnvelope};

    #[test]
    fn dispatcher_reports_unknown_legacy_signal() {
        let mut dispatcher = RuntimeDispatcher::new();
        dispatcher.mount(LegacySignalFacade::new()).unwrap();
        let command = WireEnvelope::command(
            1,
            None,
            LEGACY_SIGNAL_PROTOCOL,
            SEND,
            encode_send(u32::MAX, b"{}").unwrap(),
        );

        let error = dispatcher.dispatch_frame(command).unwrap_err();
        assert!(error.to_string().contains("unknown signal_id"));
    }

    #[test]
    fn send_body_round_trips_without_interpreting_json() {
        let json = br#"{"request":"unchanged"}"#;
        assert_eq!(
            decode_send(&encode_send(17, json).unwrap()).unwrap(),
            (17, json.to_vec())
        );
    }
}
