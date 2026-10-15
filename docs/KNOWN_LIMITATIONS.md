# Known limitations and unfinished acceptance criteria

1. **Windows build not verified**: no Windows/MSVC SDK, rustc or Cargo toolchain was available where the source was created. Compiler, Clippy, test, and runtime validation must be performed on an actual Windows system. There is no Windows executable in this release.
2. **MFPlay is legacy**: playback is native Windows Media Foundation/EVR; no explicit D3D11 device manager, decoder-selection proof, D3D11 texture sharing, GPU fence management, automatic device removal handling or verified zero-copy presentation is implemented.
3. **Loop boundaries**: MFPlay playback-end restarts through Stop+Play, which can create a brief discontinuity or black gap on some hardware. Seamless/frameless looping is **not guaranteed**; no first/last-frame diff or crossfade preparation is performed.
4. **Desktop placement is undocumented**: WorkerW hierarchies vary by Explorer build. A shell build that does not expose the expected WorkerW surface cannot attach. Explorer/taskbar notification triggers a recovery attempt, but unattended reliability has not been validated.
5. **Hardware acceleration, power and memory targets unverified**: Windows chooses decoder resources; metrics for private bytes, GPU decode utilization, render FPS and dropped frames are currently unavailable. The program never substitutes fictional statistics.
6. **Basic fullscreen heuristic**: foreground window bounding rectangles and a low-frequency fallback timer are used. This does not perfectly classify exclusive fullscreen, occlusion, virtual desktops or maximized windows in all cases.
7. **Single decoder per display**: even when multiple monitors use identical media, independent Media Foundation players are created. The memory/performance impact is unmeasured.
8. **Adaptive behavior is limited**: on AC/battery transitions, an existing Ultra Eco cache entry of the same source can be used; FeatherWall does not silently reencode in the background and does not implement GPU/CPU load-triggered adaptation.
9. **Import profiling is heuristic**: source FPS, pixel format, duration and codec are probed, but motion sampling, static-animation detection, first/last-frame loop suitability, decoder capabilities, bit depth/color management policy and explicit VFR timestamp analysis are limited or absent. A simple frame-rate discrepancy is used to flag possible VFR.
10. **Still and GIF transparency**: normalization to opaque BMP / H.264 can discard alpha; user-selectable compositing color is not implemented. Color handling and HDR compatibility need additional work.
11. **Scaling choices**: Fit and Fill are baked into newly optimized assets. Changing this setting for already cached media needs re-import/reoptimization. The UI exposes simple FPS and longest-edge overrides, but target dimensions and aspect ratio are still heuristic rather than fully manual.
12. **Missing UI sections**: there is no full metadata detail panel, no actual decoder info/GPU telemetry, no custom slider fields, and no persistent diagnostic history chart. The status line shows import/errors; logs are on disk.
13. **Failure behavior**: monitor changes and media errors are handled defensively but not all decoder/restart/graphics-device-loss combinations have been tested. Media failure is logged without an always-working fallback player.
14. **No macOS port**: Windows-focused source only; macOS background window hosting and AVFoundation are not implemented.
15. **Source hash**: cache naming uses two independent noncryptographic streams plus size. It is not suitable as a security proof against malicious collisions. Stored assets are re-probed before reuse.
16. **FFmpeg dependency**: imports require external FFmpeg/FFprobe with `libx264`. The archive is not self-contained and does not include licensed encoder binaries; video playback after import does not require FFmpeg.
17. **Installer and binaries**: a PowerShell portable-release script is provided, but no native installer or compiled binary was produced in this environment.

These are explicit incomplete/unverified items; the source archive should not be called a completed or certified Wallpaper Engine replacement.

---

## Desktop Widget Overlay: Integration Notes & Known Behaviors

1. **Local Spotify & Windows Media Session Detection**:
   - FeatherWall uses zero-dependency local inspection (`Chrome_WidgetWin_0` window class and Windows Media Transport Session query) rather than Spotify Web API OAuth.
   - If Spotify is completely closed or minimized with "Display desktop overlay when using media keys" turned off in Spotify settings, Spotify may not publish window titles. FeatherWall defensively displays an inactive state or a "▶ Open Spotify" action, without fabricating metadata or failing silently.
   - Hardware media key commands (`VK_MEDIA_PLAY_PAUSE`, `VK_MEDIA_NEXT_TRACK`, `VK_MEDIA_PREV_TRACK`) serve as a robust universal fallback across all local media players.

2. **Desktop Z-Order and WorkerW Hierarchy**:
   - Windows 10 and 11 Explorer builds expose varying internal window hierarchies (`Progman` vs `WorkerW` siblings).
   - FeatherWall's widget layer operates as a hardware-composited `WS_EX_LAYERED` window anchored to `HWND_BOTTOM` with `WS_EX_NOACTIVATE`. It floats above the wallpaper surface while ensuring normal application windows and taskbars stay on top.
   - Full click-through transparency is maintained via `WM_NCHITTEST` returning `HTTRANSPARENT` across all non-interactive background areas. Desktop icons, icon dragging, selection rectangles, and desktop context menus remain 100% interactive.
   - In the event of an Explorer restart, FeatherWall traps `TaskbarCreated` and `WM_DISPLAYCHANGE`, restoring overlay positions and monitor coordinates without orphaned windows.

