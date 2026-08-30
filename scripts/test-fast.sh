#!/bin/sh
# Quick development loop. The Dart, codegen, and wasm matrices are in test-all.sh.
set -eu

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

cargo fmt -p monero-ffi-protocol -p monero_wasm -- --check
cargo check --workspace --all-targets
cargo test -p monero-ffi-protocol
cargo test -p monero_wasm --lib
