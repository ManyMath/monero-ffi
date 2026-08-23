#!/bin/sh
# Run every repository-owned Rust, WASM, code-generation, and Dart tier.
set -eu

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FAILED=""

step() {
    echo
    echo "=== $1 ==="
}

record_failure() {
    FAILED="$FAILED
  - $1"
}

step "monero-rust dependency is checked out"
if [ ! -f "$ROOT/../monero-rust/rust/monero-rust/Cargo.toml" ]; then
    echo "The adjacent monero-rust checkout is missing." >&2
    record_failure "monero-rust dependency missing"
else
    echo "ok: $(git -C "$ROOT/../monero-rust" rev-parse --short HEAD)"
fi

step "Rust workspace tests"
if ! (cd "$ROOT" && cargo test --workspace --no-fail-fast); then
    record_failure "Rust workspace tests"
fi

step "wasm32 workspace build"
if ! (cd "$ROOT" && cargo check --workspace --target wasm32-unknown-unknown); then
    record_failure "wasm32 workspace build"
fi

step "signal generator tests"
if ! (cd "$ROOT/native/codegen" && cargo test); then
    record_failure "signal generator tests"
fi

step "generated signal surface is current"
if ! command -v dart >/dev/null 2>&1; then
    echo "dart not on PATH - skipping the generated-surface check" >&2
elif ! command -v rustfmt >/dev/null 2>&1; then
    echo "rustfmt not on PATH - skipping the generated-surface check" >&2
elif ! (cd "$ROOT/native/codegen" && cargo run -- --check); then
    record_failure "generated signal surface"
fi

step "Dart analysis and tests"
if ! command -v dart >/dev/null 2>&1; then
    echo "dart not on PATH - skipping Dart analysis and tests" >&2
else
    if ! (cd "$ROOT" && dart pub get && dart analyze --no-fatal-warnings); then
        record_failure "Dart analysis"
    fi
    if ! (cd "$ROOT" && dart test); then
        record_failure "Dart tests"
    fi
fi

echo
if [ -n "$FAILED" ]; then
    echo "FAILED:$FAILED"
    exit 1
fi
echo "all tiers passed"
