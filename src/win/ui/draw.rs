//! Native GDI double-buffered drawing primitives for FeatherWall Fluent UI.
use super::theme::{Fonts, Theme};
use crate::win::ffi as f;
use std::ffi::c_void;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UiRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl UiRect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }

    pub fn to_win_rect(&self) -> f::RECT {
        f::RECT {
            left: self.x,
            top: self.y,
            right: self.x + self.w,
            bottom: self.y + self.h,
        }
    }
}

pub struct MemDc {
    pub hdc: f::HDC,
    pub bmp: f::HBITMAP,
    pub prev_bmp: f::HGDIOBJ,
    pub width: i32,
    pub height: i32,
}

impl MemDc {
    pub fn create(target_dc: f::HDC, width: i32, height: i32) -> Option<Self> {
        unsafe {
            let hdc = f::CreateCompatibleDC(target_dc);
            if hdc == 0 {
                return None;
            }
            let bmp = f::CreateCompatibleBitmap(target_dc, width, height);
            if bmp == 0 {
                f::DeleteDC(hdc);
                return None;
            }
            let prev_bmp = f::SelectObject(hdc, bmp);
            Some(Self {
                hdc,
                bmp,
                prev_bmp,
                width,
                height,
            })
        }
    }

    pub fn flip(&self, dest_dc: f::HDC) {
        unsafe {
            f::BitBlt(
                dest_dc,
                0,
                0,
                self.width,
                self.height,
                self.hdc,
                0,
                0,
                f::SRCCOPY,
            );
        }
    }
}

impl Drop for MemDc {
    fn drop(&mut self) {
        unsafe {
            if self.hdc != 0 {
                if self.prev_bmp != 0 {
                    f::SelectObject(self.hdc, self.prev_bmp);
                }
                f::DeleteDC(self.hdc);
            }
            if self.bmp != 0 {
                f::DeleteObject(self.bmp);
            }
        }
    }
}

pub fn fill_rect(dc: f::HDC, rect: &UiRect, color: u32) {
    unsafe {
        let brush = f::CreateSolidBrush(color);
        let wr = rect.to_win_rect();
        f::FillRect(dc, &wr, brush);
        f::DeleteObject(brush);
    }
}

pub fn fill_round_rect(
    dc: f::HDC,
    rect: &UiRect,
    radius: i32,
    fill_color: u32,
    border_color: Option<u32>,
) {
    unsafe {
        let brush = f::CreateSolidBrush(fill_color);
        let pen = if let Some(bc) = border_color {
            f::CreatePen(f::PS_SOLID, 1, bc)
        } else {
            f::CreatePen(f::PS_NULL, 0, 0)
        };
        let prev_brush = f::SelectObject(dc, brush);
        let prev_pen = f::SelectObject(dc, pen);

        f::RoundRect(
            dc,
            rect.x,
            rect.y,
            rect.x + rect.w,
            rect.y + rect.h,
            radius * 2,
            radius * 2,
        );

        f::SelectObject(dc, prev_brush);
        f::SelectObject(dc, prev_pen);
        f::DeleteObject(brush);
        f::DeleteObject(pen);
    }
}

pub fn draw_text(dc: f::HDC, text: &str, rect: &UiRect, color: u32, font: f::HGDIOBJ, flags: u32) {
    unsafe {
        let prev_font = f::SelectObject(dc, font);
        f::SetBkMode(dc, f::TRANSPARENT);
        f::SetTextColor(dc, color);
        let wide = f::wide(text);
        let mut wr = rect.to_win_rect();
        f::DrawTextW(dc, wide.as_ptr(), wide.len() as i32 - 1, &mut wr, flags);
        f::SelectObject(dc, prev_font);
    }
}

pub fn draw_pill(
    dc: f::HDC,
    rect: &UiRect,
    text: &str,
    bg_color: u32,
    text_color: u32,
    border_color: Option<u32>,
    font: f::HGDIOBJ,
) {
    fill_round_rect(dc, rect, rect.h / 2, bg_color, border_color);
    draw_text(
        dc,
        text,
        rect,
        text_color,
        font,
        f::DT_CENTER | f::DT_VCENTER | f::DT_SINGLELINE,
    );
}

#[allow(clippy::too_many_arguments)]
pub fn draw_button(
    dc: f::HDC,
    rect: &UiRect,
    text: &str,
    is_primary: bool,
    is_hover: bool,
    is_active: bool,
    theme: &Theme,
    fonts: &Fonts,
) {
    let (bg, border, text_color) = if is_primary {
        if is_active {
            (theme.accent_active, None, theme.text_on_accent)
        } else if is_hover {
            (theme.accent_hover, None, theme.text_on_accent)
        } else {
            (theme.accent, None, theme.text_on_accent)
        }
    } else if is_active {
        (
            theme.surface_active,
            Some(theme.border_active),
            theme.text_primary,
        )
    } else if is_hover {
        (theme.surface_hover, Some(theme.border), theme.text_primary)
    } else {
        (
            theme.surface_card,
            Some(theme.border_subtle),
            theme.text_secondary,
        )
    };

    fill_round_rect(dc, rect, 6, bg, border);
    draw_text(
        dc,
        text,
        rect,
        text_color,
        fonts.body_bold,
        f::DT_CENTER | f::DT_VCENTER | f::DT_SINGLELINE,
    );
}

#[allow(clippy::too_many_arguments)]
pub fn draw_checkbox(
    dc: f::HDC,
    x: i32,
    y: i32,
    w: i32,
    checked: bool,
    label: &str,
    is_hover: bool,
    theme: &Theme,
    fonts: &Fonts,
) -> UiRect {
    let box_size = 18;
    let box_rect = UiRect::new(x, y + 2, box_size, box_size);

    if checked {
        fill_round_rect(dc, &box_rect, 4, theme.accent, None);
        // Draw checkmark glyph
        draw_text(
            dc,
            "✓",
            &box_rect,
            theme.text_on_accent,
            fonts.body_bold,
            f::DT_CENTER | f::DT_VCENTER | f::DT_SINGLELINE,
        );
    } else {
        let border = if is_hover { theme.accent } else { theme.border };
        let bg = if is_hover {
            theme.surface_hover
        } else {
            theme.surface
        };
        fill_round_rect(dc, &box_rect, 4, bg, Some(border));
    }

    let text_rect = UiRect::new(x + box_size + 10, y, w - box_size - 10, 24);
    let text_color = if is_hover {
        theme.text_primary
    } else {
        theme.text_secondary
    };
    draw_text(
        dc,
        label,
        &text_rect,
        text_color,
        fonts.body,
        f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
    );

    UiRect::new(x, y, w, 24)
}

pub fn draw_card(dc: f::HDC, rect: &UiRect, is_hover: bool, is_selected: bool, theme: &Theme) {
    let bg = if is_selected || is_hover {
        theme.surface_hover
    } else {
        theme.surface_card
    };

    let border = if is_selected {
        Some(theme.border_active)
    } else if is_hover {
        Some(theme.border)
    } else {
        Some(theme.border_subtle)
    };

    fill_round_rect(dc, rect, 8, bg, border);
}

#[allow(clippy::too_many_arguments)]
pub fn draw_stat_card(
    dc: f::HDC,
    rect: &UiRect,
    title: &str,
    value: &str,
    subtitle: &str,
    accent_bar: Option<u32>,
    theme: &Theme,
    fonts: &Fonts,
) {
    draw_card(dc, rect, false, false, theme);

    if let Some(color) = accent_bar {
        let bar = UiRect::new(rect.x + 12, rect.y + 14, 4, rect.h - 28);
        fill_round_rect(dc, &bar, 2, color, None);
    }

    let offset_x = if accent_bar.is_some() { 24 } else { 16 };
    let title_rect = UiRect::new(rect.x + offset_x, rect.y + 12, rect.w - offset_x - 12, 18);
    draw_text(
        dc,
        title,
        &title_rect,
        theme.text_secondary,
        fonts.caption,
        f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
    );

    let val_rect = UiRect::new(rect.x + offset_x, rect.y + 30, rect.w - offset_x - 12, 28);
    draw_text(
        dc,
        value,
        &val_rect,
        theme.text_primary,
        fonts.section,
        f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
    );

    if !subtitle.is_empty() {
        let sub_rect = UiRect::new(rect.x + offset_x, rect.y + 58, rect.w - offset_x - 12, 18);
        draw_text(
            dc,
            subtitle,
            &sub_rect,
            theme.text_tertiary,
            fonts.caption,
            f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
        );
    }
}

pub fn draw_thumbnail(
    dc: f::HDC,
    rect: &UiRect,
    bitmap: f::HBITMAP,
    fallback_title: &str,
    is_video: bool,
    theme: &Theme,
    fonts: &Fonts,
) {
    if bitmap != 0 {
        unsafe {
            let src_dc = f::CreateCompatibleDC(dc);
            if src_dc != 0 {
                let prev = f::SelectObject(src_dc, bitmap);
                let mut bmp: f::BITMAP = f::zeroed();
                if f::GetObjectW(
                    bitmap,
                    std::mem::size_of::<f::BITMAP>() as i32,
                    (&mut bmp as *mut f::BITMAP).cast::<c_void>(),
                ) != 0
                    && bmp.bmWidth > 0
                    && bmp.bmHeight > 0
                {
                    f::SetStretchBltMode(dc, f::HALFTONE);
                    f::StretchBlt(
                        dc,
                        rect.x,
                        rect.y,
                        rect.w,
                        rect.h,
                        src_dc,
                        0,
                        0,
                        bmp.bmWidth,
                        bmp.bmHeight,
                        f::SRCCOPY,
                    );
                }
                f::SelectObject(src_dc, prev);
                f::DeleteDC(src_dc);
            }
        }
    } else {
        // Fallback procedural visual tile
        fill_round_rect(
            dc,
            rect,
            6,
            theme.surface_elevated,
            Some(theme.border_subtle),
        );

        let icon = if is_video { "🎬" } else { "🖼️" };
        let icon_rect = UiRect::new(rect.x, rect.y + rect.h / 2 - 20, rect.w, 24);
        draw_text(
            dc,
            icon,
            &icon_rect,
            theme.accent,
            fonts.section,
            f::DT_CENTER | f::DT_VCENTER | f::DT_SINGLELINE,
        );

        let title_rect = UiRect::new(rect.x + 8, rect.y + rect.h / 2 + 6, rect.w - 16, 20);
        draw_text(
            dc,
            fallback_title,
            &title_rect,
            theme.text_secondary,
            fonts.caption,
            f::DT_CENTER | f::DT_VCENTER | f::DT_SINGLELINE | f::DT_END_ELLIPSIS,
        );
    }

    // Subtle inset border
    unsafe {
        let pen = f::CreatePen(f::PS_SOLID, 1, theme.border_subtle);
        let brush = f::CreateSolidBrush(0);
        let prev_pen = f::SelectObject(dc, pen);
        let prev_brush = f::SelectObject(dc, brush);
        let wr = rect.to_win_rect();
        f::FrameRect(dc, &wr, pen);
        f::SelectObject(dc, prev_brush);
        f::SelectObject(dc, prev_pen);
        f::DeleteObject(pen);
        f::DeleteObject(brush);
    }
}

pub fn draw_progress_bar(
    dc: f::HDC,
    rect: &UiRect,
    percent: u8,
    label: &str,
    theme: &Theme,
    fonts: &Fonts,
) {
    fill_round_rect(dc, rect, 4, theme.surface_active, Some(theme.border_subtle));
    let fill_w = ((rect.w as f64 * (percent.min(100) as f64 / 100.0)).round() as i32).max(0);
    if fill_w > 0 {
        let fill_rect = UiRect::new(rect.x, rect.y, fill_w, rect.h);
        fill_round_rect(dc, &fill_rect, 4, theme.accent, None);
    }

    if !label.is_empty() {
        let label_rect = UiRect::new(rect.x + 8, rect.y, rect.w - 16, rect.h);
        draw_text(
            dc,
            label,
            &label_rect,
            theme.text_on_accent,
            fonts.caption,
            f::DT_CENTER | f::DT_VCENTER | f::DT_SINGLELINE,
        );
    }
}

pub fn draw_history_chart(
    dc: f::HDC,
    rect: &UiRect,
    cpu_data: &[f64],
    mem_data: &[f64],
    theme: &Theme,
    fonts: &Fonts,
) {
    fill_round_rect(dc, rect, 6, theme.surface_card, Some(theme.border_subtle));

    let pad_left = 40;
    let pad_bottom = 24;
    let pad_top = 16;
    let pad_right = 16;

    let chart_w = rect.w - pad_left - pad_right;
    let chart_h = rect.h - pad_top - pad_bottom;
    if chart_w < 50 || chart_h < 30 {
        return;
    }

    let origin_x = rect.x + pad_left;
    let origin_y = rect.y + pad_top + chart_h;

    // Draw horizontal grid lines (0%, 25%, 50%, 75%, 100%)
    for i in 0..=4 {
        let gy = origin_y - (chart_h * i) / 4;
        let line_rect = UiRect::new(origin_x, gy, chart_w, 1);
        fill_rect(dc, &line_rect, theme.border_subtle);

        let pct = i * 25;
        let label = format!("{pct}%");
        let lbl_rect = UiRect::new(rect.x + 4, gy - 8, pad_left - 8, 16);
        draw_text(
            dc,
            &label,
            &lbl_rect,
            theme.text_tertiary,
            fonts.caption,
            f::DT_RIGHT | f::DT_VCENTER | f::DT_SINGLELINE,
        );
    }

    // Draw CPU line
    if cpu_data.len() >= 2 {
        let count = cpu_data.len();
        let step = chart_w as f64 / (count - 1).max(1) as f64;
        let mut points: Vec<f::POINT> = Vec::with_capacity(count);

        for (idx, &val) in cpu_data.iter().enumerate() {
            let px = (origin_x as f64 + idx as f64 * step).round() as i32;
            let norm = (val / 10.0).clamp(0.0, 1.0); // 0 to 10% cpu capacity scale
            let py = origin_y - (norm * chart_h as f64).round() as i32;
            points.push(f::POINT { x: px, y: py });
        }

        unsafe {
            let pen = f::CreatePen(f::PS_SOLID, 2, theme.chart_cpu);
            let prev = f::SelectObject(dc, pen);
            f::Polyline(dc, points.as_ptr(), points.len() as i32);
            f::SelectObject(dc, prev);
            f::DeleteObject(pen);
        }
    }

    // Draw Memory line
    if mem_data.len() >= 2 {
        let count = mem_data.len();
        let step = chart_w as f64 / (count - 1).max(1) as f64;
        let mut points: Vec<f::POINT> = Vec::with_capacity(count);

        for (idx, &val) in mem_data.iter().enumerate() {
            let px = (origin_x as f64 + idx as f64 * step).round() as i32;
            let norm = (val / 300.0).clamp(0.0, 1.0); // 0 to 300 MB scale
            let py = origin_y - (norm * chart_h as f64).round() as i32;
            points.push(f::POINT { x: px, y: py });
        }

        unsafe {
            let pen = f::CreatePen(f::PS_SOLID, 2, theme.chart_mem);
            let prev = f::SelectObject(dc, pen);
            f::Polyline(dc, points.as_ptr(), points.len() as i32);
            f::SelectObject(dc, prev);
            f::DeleteObject(pen);
        }
    }

    // Legend
    let leg_y = rect.y + rect.h - 18;
    let dot1 = UiRect::new(origin_x, leg_y + 4, 8, 8);
    fill_round_rect(dc, &dot1, 4, theme.chart_cpu, None);
    let lbl1 = UiRect::new(origin_x + 12, leg_y, 90, 16);
    draw_text(
        dc,
        "CPU (0-10%)",
        &lbl1,
        theme.text_secondary,
        fonts.caption,
        f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
    );

    let dot2 = UiRect::new(origin_x + 110, leg_y + 4, 8, 8);
    fill_round_rect(dc, &dot2, 4, theme.chart_mem, None);
    let lbl2 = UiRect::new(origin_x + 122, leg_y, 110, 16);
    draw_text(
        dc,
        "RAM (0-300MB)",
        &lbl2,
        theme.text_secondary,
        fonts.caption,
        f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
    );
}

pub fn draw_toast(
    dc: f::HDC,
    win_w: i32,
    win_h: i32,
    message: &str,
    is_success: bool,
    theme: &Theme,
    fonts: &Fonts,
) {
    let toast_w = 360;
    let toast_h = 44;
    let toast_x = win_w - toast_w - 24;
    let toast_y = win_h - toast_h - 24;
    let rect = UiRect::new(toast_x, toast_y, toast_w, toast_h);

    let (bg, border, icon) = if is_success {
        (theme.surface_elevated, theme.success, "✓")
    } else {
        (theme.surface_elevated, theme.danger, "✕")
    };

    fill_round_rect(dc, &rect, 8, bg, Some(border));

    let icon_rect = UiRect::new(rect.x + 12, rect.y, 24, rect.h);
    let icon_color = if is_success {
        theme.success
    } else {
        theme.danger
    };
    draw_text(
        dc,
        icon,
        &icon_rect,
        icon_color,
        fonts.heading,
        f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
    );

    let msg_rect = UiRect::new(rect.x + 36, rect.y, rect.w - 48, rect.h);
    draw_text(
        dc,
        message,
        &msg_rect,
        theme.text_primary,
        fonts.body,
        f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE | f::DT_END_ELLIPSIS,
    );
}
