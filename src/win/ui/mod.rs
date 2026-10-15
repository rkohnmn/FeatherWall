//! Complete redesigned native Fluent UI shell, navigation, and views for FeatherWall.
pub mod draw;
pub mod events;
pub mod theme;
pub mod views;

use self::theme::{Fonts, Theme};
use crate::core::{LibraryItem, MediaKind};
use crate::win::desktop;
use crate::win::ffi as f;
pub use events::UiAction;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Instant;
pub use views::ViewContext;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavTab {
    Home = 0,
    Library = 1,
    Displays = 2,
    Optimization = 3,
    Performance = 4,
    Widgets = 5,
    Settings = 6,
}

impl NavTab {
    pub const ALL: [Self; 7] = [
        Self::Home,
        Self::Library,
        Self::Displays,
        Self::Optimization,
        Self::Performance,
        Self::Widgets,
        Self::Settings,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Self::Home => "Home Dashboard",
            Self::Library => "Wallpaper Library",
            Self::Displays => "Display Management",
            Self::Optimization => "Optimization Engine",
            Self::Performance => "Performance & Diagnostics",
            Self::Widgets => "Desktop Widgets",
            Self::Settings => "Application Settings",
        }
    }

    pub fn subtitle(self) -> &'static str {
        match self {
            Self::Home => "Overview of active playback, quick controls, and engine status",
            Self::Library => {
                "Browse, preview, filter, and organize your animated wallpaper collection"
            }
            Self::Displays => "Multi-monitor geometry, display assignments, and scaling modes",
            Self::Optimization => {
                "Encoding profiles, hardware tuning, and resource efficiency policies"
            }
            Self::Performance => {
                "Real-time resource utilization, Media Foundation metrics, and logs"
            }
            Self::Widgets => {
                "Floating clock, date, intelligent greetings, and music overlay"
            }
            Self::Settings => {
                "General options, autostart, battery behavior, and storage management"
            }
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Self::Home => "⌂",
            Self::Library => "▦",
            Self::Displays => "🖥",
            Self::Optimization => "⚡",
            Self::Performance => "📈",
            Self::Widgets => "⏱",
            Self::Settings => "⚙",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Home => "Home",
            Self::Library => "Library",
            Self::Displays => "Displays",
            Self::Optimization => "Optimization",
            Self::Performance => "Performance",
            Self::Widgets => "Widgets",
            Self::Settings => "Settings",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibraryFilter {
    All,
    Videos,
    Stills,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibrarySort {
    Name,
    Date,
    Size,
}

pub struct UiState {
    pub current_tab: NavTab,
    pub theme: Theme,
    pub fonts: Rc<Fonts>,
    pub dpi: i32,
    pub search_query: String,
    pub search_focused: bool,
    pub filter: LibraryFilter,
    pub sort: LibrarySort,
    pub selected_item_id: Option<String>,
    pub selected_monitor_idx: usize,
    pub library_scroll_y: i32,
    pub hovered_nav: Option<NavTab>,
    pub hovered_item_idx: Option<usize>,
    pub hovered_btn_id: Option<&'static str>,
    pub cpu_history: Vec<f64>,
    pub mem_history: Vec<f64>,
    pub last_perf_sample: Option<Instant>,
    pub toast: Option<(String, bool, Instant)>,
    pub confirm_delete: Option<String>, // item ID to delete
    pub thumb_cache: HashMap<String, f::HBITMAP>,
    pub recent_logs: Vec<String>,
    pub cache_size_str: String,
    pub drag_hover: bool,
}

impl UiState {
    pub fn new(dark_theme: bool) -> Self {
        let dpi = 96; // default 96 DPI, updated on window creation
        let theme = if dark_theme {
            Theme::dark()
        } else {
            Theme::light()
        };
        let fonts = Rc::new(Fonts::new(dpi));
        Self {
            current_tab: NavTab::Home,
            theme,
            fonts,
            dpi,
            search_query: String::new(),
            search_focused: false,
            filter: LibraryFilter::All,
            sort: LibrarySort::Date,
            selected_item_id: None,
            selected_monitor_idx: 0,
            library_scroll_y: 0,
            hovered_nav: None,
            hovered_item_idx: None,
            hovered_btn_id: None,
            cpu_history: Vec::with_capacity(30),
            mem_history: Vec::with_capacity(30),
            last_perf_sample: None,
            toast: None,
            confirm_delete: None,
            thumb_cache: HashMap::new(),
            recent_logs: Vec::new(),
            cache_size_str: "Calculating...".into(),
            drag_hover: false,
        }
    }

    pub fn set_dark_theme(&mut self, dark: bool, hwnd: f::HWND) {
        self.theme = if dark { Theme::dark() } else { Theme::light() };
        self.theme.apply_to_window(hwnd);
        unsafe {
            f::InvalidateRect(hwnd, std::ptr::null(), 1);
        }
    }

    pub fn show_toast(&mut self, message: &str, is_success: bool) {
        self.toast = Some((message.to_string(), is_success, Instant::now()));
    }

    pub fn get_or_load_thumb(&mut self, item: &LibraryItem) -> f::HBITMAP {
        if let Some(&bmp) = self.thumb_cache.get(&item.id) {
            return bmp;
        }
        let thumb_path = item.thumb_path();
        if thumb_path.is_file() {
            if let Ok(bmp) = desktop::load_still(&thumb_path) {
                self.thumb_cache.insert(item.id.clone(), bmp);
                return bmp;
            }
        }
        // Fallback: try loading asset directly if it's still BMP
        if item.kind == MediaKind::Still && item.asset.is_file() {
            if let Ok(bmp) = desktop::load_still(&item.asset) {
                self.thumb_cache.insert(item.id.clone(), bmp);
                return bmp;
            }
        }
        0
    }

    pub fn clear_thumb_cache(&mut self) {
        for (_, bmp) in self.thumb_cache.drain() {
            if bmp != 0 {
                unsafe {
                    f::DeleteObject(bmp);
                }
            }
        }
    }
}

impl Drop for UiState {
    fn drop(&mut self) {
        self.clear_thumb_cache();
    }
}
