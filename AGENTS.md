# Synap — Agent Instructions

## Project overview

Synap is a minimal thought-capture app. Rust workspace monorepo with native clients for Android (Kotlin), Linux (GTK4), Windows (C#/WinUI), Web (SvelteKit), and a CLI. Core logic is in Rust; platforms consume it via UniFFI FFI bindings.

## Build commands

```bash
# Rust workspace build (any crate)
cargo build -p synap-core
cargo build -p synap-cli          # CLI binary: target/release/synap
cargo build -p synap-desktop-linux # Linux desktop (needs GTK4/libadwaita dev libs)

# Run tests (workspace-wide)
cargo test

# Run a single crate's tests
cargo test -p synap-core

# Run a specific test by name
cargo test -p synap-core test_name_here

# Benchmarks (core only, uses divan)
cargo bench -p synap-core

# Web
pnpm --dir web install
pnpm --dir web dev           # dev server
pnpm --dir web check         # svelte-check type checking
pnpm --dir web prepare:bindings  # generate Node UniFFI bindings (runs xtask gen-uniffi-node)

# Android (requires JDK 17, Android SDK, NDK 28)
cd android && ./gradlew assembleDebug

# Relay
cargo run -p relay -- serve   # or: docker compose -f relay/docker-compose.example.yml up
```

## Workspace structure

| Crate | Role |
|---|---|
| `core` | Pure Rust logic: KV store (redb), DAG state machine, search, crypto, sync protocol. Platform-agnostic. |
| `corenet` | Network abstractions for discovery, channels, sync. |
| `coreffi-shared` | Single maintained UDL (`src/synap.udl`) + FFI-facing Rust facade. |
| `coreffi` | UniFFI **0.31** adapter. For Android/Kotlin + Node/Web. |
| `coreffi-uniffi029` | UniFFI **0.29** adapter. For C#/WinUI (bound to `uniffi-bindgen-cs v0.10.0`). |
| `cli` | CLI frontend (`synap` binary). |
| `desktop_linux` | Linux desktop (relm4 + GTK4 + libadwaita). |
| `relay` | Zero-trust sync relay server (axum + tokio). |
| `xtask` | Codegen tool: `gen-uniffi-kotlin`, `gen-uniffi-csharp`, `gen-uniffi-node`. |
| `android` | Kotlin app. Gradle `preBuild` compiles `coreffi` .so and runs `xtask gen-uniffi-kotlin`. |
| `web` | SvelteKit + Vite + UnoCSS. Loads `coreffi` cdylib via generated Node bindings + koffi. |

## UniFFI architecture (critical)

- **One UDL**: `coreffi-shared/src/synap.udl`. Never duplicate per platform.
- **One FFI facade**: `coreffi-shared/src/*.rs`. Both `coreffi` and `coreffi-uniffi029` pull these in via `#[path = "../../coreffi-shared/src/..."]`. This is intentional — UniFFI metadata resolution requires the types to belong to the exporting crate.
- **Version split only at adapter layer**: `coreffi` (0.31) and `coreffi-uniffi029` (0.29) are thin shells. Platform code picks the adapter matching its UniFFI ecosystem.
- `target/xtask/uniffi-input/...` is generated scratch space for xtask. Not source. Never commit.

## Conditional compilation

`core` gates almost everything behind `#[cfg(not(target_arch = "wasm32"))]`. Only `dto` and `version` modules compile for wasm32. When adding new core modules, decide early whether they need wasm support.

## Commit conventions

Conventional Commits format: `type(scope): summary`

Scope uses `platform/area` when applicable: `android/net`, `core/search`, `cli/config`, `web/auth`, `xtask/release`.

## Branch & release model

- `master` is always runnable. New work on `feat/*`, `refactor/*`, `spike/*`.
- Cross-platform features commit in layer order: `core/` → `coreffi-shared/` → `coreffi|coreffi-uniffi029/` → `android|desktop|cli|web/` → `docs|build`.
- A feature may land on platforms incrementally, but merging to `master` must not break existing platforms.
- Release tags are platform-prefixed: `android-vX.Y.Z`, `cli-vX.Y.Z`, `web-vX.Y.Z`, `desktop-linux-vX.Y.Z`, `relay-vX.Y.Z`. Pre-release: `alpha`, `beta`, `rc`.

## CI

Per-platform GitHub Actions in `.github/workflows/`. Each triggers on path changes to its own directories + `core/` + `Cargo.toml` + `Cargo.lock`. Builds on push/PR to `master`; releases on tag push.

## What not to commit

Generated build artifacts: `target/`, `android/app/src/main/jniLibs/`, `android/app/src/main/java/com/fuwaki/synap/bindings/uniffi/`, `desktop_windows/obj/`, `web/dist/`, `web/node_modules/`.
