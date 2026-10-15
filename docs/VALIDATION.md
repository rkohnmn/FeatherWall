# Validation evidence from source-generation environment

Date: October 15, 2026. Host: Debian Linux x64 (no Rust compiler, Cargo, MSVC, Windows SDK, Wine, or real Windows desktop available).

## Actually executed

- Examined installed toolchains; neither `rustc` nor `cargo` was installed; no Windows-compatible build was possible.
- Parsed `Cargo.toml` with Python's `tomllib`; package metadata was valid TOML.
- Performed static delimiter-balance checks on all nine authored Rust source files; brackets, braces and parentheses were balanced. This is **not a syntax/type/API check**.
- Used the available Linux FFmpeg/FFprobe executables to generate synthetic MP4, GIF and BMP test fixtures and execute representative native FFmpeg commands with the same argument order/filter choices as the importer. FFprobe reported recognizable H.264 MP4, GIF and BMP metadata. An animated GIF was converted to a silent H.264 MP4; a BMP was normalized to a still BMP. The FFmpeg processes returned success.
- Verified the source directory includes the declared Rust modules, release script, MIT license, build documentation and notices.

## Not executed

- `cargo fmt`, `cargo fmt --check`, `cargo check`, `cargo test`, `cargo clippy`, or `cargo build` (Rust toolchain missing).
- `featherwall.exe` and `fwctl.exe` creation or execution (Windows toolchain missing).
- Windows Explorer WorkerW attachment, MFPlay video output, hardware decoding, session/power lifecycle, multi-display monitor placement, tray interaction, or shutdown tests.
- Memory, CPU, GPU engine, power, frame drop or loop-gap benchmarks on Windows.

**Verdict:** deliverable is **uncompiled, statically reviewed source** plus a successful FFmpeg command-line smoke test. This is not a certified finished Windows build.
