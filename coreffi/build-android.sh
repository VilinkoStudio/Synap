#!/usr/bin/env bash

set -e

echo "Building synap-coreffi for Android..."

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
OUTPUT_DIR="$REPO_ROOT/android/app/src/main/jniLibs"
BINDINGS_DIR="$REPO_ROOT/android/app/build/generated/source/uniffi/coreffi/kotlin"

rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR"

cargo ndk \
  -o "$OUTPUT_DIR" \
  -t arm64-v8a \
  -t armeabi-v7a \
  -t x86 \
  -t x86_64 \
  build \
  --release \
  -p synap-coreffi

echo "Build complete! Libraries are in $OUTPUT_DIR"
ls -la "$OUTPUT_DIR"/*/libuniffi_synap_coreffi.so

echo "Generating Kotlin bindings..."
rm -rf "$BINDINGS_DIR"
mkdir -p "$BINDINGS_DIR"

cargo run -p xtask -- gen-uniffi-kotlin \
  --udl "$REPO_ROOT/coreffi-shared/src/synap.udl" \
  --config "$SCRIPT_DIR/uniffi.toml" \
  --out-dir "$BINDINGS_DIR" \
  --crate-name uniffi_synap_coreffi

echo "Kotlin bindings generated in $BINDINGS_DIR"
ls -la "$BINDINGS_DIR"

echo "Verifying exported symbols..."
nm -D "$OUTPUT_DIR/arm64-v8a/libuniffi_synap_coreffi.so" | grep -c "ffi_synap_coreffi" || echo "Warning: No symbols found!"
