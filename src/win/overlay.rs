//! High-performance native Windows desktop widget overlay.
//! Implements a floating, transparent, click-through information overlay
//! (Clock, Date, Greeting, and Music Controller) inspired by Simple Clean.
//! Uses native Win32 32-bit layered window composition (UpdateLayeredWindow)
//! with hardware DWM alpha blending and event-driven minute-boundary scheduling.
use super::desktop::Monitor;
use super::ffi as f;
use super::media::{self, MediaState};
use super::ui::draw::UiRect;
use crate::core::{self, WidgetDisplayTarget, WidgetPosition, WidgetSettings};
use std::ffi::c_void;
use std::ptr;

const OVERLAY_CLASS: &str = "FeatherWallWidgetOverlay";
const TIMER_ID_TICK: usize = 1001;
const TIMER_ID_MEDIA: usize = 1002;

pub struct OverlayWindow {
    pub hwnd: f::HWND,
    pub device: String,
    pub monitor_rect: core::Rect,
    pub dpi: i32,
    pub prev_btn_rect: UiRect,
    pub play_pause_btn_rect: UiRect,
    pub next_btn_rect: UiRect,
    pub open_spotify_rect: UiRect,
    pub clock_drag_rect: UiRect,
    pub music_drag_rect: UiRect,
    pub done_edit_btn_rect: UiRect,
    pub hovered_btn: Option<&'static str>,
    pub dragging_clock: bool,
    pub dragging_music: bool,
    pub drag_start_cursor: (i32, i32),
    pub drag_start_offset: (i32, i32),
}

pub struct WidgetOverlayManager {
    pub instance: f::HINSTANCE,
    pub overlays: Vec<OverlayWindow>,
    pub media_state: MediaState,
    pub settings: WidgetSettings,
    pub suspended: bool,
    pub last_rendered_minute: u16,
}

static mut GLOBAL_OVERLAY_MANAGER: *mut WidgetOverlayManager = ptr::null_mut();

unsafe extern "system" fn overlay_wnd_proc(
    hwnd: f::HWND,
    msg: u32,
    w: usize,
    l: isize,
) -> isize {
    let mgr = if !GLOBAL_OVERLAY_MANAGER.is_null() {
        &mut *GLOBAL_OVERLAY_MANAGER
    } else {
        return f::DefWindowProcW(hwnd, msg, w, l);
    };

    let overlay_idx = mgr.overlays.iter().position(|o| o.hwnd == hwnd);

    match msg {
        f::WM_NCHITTEST => {
            if let Some(idx) = overlay_idx {
                let overlay = &mgr.overlays[idx];
                let screen_x = f::low(l as usize) as i16 as i32;
                let screen_y = f::high(l as usize) as i16 as i32;
                let client_x = screen_x - overlay.monitor_rect.left;
                let client_y = screen_y - overlay.monitor_rect.top;

                // Interactive Hit-Testing:
                // Only return HTCLIENT if the cursor is directly over interactive buttons
                // or drag handles in edit mode. Everywhere else returns HTTRANSPARENT,
                // allowing clicks to seamlessly pass through to Windows desktop icons!
                if overlay.prev_btn_rect.contains(client_x, client_y)
                    || overlay.play_pause_btn_rect.contains(client_x, client_y)
                    || overlay.next_btn_rect.contains(client_x, client_y)
                    || overlay.open_spotify_rect.contains(client_x, client_y)
                    || overlay.clock_drag_rect.contains(client_x, client_y)
                    || overlay.music_drag_rect.contains(client_x, client_y)
                    || overlay.done_edit_btn_rect.contains(client_x, client_y)
                {
                    return f::HTCLIENT;
                }
            }
            f::HTTRANSPARENT
        }

        f::WM_SETCURSOR => {
            if f::low(l as usize) == f::HTCLIENT as u16 {
                if let Some(idx) = overlay_idx {
                    let overlay = &mgr.overlays[idx];
                    let mut pt: f::POINT = f::zeroed();
                    f::GetCursorPos(&mut pt);
                    let client_x = pt.x - overlay.monitor_rect.left;
                    let client_y = pt.y - overlay.monitor_rect.top;

                    let cursor_id = if overlay.clock_drag_rect.contains(client_x, client_y)
                        || overlay.music_drag_rect.contains(client_x, client_y)
                    {
                        f::IDC_SIZEALL
                    } else if overlay.prev_btn_rect.contains(client_x, client_y)
                        || overlay.play_pause_btn_rect.contains(client_x, client_y)
                        || overlay.next_btn_rect.contains(client_x, client_y)
                        || overlay.open_spotify_rect.contains(client_x, client_y)
                        || overlay.done_edit_btn_rect.contains(client_x, client_y)
                    {
                        f::IDC_HAND
                    } else {
                        f::IDC_ARROW
                    };
                    f::SetCursor(f::LoadCursorW(0, cursor_id as *const u16));
                    return 1;
                }
            }
            f::DefWindowProcW(hwnd, msg, w, l)
        }

        f::WM_MOUSEACTIVATE => {
            // Never steal keyboard focus from the active foreground application!
            f::MA_NOACTIVATE
        }

        f::WM_WINDOWPOSCHANGING => {
            // Keep the overlay window permanently at the bottom of the Z-order,
            // anchored directly behind normal applications!
            if l != 0 {
                let wp = &mut *(l as *mut f::WINDOWPOS);
                wp.hwndInsertAfter = f::HWND_BOTTOM;
            }
            0
        }

        f::WM_MOUSEMOVE => {
            if let Some(idx) = overlay_idx {
                let overlay = &mut mgr.overlays[idx];
                let client_x = f::low(l as usize) as i16 as i32;
                let client_y = f::high(l as usize) as i16 as i32;
                let mut pt: f::POINT = f::zeroed();
                f::GetCursorPos(&mut pt);

                if overlay.dragging_clock {
                    let _dx = pt.x - overlay.drag_start_cursor.0;
                    let _dy = pt.y - overlay.drag_start_cursor.1;
                    // Position updated on release
                }

                let prev_hover = overlay.hovered_btn;
                overlay.hovered_btn = if overlay.prev_btn_rect.contains(client_x, client_y) {
                    Some("prev")
                } else if overlay.play_pause_btn_rect.contains(client_x, client_y) {
                    Some("play_pause")
                } else if overlay.next_btn_rect.contains(client_x, client_y) {
                    Some("next")
                } else if overlay.open_spotify_rect.contains(client_x, client_y) {
                    Some("open_spotify")
                } else if overlay.done_edit_btn_rect.contains(client_x, client_y) {
                    Some("done_edit")
                } else {
                    None
                };

                if prev_hover != overlay.hovered_btn {
                    // Update hovered button presentation if needed
                }
            }
            0
        }

        f::WM_LBUTTONDOWN => {
            if let Some(idx) = overlay_idx {
                let client_x = f::low(l as usize) as i16 as i32;
                let client_y = f::high(l as usize) as i16 as i32;
                let mut pt: f::POINT = f::zeroed();
                f::GetCursorPos(&mut pt);

                let overlay = &mut mgr.overlays[idx];
                if overlay.prev_btn_rect.contains(client_x, client_y) {
                    media::prev_track(&mgr.media_state);
                    mgr.media_state = media::query_media_session(true, true);
                    mgr.render_all();
                } else if overlay.play_pause_btn_rect.contains(client_x, client_y) {
                    media::play_pause(&mgr.media_state);
                    mgr.media_state = media::query_media_session(true, true);
                    mgr.render_all();
                } else if overlay.next_btn_rect.contains(client_x, client_y) {
                    media::next_track(&mgr.media_state);
                    mgr.media_state = media::query_media_session(true, true);
                    mgr.render_all();
                } else if overlay.open_spotify_rect.contains(client_x, client_y) {
                    media::open_spotify();
                } else if overlay.done_edit_btn_rect.contains(client_x, client_y) {
                    // Exit edit mode
                    mgr.exit_edit_mode();
                } else if overlay.clock_drag_rect.contains(client_x, client_y) {
                    overlay.dragging_clock = true;
                    overlay.drag_start_cursor = (pt.x, pt.y);
                } else if overlay.music_drag_rect.contains(client_x, client_y) {
                    overlay.dragging_music = true;
                    overlay.drag_start_cursor = (pt.x, pt.y);
                }
            }
            0
        }

        f::WM_LBUTTONUP => {
            if let Some(idx) = overlay_idx {
                let overlay = &mut mgr.overlays[idx];
                overlay.dragging_clock = false;
                overlay.dragging_music = false;
            }
            0
        }

        f::WM_TIMER => {
            if w == TIMER_ID_TICK {
                mgr.on_minute_tick();
            } else if w == TIMER_ID_MEDIA {
                mgr.on_media_tick();
            }
            0
        }

        _ => f::DefWindowProcW(hwnd, msg, w, l),
    }
}

impl WidgetOverlayManager {
    pub fn new(instance: f::HINSTANCE, settings: WidgetSettings) -> Result<Self, String> {
        let class_name = f::wide(OVERLAY_CLASS);
        let wnd_class = f::WNDCLASSEXW {
            cbSize: std::mem::size_of::<f::WNDCLASSEXW>() as u32,
            style: 0x0001 | 0x0002, // CS_HREDRAW | CS_VREDRAW
            lpfnWndProc: Some(overlay_wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: instance,
            hIcon: 0,
            hCursor: unsafe { f::LoadCursorW(0, f::IDC_ARROW as *const u16) },
            hbrBackground: 0,
            lpszMenuName: ptr::null(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: 0,
        };

        unsafe {
            let _ = f::RegisterClassExW(&wnd_class);
        }

        let media_state = media::query_media_session(true, true);

        Ok(Self {
            instance,
            overlays: Vec::new(),
            media_state,
            settings,
            suspended: false,
            last_rendered_minute: 999,
        })
    }

    pub fn register_global(&mut self) {
        unsafe {
            GLOBAL_OVERLAY_MANAGER = self as *mut Self;
        }
    }

    pub fn unregister_global() {
        unsafe {
            GLOBAL_OVERLAY_MANAGER = ptr::null_mut();
        }
    }

    pub fn exit_edit_mode(&mut self) {
        self.settings.edit_mode = false;
        self.render_all();
    }

    pub fn update_settings(&mut self, settings: &WidgetSettings) {
        self.settings = settings.clone();
        if !self.settings.enabled {
            self.destroy_all();
        } else {
            self.schedule_next_tick();
            self.render_all();
        }
    }

    pub fn sync_monitors(
        &mut self,
        monitors: &[Monitor],
    ) {
        if !self.settings.enabled {
            self.destroy_all();
            return;
        }

        let targets: Vec<Monitor> = match self.settings.display_target {
            WidgetDisplayTarget::Primary => monitors
                .iter()
                .filter(|m| m.primary)
                .cloned()
                .collect(),
            WidgetDisplayTarget::All => monitors.to_vec(),
            WidgetDisplayTarget::Selected => monitors
                .iter()
                .take(1)
                .cloned()
                .collect(),
        };

        // Retain matching windows, close removed ones
        self.overlays.retain(|o| {
            if !targets.iter().any(|m| m.device == o.device) {
                unsafe {
                    f::DestroyWindow(o.hwnd);
                }
                false
            } else {
                true
            }
        });

        // Create missing windows
        for m in &targets {
            if !self.overlays.iter().any(|o| o.device == m.device) {
                let overlay = self.create_overlay_window(m);
                if let Some(o) = overlay {
                    self.overlays.push(o);
                }
            } else if let Some(o) = self.overlays.iter_mut().find(|o| o.device == m.device) {
                o.monitor_rect = m.rect;
                unsafe {
                    f::SetWindowPos(
                        o.hwnd,
                        f::HWND_BOTTOM,
                        m.rect.left,
                        m.rect.top,
                        m.rect.width(),
                        m.rect.height(),
                        f::SWP_NOACTIVATE | f::SWP_SHOWWINDOW,
                    );
                }
            }
        }

        self.schedule_next_tick();
        self.render_all();
    }

    fn create_overlay_window(&self, monitor: &Monitor) -> Option<OverlayWindow> {
        let class = f::wide(OVERLAY_CLASS);
        let title = f::wide("FeatherWall Desktop Overlay");

        let hwnd = unsafe {
            f::CreateWindowExW(
                f::WS_EX_LAYERED | f::WS_EX_TOOLWINDOW | f::WS_EX_NOACTIVATE,
                class.as_ptr(),
                title.as_ptr(),
                f::WS_POPUP | f::WS_VISIBLE | f::WS_CLIPSIBLINGS,
                monitor.rect.left,
                monitor.rect.top,
                monitor.rect.width(),
                monitor.rect.height(),
                0,
                0,
                self.instance,
                ptr::null_mut(),
            )
        };

        if hwnd == 0 {
            return None;
        }

        unsafe {
            // Anchor to the very bottom of the Z-order
            f::SetWindowPos(
                hwnd,
                f::HWND_BOTTOM,
                monitor.rect.left,
                monitor.rect.top,
                monitor.rect.width(),
                monitor.rect.height(),
                f::SWP_NOACTIVATE | f::SWP_SHOWWINDOW,
            );
        }

        let dpi = unsafe {
            let d = f::GetDpiForWindow(hwnd);
            if d > 0 { d as i32 } else { 96 }
        };

        Some(OverlayWindow {
            hwnd,
            device: monitor.device.clone(),
            monitor_rect: monitor.rect,
            dpi,
            prev_btn_rect: UiRect::default(),
            play_pause_btn_rect: UiRect::default(),
            next_btn_rect: UiRect::default(),
            open_spotify_rect: UiRect::default(),
            clock_drag_rect: UiRect::default(),
            music_drag_rect: UiRect::default(),
            done_edit_btn_rect: UiRect::default(),
            hovered_btn: None,
            dragging_clock: false,
            dragging_music: false,
            drag_start_cursor: (0, 0),
            drag_start_offset: (0, 0),
        })
    }

    pub fn schedule_next_tick(&self) {
        if self.overlays.is_empty() || self.suspended || !self.settings.enabled {
            return;
        }

        let mut st: f::SYSTEMTIME = f::zeroed();
        unsafe {
            f::GetLocalTime(&mut st);
        }

        let first_hwnd = self.overlays[0].hwnd;

        // When seconds are enabled: wake up every 1000ms.
        // When seconds are disabled: calculate milliseconds until the exact next minute boundary!
        // This ensures effectively 0% idle CPU and ZERO wakeups between minutes!
        let elapse_ms = if self.settings.clock_seconds {
            1000
        } else {
            let ms_remaining = (60 - st.wSecond as u32) * 1000 - st.wMilliseconds as u32 + 25;
            ms_remaining.max(250)
        };

        unsafe {
            f::KillTimer(first_hwnd, TIMER_ID_TICK);
            f::SetTimer(first_hwnd, TIMER_ID_TICK, elapse_ms, ptr::null());

            // If music widget is active, set a low-frequency media refresh timer (3000ms)
            if self.settings.music_enabled {
                f::KillTimer(first_hwnd, TIMER_ID_MEDIA);
                f::SetTimer(first_hwnd, TIMER_ID_MEDIA, 3000, ptr::null());
            } else {
                f::KillTimer(first_hwnd, TIMER_ID_MEDIA);
            }
        }
    }

    pub fn on_minute_tick(&mut self) {
        if self.suspended {
            return;
        }
        self.render_all();
    }

    pub fn on_media_tick(&mut self) {
        if self.suspended {
            return;
        }
        let updated = media::query_media_session(true, true);
        if updated != self.media_state {
            self.media_state = updated;
            self.render_all();
        }
    }

    pub fn suspend(&mut self) {
        self.suspended = true;
        for o in &self.overlays {
            unsafe {
                f::KillTimer(o.hwnd, TIMER_ID_TICK);
                f::KillTimer(o.hwnd, TIMER_ID_MEDIA);
                f::ShowWindow(o.hwnd, f::SW_HIDE);
            }
        }
    }

    pub fn resume(&mut self) {
        self.suspended = false;
        for o in &self.overlays {
            unsafe {
                f::ShowWindow(o.hwnd, f::SW_SHOW);
                f::SetWindowPos(
                    o.hwnd,
                    f::HWND_BOTTOM,
                    0,
                    0,
                    0,
                    0,
                    f::SWP_NOMOVE | f::SWP_NOSIZE | f::SWP_NOACTIVATE,
                );
            }
        }
        self.schedule_next_tick();
        self.render_all();
    }

    pub fn render_all(&mut self) {
        if self.suspended {
            return;
        }

        let mut st: f::SYSTEMTIME = f::zeroed();
        unsafe {
            f::GetLocalTime(&mut st);
        }

        for o in &mut self.overlays {
            render_overlay_surface(o, &st, &self.media_state, &self.settings);
        }

        self.last_rendered_minute = st.wMinute;
    }

    pub fn destroy_all(&mut self) {
        for o in self.overlays.drain(..) {
            unsafe {
                f::KillTimer(o.hwnd, TIMER_ID_TICK);
                f::KillTimer(o.hwnd, TIMER_ID_MEDIA);
                f::DestroyWindow(o.hwnd);
            }
        }
    }
}

impl Drop for WidgetOverlayManager {
    fn drop(&mut self) {
        Self::unregister_global();
        self.destroy_all();
    }
}

fn render_overlay_surface(
    overlay: &mut OverlayWindow,
    st: &f::SYSTEMTIME,
    media: &MediaState,
    settings: &WidgetSettings,
) {

    let w = overlay.monitor_rect.width();
    let h = overlay.monitor_rect.height();
    if w <= 0 || h <= 0 {
        return;
    }

    let dpi = overlay.dpi;
    let scale = |px: i32| (px * dpi) / 96;

    // Reset interactive bounding rects
    overlay.prev_btn_rect = UiRect::default();
    overlay.play_pause_btn_rect = UiRect::default();
    overlay.next_btn_rect = UiRect::default();
    overlay.open_spotify_rect = UiRect::default();
    overlay.clock_drag_rect = UiRect::default();
    overlay.music_drag_rect = UiRect::default();
    overlay.done_edit_btn_rect = UiRect::default();

    unsafe {
        let hdc_screen = f::GetDC(0);
        if hdc_screen == 0 {
            return;
        }

        let mut bmi: f::BITMAPINFO = f::zeroed();
        bmi.bmiHeader.biSize = std::mem::size_of::<f::BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = w;
        bmi.bmiHeader.biHeight = -h; // Top-down DIB
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = f::BI_RGB;

        let mut bits: *mut c_void = ptr::null_mut();
        let hbitmap = f::CreateDIBSection(
            hdc_screen,
            &bmi,
            f::DIB_RGB_COLORS,
            &mut bits,
            0,
            0,
        );

        if hbitmap == 0 || bits.is_null() {
            f::ReleaseDC(0, hdc_screen);
            return;
        }

        let hdc_mem = f::CreateCompatibleDC(hdc_screen);
        let prev_bmp = f::SelectObject(hdc_mem, hbitmap);

        // 1. Prepare Clear Canvas (Zero out 32-bit pixel buffer)
        let total_pixels = (w * h) as usize;
        ptr::write_bytes(bits, 0, total_pixels * 4);

        // 2. Prepare Formatted Text Strings
        // Clock String
        let clock_str = if settings.clock_24h {
            if settings.clock_seconds {
                format!("{:02}:{:02}:{:02}", st.wHour, st.wMinute, st.wSecond)
            } else {
                format!("{:02}:{:02}", st.wHour, st.wMinute)
            }
        } else {
            let (h12, am_pm) = if st.wHour == 0 {
                (12, "AM")
            } else if st.wHour < 12 {
                (st.wHour, "AM")
            } else if st.wHour == 12 {
                (12, "PM")
            } else {
                (st.wHour - 12, "PM")
            };
            if settings.clock_seconds {
                format!("{}:{:02}:{:02} {}", h12, st.wMinute, st.wSecond, am_pm)
            } else {
                format!("{}:{:02} {}", h12, st.wMinute, am_pm)
            }
        };

        // Date String
        let date_str = settings.date_format.format(
            st.wYear as u32,
            st.wMonth as u32,
            st.wDay as u32,
            st.wDayOfWeek as u32,
        );

        // Greeting String
        let windows_user = f::current_windows_username().unwrap_or_else(|| "Alex".into());
        let effective_name = if !settings.greeting_name.trim().is_empty() {
            &settings.greeting_name
        } else {
            &windows_user
        };

        let greeting_str = core::resolve_greeting(
            settings.greeting_mode,
            st.wHour as u32,
            st.wMinute as u32,
            effective_name,
            settings.greeting_interval_mins,
            &settings.custom_greeting,
        );

        // 3. Layout Positioning
        // Calculate anchor for Top-Right (default Rainmeter Simple Clean layout)
        let anchor_margin_x = scale(settings.offset_x.abs());
        let anchor_margin_y = scale(settings.offset_y.abs());

        let (clock_x, clock_y, alignment_flags) = match settings.position {
            WidgetPosition::TopRight => (
                w - anchor_margin_x - scale(500),
                anchor_margin_y,
                f::DT_RIGHT,
            ),
            WidgetPosition::TopLeft => (
                anchor_margin_x,
                anchor_margin_y,
                f::DT_LEFT,
            ),
            WidgetPosition::TopCenter => (
                (w - scale(500)) / 2,
                anchor_margin_y,
                f::DT_CENTER,
            ),
            WidgetPosition::Center => (
                (w - scale(500)) / 2,
                (h - scale(200)) / 2,
                f::DT_CENTER,
            ),
            WidgetPosition::BottomLeft => (
                anchor_margin_x,
                h - anchor_margin_y - scale(180),
                f::DT_LEFT,
            ),
            WidgetPosition::BottomRight => (
                w - anchor_margin_x - scale(500),
                h - anchor_margin_y - scale(180),
                f::DT_RIGHT,
            ),
            WidgetPosition::BottomCenter => (
                (w - scale(500)) / 2,
                h - anchor_margin_y - scale(180),
                f::DT_CENTER,
            ),
        };

        let block_w = scale(500);

        // 4. Render Typography into DIB Section
        // A. CLOCK
        let clock_font_size = scale(settings.clock_size as i32);
        let font_clock = make_font(clock_font_size, 300, "Segoe UI Variable Display", "Segoe UI");
        let mut cur_y = clock_y;

        if settings.clock_enabled {
            let clock_rect = UiRect::new(clock_x, cur_y, block_w, clock_font_size + scale(10));
            draw_alpha_text(
                hdc_mem,
                bits.cast::<u32>(),
                w,
                h,
                &clock_str,
                &clock_rect,
                font_clock,
                settings.clock_color,
                settings.opacity,
                settings.text_shadow,
                alignment_flags | f::DT_SINGLELINE,
            );
            cur_y += clock_font_size + scale(6);
        }

        // B. DATE
        let date_font_size = scale(settings.date_size as i32);
        let font_date = make_font(date_font_size, 400, "Segoe UI Variable Text", "Segoe UI");

        if settings.date_enabled {
            let date_rect = UiRect::new(clock_x, cur_y, block_w, date_font_size + scale(8));
            draw_alpha_text(
                hdc_mem,
                bits.cast::<u32>(),
                w,
                h,
                &date_str,
                &date_rect,
                font_date,
                0x00E2E8F0, // Soft crisp silver-white
                (settings.opacity as f32 * 0.92) as u8,
                settings.text_shadow,
                alignment_flags | f::DT_SINGLELINE,
            );
            cur_y += date_font_size + scale(6);
        }

        // C. GREETING
        let greeting_font_size = scale(16);
        let font_greeting = make_font(greeting_font_size, 400, "Segoe UI Variable Text", "Segoe UI");

        if settings.greeting_enabled && !greeting_str.is_empty() {
            let greet_rect = UiRect::new(clock_x, cur_y, block_w, greeting_font_size + scale(8));
            draw_alpha_text(
                hdc_mem,
                bits.cast::<u32>(),
                w,
                h,
                &greeting_str,
                &greet_rect,
                font_greeting,
                0x00CBD5E1, // Gentle muted accent
                (settings.opacity as f32 * 0.85) as u8,
                settings.text_shadow,
                alignment_flags | f::DT_SINGLELINE,
            );
            cur_y += greeting_font_size + scale(10);
        }

        // Store Clock drag box for edit mode
        if settings.edit_mode {
            let box_h = cur_y - clock_y + scale(8);
            overlay.clock_drag_rect = UiRect::new(clock_x, clock_y, block_w, box_h);
            draw_edit_frame(
                bits.cast::<u32>(),
                w,
                h,
                &overlay.clock_drag_rect,
                "✥ Drag Clock & Date",
                dpi,
            );
        }

        // 5. MUSIC CONTROLLER (Floating Pill at Bottom Center)
        if settings.music_enabled {
            let card_w = scale(360);
            let card_h = scale(52);
            let music_x = (w - card_w) / 2 + scale(settings.music_offset_x);
            let music_y = h - scale(settings.music_offset_y.abs()) - card_h;

            let card_rect = UiRect::new(music_x, music_y, card_w, card_h);

            // Draw Frosted Dark Capsule Background
            draw_pill_background(
                bits.cast::<u32>(),
                w,
                h,
                &card_rect,
                (20, 22, 28, 190), // Translucent obsidian glass
                (255, 255, 255, 30), // Subtle highlight border
            );

            // Left: Disc / Note glyph
            let icon_rect = UiRect::new(music_x + scale(12), music_y, scale(28), card_h);
            let font_icon = make_font(scale(16), 600, "Segoe UI Symbol", "Segoe UI");
            let icon_glyph = if media.is_playing { "♫" } else { "▶" };
            draw_alpha_text(
                hdc_mem,
                bits.cast::<u32>(),
                w,
                h,
                icon_glyph,
                &icon_rect,
                font_icon,
                settings.accent_color,
                255,
                false,
                f::DT_CENTER | f::DT_VCENTER | f::DT_SINGLELINE,
            );

            // Middle: Track Title & Artist
            let font_track = make_font(scale(11), 600, "Segoe UI Variable Text", "Segoe UI");
            let font_artist = make_font(scale(9), 400, "Segoe UI Variable Text", "Segoe UI");

            let track_title = if media.is_active {
                &media.title
            } else {
                "No Media Playing"
            };
            let track_artist = if media.is_active {
                &media.artist
            } else {
                "Spotify is idle"
            };

            let text_w = scale(180);
            let title_rect = UiRect::new(music_x + scale(42), music_y + scale(9), text_w, scale(16));
            draw_alpha_text(
                hdc_mem,
                bits.cast::<u32>(),
                w,
                h,
                track_title,
                &title_rect,
                font_track,
                0x00FFFFFF,
                255,
                false,
                f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE | f::DT_END_ELLIPSIS,
            );

            let artist_rect = UiRect::new(music_x + scale(42), music_y + scale(27), text_w, scale(14));
            draw_alpha_text(
                hdc_mem,
                bits.cast::<u32>(),
                w,
                h,
                track_artist,
                &artist_rect,
                font_artist,
                0x0094A3B8,
                220,
                false,
                f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE | f::DT_END_ELLIPSIS,
            );

            // Right: Interactive Media Buttons [ ◀◀ ] [ ▶/❚❚ ] [ ▶▶ ]
            let font_btn = make_font(scale(12), 600, "Segoe UI Symbol", "Segoe UI");
            let btn_size = scale(28);
            let btn_y = music_y + (card_h - btn_size) / 2;
            let mut btn_x = music_x + card_w - scale(115);

            // Previous Button
            overlay.prev_btn_rect = UiRect::new(btn_x, btn_y, btn_size, btn_size);
            draw_button_glyph(
                hdc_mem,
                bits.cast::<u32>(),
                w,
                h,
                "⏮",
                &overlay.prev_btn_rect,
                font_btn,
                0x00CBD5E1,
            );
            btn_x += btn_size + scale(6);

            // Play / Pause Button
            overlay.play_pause_btn_rect = UiRect::new(btn_x, btn_y, btn_size, btn_size);
            let play_glyph = if media.is_playing { "❚❚" } else { "▶" };
            draw_button_glyph(
                hdc_mem,
                bits.cast::<u32>(),
                w,
                h,
                play_glyph,
                &overlay.play_pause_btn_rect,
                font_btn,
                settings.accent_color,
            );
            btn_x += btn_size + scale(6);

            // Next Button
            overlay.next_btn_rect = UiRect::new(btn_x, btn_y, btn_size, btn_size);
            draw_button_glyph(
                hdc_mem,
                bits.cast::<u32>(),
                w,
                h,
                "⏭",
                &overlay.next_btn_rect,
                font_btn,
                0x00CBD5E1,
            );

            if !media.is_active {
                // If Spotify is idle, provide a small clickable "Open Spotify" badge
                overlay.open_spotify_rect = UiRect::new(music_x + scale(42), music_y + scale(27), text_w, scale(14));
            }

            // Edit Mode drag handle for Music
            if settings.edit_mode {
                overlay.music_drag_rect = card_rect;
                draw_edit_frame(
                    bits.cast::<u32>(),
                    w,
                    h,
                    &overlay.music_drag_rect,
                    "✥ Drag Music",
                    dpi,
                );
            }

            f::DeleteObject(font_track);
            f::DeleteObject(font_artist);
            f::DeleteObject(font_btn);
            f::DeleteObject(font_icon);
        }

        // D. EDIT MODE: "Done Repositioning" Button at Top Center
        if settings.edit_mode {
            let done_w = scale(180);
            let done_h = scale(36);
            let done_rect = UiRect::new((w - done_w) / 2, scale(20), done_w, done_h);
            overlay.done_edit_btn_rect = done_rect;

            draw_pill_background(
                bits.cast::<u32>(),
                w,
                h,
                &done_rect,
                (99, 102, 241, 230),
                (255, 255, 255, 100),
            );

            let font_done = make_font(scale(11), 600, "Segoe UI Variable Text", "Segoe UI");
            draw_alpha_text(
                hdc_mem,
                bits.cast::<u32>(),
                w,
                h,
                "✓ Done Repositioning",
                &done_rect,
                font_done,
                0x00FFFFFF,
                255,
                false,
                f::DT_CENTER | f::DT_VCENTER | f::DT_SINGLELINE,
            );
            f::DeleteObject(font_done);
        }

        // Clean up GDI fonts
        f::DeleteObject(font_clock);
        f::DeleteObject(font_date);
        f::DeleteObject(font_greeting);

        // 6. Push 32-bit Premultiplied ARGB to DWM Layered Window
        let pt_src = f::POINT { x: 0, y: 0 };
        let pt_dst = f::POINT {
            x: overlay.monitor_rect.left,
            y: overlay.monitor_rect.top,
        };
        let size = f::SIZE { cx: w, cy: h };
        let blend = f::BLENDFUNCTION {
            BlendOp: f::AC_SRC_OVER,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: f::AC_SRC_ALPHA, // Per-pixel 32-bit premultiplied alpha
        };

        f::UpdateLayeredWindow(
            overlay.hwnd,
            hdc_screen,
            &pt_dst,
            &size,
            hdc_mem,
            &pt_src,
            0,
            &blend,
            f::ULW_ALPHA,
        );

        // Cleanup GDI Handles
        f::SelectObject(hdc_mem, prev_bmp);
        f::DeleteDC(hdc_mem);
        f::DeleteObject(hbitmap);
        f::ReleaseDC(0, hdc_screen);
    }
}

fn make_font(height: i32, weight: i32, face: &str, fallback: &str) -> f::HGDIOBJ {
    let face_w = f::wide(face);
    let fallback_w = f::wide(fallback);

    unsafe {
        let h = f::CreateFontW(
            -height,
            0,
            0,
            0,
            weight,
            0,
            0,
            0,
            1, // DEFAULT_CHARSET
            0,
            0,
            f::ANTIALIASED_QUALITY,
            0,
            face_w.as_ptr(),
        );
        if h != 0 {
            h
        } else {
            f::CreateFontW(
                -height,
                0,
                0,
                0,
                weight,
                0,
                0,
                0,
                1,
                0,
                0,
                f::ANTIALIASED_QUALITY,
                0,
                fallback_w.as_ptr(),
            )
        }
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn draw_alpha_text(
    dc: f::HDC,
    pixels: *mut u32,
    canvas_w: i32,
    canvas_h: i32,
    text: &str,
    rect: &UiRect,
    font: f::HGDIOBJ,
    color: u32,
    opacity_pct: u8,
    shadow: bool,
    flags: u32,
) {
    if text.is_empty() || rect.w <= 0 || rect.h <= 0 {
        return;
    }

    let rx = rect.x.clamp(0, canvas_w - 1);
    let ry = rect.y.clamp(0, canvas_h - 1);
    let rw = rect.w.min(canvas_w - rx);
    let rh = rect.h.min(canvas_h - ry);
    if rw <= 0 || rh <= 0 {
        return;
    }

    let sub_pixels = (rw * rh) as usize;
    let mut temp_mask = vec![0u8; sub_pixels];

    // Render pure white text on black background to capture anti-aliased font coverage
    let prev_font = f::SelectObject(dc, font);
    f::SetBkMode(dc, f::TRANSPARENT);
    f::SetTextColor(dc, 0x00FFFFFF);

    let wide = f::wide(text);
    let mut wr = rect.to_win_rect();
    f::DrawTextW(dc, wide.as_ptr(), wide.len() as i32 - 1, &mut wr, flags);
    f::SelectObject(dc, prev_font);

    // Extract glyph intensities from DIB memory
    for y in 0..rh {
        let src_row = (ry + y) * canvas_w;
        let dst_row = y * rw;
        for x in 0..rw {
            let px = *pixels.add((src_row + rx + x) as usize);
            let r = ((px >> 16) & 0xFF) as u8;
            let g = ((px >> 8) & 0xFF) as u8;
            let b = (px & 0xFF) as u8;
            temp_mask[(dst_row + x) as usize] = r.max(g).max(b);
        }
    }

    // Color Components
    let cr = ((color >> 16) & 0xFF) as f32;
    let cg = ((color >> 8) & 0xFF) as f32;
    let cb = (color & 0xFF) as f32;
    let opacity = opacity_pct as f32 / 100.0;

    // Apply Soft Drop-Shadow and Premultiplied Alpha
    let shadow_dx = 2;
    let shadow_dy = 2;

    for y in 0..rh {
        let canvas_y = ry + y;
        let dst_row = (canvas_y * canvas_w + rx) as usize;

        for x in 0..rw {
            let idx = (y * rw + x) as usize;
            let text_cov = temp_mask[idx] as f32 / 255.0;
            let alpha_text = text_cov * opacity;

            let alpha_shadow = if shadow && x >= shadow_dx && y >= shadow_dy {
                let s_idx = ((y - shadow_dy) * rw + (x - shadow_dx)) as usize;
                let s_cov = temp_mask[s_idx] as f32 / 255.0;
                s_cov * 0.55 * opacity * (1.0 - alpha_text)
            } else {
                0.0
            };

            let final_alpha = (alpha_text + alpha_shadow).clamp(0.0, 1.0);
            if final_alpha > 0.001 {
                let out_r = ((cr * alpha_text).clamp(0.0, 255.0)) as u32;
                let out_g = ((cg * alpha_text).clamp(0.0, 255.0)) as u32;
                let out_b = ((cb * alpha_text).clamp(0.0, 255.0)) as u32;
                let out_a = ((final_alpha * 255.0).clamp(0.0, 255.0)) as u32;

                let px_out = (out_a << 24) | (out_r << 16) | (out_g << 8) | out_b;
                *pixels.add(dst_row + x as usize) = px_out;
            } else {
                *pixels.add(dst_row + x as usize) = 0;
            }
        }
    }
}

unsafe fn draw_pill_background(
    pixels: *mut u32,
    canvas_w: i32,
    canvas_h: i32,
    rect: &UiRect,
    bg_rgba: (u8, u8, u8, u8),
    border_rgba: (u8, u8, u8, u8),
) {
    let rx = rect.x.clamp(0, canvas_w - 1);
    let ry = rect.y.clamp(0, canvas_h - 1);
    let rw = rect.w.min(canvas_w - rx);
    let rh = rect.h.min(canvas_h - ry);
    if rw <= 0 || rh <= 0 {
        return;
    }

    let radius = rh / 2;
    let radius_sq = (radius * radius) as f32;

    let (br, bg, bb, ba) = bg_rgba;
    let (bor_r, bor_g, bor_b, bor_a) = border_rgba;

    let bg_a_norm = ba as f32 / 255.0;
    let bor_a_norm = bor_a as f32 / 255.0;

    for y in 0..rh {
        let canvas_y = ry + y;
        let dst_row = (canvas_y * canvas_w + rx) as usize;
        let dy = y - radius;

        for x in 0..rw {
            let mut inside = true;
            let mut border = false;

            if x < radius {
                let dx = x - radius;
                let d2 = (dx * dx + dy * dy) as f32;
                if d2 > radius_sq {
                    inside = false;
                } else if d2 > (radius - 1) as f32 * (radius - 1) as f32 {
                    border = true;
                }
            } else if x > rw - radius {
                let dx = x - (rw - radius);
                let d2 = (dx * dx + dy * dy) as f32;
                if d2 > radius_sq {
                    inside = false;
                } else if d2 > (radius - 1) as f32 * (radius - 1) as f32 {
                    border = true;
                }
            } else if y == 0 || y == rh - 1 {
                border = true;
            }

            if inside {
                let (r, g, b, a_norm) = if border {
                    (bor_r as f32, bor_g as f32, bor_b as f32, bor_a_norm)
                } else {
                    (br as f32, bg as f32, bb as f32, bg_a_norm)
                };

                let pr = (r * a_norm) as u32;
                let pg = (g * a_norm) as u32;
                let pb = (b * a_norm) as u32;
                let pa = (a_norm * 255.0) as u32;

                *pixels.add(dst_row + x as usize) = (pa << 24) | (pr << 16) | (pg << 8) | pb;
            }
        }
    }
}

unsafe fn draw_button_glyph(
    dc: f::HDC,
    pixels: *mut u32,
    canvas_w: i32,
    canvas_h: i32,
    glyph: &str,
    rect: &UiRect,
    font: f::HGDIOBJ,
    color: u32,
) {
    draw_alpha_text(
        dc,
        pixels,
        canvas_w,
        canvas_h,
        glyph,
        rect,
        font,
        color,
        255,
        false,
        f::DT_CENTER | f::DT_VCENTER | f::DT_SINGLELINE,
    );
}

unsafe fn draw_edit_frame(
    pixels: *mut u32,
    canvas_w: i32,
    canvas_h: i32,
    rect: &UiRect,
    _label: &str,
    _dpi: i32,
) {
    let rx = rect.x.clamp(0, canvas_w - 1);
    let ry = rect.y.clamp(0, canvas_h - 1);
    let rw = rect.w.min(canvas_w - rx);
    let rh = rect.h.min(canvas_h - ry);
    if rw <= 4 || rh <= 4 {
        return;
    }

    // Draw dashed indigo border
    let accent_color = 0x00818CF8; // Soft light indigo
    let alpha = 200u32;
    let r = ((accent_color >> 16) & 0xFF) * alpha / 255;
    let g = ((accent_color >> 8) & 0xFF) * alpha / 255;
    let b = (accent_color & 0xFF) * alpha / 255;
    let px_val = (alpha << 24) | (r << 16) | (g << 8) | b;

    for x in 0..rw {
        if (x / 6) % 2 == 0 {
            *pixels.add((ry * canvas_w + rx + x) as usize) = px_val;
            *pixels.add(((ry + rh - 1) * canvas_w + rx + x) as usize) = px_val;
        }
    }
    for y in 0..rh {
        if (y / 6) % 2 == 0 {
            *pixels.add(((ry + y) * canvas_w + rx) as usize) = px_val;
            *pixels.add(((ry + y) * canvas_w + rx + rw - 1) as usize) = px_val;
        }
    }
}
