# Building and packaging FeatherWall

## Supported target

The intended primary target is **Windows 10 version 1607 x64 or newer, including Windows 11 x64**. Windows 1607 is the conservative minimum chosen for this source's user/session/power APIs; **it has not been compatibility-tested on that build**. MFPlay itself was introduced earlier, but the undocumented WorkerW placement technique is not a contractual Windows API and requires runtime verification on each shell build.

Build on a real Windows system using current stable MSVC Rust with the Visual Studio C++ toolchain and Windows SDK. The crate intentionally declares no third-party runtime Rust dependency. It uses the `mfplay.lib`, `user32.lib`, `gdi32.lib`, `kernel32.lib`, `shell32.lib`, `comdlg32.lib`, `advapi32.lib`, `wtsapi32.lib`, `ole32.lib`, `shcore.lib` and `psapi.lib` system import libraries provided by the SDK/MSVC build environment.

```powershell
rustup target add x86_64-pc-windows-msvc
cargo fmt --all
cargo fmt --all --check
cargo check --workspace --target x86_64-pc-windows-msvc
cargo test --workspace --target x86_64-pc-windows-msvc
cargo clippy --workspace --all-targets --target x86_64-pc-windows-msvc
cargo build --release --target x86_64-pc-windows-msvc
```

In Visual Studio Developer PowerShell, run `scripts\release.ps1` **only after compiler errors, if any, have been repaired and all checks pass**. The script builds both binaries and a portable package. FFmpeg/FFprobe remain external dependencies of the import path.

## FFmpeg tools

Choose trustworthy FFmpeg binaries appropriate to your license obligations. In PowerShell you may set:

```powershell
$env:FEATHERWALL_FFMPEG="C:\Utilities\ffmpeg\bin\ffmpeg.exe"
$env:FEATHERWALL_FFPROBE="C:\Utilities\ffmpeg\bin\ffprobe.exe"
```

Alternatively place the executables beside FeatherWall, or add their directory to `PATH`. Normal wallpaper playback requires only the preprocessed cache, not these binaries. GIF alpha is flattened to opaque H.264; compositing is currently FFmpeg-dependent and should be visually inspected.

## Reproducibility and verification

The source was authored in a **Linux environment without `rustc`, Cargo, MSVC, a Windows SDK, or Windows graphics hardware**. None of the Windows compilation, unit tests, Clippy checks, Win32 runtime acceptance tests, GPU assertions, battery-policy tests or performance benchmarks have been executed during source generation. A minimal dependency-free `Cargo.lock` is included for source completeness, but it was not generated or verified by Cargo; Cargo may update it on the first successful build. Reproduce all build and test commands on Windows; do not treat project presence as a successful build.

Until an actual Windows run passes, the output is a source release intended for review and validation, not a trusted production binary.
