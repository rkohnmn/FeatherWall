# Windows manual acceptance checklist

**Initial status for every item: NOT RUN.** Use an actual Windows 10/11 x64 installation. For each step record OS build, GPU/driver, reproduction commands, pass/fail, screenshot/log if relevant, and memory/CPU telemetry.

- [ ] Clean release build and startup on Windows 10 1607-or-newer x64.
- [ ] First-run tray icon appears; opening and closing settings behaves correctly.
- [ ] Import MP4 and animated GIF; FFmpeg creates a playable silent MP4 and useful metadata.
- [ ] Invalid/unsupported input fails with diagnostics but does not crash application.
- [ ] GIF alpha and unusual per-frame delays are visually inspected.
- [ ] Import Unicode, apostrophe, whitespace, and special-character paths.
- [ ] Cancel GIF and MP4 imports; verify subprocesses are gone and `.partial` files are removed.
- [ ] Repeat the same import; verify cached asset is reused.
- [ ] Apply wallpaper behind icons; click desktop icons, right-click desktop and use taskbar normally.
- [ ] Observe looping for at least 20 cycles and inspect gap/black-frame behavior.
- [ ] Verify image wallpaper consumes no decoder resources after preparation.
- [ ] Pause/resume/stop from settings, tray and fwctl console; use tracing to verify actual decoder activity decreases.
- [ ] Set different media on two monitors with mixed DPI, negative offsets and refresh rates.
- [ ] Open fullscreen and borderless fullscreen apps separately on either display; observe independent pause/resume.
- [ ] Lock/unlock session; turn display off/on; sleep and resume.
- [ ] Battery/battery-saver/RDP configuration changes take effect as intended.
- [ ] Change screen arrangement, unplug monitor, resize, alter primary display and reconnect.
- [ ] Kill and restart Explorer; confirm WorkerW host recovery and no duplicated video layers.
- [ ] Flip start-with-Windows in both directions, verify HKCU Run value and launch behavior.
- [ ] Remove library item, verify original source remains and cache cleanup deletes only unreferenced assets.
- [ ] Intentionally move/remove original source after optimization; cached asset still plays.
- [ ] Switch wallpapers 50+ times and inspect processes, handles, decoder resources and working set for leaks.
- [ ] Shutdown using Exit; no orphaned player process, child window, or tray icon remains.
- [ ] Benchmark stopped/static/15/30/60 FPS and two-monitor scenarios (see PERFORMANCE.md).
