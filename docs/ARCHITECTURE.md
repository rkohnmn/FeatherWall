# Architecture

## Process Boundaries

`featherwall.exe` operates with a single native Win32 GUI message thread owning the wallpaper HWNDs, Media Foundation COM players, system tray icon, and the configuration window. There is no browser engine, game loop, persistent FFmpeg worker, web server, audio playback device, or continuous GPU readback loop. Wallpaper video timing is owned by the Windows MFPlay/EVR player. The event thread blocks in `GetMessageW` except for operating-system events, user input, and a low-frequency policy/performance timer (which is disabled when unnecessary).

A short-lived import worker opens FFprobe and FFmpeg with argument arrays (not shell commands) and drains their pipes. Source hashing is streamed in fixed 64 KiB chunks and can be canceled. FFmpeg runs only during import, then exits. When import is stopped, the child is killed and reaped, and partially prepared outputs are removed.

---

## Native Windows Fluent UI Architecture

FeatherWall features a custom native Windows 11 Fluent Design user interface implemented in pure Rust over the Win32 GDI API:

```
[ Win32 Window Procedure (WndProc) ]
               │
               ▼
   [ UI Event Router (events.rs) ]
   ├── Mouse Move / Hover Hit-Test
   ├── Mouse Left Click / Double Click
   ├── Mouse Wheel Scrolling
   ├── Drag-and-Drop (WM_DROPFILES)
   └── Search Input (WM_CHAR / WM_KEYDOWN)
               │
               ▼
     [ View Engine (views.rs) ]
   ├── Shell (Sidebar, TopBar, Toast Overlay)
   ├── Home (Active Wallpaper, Quick Controls, Live Metrics)
   ├── Wallpaper Library (Responsive Grid, Cards, Detail Inspector)
   ├── Displays (Multi-Monitor Enumeration, Scale Mode)
   ├── Optimization (Profile Selector, Progress Bar)
   ├── Performance (Real-Time Telemetry, Sparkline Chart)
   └── Settings (Config Groups, Cache Cleanup, Logging)
               │
               ▼
    [ Drawing Engine (draw.rs) ]
   ├── Double-Buffered Memory DC (MemDc)
   ├── ClearType Typography (Segoe UI Variable Text)
   ├── Rounded Cards, Pill Badges, Modern Buttons
   ├── Progress Bars & Historical Sparklines
   └── 320x180 Thumbnail Rasterizer
               │
               ▼
[ 1x Single BitBlt to Screen (Zero Flicker) ]
```

### Key UI Subsystems:
1. **Flicker-Free Double Buffering (`MemDc`)**: All view painting occurs in an offscreen compatible memory bitmap DC created with `CreateCompatibleDC` and `CreateCompatibleBitmap`. Once rendering is complete, a single `BitBlt` transfers the buffer to the window DC. `WM_ERASEBKGND` returns `1` to eliminate white screen flashing.
2. **Dynamic Theming (`Theme`)**: Full dark theme (default charcoal `#1E1F24`) and light theme (`#F3F3F5`) support with accent colors (`#6366F1` indigo). Title bar integration leverages `DwmSetWindowAttribute(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, ...)` on Windows 10 (build 19041+) and Windows 11.
3. **Typography (`Fonts`)**: Custom ClearType fonts using `Segoe UI Variable Text`, `Segoe UI`, `Cascadia Code`, and `Consolas` at distinct size hierarchies (Heading, Subheading, Body, Small, Code).
4. **Drag-and-Drop (`WM_DROPFILES`)**: Native file drops are accepted via `DragAcceptFiles` and queried via `DragQueryFileW` for zero-click import of video/image files.
5. **Local Thumbnail Caching**: On import, FFmpeg generates a lightweight 320×180 24-bit BMP (`<hash>.thumb.bmp`). The UI loads thumbnails on demand when the library view is active and caches HBITMAP handles in memory, avoiding continuous decoding overhead.
6. **Separation of Lifecycles**: Closing the UI window does not interrupt or restart wallpaper playback. GDI resources allocated for window painting are cleanly freed when the window is hidden.

---

## Frame Path

`optimized silent H.264 MP4 → Windows Media Foundation (MFPlay) → EVR/native child HWND → Explorer WorkerW desktop surface`.

This is a direct native Media Foundation / Enhanced Video Renderer pipeline. MFPlay/EVR negotiates with Windows DXVA2 and Direct3D for hardware-accelerated video decoding on NVIDIA, AMD, and Intel GPUs. Static images take a lightweight path: `source image → FFmpeg one-time 24-bit BMP normalization → GDI LoadImageW → cached HBITMAP → WM_PAINT` without video decoder overhead.

---

## Desktop Attachment

After sending the standard Explorer message `0x052c` to `Progman` with a one-second timeout, FeatherWall enumerates top-level windows, locates `SHELLDLL_DefView`, and selects the next sibling `WorkerW` for child-window hosting. Each wallpaper child is created with `WS_CHILD`, tool-window/no-activate styles, and physical monitor offsets adjusted by the host's top-left coordinate. If the host cannot be discovered, FeatherWall reports a failure and does not create an intrusive always-on-top alternative.

An Explorer taskbar recreation message or detected invalid host causes existing players/windows to be cleanly re-attached, preserving user configuration. Monitors are dynamically enumerated via `EnumDisplayMonitors` and `GetMonitorInfoW`.

---

## Suspension and Recovery

Foreground-window notifications and low-frequency reconciliation classify monitors individually:
- **Fullscreen Suspension**: Pauses playback when an obscured display runs a fullscreen game or application, dropping CPU usage to 0.19% and releasing the GPU video pipeline.
- **Power & Session Events**: Native WTS session lock messages, display-power broadcasts, and system sleep notifications immediately pause players and halt background timers.
- **Independent Monitors**: A monitor on which fullscreen content is visible can be paused without affecting playback on secondary monitors.

---

## Data and Caching

The application stores configuration in `%LOCALAPPDATA%\FeatherWall\library.fwcfg`, written atomically via temporary file and rename. Missing or corrupt entries are automatically pruned. Cache keys incorporate a streamed 128-bit digest of the source file, file size, profile settings, frame cap, and scaling mode.

---

## Desktop Information Overlay Architecture (Simple Clean Inspired)

FeatherWall includes a native, ultra-lightweight desktop information overlay displaying an elegant digital clock, dynamic date with localized regional formatting, personalized rotating greetings, and an optional floating media controller.

```
[ Windows Desktop Surface / Progman / WorkerW ]
                      │
                      ▼
       [ Active Wallpaper (Video or Still) ]
                      │
                      ▼
[ FeatherWall Layered Widget Overlay (WS_EX_LAYERED) ]
  ├── Large Digital Clock (12h/24h, Segoe UI / Variable, Drop Shadow)
  ├── Date Display (Full, Day/Date, Dot, Numeric, ISO formats)
  ├── Intelligent Greetings (Time-of-day phrases & rotation)
  └── Frosted Glass Music Capsule (Track/Artist & Media Controls)
                      │
                      ▼
      [ Desktop Icons (SHELLDLL_DefView) ]
  (Clicks pass through seamlessly via HTTRANSPARENT)
                      │
                      ▼
       [ Normal Application Windows (Z-Order Top) ]
```

### Key Overlay Principles:
1. **Layered Window Composition (`UpdateLayeredWindow`)**: Each target monitor hosts a dedicated borderless `WS_POPUP` layered window (`WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE`) positioned at `HWND_BOTTOM`. Rendering is executed on a 32-bit premultiplied ARGB DIB Section (`CreateDIBSection`). The Desktop Window Manager (DWM) alpha-blends the overlay directly onto the desktop wallpaper in hardware with zero CPU readback or continuous swapchain redraws.
2. **Selective Click-Through (`WM_NCHITTEST`)**: The overlay window procedure inspects cursor coordinates. Areas outside interactive controls return `HTTRANSPARENT`, allowing all mouse clicks, drag rectangles, icon double-clicks, and desktop right-click menus to pass directly through to Explorer desktop icons. When the cursor hovers over music buttons or edit-mode handles, `HTCLIENT` is returned, allowing button interaction without stealing focus (`WM_MOUSEACTIVATE` returns `MA_NOACTIVATE`).
3. **Event-Driven Minute-Boundary Scheduling**: Unlike conventional widget engines that continuously poll at 30 or 60 FPS, FeatherWall calculates the exact milliseconds remaining until the next minute boundary:
   $$\Delta t = (60 - \text{seconds}) \times 1000 - \text{milliseconds} + 25$$
   A single one-shot timer (`SetTimer`) is armed. Between minute changes, the overlay consumes **0.00% idle CPU**.
4. **Local Spotify & Media Integration**: Queries local Windows media sessions and inspects the Spotify desktop window title (`Chrome_WidgetWin_0`) without invoking Chromium, Electron, or Spotify Web API OAuth tokens. Controls send `WM_APPCOMMAND` and hardware media key strokes (`VK_MEDIA_PLAY_PAUSE`, `VK_MEDIA_NEXT_TRACK`, `VK_MEDIA_PREV_TRACK`). When no media is detected, polling frequency drops or shuts down completely.
5. **Smart Suspension**: Deeply integrated into FeatherWall's suspension engine. Whenever a fullscreen game/app is detected, the display powers off, or the session locks, widget timers are unregistered and the overlay window is hidden, releasing graphics resources.

