# monero-ffi

Language and ABI adapters for `monero-rust`.

This repository currently preserves the API used by the Monero Flutter extension on native
and WebAssembly targets. Its long-term stable boundary is a small versioned command/event
dispatcher with explicit host ports for RPC, storage, and hardware integrations. The current
signal API and future wallet2-compatible APIs are compatibility facades over that boundary.

The typed Monero implementation belongs in the adjacent `monero-rust` repository. Application
UI and browser-extension behavior belong in `monero-extension`.

## Current source layout

- `native/monero_wasm`: existing native C and WebAssembly signal runtime
- `native/monero_ffi_protocol`: versioned CBOR envelopes and host-port identifiers
- `native/codegen`: generator for the Rust/Dart signal surface
- `lib/src/ffi`: existing Dart signal facade
- `lib/monero_ffi.dart`: platform-neutral Dart package entry point
- `lib/monero_ffi_web.dart`: Web Worker transport entry point
- `web/pkg/monero_wasm_worker.js`: Web Worker transport for the WASM runtime

The language-neutral envelope, native/WASM entry points, and currently mounted
wallet2 operations are specified in [`docs/protocol.md`](docs/protocol.md).

The extraction preserved the source history from `monero-extension`. Reorganization behind the
compatibility surface will happen incrementally so existing Flutter consumers remain usable.

## Development

The repositories are expected to be adjacent during development:

```text
workspace/
  monero-rust/
  monero-ffi/
  monero-cli/
```

Use the fast Rust adapter loop during routine development:

```sh
scripts/test-fast.sh
```

Run the authoritative Rust, wasm, generated-signal, and Dart matrix with:

```sh
scripts/test-all.sh
```
