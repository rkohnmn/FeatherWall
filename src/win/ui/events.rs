//! Event handling and interaction logic for FeatherWall Fluent UI.
use super::draw::UiRect;
use super::{LibraryFilter, NavTab, UiState};
use crate::core::{DateFormat, GreetingMode, LibraryItem, MediaKind, Profile, WidgetDisplayTarget, WidgetPosition};
use crate::win::desktop::Monitor;

#[derive(Clone, Debug, PartialEq)]
pub enum UiAction {
    None,
    ImportFile,
    TogglePause,
    Resume,
    Pause,
    Stop,
    ApplySelected,
    ApplyToMonitor(String),
    ClearMonitor(String),
    ReoptimizeSelected,
    ShowFileInExplorer(PathBuf),
    RemoveItem(String),
    CancelWorker,
    SetProfile(Profile),
    ToggleAutostart,
    ToggleSetting(&'static str),
    CleanCache,
    OpenCacheFolder,
    OpenLogFile,
    ToggleTheme,
    // Widget Overlay Actions
    ToggleWidgetsMaster,
    ToggleClock,
    ToggleClock24h,
    ToggleClockSeconds,
    SetClockSize(u32),
    SetClockColor(u32),
    SetClockOpacity(u8),
    ToggleDate,
    SetDateFormat(DateFormat),
    SetDateSize(u32),
    ToggleGreeting,
    SetGreetingMode(GreetingMode),
    SetGreetingInterval(u32),
    ToggleMusic,
    ToggleMusicSpotifyPreferred,
    ToggleMusicGeneralMedia,
    SetWidgetPosition(WidgetPosition),
    AdjustWidgetOffsetX(i32),
    AdjustWidgetOffsetY(i32),
    ToggleTextShadow,
    SetWidgetDisplayTarget(WidgetDisplayTarget),
    ToggleWidgetEditMode,
    OpenSpotifyApp,
}

use std::path::PathBuf;

pub fn handle_mouse_move(
    ui: &mut UiState,
    px: i32,
    py: i32,
    win_w: i32,
    win_h: i32,
    items: &[LibraryItem],
    monitors: &[Monitor],
) -> bool {
    let prev_hover_nav = ui.hovered_nav;
    let prev_hover_btn = ui.hovered_btn_id;
    let prev_hover_item = ui.hovered_item_idx;

    ui.hovered_nav = None;
    ui.hovered_btn_id = None;
    ui.hovered_item_idx = None;

    // 1. Check sidebar nav items (X: 12..208)
    if (12..=208).contains(&px) {
        let nav_y_start = 68;
        let nav_h = 38;
        let nav_spacing = 4;
        for (idx, &tab) in NavTab::ALL.iter().enumerate() {
            let item_y = nav_y_start + (idx as i32) * (nav_h + nav_spacing);
            if py >= item_y && py < item_y + nav_h {
                ui.hovered_nav = Some(tab);
                break;
            }
        }
    }

    // 2. Check top bar buttons (Y: 0..60)
    if py <= 60 {
        let mut right = win_w - 24;
        let import_rect = UiRect::new(right - 148, 14, 148, 32);
        if import_rect.contains(px, py) {
            ui.hovered_btn_id = Some("top_import");
        }
        right -= 148 + 10;

        let theme_rect = UiRect::new(right - 82, 14, 82, 32);
        if theme_rect.contains(px, py) {
            ui.hovered_btn_id = Some("top_theme");
        }
    }

    // 3. Modal buttons hover
    if ui.confirm_delete.is_some() {
        let modal_w = 420;
        let modal_h = 190;
        let modal_x = (win_w - modal_w) / 2;
        let modal_y = (win_h - modal_h) / 2;

        let btn_cancel = UiRect::new(modal_x + modal_w - 220, modal_y + modal_h - 48, 95, 32);
        if btn_cancel.contains(px, py) {
            ui.hovered_btn_id = Some("modal_cancel");
        }

        let btn_delete = UiRect::new(modal_x + modal_w - 115, modal_y + modal_h - 48, 95, 32);
        if btn_delete.contains(px, py) {
            ui.hovered_btn_id = Some("modal_confirm_delete");
        }
        return prev_hover_btn != ui.hovered_btn_id;
    }

    // 4. Content Area Hovers
    let sidebar_w = 220;
    let content_x = sidebar_w + 24;
    let content_y = 60 + 20;
    let content_w = win_w - content_x - 24;
    let content_h = win_h - content_y - 16;
    let content_rect = UiRect::new(content_x, content_y, content_w, content_h);

    match ui.current_tab {
        NavTab::Home => {
            let hero_rect = UiRect::new(content_rect.x, content_rect.y, content_rect.w, 210);
            let details_x = hero_rect.x + 260 + 44;
            let ctrl_y = hero_rect.y + 138;

            let btn_pause = UiRect::new(details_x, ctrl_y, 145, 34);
            if btn_pause.contains(px, py) {
                ui.hovered_btn_id = Some("home_pause");
            }
            let btn_stop = UiRect::new(details_x + 155, ctrl_y, 100, 34);
            if btn_stop.contains(px, py) {
                ui.hovered_btn_id = Some("home_stop");
            }
            let btn_lib = UiRect::new(details_x + 265, ctrl_y, 140, 34);
            if btn_lib.contains(px, py) {
                ui.hovered_btn_id = Some("home_open_library");
            }

            // Recent items row hover
            let strip_y = content_rect.y + 210 + 18 + 88 + 20;
            let items_row_y = strip_y + 26;
            let item_w = 175;
            let item_h = 130;
            for idx in 0..items.len().min(4) {
                let card_x = content_rect.x + (idx as i32) * (item_w + 14);
                let r = UiRect::new(card_x, items_row_y, item_w, item_h);
                if r.contains(px, py) {
                    ui.hovered_item_idx = Some(idx);
                    break;
                }
            }
        }
        NavTab::Library => {
            // Filter buttons
            let f_all = UiRect::new(content_rect.x, content_rect.y, 76, 28);
            if f_all.contains(px, py) {
                ui.hovered_btn_id = Some("filter_all");
            }
            let f_vid = UiRect::new(content_rect.x + 84, content_rect.y, 76, 28);
            if f_vid.contains(px, py) {
                ui.hovered_btn_id = Some("filter_videos");
            }
            let f_still = UiRect::new(content_rect.x + 168, content_rect.y, 76, 28);
            if f_still.contains(px, py) {
                ui.hovered_btn_id = Some("filter_stills");
            }

            let detail_w = 285;
            let grid_w = content_rect.w - detail_w - 18;
            let grid_top = content_rect.y + 28 + 14;

            // Grid cards hover
            let card_gap = 14;
            let cols = if grid_w > 540 { 3 } else { 2 };
            let card_w = (grid_w - (cols - 1) * card_gap) / cols;
            let card_h = 195;

            for idx in 0..items.len() {
                let col = (idx as i32) % cols;
                let row = (idx as i32) / cols;
                let card_x = content_rect.x + col * (card_w + card_gap);
                let card_y = grid_top + row * (card_h + card_gap) - ui.library_scroll_y;

                if card_x <= px && px < card_x + card_w && card_y <= py && py < card_y + card_h {
                    ui.hovered_item_idx = Some(idx);
                    break;
                }
            }

            // Detail pane button hovers
            let detail_x = content_rect.x + grid_w + 18;
            let p_x = detail_x + 16;
            let p_w = detail_w - 32;
            let prev_h = 145;
            let mut row_y = grid_top + 16 + prev_h + 12 + 30 + 20 * 5 + 36;

            let mon_btn = UiRect::new(p_x, row_y, p_w, 28);
            if mon_btn.contains(px, py) {
                ui.hovered_btn_id = Some("lib_cycle_monitor");
            }
            row_y += 36;
            let btn_apply = UiRect::new(p_x, row_y, p_w, 34);
            if btn_apply.contains(px, py) {
                ui.hovered_btn_id = Some("lib_apply");
            }
            row_y += 40;
            let btn_reopt = UiRect::new(p_x, row_y, (p_w - 8) / 2, 28);
            if btn_reopt.contains(px, py) {
                ui.hovered_btn_id = Some("lib_reoptimize");
            }
            let btn_reveal = UiRect::new(p_x + (p_w - 8) / 2 + 8, row_y, (p_w - 8) / 2, 28);
            if btn_reveal.contains(px, py) {
                ui.hovered_btn_id = Some("lib_reveal");
            }
            row_y += 34;
            let btn_remove = UiRect::new(p_x, row_y, p_w, 28);
            if btn_remove.contains(px, py) {
                ui.hovered_btn_id = Some("lib_remove");
            }
        }
        NavTab::Displays => {
            let cards_top = content_rect.y + 32;
            let _card_w = (content_rect.w - 18).clamp(320, 480);
            let card_h = 160;

            for (idx, _) in monitors.iter().enumerate() {
                let card_y = cards_top + (idx as i32) * (card_h + 16);
                let btn_y = card_y + 116;
                let btn_apply = UiRect::new(content_rect.x + 16, btn_y, 160, 30);
                if btn_apply.contains(px, py) {
                    ui.hovered_btn_id = Some(if idx == 0 {
                        "disp_apply_0"
                    } else {
                        "disp_apply_1"
                    });
                }
                let btn_clear = UiRect::new(content_rect.x + 184, btn_y, 110, 30);
                if btn_clear.contains(px, py) {
                    ui.hovered_btn_id = Some(if idx == 0 {
                        "disp_clear_0"
                    } else {
                        "disp_clear_1"
                    });
                }
            }
        }
        NavTab::Optimization => {
            let grid_y = content_rect.y + 32;
            let card_gap = 16;
            let card_w = (content_rect.w - card_gap) / 2;
            let card_h = 120;

            let c1 = UiRect::new(content_rect.x, grid_y, card_w, card_h);
            if c1.contains(px, py) {
                ui.hovered_btn_id = Some("prof_eco");
            }
            let c2 = UiRect::new(content_rect.x + card_w + card_gap, grid_y, card_w, card_h);
            if c2.contains(px, py) {
                ui.hovered_btn_id = Some("prof_balanced");
            }
            let c3 = UiRect::new(content_rect.x, grid_y + card_h + card_gap, card_w, card_h);
            if c3.contains(px, py) {
                ui.hovered_btn_id = Some("prof_quality");
            }
            let c4 = UiRect::new(
                content_rect.x + card_w + card_gap,
                grid_y + card_h + card_gap,
                card_w,
                card_h,
            );
            if c4.contains(px, py) {
                ui.hovered_btn_id = Some("prof_auto");
            }

            let tune_y = grid_y + card_h * 2 + card_gap + 16;
            let reopt_rect =
                UiRect::new(content_rect.x + content_rect.w - 230, tune_y + 62, 210, 32);
            if reopt_rect.contains(px, py) {
                ui.hovered_btn_id = Some("opt_reoptimize");
            }
        }
        NavTab::Performance => {
            let diag_y = content_rect.y + 88 + 16 + 165 + 16;
            let btn_open_log =
                UiRect::new(content_rect.x + content_rect.w - 145, diag_y + 10, 130, 26);
            if btn_open_log.contains(px, py) {
                ui.hovered_btn_id = Some("perf_open_log");
            }
        }
        NavTab::Settings => {
            let card_w = (content_rect.w - 16) / 2;
            let theme_btn = UiRect::new(content_rect.x + 160, content_rect.y + 78, 120, 28);
            if theme_btn.contains(px, py) {
                ui.hovered_btn_id = Some("set_theme_toggle");
            }

            let row2_y = content_rect.y + 240 + 16;
            let btn_clean = UiRect::new(content_rect.x + 18, row2_y + 120, 140, 30);
            if btn_clean.contains(px, py) {
                ui.hovered_btn_id = Some("set_clean_cache");
            }
            let btn_folder = UiRect::new(content_rect.x + 168, row2_y + 120, 150, 30);
            if btn_folder.contains(px, py) {
                ui.hovered_btn_id = Some("set_open_folder");
            }

            let right_card_x = content_rect.x + card_w + 16;
            let btn_log = UiRect::new(right_card_x + 18, row2_y + 120, 140, 30);
            if btn_log.contains(px, py) {
                ui.hovered_btn_id = Some("set_open_log");
            }
        }
        NavTab::Widgets => {
            let preview_h = 135;
            let grid_y = content_rect.y + preview_h + 14;
            let card_gap = 14;
            let card_w = (content_rect.w - card_gap) / 2;
            let c_left = UiRect::new(content_rect.x, grid_y, card_w, 325);
            let c_right = UiRect::new(content_rect.x + card_w + card_gap, grid_y, card_w, 325);

            // Left Card: Clock & Date
            let mut ly = c_left.y + 42;
            if UiRect::new(c_left.x + 18, ly, card_w - 36, 24).contains(px, py) {
                ui.hovered_btn_id = Some("w_master_toggle");
            }
            ly += 26;
            if UiRect::new(c_left.x + 18, ly, card_w - 36, 24).contains(px, py) {
                ui.hovered_btn_id = Some("w_clock_toggle");
            }
            ly += 28;
            if UiRect::new(c_left.x + 18, ly, 100, 26).contains(px, py) {
                ui.hovered_btn_id = Some("w_clock_24h");
            } else if UiRect::new(c_left.x + 124, ly, 105, 26).contains(px, py) {
                ui.hovered_btn_id = Some("w_clock_12h");
            } else if UiRect::new(c_left.x + 235, ly, 105, 26).contains(px, py) {
                ui.hovered_btn_id = Some("w_clock_sec_toggle");
            }
            ly += 32;
            if UiRect::new(c_left.x + 95, ly, 55, 24).contains(px, py) {
                ui.hovered_btn_id = Some("w_size_48");
            } else if UiRect::new(c_left.x + 155, ly, 55, 24).contains(px, py) {
                ui.hovered_btn_id = Some("w_size_64");
            } else if UiRect::new(c_left.x + 215, ly, 55, 24).contains(px, py) {
                ui.hovered_btn_id = Some("w_size_72");
            } else if UiRect::new(c_left.x + 275, ly, 55, 24).contains(px, py) {
                ui.hovered_btn_id = Some("w_size_96");
            }
            ly += 30;
            if UiRect::new(c_left.x + 95, ly, 55, 24).contains(px, py) {
                ui.hovered_btn_id = Some("w_color_white");
            } else if UiRect::new(c_left.x + 155, ly, 55, 24).contains(px, py) {
                ui.hovered_btn_id = Some("w_color_silver");
            } else if UiRect::new(c_left.x + 215, ly, 55, 24).contains(px, py) {
                ui.hovered_btn_id = Some("w_color_gold");
            } else if UiRect::new(c_left.x + 275, ly, 55, 24).contains(px, py) {
                ui.hovered_btn_id = Some("w_color_indigo");
            }
            ly += 30;
            if UiRect::new(c_left.x + 18, ly, card_w - 36, 24).contains(px, py) {
                ui.hovered_btn_id = Some("w_date_toggle");
            }
            ly += 26;
            if UiRect::new(c_left.x + 18, ly, 75, 24).contains(px, py) {
                ui.hovered_btn_id = Some("w_date_full");
            } else if UiRect::new(c_left.x + 98, ly, 80, 24).contains(px, py) {
                ui.hovered_btn_id = Some("w_date_monthday");
            } else if UiRect::new(c_left.x + 183, ly, 75, 24).contains(px, py) {
                ui.hovered_btn_id = Some("w_date_dot");
            } else if UiRect::new(c_left.x + 263, ly, 68, 24).contains(px, py) {
                ui.hovered_btn_id = Some("w_date_numeric");
            }

            // Right Card: Greetings, Music & Position
            let mut ry = c_right.y + 42;
            if UiRect::new(c_right.x + 18, ry, card_w - 36, 24).contains(px, py) {
                ui.hovered_btn_id = Some("w_greet_toggle");
            }
            ry += 26;
            if UiRect::new(c_right.x + 18, ry, 100, 24).contains(px, py) {
                ui.hovered_btn_id = Some("w_greet_time");
            } else if UiRect::new(c_right.x + 124, ry, 100, 24).contains(px, py) {
                ui.hovered_btn_id = Some("w_greet_rotating");
            } else if UiRect::new(c_right.x + 230, ry, 95, 24).contains(px, py) {
                ui.hovered_btn_id = Some("w_greet_fixed");
            }
            ry += 30;
            if UiRect::new(c_right.x + 18, ry, card_w - 36, 24).contains(px, py) {
                ui.hovered_btn_id = Some("w_music_toggle");
            }
            ry += 26;
            if UiRect::new(c_right.x + 18, ry, 110, 26).contains(px, py) {
                ui.hovered_btn_id = Some("w_music_spotify");
            } else if UiRect::new(c_right.x + 134, ry, 110, 26).contains(px, py) {
                ui.hovered_btn_id = Some("w_music_general");
            } else if UiRect::new(c_right.x + 250, ry, 90, 26).contains(px, py) {
                ui.hovered_btn_id = Some("w_open_spotify");
            }
            ry += 32;
            if UiRect::new(c_right.x + 95, ry, 75, 24).contains(px, py) {
                ui.hovered_btn_id = Some("w_pos_tr");
            } else if UiRect::new(c_right.x + 175, ry, 75, 24).contains(px, py) {
                ui.hovered_btn_id = Some("w_pos_tl");
            } else if UiRect::new(c_right.x + 255, ry, 75, 24).contains(px, py) {
                ui.hovered_btn_id = Some("w_pos_c");
            }
            ry += 28;
            if UiRect::new(c_right.x + 95, ry, 34, 24).contains(px, py) {
                ui.hovered_btn_id = Some("w_offset_x_minus");
            } else if UiRect::new(c_right.x + 133, ry, 34, 24).contains(px, py) {
                ui.hovered_btn_id = Some("w_offset_x_plus");
            } else if UiRect::new(c_right.x + 175, ry, 34, 24).contains(px, py) {
                ui.hovered_btn_id = Some("w_offset_y_minus");
            } else if UiRect::new(c_right.x + 213, ry, 34, 24).contains(px, py) {
                ui.hovered_btn_id = Some("w_offset_y_plus");
            } else if UiRect::new(c_right.x + 255, ry, 75, 24).contains(px, py) {
                ui.hovered_btn_id = Some("w_shadow_toggle");
            }
            ry += 30;
            if UiRect::new(c_right.x + 18, ry, card_w - 36, 30).contains(px, py) {
                ui.hovered_btn_id = Some("w_edit_mode");
            }
        }
    }

    prev_hover_nav != ui.hovered_nav
        || prev_hover_btn != ui.hovered_btn_id
        || prev_hover_item != ui.hovered_item_idx
}

pub fn handle_click(
    ui: &mut UiState,
    px: i32,
    py: i32,
    win_w: i32,
    win_h: i32,
    items: &[LibraryItem],
    monitors: &[Monitor],
) -> UiAction {
    // 1. Modal clicks intercept everything
    if let Some(id) = ui.confirm_delete.clone() {
        let modal_w = 420;
        let modal_h = 190;
        let modal_x = (win_w - modal_w) / 2;
        let modal_y = (win_h - modal_h) / 2;

        let btn_cancel = UiRect::new(modal_x + modal_w - 220, modal_y + modal_h - 48, 95, 32);
        if btn_cancel.contains(px, py) {
            ui.confirm_delete = None;
            return UiAction::None;
        }

        let btn_delete = UiRect::new(modal_x + modal_w - 115, modal_y + modal_h - 48, 95, 32);
        if btn_delete.contains(px, py) {
            ui.confirm_delete = None;
            return UiAction::RemoveItem(id);
        }
        return UiAction::None;
    }

    // 2. Sidebar Navigation Click
    if (12..=208).contains(&px) {
        let nav_y_start = 68;
        let nav_h = 38;
        let nav_spacing = 4;
        for (idx, &tab) in NavTab::ALL.iter().enumerate() {
            let item_y = nav_y_start + (idx as i32) * (nav_h + nav_spacing);
            if py >= item_y && py < item_y + nav_h {
                ui.current_tab = tab;
                return UiAction::None;
            }
        }
    }

    // 3. Top bar clicks
    if py <= 60 {
        let mut right = win_w - 24;
        let import_rect = UiRect::new(right - 148, 14, 148, 32);
        if import_rect.contains(px, py) {
            return UiAction::ImportFile;
        }
        right -= 148 + 10;

        let theme_rect = UiRect::new(right - 82, 14, 82, 32);
        if theme_rect.contains(px, py) {
            return UiAction::ToggleTheme;
        }
    }

    // 4. Content Tab Clicks
    let sidebar_w = 220;
    let content_x = sidebar_w + 24;
    let content_y = 60 + 20;
    let content_w = win_w - content_x - 24;
    let content_h = win_h - content_y - 16;
    let content_rect = UiRect::new(content_x, content_y, content_w, content_h);

    match ui.current_tab {
        NavTab::Home => {
            let hero_rect = UiRect::new(content_rect.x, content_rect.y, content_rect.w, 210);
            let details_x = hero_rect.x + 260 + 44;
            let ctrl_y = hero_rect.y + 138;

            let btn_pause = UiRect::new(details_x, ctrl_y, 145, 34);
            if btn_pause.contains(px, py) {
                return UiAction::TogglePause;
            }
            let btn_stop = UiRect::new(details_x + 155, ctrl_y, 100, 34);
            if btn_stop.contains(px, py) {
                return UiAction::Stop;
            }
            let btn_lib = UiRect::new(details_x + 265, ctrl_y, 140, 34);
            if btn_lib.contains(px, py) {
                ui.current_tab = NavTab::Library;
                return UiAction::None;
            }

            // Recent item click
            let strip_y = content_rect.y + 210 + 18 + 88 + 20;
            let items_row_y = strip_y + 26;
            let item_w = 175;
            let item_h = 130;
            for (idx, item) in items.iter().take(4).enumerate() {
                let card_x = content_rect.x + (idx as i32) * (item_w + 14);
                let r = UiRect::new(card_x, items_row_y, item_w, item_h);
                if r.contains(px, py) {
                    ui.selected_item_id = Some(item.id.clone());
                    return UiAction::ApplySelected;
                }
            }
        }
        NavTab::Library => {
            // Filters
            let f_all = UiRect::new(content_rect.x, content_rect.y, 76, 28);
            if f_all.contains(px, py) {
                ui.filter = LibraryFilter::All;
                return UiAction::None;
            }
            let f_vid = UiRect::new(content_rect.x + 84, content_rect.y, 76, 28);
            if f_vid.contains(px, py) {
                ui.filter = LibraryFilter::Videos;
                return UiAction::None;
            }
            let f_still = UiRect::new(content_rect.x + 168, content_rect.y, 76, 28);
            if f_still.contains(px, py) {
                ui.filter = LibraryFilter::Stills;
                return UiAction::None;
            }

            let detail_w = 285;
            let grid_w = content_rect.w - detail_w - 18;
            let grid_top = content_rect.y + 28 + 14;

            // Grid card click
            let card_gap = 14;
            let cols = if grid_w > 540 { 3 } else { 2 };
            let card_w = (grid_w - (cols - 1) * card_gap) / cols;
            let card_h = 195;

            let filtered_items: Vec<&LibraryItem> = items
                .iter()
                .filter(|item| match ui.filter {
                    LibraryFilter::All => true,
                    LibraryFilter::Videos => item.kind == MediaKind::Video,
                    LibraryFilter::Stills => item.kind == MediaKind::Still,
                })
                .collect();

            for (idx, item) in filtered_items.iter().enumerate() {
                let col = (idx as i32) % cols;
                let row = (idx as i32) / cols;
                let card_x = content_rect.x + col * (card_w + card_gap);
                let card_y = grid_top + row * (card_h + card_gap) - ui.library_scroll_y;

                if card_x <= px && px < card_x + card_w && card_y <= py && py < card_y + card_h {
                    ui.selected_item_id = Some(item.id.clone());
                    return UiAction::None;
                }
            }

            // Detail pane button clicks
            let detail_x = content_rect.x + grid_w + 18;
            let p_x = detail_x + 16;
            let p_w = detail_w - 32;
            let prev_h = 145;
            let mut row_y = grid_top + 16 + prev_h + 12 + 30 + 20 * 5 + 36;

            let mon_btn = UiRect::new(p_x, row_y, p_w, 28);
            if mon_btn.contains(px, py) {
                if !monitors.is_empty() {
                    ui.selected_monitor_idx = (ui.selected_monitor_idx + 1) % monitors.len();
                }
                return UiAction::None;
            }
            row_y += 36;
            let btn_apply = UiRect::new(p_x, row_y, p_w, 34);
            if btn_apply.contains(px, py) {
                return UiAction::ApplySelected;
            }
            row_y += 40;
            let btn_reopt = UiRect::new(p_x, row_y, (p_w - 8) / 2, 28);
            if btn_reopt.contains(px, py) {
                return UiAction::ReoptimizeSelected;
            }
            let btn_reveal = UiRect::new(p_x + (p_w - 8) / 2 + 8, row_y, (p_w - 8) / 2, 28);
            if btn_reveal.contains(px, py) {
                if let Some(item) = ui
                    .selected_item_id
                    .as_ref()
                    .and_then(|id| items.iter().find(|i| &i.id == id))
                {
                    return UiAction::ShowFileInExplorer(item.asset.clone());
                }
            }
            row_y += 34;
            let btn_remove = UiRect::new(p_x, row_y, p_w, 28);
            if btn_remove.contains(px, py) {
                if let Some(item_id) = &ui.selected_item_id {
                    ui.confirm_delete = Some(item_id.clone());
                }
                return UiAction::None;
            }
        }
        NavTab::Displays => {
            let cards_top = content_rect.y + 32;
            let _card_w = (content_rect.w - 18).clamp(320, 480);
            let card_h = 160;

            for (idx, mon) in monitors.iter().enumerate() {
                let card_y = cards_top + (idx as i32) * (card_h + 16);
                let btn_y = card_y + 116;
                let btn_apply = UiRect::new(content_rect.x + 16, btn_y, 160, 30);
                if btn_apply.contains(px, py) {
                    return UiAction::ApplyToMonitor(mon.device.clone());
                }
                let btn_clear = UiRect::new(content_rect.x + 184, btn_y, 110, 30);
                if btn_clear.contains(px, py) {
                    return UiAction::ClearMonitor(mon.device.clone());
                }
            }
        }
        NavTab::Optimization => {
            let grid_y = content_rect.y + 32;
            let card_gap = 16;
            let card_w = (content_rect.w - card_gap) / 2;
            let card_h = 120;

            let c1 = UiRect::new(content_rect.x, grid_y, card_w, card_h);
            if c1.contains(px, py) {
                return UiAction::SetProfile(Profile::UltraEco);
            }
            let c2 = UiRect::new(content_rect.x + card_w + card_gap, grid_y, card_w, card_h);
            if c2.contains(px, py) {
                return UiAction::SetProfile(Profile::Balanced);
            }
            let c3 = UiRect::new(content_rect.x, grid_y + card_h + card_gap, card_w, card_h);
            if c3.contains(px, py) {
                return UiAction::SetProfile(Profile::Quality);
            }
            let c4 = UiRect::new(
                content_rect.x + card_w + card_gap,
                grid_y + card_h + card_gap,
                card_w,
                card_h,
            );
            if c4.contains(px, py) {
                return UiAction::SetProfile(Profile::Automatic);
            }

            let tune_y = grid_y + card_h * 2 + card_gap + 16;
            let reopt_rect =
                UiRect::new(content_rect.x + content_rect.w - 230, tune_y + 62, 210, 32);
            if reopt_rect.contains(px, py) {
                return UiAction::ReoptimizeSelected;
            }
        }
        NavTab::Performance => {
            let diag_y = content_rect.y + 88 + 16 + 165 + 16;
            let btn_open_log =
                UiRect::new(content_rect.x + content_rect.w - 145, diag_y + 10, 130, 26);
            if btn_open_log.contains(px, py) {
                return UiAction::OpenLogFile;
            }
        }
        NavTab::Settings => {
            let mut y1 = content_rect.y + 44;
            let chk_autostart = UiRect::new(content_rect.x + 18, y1, 200, 24);
            if chk_autostart.contains(px, py) {
                return UiAction::ToggleAutostart;
            }
            y1 += 34;
            let theme_btn = UiRect::new(content_rect.x + 160, y1, 120, 28);
            if theme_btn.contains(px, py) {
                return UiAction::ToggleTheme;
            }

            let card_w = (content_rect.w - 16) / 2;
            let right_card_x = content_rect.x + card_w + 16;
            let mut y2 = content_rect.y + 44;
            let chk_fs = UiRect::new(right_card_x + 18, y2, card_w - 36, 24);
            if chk_fs.contains(px, py) {
                return UiAction::ToggleSetting("pause_fullscreen");
            }
            y2 += 28;
            let chk_max = UiRect::new(right_card_x + 18, y2, card_w - 36, 24);
            if chk_max.contains(px, py) {
                return UiAction::ToggleSetting("pause_maximized");
            }
            y2 += 28;
            let chk_bat = UiRect::new(right_card_x + 18, y2, card_w - 36, 24);
            if chk_bat.contains(px, py) {
                return UiAction::ToggleSetting("pause_battery");
            }
            y2 += 28;
            let chk_sav = UiRect::new(right_card_x + 18, y2, card_w - 36, 24);
            if chk_sav.contains(px, py) {
                return UiAction::ToggleSetting("pause_saver");
            }
            y2 += 28;
            let chk_rdp = UiRect::new(right_card_x + 18, y2, card_w - 36, 24);
            if chk_rdp.contains(px, py) {
                return UiAction::ToggleSetting("pause_remote");
            }
            y2 += 28;
            let chk_adapt = UiRect::new(right_card_x + 18, y2, card_w - 36, 24);
            if chk_adapt.contains(px, py) {
                return UiAction::ToggleSetting("adaptive");
            }

            let row2_y = content_rect.y + 240 + 16;
            let btn_clean = UiRect::new(content_rect.x + 18, row2_y + 120, 140, 30);
            if btn_clean.contains(px, py) {
                return UiAction::CleanCache;
            }
            let btn_folder = UiRect::new(content_rect.x + 168, row2_y + 120, 150, 30);
            if btn_folder.contains(px, py) {
                return UiAction::OpenCacheFolder;
            }

            let btn_log = UiRect::new(right_card_x + 18, row2_y + 120, 140, 30);
            if btn_log.contains(px, py) {
                return UiAction::OpenLogFile;
            }
        }
        NavTab::Widgets => {
            let preview_h = 135;
            let grid_y = content_rect.y + preview_h + 14;
            let card_gap = 14;
            let card_w = (content_rect.w - card_gap) / 2;
            let c_left = UiRect::new(content_rect.x, grid_y, card_w, 325);
            let c_right = UiRect::new(content_rect.x + card_w + card_gap, grid_y, card_w, 325);

            // Left Card: Clock & Date
            let mut ly = c_left.y + 42;
            if UiRect::new(c_left.x + 18, ly, card_w - 36, 24).contains(px, py) {
                return UiAction::ToggleWidgetsMaster;
            }
            ly += 26;
            if UiRect::new(c_left.x + 18, ly, card_w - 36, 24).contains(px, py) {
                return UiAction::ToggleClock;
            }
            ly += 28;
            if UiRect::new(c_left.x + 18, ly, 100, 26).contains(px, py) {
                return UiAction::ToggleClock24h;
            }
            if UiRect::new(c_left.x + 124, ly, 105, 26).contains(px, py) {
                return UiAction::ToggleClock24h;
            }
            if UiRect::new(c_left.x + 235, ly, 105, 26).contains(px, py) {
                return UiAction::ToggleClockSeconds;
            }
            ly += 32;
            if UiRect::new(c_left.x + 95, ly, 55, 24).contains(px, py) {
                return UiAction::SetClockSize(48);
            }
            if UiRect::new(c_left.x + 155, ly, 55, 24).contains(px, py) {
                return UiAction::SetClockSize(64);
            }
            if UiRect::new(c_left.x + 215, ly, 55, 24).contains(px, py) {
                return UiAction::SetClockSize(72);
            }
            if UiRect::new(c_left.x + 275, ly, 55, 24).contains(px, py) {
                return UiAction::SetClockSize(96);
            }
            ly += 30;
            if UiRect::new(c_left.x + 95, ly, 55, 24).contains(px, py) {
                return UiAction::SetClockColor(0x00FFFFFF);
            }
            if UiRect::new(c_left.x + 155, ly, 55, 24).contains(px, py) {
                return UiAction::SetClockColor(0x00E2E8F0);
            }
            if UiRect::new(c_left.x + 215, ly, 55, 24).contains(px, py) {
                return UiAction::SetClockColor(0x00FBBF24);
            }
            if UiRect::new(c_left.x + 275, ly, 55, 24).contains(px, py) {
                return UiAction::SetClockColor(0x00818CF8);
            }
            ly += 30;
            if UiRect::new(c_left.x + 18, ly, card_w - 36, 24).contains(px, py) {
                return UiAction::ToggleDate;
            }
            ly += 26;
            if UiRect::new(c_left.x + 18, ly, 75, 24).contains(px, py) {
                return UiAction::SetDateFormat(DateFormat::Full);
            }
            if UiRect::new(c_left.x + 98, ly, 80, 24).contains(px, py) {
                return UiAction::SetDateFormat(DateFormat::MonthDay);
            }
            if UiRect::new(c_left.x + 183, ly, 75, 24).contains(px, py) {
                return UiAction::SetDateFormat(DateFormat::Dot);
            }
            if UiRect::new(c_left.x + 263, ly, 68, 24).contains(px, py) {
                return UiAction::SetDateFormat(DateFormat::Numeric);
            }

            // Right Card: Greetings, Music & Position
            let mut ry = c_right.y + 42;
            if UiRect::new(c_right.x + 18, ry, card_w - 36, 24).contains(px, py) {
                return UiAction::ToggleGreeting;
            }
            ry += 26;
            if UiRect::new(c_right.x + 18, ry, 100, 24).contains(px, py) {
                return UiAction::SetGreetingMode(GreetingMode::TimeBased);
            }
            if UiRect::new(c_right.x + 124, ry, 100, 24).contains(px, py) {
                return UiAction::SetGreetingMode(GreetingMode::Rotating);
            }
            if UiRect::new(c_right.x + 230, ry, 95, 24).contains(px, py) {
                return UiAction::SetGreetingMode(GreetingMode::Fixed);
            }
            ry += 30;
            if UiRect::new(c_right.x + 18, ry, card_w - 36, 24).contains(px, py) {
                return UiAction::ToggleMusic;
            }
            ry += 26;
            if UiRect::new(c_right.x + 18, ry, 110, 26).contains(px, py) {
                return UiAction::ToggleMusicSpotifyPreferred;
            }
            if UiRect::new(c_right.x + 134, ry, 110, 26).contains(px, py) {
                return UiAction::ToggleMusicGeneralMedia;
            }
            if UiRect::new(c_right.x + 250, ry, 90, 26).contains(px, py) {
                return UiAction::OpenSpotifyApp;
            }
            ry += 32;
            if UiRect::new(c_right.x + 95, ry, 75, 24).contains(px, py) {
                return UiAction::SetWidgetPosition(WidgetPosition::TopRight);
            }
            if UiRect::new(c_right.x + 175, ry, 75, 24).contains(px, py) {
                return UiAction::SetWidgetPosition(WidgetPosition::TopLeft);
            }
            if UiRect::new(c_right.x + 255, ry, 75, 24).contains(px, py) {
                return UiAction::SetWidgetPosition(WidgetPosition::Center);
            }
            ry += 28;
            if UiRect::new(c_right.x + 95, ry, 34, 24).contains(px, py) {
                return UiAction::AdjustWidgetOffsetX(-10);
            }
            if UiRect::new(c_right.x + 133, ry, 34, 24).contains(px, py) {
                return UiAction::AdjustWidgetOffsetX(10);
            }
            if UiRect::new(c_right.x + 175, ry, 34, 24).contains(px, py) {
                return UiAction::AdjustWidgetOffsetY(-10);
            }
            if UiRect::new(c_right.x + 213, ry, 34, 24).contains(px, py) {
                return UiAction::AdjustWidgetOffsetY(10);
            }
            if UiRect::new(c_right.x + 255, ry, 75, 24).contains(px, py) {
                return UiAction::ToggleTextShadow;
            }
            ry += 30;
            if UiRect::new(c_right.x + 18, ry, card_w - 36, 30).contains(px, py) {
                return UiAction::ToggleWidgetEditMode;
            }
        }
    }

    UiAction::None
}

pub fn handle_double_click(
    ui: &mut UiState,
    px: i32,
    py: i32,
    win_w: i32,
    _win_h: i32,
    items: &[LibraryItem],
) -> UiAction {
    if ui.current_tab == NavTab::Library {
        let sidebar_w = 220;
        let content_x = sidebar_w + 24;
        let content_y = 60 + 20;
        let content_w = win_w - content_x - 24;
        let detail_w = 285;
        let grid_w = content_w - detail_w - 18;
        let grid_top = content_y + 28 + 14;

        let card_gap = 14;
        let cols = if grid_w > 540 { 3 } else { 2 };
        let card_w = (grid_w - (cols - 1) * card_gap) / cols;
        let card_h = 195;

        for (idx, item) in items.iter().enumerate() {
            let col = (idx as i32) % cols;
            let row = (idx as i32) / cols;
            let card_x = content_x + col * (card_w + card_gap);
            let card_y = grid_top + row * (card_h + card_gap) - ui.library_scroll_y;

            if card_x <= px && px < card_x + card_w && card_y <= py && py < card_y + card_h {
                ui.selected_item_id = Some(item.id.clone());
                return UiAction::ApplySelected;
            }
        }
    }
    UiAction::None
}

pub fn handle_scroll(ui: &mut UiState, delta: i32) -> bool {
    if ui.current_tab == NavTab::Library {
        let prev = ui.library_scroll_y;
        let step = 48;
        if delta > 0 {
            ui.library_scroll_y = (ui.library_scroll_y - step).max(0);
        } else {
            ui.library_scroll_y += step;
        }
        return prev != ui.library_scroll_y;
    }
    false
}
