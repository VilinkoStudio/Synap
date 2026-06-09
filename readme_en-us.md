# Synap

A minimalist app for rapid thought capture.

[简体中文 README](readme.md) | [English README](readme_en-us.md)（Now）

## Download & Experience

The software is currently in its early development stages. You can try out the development releases by visiting the project's [GitHub Releases](https://github.com/VilinkoStudio/Synap/releases) page.

[Official Website](https://vilinkostudio.github.io/synap.vilinko.com/)

## Inspiration

Traditional note-taking apps often force you to think before you write: "Which folder does this idea belong in?" This psychological burden of mandatory categorization often kills fleeting inspiration.

Therefore, the capture of inspiration and systematic organization should be completely separated. This software does not force you to build a massive knowledge base; it simply provides a pure interface to faithfully record the flow, divergence, and convergence of your consciousness. It is as clean and lightweight as a system memo, yet possesses thought-evolution capabilities, powerful search, and browsing functions far beyond it, making future retrieval and organization effortless.

## Core Experience

* Rapid Thought Capture

  Unlike the mandatory "tree directories" and "categorical tags" of traditional notes, Synap returns to the essence of recording. Open, write down, and leave—no redundant actions required. This reserves your cognitive bandwidth entirely for thinking, while the software handles the connection and structuring of your ideas.

* Preserve Complete Thought Context and Causality

  Break the barriers of fragmented recording and let your ideas connect with one another. You can clearly see how a rough thought deduces step-by-step to reach a final conclusion. It preserves the most complete contextual information for your future reviews and recollections.

* Fully Local Data Storage for Security

  The software utilizes an extremely compact, single-file local database. Using plain text for data records (compatible with Markdown), thousands of notes require only about 1MB of space, and it supports both compression and encryption.

* Pure Native Development for Silky Smoothness

  We are tired of "web-wrapper" apps that take up hundreds of megabytes and require long waiting times to launch. Synap insists on pure native development, ensuring silky smooth animations and extremely low response times. Add a desktop widget to start recording with a single click, without any latency.

* Seamless Aggregation of Cross-Platform Ideas

  Thanks to the software's unique underlying evolution-tracking architecture, there's no need to worry about data conflicts or loss across multiple devices. No matter when or where, as soon as your devices reconnect, notes from all your devices will merge seamlessly.

## Engineering Architecture

This project adopts a Monorepo organizational structure. The current Rust workspace members are `core`, `corenet`, `coreffi-shared`, `coreffi`, `coreffi-uniffi029`, `cli`, `desktop_linux`, `relay`, and `xtask`; `android` and `web` are managed by Gradle and pnpm/Vite, respectively.

* `core/`: The Rust logical kernel. It is responsible for pure Rust KV data persistence, immutable DAG state machine maintenance, read-time filtering/rendering algorithms, and the synchronization protocol.
* `corenet/`: Cross-device synchronization network layer. Handles the network abstraction for discovery, channels, and sync services.
* `coreffi-shared/`: UniFFI shared FFI facade. This contains the solely maintained `src/synap.udl`, as well as FFI-facing Rust source codes like `error`, `types`, `service`, and `adapter`.
* `coreffi/`: UniFFI `0.31.0` adapter crate. Targets Android/Kotlin and Node/Web, exporting `uniffi_synap_coreffi`.
* `coreffi-uniffi029/`: UniFFI `0.29.4` adapter crate. Targets C# / WinUI and other consumers pinned by the UniFFI 0.29 ecosystem, exporting `uniffi_synap_coreffi_uniffi029`.
* `cli/`: Rust command-line frontend. Provides terminal-only entry points for capture, search, graph visualization, and synchronization.
* `desktop_linux/`: The current Linux desktop implementation. Built with Rust + GTK4 + libadwaita, and released exclusively for the Linux platform.
* `relay/`: Zero-trust synchronization relay service.
* `android/`: Kotlin native application. Gradle compiles `coreffi` during the build phase and integrates the auto-generated UniFFI Kotlin bindings.
* `xtask/`: Rust tooling target. Responsible for generating Kotlin, C#, and Node bindings from `coreffi-shared/src/synap.udl`, and creating temporary input crates dedicated to version adapters under `target/xtask/uniffi-input/...`.
* `web/`: SvelteKit + Vite frontend. Loads the `coreffi` dynamic library via the generated Node UniFFI bindings.

UniFFI Structural Conventions:

* `coreffi-shared/src/synap.udl` is the only maintained UDL. Do not duplicate the UDL for different platforms.
* `coreffi-shared/src/*.rs` contains the exclusively maintained FFI-facing Rust facade source code. Both `coreffi` and `coreffi-uniffi029` use `#[path = "../../coreffi-shared/src/..."]` to include these modules in their respective adapter crates. This approach is intentionally preserved: types generated by UniFFI must belong to the current exported crate; standard dependency re-exports would break binding metadata parsing.
* Different UniFFI versions only fork at the adapter crate level. Platform code selects its compatible adapter without needing to maintain a second UDL or a secondary business FFI layer.
* `target/xtask/uniffi-input/...` holds generation tool inputs, not source code. It is used to let the UniFFI/Node generator parse metadata by adapter crate name and should not be committed to version control.

Current Platform Strategy:

* Linux: `desktop_linux` is provided as the only actively maintained desktop implementation at present.
* Windows: The native desktop client utilizes a shared Rust core via C# + WinUI + UniFFI. Because `uniffi-bindgen-cs v0.10.0` corresponds to UniFFI `0.29.4`, Windows currently uses `coreffi-uniffi029`, though the UDL and FFI facade are still sourced from `coreffi-shared`.
* Apple Ecosystem: There are currently no plans to support macOS, iOS, or other Apple platforms, nor are there corresponding release schedules.

## Build and Run

Ensure you have the Rust toolchain installed. If you need to build for Android, a valid JDK, Android SDK, and NDK are also required.

Navigate to the workspace root directory to compile the CLI tool:

cargo build --release -p synap-cli
./target/release/synap --help


Run the desktop client:

cargo run -p synap-desktop-linux


Build the Android debug APK:

cd android
./gradlew assembleDebug


Android's `preBuild` phase will execute two things first:

* Compile the Android dynamic library `libuniffi_synap_coreffi.so` corresponding to `coreffi`.
* Generate Kotlin UniFFI bindings to `android/app/build/generated/...` via `cargo run -p xtask -- gen-uniffi-kotlin --udl coreffi-shared/src/synap.udl --config coreffi/uniffi.toml ...`.

Generate Web/Node bindings:

pnpm --dir web prepare:bindings


This command invokes `xtask gen-uniffi-node` to build the `synap-coreffi` cdylib, generates `target/generated/nodejs/synap-coreffi`, and copies the current platform's dynamic library. The Web server manually calls `load()` on this dynamic library via the generated ESM package.

Start the Web application:

pnpm --dir web dev


Check Web typings:

pnpm --dir web check


Use the 0.29 adapter when generating C# bindings:

cargo run -p xtask -- gen-uniffi-csharp \
  --udl coreffi-shared/src/synap.udl \
  --config coreffi-uniffi029/uniffi.toml \
  --out-dir desktop_windows/obj/Generated/UniFFI \
  --crate-name uniffi_synap_coreffi_uniffi029


## Repository Conventions

This repository is a monorepo for multiple platforms, but it does not require every feature to be adapted for all platforms at once. The main branch maintains the "shared core + currently maintained platform collection," not an "all shell layers synced forever" state. Conventions are as follows:

* Do not perform long-term development directly on `master`. New features, refactoring, and experiments should uniformly start from `feat/*`, `refactor/*`, or `spike/*` branches; `master` only keeps runnable and reversible commits.
* A cross-platform feature should have its commits split by "layers" rather than "what was changed today." The recommended sequence is `core/` -> `coreffi-shared/` -> `coreffi|coreffi-uniffi029/` -> `android|desktop|cli|web/` -> `docs|build`.
* A feature can be implemented platform by platform, but commits merged into `master` should not break the platforms involved in that iteration.
* Local WIP (Work In Progress) commits are allowed, but they must be squashed into a clear, reversible commit chain before being merged into the main branch.
* Build artifacts, local databases, generated UniFFI bindings, `jniLibs`, and other local outputs do not enter version control; they should be regenerated by the build process.
* For long-cycle parallel development, prioritize using `git worktree` rather than stacking everything in a single workspace.

A typical cross-platform feature is recommended to progress like this:

git switch -c feat/note-tag-flow
git add core coreffi
git commit -m "feat(core): add note tag service flow"
git add android/app/src/main/java android/app/src/main/AndroidManifest.xml android/app/build.gradle.kts
git commit -m "feat(android): wire note tag flow into app"


If a feature is going to take several days, it is safer to open an additional worktree:

git worktree add ../synap-note-tag feat/note-tag-flow