//! Language-neutral messages shared by every `monero-ffi` transport.
//!
//! The eventual C, Dart, Java, JavaScript, and C++ facades exchange copies of
//! these envelopes. No Rust layout is part of the public ABI.

use std::collections::BTreeMap;
use std::fmt;

use minicbor::{Decoder, Encoder};

pub const ABI_MAJOR: u16 = 1;
pub const ABI_MINOR: u16 = 0;
pub const MAX_BODY_LEN: usize = 16 * 1024 * 1024;
pub const MAX_EXTENSIONS: usize = 64;
pub const MAX_NAME_LEN: usize = 128;

pub const RUNTIME_PROTOCOL: &str = "org.sneurlax.monero.runtime";
pub const WALLET_PROTOCOL: &str = "org.sneurlax.monero.wallet";
pub const LEGACY_SIGNAL_PROTOCOL: &str = "org.sneurlax.monero-extension.signals";
pub const WALLET2_PROTOCOL: &str = "org.getmonero.wallet2.facade";

const FIELD_WIRE: u8 = 0;
const FIELD_KIND: u8 = 1;
const FIELD_ID: u8 = 2;
const FIELD_WALLET: u8 = 3;
const FIELD_PROTOCOL: u8 = 4;
const FIELD_PROTOCOL_VERSION: u8 = 5;
const FIELD_OPERATION: u8 = 6;
const FIELD_BODY: u8 = 7;
const FIELD_EXTENSIONS: u8 = 8;
const FIELD_COUNT: u64 = 9;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Version {
    pub major: u16,
    pub minor: u16,
}

impl Version {
    pub const CURRENT: Self = Self {
        major: ABI_MAJOR,
        minor: ABI_MINOR,
    };

    pub const fn new(major: u16, minor: u16) -> Self {
        Self { major, minor }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum FrameKind {
    Command = 0,
    Completion = 1,
    Event = 2,
    HostCall = 3,
    HostReply = 4,
}

impl TryFrom<u8> for FrameKind {
    type Error = ProtocolError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Command),
            1 => Ok(Self::Completion),
            2 => Ok(Self::Event),
            3 => Ok(Self::HostCall),
            4 => Ok(Self::HostReply),
            other => Err(ProtocolError::UnknownFrameKind(other)),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuiltinPort {
    Rpc,
    Storage,
    Signer,
    Callback,
}

impl BuiltinPort {
    pub const fn protocol_id(self) -> &'static str {
        match self {
            Self::Rpc => "org.sneurlax.monero.host.rpc",
            Self::Storage => "org.sneurlax.monero.host.storage",
            Self::Signer => "org.sneurlax.monero.host.signer",
            Self::Callback => "org.sneurlax.monero.host.callback",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WireEnvelope {
    pub wire: Version,
    pub kind: FrameKind,
    pub id: u64,
    pub wallet: Option<u64>,
    pub protocol: String,
    pub protocol_version: Version,
    pub operation: String,
    pub body: Vec<u8>,
    pub extensions: BTreeMap<String, Vec<u8>>,
}

impl WireEnvelope {
    pub fn command(
        id: u64,
        wallet: Option<u64>,
        protocol: impl Into<String>,
        operation: impl Into<String>,
        body: Vec<u8>,
    ) -> Self {
        Self {
            wire: Version::CURRENT,
            kind: FrameKind::Command,
            id,
            wallet,
            protocol: protocol.into(),
            protocol_version: Version::CURRENT,
            operation: operation.into(),
            body,
            extensions: BTreeMap::new(),
        }
    }

    pub fn host_call(
        id: u64,
        wallet: Option<u64>,
        port: BuiltinPort,
        operation: impl Into<String>,
        body: Vec<u8>,
    ) -> Self {
        let mut frame = Self::command(id, wallet, port.protocol_id(), operation, body);
        frame.kind = FrameKind::HostCall;
        frame
    }

    pub fn validate(&self) -> Result<(), ProtocolError> {
        if self.wire.major != ABI_MAJOR || self.wire.minor > ABI_MINOR {
            return Err(ProtocolError::IncompatibleVersion {
                received: self.wire,
                supported: Version::CURRENT,
            });
        }
        validate_name("protocol", &self.protocol)?;
        validate_name("operation", &self.operation)?;
        if self.body.len() > MAX_BODY_LEN {
            return Err(ProtocolError::BodyTooLarge(self.body.len()));
        }
        if self.extensions.len() > MAX_EXTENSIONS {
            return Err(ProtocolError::TooManyExtensions(self.extensions.len()));
        }
        let mut extension_bytes = 0usize;
        for name in self.extensions.keys() {
            validate_name("extension", name)?;
        }
        for value in self.extensions.values() {
            extension_bytes = extension_bytes
                .checked_add(value.len())
                .ok_or(ProtocolError::PayloadTooLarge(usize::MAX))?;
        }
        if extension_bytes > MAX_BODY_LEN {
            return Err(ProtocolError::PayloadTooLarge(extension_bytes));
        }
        Ok(())
    }

    pub fn encode(&self) -> Result<Vec<u8>, ProtocolError> {
        self.validate()?;

        let mut bytes = Vec::new();
        let mut encoder = Encoder::new(&mut bytes);
        encoder.map(FIELD_COUNT).map_err(encode_error)?;

        encoder.u8(FIELD_WIRE).map_err(encode_error)?;
        encoder.array(2).map_err(encode_error)?;
        encoder.u16(self.wire.major).map_err(encode_error)?;
        encoder.u16(self.wire.minor).map_err(encode_error)?;

        encoder.u8(FIELD_KIND).map_err(encode_error)?;
        encoder.u8(self.kind as u8).map_err(encode_error)?;

        encoder.u8(FIELD_ID).map_err(encode_error)?;
        encoder.u64(self.id).map_err(encode_error)?;

        encoder.u8(FIELD_WALLET).map_err(encode_error)?;
        match self.wallet {
            Some(wallet) => encoder.u64(wallet).map_err(encode_error)?,
            None => encoder.null().map_err(encode_error)?,
        };

        encoder.u8(FIELD_PROTOCOL).map_err(encode_error)?;
        encoder.str(&self.protocol).map_err(encode_error)?;

        encoder.u8(FIELD_PROTOCOL_VERSION).map_err(encode_error)?;
        encode_version(&mut encoder, self.protocol_version)?;

        encoder.u8(FIELD_OPERATION).map_err(encode_error)?;
        encoder.str(&self.operation).map_err(encode_error)?;

        encoder.u8(FIELD_BODY).map_err(encode_error)?;
        encoder.bytes(&self.body).map_err(encode_error)?;

        encoder.u8(FIELD_EXTENSIONS).map_err(encode_error)?;
        encoder
            .map(self.extensions.len() as u64)
            .map_err(encode_error)?;
        for (name, value) in &self.extensions {
            encoder.str(name).map_err(encode_error)?;
            encoder.bytes(value).map_err(encode_error)?;
        }

        Ok(bytes)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, ProtocolError> {
        let mut decoder = Decoder::new(bytes);
        let fields = definite_len(decoder.map().map_err(decode_error)?, "envelope")?;

        let mut wire = None;
        let mut kind = None;
        let mut id = None;
        let mut wallet = None;
        let mut protocol = None;
        let mut protocol_version = None;
        let mut operation = None;
        let mut body = None;
        let mut extensions = None;

        for _ in 0..fields {
            match decoder.u8().map_err(decode_error)? {
                FIELD_WIRE => {
                    let len = definite_len(decoder.array().map_err(decode_error)?, "wire")?;
                    if len != 2 {
                        return Err(ProtocolError::Malformed("wire must have two items"));
                    }
                    set_once(
                        &mut wire,
                        Version::new(
                            decoder.u16().map_err(decode_error)?,
                            decoder.u16().map_err(decode_error)?,
                        ),
                        "wire",
                    )?;
                }
                FIELD_KIND => {
                    let value = FrameKind::try_from(decoder.u8().map_err(decode_error)?)?;
                    set_once(&mut kind, value, "kind")?;
                }
                FIELD_ID => {
                    let value = decoder.u64().map_err(decode_error)?;
                    set_once(&mut id, value, "id")?;
                }
                FIELD_WALLET => {
                    let value = if decoder.datatype().map_err(decode_error)?
                        == minicbor::data::Type::Null
                    {
                        decoder.null().map_err(decode_error)?;
                        None
                    } else {
                        Some(decoder.u64().map_err(decode_error)?)
                    };
                    set_once(&mut wallet, value, "wallet")?;
                }
                FIELD_PROTOCOL => {
                    let value = decoder.str().map_err(decode_error)?.to_owned();
                    set_once(&mut protocol, value, "protocol")?;
                }
                FIELD_PROTOCOL_VERSION => {
                    let value = decode_version(&mut decoder, "protocol version")?;
                    set_once(&mut protocol_version, value, "protocol_version")?;
                }
                FIELD_OPERATION => {
                    let value = decoder.str().map_err(decode_error)?.to_owned();
                    set_once(&mut operation, value, "operation")?;
                }
                FIELD_BODY => {
                    let value = decoder.bytes().map_err(decode_error)?;
                    if value.len() > MAX_BODY_LEN {
                        return Err(ProtocolError::BodyTooLarge(value.len()));
                    }
                    set_once(&mut body, value.to_vec(), "body")?;
                }
                FIELD_EXTENSIONS => {
                    let len = definite_len(decoder.map().map_err(decode_error)?, "extensions")?;
                    if len > MAX_EXTENSIONS as u64 {
                        return Err(ProtocolError::TooManyExtensions(len as usize));
                    }
                    let mut decoded = BTreeMap::new();
                    let mut payload_len = 0usize;
                    for _ in 0..len {
                        let name = decoder.str().map_err(decode_error)?.to_owned();
                        let value = decoder.bytes().map_err(decode_error)?;
                        payload_len = payload_len
                            .checked_add(value.len())
                            .ok_or(ProtocolError::PayloadTooLarge(usize::MAX))?;
                        if payload_len > MAX_BODY_LEN {
                            return Err(ProtocolError::PayloadTooLarge(payload_len));
                        }
                        if decoded.insert(name, value.to_vec()).is_some() {
                            return Err(ProtocolError::DuplicateField("extension"));
                        }
                    }
                    set_once(&mut extensions, decoded, "extensions")?;
                }
                _ => decoder.skip().map_err(decode_error)?,
            }
        }

        if decoder.position() != bytes.len() {
            return Err(ProtocolError::TrailingData);
        }

        let frame = Self {
            wire: required(wire, "wire")?,
            kind: required(kind, "kind")?,
            id: required(id, "id")?,
            wallet: required(wallet, "wallet")?,
            protocol: required(protocol, "protocol")?,
            protocol_version: required(protocol_version, "protocol_version")?,
            operation: required(operation, "operation")?,
            body: required(body, "body")?,
            extensions: required(extensions, "extensions")?,
        };
        frame.validate()?;
        Ok(frame)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Capability {
    pub protocol: String,
    pub version: Version,
}

#[derive(Clone, Debug, Default)]
pub struct Dispatcher {
    capabilities: BTreeMap<String, Version>,
}

impl Dispatcher {
    pub fn new(capabilities: impl IntoIterator<Item = Capability>) -> Result<Self, ProtocolError> {
        let mut dispatcher = Self::default();
        for capability in capabilities {
            validate_name("protocol", &capability.protocol)?;
            dispatcher
                .capabilities
                .insert(capability.protocol, capability.version);
        }
        Ok(dispatcher)
    }

    pub fn accepts(&self, frame: &WireEnvelope) -> Result<(), ProtocolError> {
        frame.validate()?;
        let Some(version) = self.capabilities.get(&frame.protocol) else {
            return Err(ProtocolError::UnsupportedProtocol(frame.protocol.clone()));
        };
        if version.major != frame.protocol_version.major
            || version.minor < frame.protocol_version.minor
        {
            return Err(ProtocolError::IncompatibleVersion {
                received: frame.protocol_version,
                supported: *version,
            });
        }
        Ok(())
    }

    pub fn decode(&self, bytes: &[u8]) -> Result<WireEnvelope, ProtocolError> {
        let frame = WireEnvelope::decode(bytes)?;
        self.accepts(&frame)?;
        Ok(frame)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProtocolError {
    Encode(String),
    Decode(String),
    Malformed(&'static str),
    IndefiniteLength(&'static str),
    MissingField(&'static str),
    DuplicateField(&'static str),
    TrailingData,
    UnknownFrameKind(u8),
    IncompatibleVersion {
        received: Version,
        supported: Version,
    },
    InvalidName {
        field: &'static str,
        reason: &'static str,
    },
    BodyTooLarge(usize),
    PayloadTooLarge(usize),
    TooManyExtensions(usize),
    UnsupportedProtocol(String),
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ProtocolError {}

fn validate_name(field: &'static str, value: &str) -> Result<(), ProtocolError> {
    if value.is_empty() {
        return Err(ProtocolError::InvalidName {
            field,
            reason: "must not be empty",
        });
    }
    if value.len() > MAX_NAME_LEN {
        return Err(ProtocolError::InvalidName {
            field,
            reason: "is too long",
        });
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
    {
        return Err(ProtocolError::InvalidName {
            field,
            reason: "contains a non-portable character",
        });
    }
    Ok(())
}

fn definite_len(len: Option<u64>, field: &'static str) -> Result<u64, ProtocolError> {
    len.ok_or(ProtocolError::IndefiniteLength(field))
}

fn encode_version(
    encoder: &mut Encoder<&mut Vec<u8>>,
    version: Version,
) -> Result<(), ProtocolError> {
    encoder.array(2).map_err(encode_error)?;
    encoder.u16(version.major).map_err(encode_error)?;
    encoder.u16(version.minor).map_err(encode_error)?;
    Ok(())
}

fn decode_version(
    decoder: &mut Decoder<'_>,
    field: &'static str,
) -> Result<Version, ProtocolError> {
    let len = definite_len(decoder.array().map_err(decode_error)?, field)?;
    if len != 2 {
        return Err(ProtocolError::Malformed(
            "version must contain major and minor",
        ));
    }
    Ok(Version::new(
        decoder.u16().map_err(decode_error)?,
        decoder.u16().map_err(decode_error)?,
    ))
}

fn required<T>(value: Option<T>, field: &'static str) -> Result<T, ProtocolError> {
    value.ok_or(ProtocolError::MissingField(field))
}

fn set_once<T>(slot: &mut Option<T>, value: T, field: &'static str) -> Result<(), ProtocolError> {
    if slot.replace(value).is_some() {
        return Err(ProtocolError::DuplicateField(field));
    }
    Ok(())
}

fn encode_error<E: fmt::Display>(error: E) -> ProtocolError {
    ProtocolError::Encode(error.to_string())
}

fn decode_error(error: minicbor::decode::Error) -> ProtocolError {
    ProtocolError::Decode(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wallet_dispatcher() -> Dispatcher {
        Dispatcher::new([Capability {
            protocol: WALLET_PROTOCOL.to_owned(),
            version: Version::CURRENT,
        }])
        .unwrap()
    }

    #[test]
    fn command_round_trips_canonically() {
        let mut command =
            WireEnvelope::command(41, Some(7), WALLET_PROTOCOL, "wallet.create", vec![1, 2, 3]);
        command.extensions.insert("trace_id".into(), vec![9]);

        let encoded = command.encode().unwrap();
        assert_eq!(encoded[0], 0xa9, "the envelope is a nine-entry map");
        let decoded = WireEnvelope::decode(&encoded).unwrap();
        assert_eq!(decoded, command);
        assert_eq!(decoded.encode().unwrap(), encoded);
    }

    #[test]
    fn dispatcher_rejects_an_unmounted_protocol() {
        let frame = WireEnvelope::command(1, None, WALLET2_PROTOCOL, "wallet.open", vec![]);
        assert_eq!(
            wallet_dispatcher().accepts(&frame),
            Err(ProtocolError::UnsupportedProtocol(
                WALLET2_PROTOCOL.to_owned()
            ))
        );
    }

    #[test]
    fn newer_wire_versions_are_rejected() {
        let mut frame = WireEnvelope::command(1, None, WALLET_PROTOCOL, "wallet.open", vec![]);
        frame.wire = Version::new(ABI_MAJOR, ABI_MINOR + 1);
        assert!(matches!(
            frame.validate(),
            Err(ProtocolError::IncompatibleVersion { .. })
        ));
    }

    #[test]
    fn protocol_versions_are_negotiated_independently_from_the_wire() {
        let mut frame = WireEnvelope::command(1, None, WALLET_PROTOCOL, "wallet.open", vec![]);
        frame.protocol_version = Version::new(2, 0);
        assert!(matches!(
            wallet_dispatcher().accepts(&frame),
            Err(ProtocolError::IncompatibleVersion {
                received: Version { major: 2, minor: 0 },
                supported: Version { major: 1, minor: 0 },
            })
        ));
    }

    #[test]
    fn trailing_bytes_are_rejected() {
        let frame = WireEnvelope::command(1, None, WALLET_PROTOCOL, "wallet.open", vec![]);
        let mut bytes = frame.encode().unwrap();
        bytes.push(0);
        assert_eq!(
            WireEnvelope::decode(&bytes),
            Err(ProtocolError::TrailingData)
        );
    }

    #[test]
    fn duplicate_fields_are_rejected() {
        let frame = WireEnvelope::command(1, None, WALLET_PROTOCOL, "wallet.open", vec![]);
        let mut bytes = frame.encode().unwrap();
        bytes[0] = 0xaa;
        bytes.extend([FIELD_ID, 2]);
        assert_eq!(
            WireEnvelope::decode(&bytes),
            Err(ProtocolError::DuplicateField("id"))
        );
    }

    #[test]
    fn host_calls_use_explicit_port_protocols() {
        let frame = WireEnvelope::host_call(9, Some(2), BuiltinPort::Rpc, "request", vec![0x80]);
        assert_eq!(frame.kind, FrameKind::HostCall);
        assert_eq!(frame.protocol, "org.sneurlax.monero.host.rpc");
    }

    #[test]
    fn names_are_bounded_and_language_neutral() {
        let frame = WireEnvelope::command(1, None, WALLET_PROTOCOL, "wallet/create", vec![]);
        assert!(matches!(
            frame.validate(),
            Err(ProtocolError::InvalidName {
                field: "operation",
                ..
            })
        ));
    }
}
