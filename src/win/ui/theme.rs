//! Theme configuration, Fluent Design palettes, and native font management.
use crate::win::ffi as f;
use std::ffi::c_void;

#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub is_dark: bool,
    pub bg: u32,
    pub surface: u32,
    pub surface_hover: u32,
    pub surface_active: u32,
    pub surface_card: u32,
    pub surface_elevated: u32,
    pub border: u32,
    pub border_subtle: u32,
    pub border_active: u32,
    pub accent: u32,
    pub accent_hover: u32,
    pub accent_active: u32,
    pub accent_subtle: u32,
    pub text_primary: u32,
    pub text_secondary: u32,
    pub text_tertiary: u32,
    pub text_on_accent: u32,
    pub success: u32,
    pub success_subtle: u32,
    pub warning: u32,
    pub warning_subtle: u32,
    pub danger: u32,
    pub danger_hover: u32,
    pub danger_subtle: u32,
    pub chart_cpu: u32,
    pub chart_mem: u32,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            is_dark: true,
            bg: f::rgb(18, 19, 22),      // #121316 Soft charcoal background
            surface: f::rgb(26, 27, 32), // #1A1B20 Slightly elevated surface
            surface_hover: f::rgb(35, 37, 44), // #23252C Hover surface
            surface_active: f::rgb(44, 47, 56), // #2C2F38 Active surface
            surface_card: f::rgb(28, 30, 36), // #1C1E24 Card surface
            surface_elevated: f::rgb(36, 38, 46), // #24262E Top bar / modals
            border: f::rgb(46, 49, 59),  // #2E313B Subtle border
            border_subtle: f::rgb(34, 36, 43), // #22242B Very subtle divider
            border_active: f::rgb(99, 102, 241), // #6366F1 Accent border
            accent: f::rgb(99, 102, 241), // #6366F1 Vibrant Indigo/Purple
            accent_hover: f::rgb(129, 140, 248), // #818CF8 Lighter indigo on hover
            accent_active: f::rgb(79, 70, 229), // #4F46E5 Darker indigo when pressed
            accent_subtle: f::rgb(35, 36, 68), // Dark purple/indigo wash
            text_primary: f::rgb(243, 244, 246), // #F3F4F6 Crisp white-gray text
            text_secondary: f::rgb(156, 163, 175), // #9CA3AF Muted gray description
            text_tertiary: f::rgb(107, 114, 128), // #6B7280 Dim helper text
            text_on_accent: f::rgb(255, 255, 255), // Clean white
            success: f::rgb(16, 185, 129), // #10B981 Emerald green
            success_subtle: f::rgb(20, 50, 40),
            warning: f::rgb(245, 158, 11), // #F59E0B Amber
            warning_subtle: f::rgb(60, 45, 15),
            danger: f::rgb(239, 68, 68), // #EF4444 Rose red
            danger_hover: f::rgb(220, 38, 38),
            danger_subtle: f::rgb(60, 20, 20),
            chart_cpu: f::rgb(99, 102, 241), // Indigo for CPU
            chart_mem: f::rgb(16, 185, 129), // Green for Memory
        }
    }

    pub fn light() -> Self {
        Self {
            is_dark: false,
            bg: f::rgb(243, 244, 246),      // #F3F4F6 Light gray background
            surface: f::rgb(255, 255, 255), // #FFFFFF Pure white surface
            surface_hover: f::rgb(241, 243, 247), // #F1F3F7 Soft hover
            surface_active: f::rgb(229, 231, 235),
            surface_card: f::rgb(255, 255, 255),
            surface_elevated: f::rgb(255, 255, 255),
            border: f::rgb(222, 226, 230), // #DEE2E6 Crisp light border
            border_subtle: f::rgb(235, 238, 242),
            border_active: f::rgb(79, 70, 229),
            accent: f::rgb(79, 70, 229), // #4F46E5 Deep indigo
            accent_hover: f::rgb(99, 102, 241),
            accent_active: f::rgb(67, 56, 202),
            accent_subtle: f::rgb(238, 242, 255), // Soft blue/indigo tint
            text_primary: f::rgb(17, 24, 39),     // #111827 Charcoal black
            text_secondary: f::rgb(75, 85, 99),   // #4B5563 Slate gray
            text_tertiary: f::rgb(156, 163, 175),
            text_on_accent: f::rgb(255, 255, 255),
            success: f::rgb(16, 185, 129),
            success_subtle: f::rgb(236, 253, 245),
            warning: f::rgb(217, 119, 6),
            warning_subtle: f::rgb(254, 243, 199),
            danger: f::rgb(220, 38, 38),
            danger_hover: f::rgb(185, 28, 28),
            danger_subtle: f::rgb(254, 226, 226),
            chart_cpu: f::rgb(79, 70, 229),
            chart_mem: f::rgb(16, 185, 129),
        }
    }

    pub fn apply_to_window(&self, hwnd: f::HWND) {
        let mode: i32 = if self.is_dark { 1 } else { 0 };
        unsafe {
            f::DwmSetWindowAttribute(
                hwnd,
                f::DWMWA_USE_IMMERSIVE_DARK_MODE,
                &mode as *const i32 as *const c_void,
                std::mem::size_of::<i32>() as u32,
            );
        }
    }
}

pub struct Fonts {
    pub title: f::HGDIOBJ,     // 20pt bold
    pub section: f::HGDIOBJ,   // 14pt semi-bold
    pub heading: f::HGDIOBJ,   // 11pt semi-bold
    pub body: f::HGDIOBJ,      // 10pt regular
    pub body_bold: f::HGDIOBJ, // 10pt bold
    pub caption: f::HGDIOBJ,   // 8.5pt medium
    pub mono: f::HGDIOBJ,      // 10pt monospace
}

impl Fonts {
    pub fn new(dpi: i32) -> Self {
        let scale = |pt: i32| (pt * dpi) / 96;
        let segoe = f::wide("Segoe UI Variable Text");
        let fallback = f::wide("Segoe UI");
        let mono_face = f::wide("Cascadia Code");
        let mono_fallback = f::wide("Consolas");

        let make = |height: i32, weight: i32, face: &[u16], alt: &[u16]| -> f::HGDIOBJ {
            let h = unsafe {
                f::CreateFontW(
                    -scale(height),
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
                    5, // CLEARTYPE_QUALITY
                    0,
                    face.as_ptr(),
                )
            };
            if h != 0 {
                h
            } else {
                unsafe {
                    f::CreateFontW(
                        -scale(height),
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
                        5,
                        0,
                        alt.as_ptr(),
                    )
                }
            }
        };

        Self {
            title: make(24, 700, &segoe, &fallback),
            section: make(16, 600, &segoe, &fallback),
            heading: make(13, 600, &segoe, &fallback),
            body: make(11, 400, &segoe, &fallback),
            body_bold: make(11, 600, &segoe, &fallback),
            caption: make(9, 500, &segoe, &fallback),
            mono: make(11, 600, &mono_face, &mono_fallback),
        }
    }
}

impl Drop for Fonts {
    fn drop(&mut self) {
        unsafe {
            if self.title != 0 {
                f::DeleteObject(self.title);
            }
            if self.section != 0 {
                f::DeleteObject(self.section);
            }
            if self.heading != 0 {
                f::DeleteObject(self.heading);
            }
            if self.body != 0 {
                f::DeleteObject(self.body);
            }
            if self.body_bold != 0 {
                f::DeleteObject(self.body_bold);
            }
            if self.caption != 0 {
                f::DeleteObject(self.caption);
            }
            if self.mono != 0 {
                f::DeleteObject(self.mono);
            }
        }
    }
}
