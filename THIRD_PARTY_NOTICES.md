# Third-party notices and distribution dependencies

FeatherWall's Rust source currently declares **zero external Rust crates**; it uses the Rust standard library and Windows-provided native APIs. This does not eliminate OS and toolchain prerequisites.

**FFmpeg / FFprobe:** Required for media import, not for optimized runtime video playback. The binaries are not included. FFmpeg licensing depends on build configuration and enabled codecs. Builds that contain libx264 often carry GPL obligations. Redistributors must obtain appropriate licenses and comply with FFmpeg, its dependencies, and relevant codec patent obligations; check https://ffmpeg.org/legal.html . Do not imply a bundled FFmpeg executable is part of this release.

**Windows Media Foundation, MFPlay, Win32 and Explorer:** Supplied by Microsoft Windows. MFPlay is a Microsoft legacy Media Foundation component; availability and behavior are dependent on the Windows installation and codec support. Windows 'N' editions may require additional media components.

**Rust:** The program is compiled using the Rust toolchain; the toolchain itself is not distributed with this source archive.
