# Versioned FFI protocol

`monero-ffi-protocol` defines a canonical CBOR envelope. The envelope is copied
across an ABI boundary; no Rust struct layout, pointer, or enum representation
is part of the wire contract.

The envelope is a nine-field CBOR map whose integer keys are stable:

| Key | Value |
| --- | --- |
| 0 | wire version `[major, minor]` |
| 1 | frame kind: command `0`, completion `1`, event `2`, host call `3`, host reply `4` |
| 2 | caller-selected request ID |
| 3 | wallet handle or null |
| 4 | protocol identifier |
| 5 | protocol version `[major, minor]` |
| 6 | operation identifier |
| 7 | protocol-owned byte body |
| 8 | extension-name to byte-body map |

Wire and facade versions are negotiated independently. A runtime accepts the
same major version and a requested minor version no newer than its own. Unknown
protocols, duplicate fields, indefinite maps, oversized bodies, and trailing
data are rejected before a facade is invoked.

## Transports

Native callers use `hub_dispatch_envelope`. Its `HubDispatchResult` has status
zero and an encoded completion on success. A nonzero status carries a UTF-8
error. Both buffers must be released with `hub_free_bytes`.

WebAssembly callers use the exported `dispatch_envelope(Uint8Array)` function.
It returns the encoded completion or throws a JavaScript error. Both transports
retain facade state between calls.

The existing numeric Dart signal ABI remains supported. It is a compatibility
facade, not the format new language adapters should reproduce.

## wallet2 facade version 1.0

Protocol identifier: `org.getmonero.wallet2.facade`.

Body maps use small integer CBOR keys. Keys unknown to a given minor version may
be skipped. Secret and public key values are raw 32-byte strings, never hex.

### `wallet.open_seed`

The command has no wallet handle. Its body contains:

| Key | Type | Meaning |
| --- | --- | --- |
| 0 | text | mnemonic seed |
| 1 | text | `mainnet`, `testnet`, or `stagenet` |
| 2 | text | optional seed-offset passphrase |

The completion supplies a new wallet handle and body `{0: primary-address}`.

### `wallet.get_primary_address`

The command supplies a wallet handle and an empty body. The completion body is
`{0: primary-address}`.

### `wallet.get_public_keys`

The command supplies a wallet handle. The completion body is `{0: spend-key,
1: view-key}`, with both values encoded as 32-byte CBOR byte strings.

### `wallet.close`

The command supplies a wallet handle. It removes the facade session and returns
an empty completion body. Later use of that handle fails.

These initial operations intentionally prove the adapter boundary: session
state and wallet2 naming live in `monero-ffi`, while derivation and Monero types
come from `monero-rust`. Additional wallet2 operations extend this facade rather
than adding language-specific APIs to core.
