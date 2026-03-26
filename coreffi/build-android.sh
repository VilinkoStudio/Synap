#!/bin/bash

set -e

echo "Building synap-coreffi for Android..."

OUTPUT_DIR="../android/app/src/main/jniLibs"
rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR"

cargo ndk -o "$OUTPUT_DIR" -t arm64-v8a -t armeabi-v7a -t x86 -t x86_64 build --release

echo "Build complete! Libraries are in $OUTPUT_DIR"
# 改这里：文件名现在是 libuniffi_synap_coreffi.so
ls -la "$OUTPUT_DIR"/*/libuniffi_synap_coreffi.so

echo "Generating Kotlin bindings..."
BINDINGS_DIR="../android/app/src/main/java/com/fuwaki/synap/bindings"
rm -rf "$BINDINGS_DIR"
mkdir -p "$BINDINGS_DIR"

# 加上 --config
uniffi-bindgen generate src/synap.udl \
  --language kotlin \
  --config uniffi.toml \
  --out-dir "$BINDINGS_DIR"

echo "Fixing package names..."
find "$BINDINGS_DIR" -name "*.kt" -type f \
  -exec sed -i 's/^package uniffi\.synap_coreffi/package com.fuwaki.synap.bindings.uniffi.synap_coreffi/g' {} \;

echo "Kotlin bindings generated in $BINDINGS_DIR"
ls -la "$BINDINGS_DIR"

# 验证符号导出
echo "Verifying exported symbols..."
nm -D "$OUTPUT_DIR/arm64-v8a/libuniffi_synap_coreffi.so" | grep -c "ffi_synap_coreffi" || echo "Warning: No symbols found!"