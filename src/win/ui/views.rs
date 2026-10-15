//! View renderers for all FeatherWall screens: Home, Library, Displays, Optimization, Performance, Settings.
use super::draw::{self, UiRect};
use super::theme::{Fonts, Theme};
use super::{LibraryFilter, NavTab, UiState};
use crate::core::{DateFormat, GreetingMode, LibraryItem, MediaKind, Profile, WidgetPosition, WidgetSettings};
use crate::win::desktop::Monitor;
use crate::win::ffi as f;

pub struct ViewContext<'a> {
    pub theme: Theme,
    pub fonts: &'a Fonts,
    pub win_w: i32,
    pub win_h: i32,
    pub active_tab: NavTab,
    pub cpu_pct: f64,
    pub working_set_mb: f64,
    pub private_commit_mb: f64,
    pub playing_count: usize,
    pub paused_count: usize,
    pub worker_active: bool,
    pub worker_progress: u8,
    pub worker_status: &'a str,
    pub widgets: &'a WidgetSettings,
}

pub fn render_shell(
    dc: f::HDC,
    ui: &mut UiState,
    items: &[LibraryItem],
    monitors: &[Monitor],
    assignments: &std::collections::BTreeMap<String, String>,
    ctx: &ViewContext,
) {
    let theme = &ctx.theme;
    let fonts = ctx.fonts;
    let w = ctx.win_w;
    let h = ctx.win_h;

    // 1. Background Fill
    let bg_rect = UiRect::new(0, 0, w, h);
    draw::fill_rect(dc, &bg_rect, theme.bg);

    // 2. Sidebar (X: 0..220, Y: 0..h)
    let sidebar_w = 220;
    let sidebar_rect = UiRect::new(0, 0, sidebar_w, h);
    draw::fill_rect(dc, &sidebar_rect, theme.surface);
    let sep_line = UiRect::new(sidebar_w - 1, 0, 1, h);
    draw::fill_rect(dc, &sep_line, theme.border_subtle);

    // Branding in Sidebar
    let logo_icon_rect = UiRect::new(18, 16, 28, 28);
    draw::fill_round_rect(dc, &logo_icon_rect, 6, theme.accent, None);
    draw::draw_text(
        dc,
        "🪶",
        &logo_icon_rect,
        theme.text_on_accent,
        fonts.heading,
        f::DT_CENTER | f::DT_VCENTER | f::DT_SINGLELINE,
    );

    let brand_text_rect = UiRect::new(54, 14, 150, 18);
    draw::draw_text(
        dc,
        "FeatherWall",
        &brand_text_rect,
        theme.text_primary,
        fonts.body_bold,
        f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
    );

    let brand_sub_rect = UiRect::new(54, 32, 150, 14);
    draw::draw_text(
        dc,
        "WALLPAPER ENGINE",
        &brand_sub_rect,
        theme.accent,
        fonts.caption,
        f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
    );

    // Navigation Items
    let nav_y_start = 68;
    let nav_h = 38;
    let nav_spacing = 4;

    for (idx, &tab) in NavTab::ALL.iter().enumerate() {
        let item_y = nav_y_start + (idx as i32) * (nav_h + nav_spacing);
        let item_rect = UiRect::new(12, item_y, sidebar_w - 24, nav_h);
        let is_selected = ui.current_tab == tab;
        let is_hovered = ui.hovered_nav == Some(tab);

        if is_selected {
            draw::fill_round_rect(dc, &item_rect, 6, theme.accent, None);
            let bar_rect = UiRect::new(item_rect.x + 2, item_rect.y + 8, 3, item_rect.h - 16);
            draw::fill_round_rect(dc, &bar_rect, 2, theme.text_on_accent, None);
        } else if is_hovered {
            draw::fill_round_rect(dc, &item_rect, 6, theme.surface_hover, None);
        }

        let icon_rect = UiRect::new(item_rect.x + 14, item_rect.y, 24, item_rect.h);
        let icon_color = if is_selected {
            theme.text_on_accent
        } else if is_hovered {
            theme.accent
        } else {
            theme.text_secondary
        };
        draw::draw_text(
            dc,
            tab.icon(),
            &icon_rect,
            icon_color,
            fonts.heading,
            f::DT_CENTER | f::DT_VCENTER | f::DT_SINGLELINE,
        );

        let label_rect = UiRect::new(item_rect.x + 42, item_rect.y, item_rect.w - 50, item_rect.h);
        let label_color = if is_selected {
            theme.text_on_accent
        } else if is_hovered {
            theme.text_primary
        } else {
            theme.text_secondary
        };
        draw::draw_text(
            dc,
            tab.label(),
            &label_rect,
            label_color,
            if is_selected {
                fonts.body_bold
            } else {
                fonts.body
            },
            f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
        );
    }

    // Sidebar Bottom Info Card
    let btm_card = UiRect::new(12, h - 72, sidebar_w - 24, 60);
    draw::fill_round_rect(
        dc,
        &btm_card,
        6,
        theme.surface_card,
        Some(theme.border_subtle),
    );

    let status_dot_color = if ctx.playing_count > 0 {
        theme.success
    } else if ctx.paused_count > 0 {
        theme.warning
    } else {
        theme.text_tertiary
    };

    let dot_rect = UiRect::new(btm_card.x + 12, btm_card.y + 14, 8, 8);
    draw::fill_round_rect(dc, &dot_rect, 4, status_dot_color, None);

    let status_text = if ctx.playing_count > 0 {
        format!("Engine: {} Playing", ctx.playing_count)
    } else if ctx.paused_count > 0 {
        "Engine: Paused".into()
    } else {
        "Engine: Idle".into()
    };

    let status_rect = UiRect::new(btm_card.x + 26, btm_card.y + 9, btm_card.w - 36, 18);
    draw::draw_text(
        dc,
        &status_text,
        &status_rect,
        theme.text_primary,
        fonts.caption,
        f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
    );

    let ver_rect = UiRect::new(btm_card.x + 12, btm_card.y + 32, btm_card.w - 24, 18);
    draw::draw_text(
        dc,
        "v0.2.0-native • x64 release",
        &ver_rect,
        theme.text_tertiary,
        fonts.caption,
        f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
    );

    // 3. Top Bar (X: sidebar_w..w, Y: 0..60)
    let topbar_h = 60;
    let topbar_rect = UiRect::new(sidebar_w, 0, w - sidebar_w, topbar_h);
    draw::fill_rect(dc, &topbar_rect, theme.surface);
    let top_sep = UiRect::new(sidebar_w, topbar_h - 1, w - sidebar_w, 1);
    draw::fill_rect(dc, &top_sep, theme.border_subtle);

    // Page Title & Subtitle
    let page_title_rect = UiRect::new(sidebar_w + 24, 10, 320, 22);
    draw::draw_text(
        dc,
        ui.current_tab.title(),
        &page_title_rect,
        theme.text_primary,
        fonts.section,
        f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
    );

    let page_sub_rect = UiRect::new(sidebar_w + 24, 32, 450, 16);
    draw::draw_text(
        dc,
        ui.current_tab.subtitle(),
        &page_sub_rect,
        theme.text_secondary,
        fonts.caption,
        f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
    );

    // Right-side Top Bar Controls
    let mut right_cursor = w - 24;

    // "Import Wallpaper" Button
    let import_btn_w = 148;
    right_cursor -= import_btn_w;
    let import_btn_rect = UiRect::new(right_cursor, 14, import_btn_w, 32);
    let is_import_hover = ui.hovered_btn_id == Some("top_import");
    draw::draw_button(
        dc,
        &import_btn_rect,
        "+ Import Media...",
        true,
        is_import_hover,
        false,
        theme,
        fonts,
    );

    // Theme Switcher Button [ 🌙 / ☀️ ]
    let theme_btn_w = 82;
    right_cursor -= theme_btn_w + 10;
    let theme_btn_rect = UiRect::new(right_cursor, 14, theme_btn_w, 32);
    let is_theme_hover = ui.hovered_btn_id == Some("top_theme");
    let theme_label = if theme.is_dark {
        "🌙 Dark"
    } else {
        "☀️ Light"
    };
    draw::draw_button(
        dc,
        &theme_btn_rect,
        theme_label,
        false,
        is_theme_hover,
        false,
        theme,
        fonts,
    );

    // Status Pill
    let (status_bg, status_fg, status_label) = if ctx.playing_count > 0 {
        (theme.success_subtle, theme.success, "● Playing")
    } else if ctx.paused_count > 0 {
        (theme.warning_subtle, theme.warning, "❚❚ Paused")
    } else {
        (theme.surface_active, theme.text_secondary, "■ Stopped")
    };
    let pill_w = 88;
    right_cursor -= pill_w + 10;
    let pill_rect = UiRect::new(right_cursor, 16, pill_w, 28);
    draw::draw_pill(
        dc,
        &pill_rect,
        status_label,
        status_bg,
        status_fg,
        Some(status_fg),
        fonts.caption,
    );

    // Search bar (only visible if Library tab is active)
    if ui.current_tab == NavTab::Library {
        let search_w = (right_cursor - (sidebar_w + 480)).clamp(120, 220);
        if search_w >= 120 {
            right_cursor -= search_w + 14;
            let search_rect = UiRect::new(right_cursor, 14, search_w, 32);
            let search_border = if ui.search_focused {
                theme.border_active
            } else {
                theme.border
            };
            draw::fill_round_rect(dc, &search_rect, 6, theme.surface_card, Some(search_border));

            let search_text = if ui.search_query.is_empty() {
                "🔍 Search..."
            } else {
                &ui.search_query
            };
            let search_color = if ui.search_query.is_empty() {
                theme.text_tertiary
            } else {
                theme.text_primary
            };
            let text_box = UiRect::new(
                search_rect.x + 8,
                search_rect.y,
                search_rect.w - 16,
                search_rect.h,
            );
            draw::draw_text(
                dc,
                search_text,
                &text_box,
                search_color,
                fonts.body,
                f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
            );
        }
    }

    // 4. Content Area
    let content_x = sidebar_w + 24;
    let content_y = topbar_h + 20;
    let content_w = w - content_x - 24;
    let content_h = h - content_y - 16;
    let content_rect = UiRect::new(content_x, content_y, content_w, content_h);

    match ui.current_tab {
        NavTab::Home => render_home(dc, ui, items, monitors, assignments, &content_rect, ctx),
        NavTab::Library => render_library(dc, ui, items, monitors, assignments, &content_rect, ctx),
        NavTab::Displays => {
            render_displays(dc, ui, items, monitors, assignments, &content_rect, ctx)
        }
        NavTab::Optimization => render_optimization(dc, ui, items, monitors, &content_rect, ctx),
        NavTab::Performance => render_performance(dc, ui, &content_rect, ctx),
        NavTab::Widgets => render_widgets_view(dc, ui, monitors, &content_rect, ctx),
        NavTab::Settings => render_settings(dc, ui, monitors, &content_rect, ctx),
    }

    // 5. Toast Notification (if active)
    if let Some((msg, is_success, started)) = &ui.toast {
        if started.elapsed().as_secs_f64() < 3.5 {
            draw::draw_toast(dc, w, h, msg, *is_success, theme, fonts);
        }
    }

    // 6. Confirmation Modal (if active)
    if let Some(id_to_delete) = &ui.confirm_delete {
        let item_title = items
            .iter()
            .find(|i| &i.id == id_to_delete)
            .map(|i| i.title.as_str())
            .unwrap_or("this wallpaper");

        render_confirm_modal(dc, w, h, item_title, ui, theme, fonts);
    }
}

fn render_home(
    dc: f::HDC,
    ui: &mut UiState,
    items: &[LibraryItem],
    monitors: &[Monitor],
    assignments: &std::collections::BTreeMap<String, String>,
    bounds: &UiRect,
    ctx: &ViewContext,
) {
    let theme = &ctx.theme;
    let fonts = ctx.fonts;

    // Active Wallpaper Hero Card
    let hero_h = 210;
    let hero_rect = UiRect::new(bounds.x, bounds.y, bounds.w, hero_h);
    draw::draw_card(dc, &hero_rect, false, false, theme);

    // Find active assigned wallpaper
    let primary_dev = monitors
        .iter()
        .find(|m| m.primary)
        .or_else(|| monitors.first());
    let active_item_id = primary_dev.and_then(|m| assignments.get(&m.device));
    let active_item = active_item_id.and_then(|id| items.iter().find(|i| &i.id == id));

    // Thumbnail Preview (Left side of hero)
    let thumb_w = 260;
    let thumb_h = 162;
    let thumb_rect = UiRect::new(hero_rect.x + 20, hero_rect.y + 24, thumb_w, thumb_h);

    if let Some(item) = active_item {
        let hbmp = ui.get_or_load_thumb(item);
        draw::draw_thumbnail(
            dc,
            &thumb_rect,
            hbmp,
            &item.title,
            item.kind == MediaKind::Video,
            theme,
            fonts,
        );

        // Right side details
        let details_x = thumb_rect.x + thumb_w + 24;
        let details_w = hero_rect.w - (details_x - hero_rect.x) - 20;

        let tag_rect = UiRect::new(details_x, hero_rect.y + 20, 140, 16);
        draw::draw_text(
            dc,
            "ACTIVE DESKTOP WALLPAPER",
            &tag_rect,
            theme.accent,
            fonts.caption,
            f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
        );

        let title_rect = UiRect::new(details_x, hero_rect.y + 38, details_w, 28);
        draw::draw_text(
            dc,
            &item.title,
            &title_rect,
            theme.text_primary,
            fonts.section,
            f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE | f::DT_END_ELLIPSIS,
        );

        // Metadata Pills row
        let meta_y = hero_rect.y + 72;
        let p1 = UiRect::new(details_x, meta_y, 90, 22);
        draw::draw_pill(
            dc,
            &p1,
            &format!("{}×{}", item.width, item.height),
            theme.surface_active,
            theme.text_secondary,
            None,
            fonts.caption,
        );

        let p2 = UiRect::new(details_x + 98, meta_y, 76, 22);
        draw::draw_pill(
            dc,
            &p2,
            &format!("{:.0} FPS", item.fps),
            theme.surface_active,
            theme.text_secondary,
            None,
            fonts.caption,
        );

        let p3 = UiRect::new(details_x + 182, meta_y, 94, 22);
        draw::draw_pill(
            dc,
            &p3,
            item.profile.text(),
            theme.accent_subtle,
            theme.accent,
            None,
            fonts.caption,
        );

        let mon_name = primary_dev.map(|m| m.device.as_str()).unwrap_or("Desktop");
        let mon_rect = UiRect::new(details_x + 284, meta_y, 160, 22);
        draw::draw_text(
            dc,
            &format!("Target: {mon_name}"),
            &mon_rect,
            theme.text_tertiary,
            fonts.caption,
            f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
        );

        // Controls Row
        let ctrl_y = hero_rect.y + 138;
        let (pause_label, pause_id) = if ctx.paused_count > 0 {
            ("▶ Resume Playback", "home_resume")
        } else {
            ("❚❚ Pause Playback", "home_pause")
        };

        let btn_pause_rect = UiRect::new(details_x, ctrl_y, 145, 34);
        let is_pause_hover = ui.hovered_btn_id == Some(pause_id);
        draw::draw_button(
            dc,
            &btn_pause_rect,
            pause_label,
            true,
            is_pause_hover,
            false,
            theme,
            fonts,
        );

        let btn_stop_rect = UiRect::new(details_x + 155, ctrl_y, 100, 34);
        let is_stop_hover = ui.hovered_btn_id == Some("home_stop");
        draw::draw_button(
            dc,
            &btn_stop_rect,
            "■ Stop",
            false,
            is_stop_hover,
            false,
            theme,
            fonts,
        );

        let btn_lib_rect = UiRect::new(details_x + 265, ctrl_y, 140, 34);
        let is_lib_hover = ui.hovered_btn_id == Some("home_open_library");
        draw::draw_button(
            dc,
            &btn_lib_rect,
            "Browse Library ➜",
            false,
            is_lib_hover,
            false,
            theme,
            fonts,
        );
    } else {
        // No active wallpaper empty hero
        draw::draw_thumbnail(
            dc,
            &thumb_rect,
            0,
            "No Active Wallpaper",
            false,
            theme,
            fonts,
        );

        let details_x = thumb_rect.x + thumb_w + 24;
        let details_w = hero_rect.w - (details_x - hero_rect.x) - 20;

        let empty_title_rect = UiRect::new(details_x, hero_rect.y + 40, details_w, 28);
        draw::draw_text(
            dc,
            "No Wallpaper Currently Active",
            &empty_title_rect,
            theme.text_primary,
            fonts.section,
            f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
        );

        let empty_desc_rect = UiRect::new(details_x, hero_rect.y + 72, details_w, 40);
        draw::draw_text(
            dc,
            "Select an imported wallpaper from your library or import a new video or GIF to set up an animated desktop.",
            &empty_desc_rect,
            theme.text_secondary,
            fonts.body,
            f::DT_LEFT | f::DT_TOP | f::DT_WORDBREAK,
        );

        let btn_pick_rect = UiRect::new(details_x, hero_rect.y + 138, 160, 34);
        let is_pick_hover = ui.hovered_btn_id == Some("home_open_library");
        draw::draw_button(
            dc,
            &btn_pick_rect,
            "Open Wallpaper Library",
            true,
            is_pick_hover,
            false,
            theme,
            fonts,
        );
    }

    // System Status Grid (4 Metric Cards across)
    let stats_y = bounds.y + hero_h + 18;
    let card_gap = 14;
    let card_w = (bounds.w - card_gap * 3) / 4;
    let card_h = 88;

    let c1 = UiRect::new(bounds.x, stats_y, card_w, card_h);
    let cpu_str = format!("{:.2}%", ctx.cpu_pct);
    draw::draw_stat_card(
        dc,
        &c1,
        "SYSTEM CPU CAPACITY",
        &cpu_str,
        "FeatherWall process load",
        Some(theme.chart_cpu),
        theme,
        fonts,
    );

    let c2 = UiRect::new(bounds.x + card_w + card_gap, stats_y, card_w, card_h);
    let ram_str = format!("{:.1} MiB", ctx.working_set_mb);
    draw::draw_stat_card(
        dc,
        &c2,
        "WORKING SET RAM",
        &ram_str,
        &format!("{:.1} MiB committed", ctx.private_commit_mb),
        Some(theme.chart_mem),
        theme,
        fonts,
    );

    let c3 = UiRect::new(bounds.x + (card_w + card_gap) * 2, stats_y, card_w, card_h);
    let mon_str = format!("{} / {}", ctx.playing_count, monitors.len());
    draw::draw_stat_card(
        dc,
        &c3,
        "DISPLAYS PLAYING",
        &mon_str,
        "Active WorkerW surfaces",
        Some(theme.accent),
        theme,
        fonts,
    );

    let c4 = UiRect::new(bounds.x + (card_w + card_gap) * 3, stats_y, card_w, card_h);
    draw::draw_stat_card(
        dc,
        &c4,
        "DECODER ACCELERATION",
        "Direct3D EVR",
        "OS Media Foundation acceleration",
        Some(theme.warning),
        theme,
        fonts,
    );

    // Quick Library Strip (Recent wallpapers)
    let strip_y = stats_y + card_h + 20;
    let strip_h = bounds.h - (strip_y - bounds.y);
    if strip_h >= 110 {
        let strip_header_rect = UiRect::new(bounds.x, strip_y, 300, 20);
        draw::draw_text(
            dc,
            "RECENT WALLPAPERS IN LIBRARY",
            &strip_header_rect,
            theme.text_secondary,
            fonts.caption,
            f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
        );

        let items_row_y = strip_y + 26;
        let item_card_w = 175;
        let item_card_h = (strip_h - 32).min(130);

        for (idx, item) in items.iter().take(4).enumerate() {
            let card_x = bounds.x + (idx as i32) * (item_card_w + 14);
            if card_x + item_card_w > bounds.x + bounds.w {
                break;
            }
            let card_rect = UiRect::new(card_x, items_row_y, item_card_w, item_card_h);
            let is_hover = ui.hovered_item_idx == Some(idx);
            let is_sel = ui.selected_item_id.as_deref() == Some(&item.id);
            draw::draw_card(dc, &card_rect, is_hover, is_sel, theme);

            let thumb_box = UiRect::new(card_rect.x + 8, card_rect.y + 8, card_rect.w - 16, 76);
            let hbmp = ui.get_or_load_thumb(item);
            draw::draw_thumbnail(
                dc,
                &thumb_box,
                hbmp,
                &item.title,
                item.kind == MediaKind::Video,
                theme,
                fonts,
            );

            let name_rect = UiRect::new(card_rect.x + 8, card_rect.y + 88, card_rect.w - 16, 18);
            draw::draw_text(
                dc,
                &item.title,
                &name_rect,
                theme.text_primary,
                fonts.caption,
                f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE | f::DT_END_ELLIPSIS,
            );

            let sub_rect = UiRect::new(card_rect.x + 8, card_rect.y + 104, card_rect.w - 16, 16);
            draw::draw_text(
                dc,
                &format!("{} • {:.0}s", item.profile.text(), item.duration),
                &sub_rect,
                theme.text_tertiary,
                fonts.caption,
                f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
            );
        }
    }
}

fn render_library(
    dc: f::HDC,
    ui: &mut UiState,
    items: &[LibraryItem],
    monitors: &[Monitor],
    assignments: &std::collections::BTreeMap<String, String>,
    bounds: &UiRect,
    ctx: &ViewContext,
) {
    let theme = &ctx.theme;
    let fonts = ctx.fonts;

    // Filter Items
    let filtered_items: Vec<&LibraryItem> = items
        .iter()
        .filter(|item| {
            // Media kind filter
            match ui.filter {
                LibraryFilter::All => true,
                LibraryFilter::Videos => item.kind == MediaKind::Video,
                LibraryFilter::Stills => item.kind == MediaKind::Still,
            }
        })
        .filter(|item| {
            // Search query filter
            if ui.search_query.trim().is_empty() {
                true
            } else {
                item.title
                    .to_lowercase()
                    .contains(&ui.search_query.trim().to_lowercase())
            }
        })
        .collect();

    // Top Controls Bar (Filter Chips + Sort + Count)
    let filter_y = bounds.y;
    let chip_w = 76;
    let chip_h = 28;

    let f_all = UiRect::new(bounds.x, filter_y, chip_w, chip_h);
    let is_all_sel = ui.filter == LibraryFilter::All;
    draw::draw_button(
        dc,
        &f_all,
        "All Items",
        is_all_sel,
        ui.hovered_btn_id == Some("filter_all"),
        false,
        theme,
        fonts,
    );

    let f_vid = UiRect::new(bounds.x + chip_w + 8, filter_y, chip_w, chip_h);
    let is_vid_sel = ui.filter == LibraryFilter::Videos;
    draw::draw_button(
        dc,
        &f_vid,
        "Videos",
        is_vid_sel,
        ui.hovered_btn_id == Some("filter_videos"),
        false,
        theme,
        fonts,
    );

    let f_still = UiRect::new(bounds.x + (chip_w + 8) * 2, filter_y, chip_w, chip_h);
    let is_still_sel = ui.filter == LibraryFilter::Stills;
    draw::draw_button(
        dc,
        &f_still,
        "Stills",
        is_still_sel,
        ui.hovered_btn_id == Some("filter_stills"),
        false,
        theme,
        fonts,
    );

    // Count label
    let count_text = format!(
        "Showing {} of {} wallpapers",
        filtered_items.len(),
        items.len()
    );
    let count_rect = UiRect::new(bounds.x + (chip_w + 8) * 3 + 16, filter_y, 220, chip_h);
    draw::draw_text(
        dc,
        &count_text,
        &count_rect,
        theme.text_tertiary,
        fonts.caption,
        f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
    );

    // Import progress banner if worker is active
    let mut grid_top = bounds.y + chip_h + 14;
    if ctx.worker_active {
        let banner_h = 42;
        let banner_rect = UiRect::new(bounds.x, grid_top, bounds.w, banner_h);
        draw::fill_round_rect(
            dc,
            &banner_rect,
            6,
            theme.surface_elevated,
            Some(theme.accent),
        );

        let progress_rect = UiRect::new(
            banner_rect.x + 12,
            banner_rect.y + 11,
            banner_rect.w - 120,
            20,
        );
        draw::draw_progress_bar(
            dc,
            &progress_rect,
            ctx.worker_progress,
            ctx.worker_status,
            theme,
            fonts,
        );

        let cancel_btn = UiRect::new(
            banner_rect.x + banner_rect.w - 95,
            banner_rect.y + 8,
            85,
            26,
        );
        let is_cancel_hover = ui.hovered_btn_id == Some("lib_cancel_worker");
        draw::draw_button(
            dc,
            &cancel_btn,
            "✕ Cancel",
            false,
            is_cancel_hover,
            false,
            theme,
            fonts,
        );

        grid_top += banner_h + 12;
    }

    // Split: Left = Grid, Right = Detail Pane
    let detail_w = 285;
    let grid_w = bounds.w - detail_w - 18;
    let grid_h = bounds.h - (grid_top - bounds.y);

    // Selected item for detail pane
    let selected_item = ui
        .selected_item_id
        .as_ref()
        .and_then(|id| items.iter().find(|i| &i.id == id))
        .or_else(|| filtered_items.first().copied());

    // 1. Grid of Wallpaper Cards
    if filtered_items.is_empty() {
        let empty_rect = UiRect::new(bounds.x, grid_top, grid_w, grid_h);
        draw::fill_round_rect(
            dc,
            &empty_rect,
            8,
            theme.surface_card,
            Some(theme.border_subtle),
        );

        let e_icon = UiRect::new(empty_rect.x, empty_rect.y + grid_h / 2 - 50, grid_w, 36);
        draw::draw_text(
            dc,
            "📂",
            &e_icon,
            theme.accent,
            fonts.title,
            f::DT_CENTER | f::DT_VCENTER | f::DT_SINGLELINE,
        );

        let e_title = UiRect::new(empty_rect.x, empty_rect.y + grid_h / 2 - 10, grid_w, 24);
        draw::draw_text(
            dc,
            "No Wallpapers Found",
            &e_title,
            theme.text_primary,
            fonts.section,
            f::DT_CENTER | f::DT_VCENTER | f::DT_SINGLELINE,
        );

        let e_sub = UiRect::new(empty_rect.x, empty_rect.y + grid_h / 2 + 18, grid_w, 20);
        draw::draw_text(
            dc,
            "Import a video or GIF, or drag and drop media files directly into this window.",
            &e_sub,
            theme.text_secondary,
            fonts.body,
            f::DT_CENTER | f::DT_VCENTER | f::DT_SINGLELINE,
        );
    } else {
        let card_gap = 14;
        let cols = if grid_w > 540 { 3 } else { 2 };
        let card_w = (grid_w - (cols - 1) * card_gap) / cols;
        let card_h = 195;

        for (idx, item) in filtered_items.iter().enumerate() {
            let col = (idx as i32) % cols;
            let row = (idx as i32) / cols;

            let card_x = bounds.x + col * (card_w + card_gap);
            let card_y = grid_top + row * (card_h + card_gap) - ui.library_scroll_y;

            // Simple viewport culling
            if card_y + card_h < grid_top || card_y > bounds.y + bounds.h {
                continue;
            }

            let card_rect = UiRect::new(card_x, card_y, card_w, card_h);
            let is_sel = selected_item.map(|s| s.id == item.id).unwrap_or(false);
            let is_hover = ui.hovered_item_idx == Some(idx);

            draw::draw_card(dc, &card_rect, is_hover, is_sel, theme);

            // Thumbnail
            let thumb_rect = UiRect::new(card_rect.x + 8, card_rect.y + 8, card_rect.w - 16, 110);
            let hbmp = ui.get_or_load_thumb(item);
            draw::draw_thumbnail(
                dc,
                &thumb_rect,
                hbmp,
                &item.title,
                item.kind == MediaKind::Video,
                theme,
                fonts,
            );

            // Badges over thumbnail
            let kind_tag = if item.kind == MediaKind::Video {
                "VIDEO"
            } else {
                "STILL"
            };
            let tag_rect = UiRect::new(thumb_rect.x + 6, thumb_rect.y + 6, 44, 18);
            draw::draw_pill(
                dc,
                &tag_rect,
                kind_tag,
                theme.surface_elevated,
                theme.text_primary,
                None,
                fonts.caption,
            );

            if item.duration > 0.0 {
                let dur_str = format!("{:.1}s", item.duration);
                let dur_rect = UiRect::new(
                    thumb_rect.x + thumb_rect.w - 48,
                    thumb_rect.y + thumb_rect.h - 22,
                    42,
                    16,
                );
                draw::draw_pill(
                    dc,
                    &dur_rect,
                    &dur_str,
                    theme.surface_elevated,
                    theme.text_secondary,
                    None,
                    fonts.caption,
                );
            }

            // Check if active on any display
            let is_active_any = assignments.values().any(|id| id == &item.id);
            if is_active_any {
                let active_rect =
                    UiRect::new(thumb_rect.x + thumb_rect.w - 56, thumb_rect.y + 6, 50, 18);
                draw::draw_pill(
                    dc,
                    &active_rect,
                    "ACTIVE",
                    theme.success_subtle,
                    theme.success,
                    Some(theme.success),
                    fonts.caption,
                );
            }

            // Title
            let title_rect = UiRect::new(card_rect.x + 10, card_rect.y + 124, card_rect.w - 20, 20);
            draw::draw_text(
                dc,
                &item.title,
                &title_rect,
                theme.text_primary,
                fonts.body_bold,
                f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE | f::DT_END_ELLIPSIS,
            );

            // Metadata row
            let meta_rect = UiRect::new(card_rect.x + 10, card_rect.y + 146, card_rect.w - 20, 16);
            let meta_text = format!("{}×{} • {}", item.width, item.height, item.profile.text());
            draw::draw_text(
                dc,
                &meta_text,
                &meta_rect,
                theme.text_tertiary,
                fonts.caption,
                f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
            );

            let size_mb = item.bytes as f64 / 1_048_576.0;
            let size_rect = UiRect::new(card_rect.x + 10, card_rect.y + 166, card_rect.w - 20, 16);
            draw::draw_text(
                dc,
                &format!("{:.1} MiB cache", size_mb),
                &size_rect,
                theme.text_tertiary,
                fonts.caption,
                f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
            );
        }
    }

    // 2. Detail Panel (Right side)
    let detail_x = bounds.x + grid_w + 18;
    let detail_rect = UiRect::new(detail_x, grid_top, detail_w, grid_h);
    draw::draw_card(dc, &detail_rect, false, false, theme);

    if let Some(item) = selected_item {
        let p_x = detail_rect.x + 16;
        let p_w = detail_rect.w - 32;

        // Large Preview
        let prev_h = 145;
        let prev_rect = UiRect::new(p_x, detail_rect.y + 16, p_w, prev_h);
        let hbmp = ui.get_or_load_thumb(item);
        draw::draw_thumbnail(
            dc,
            &prev_rect,
            hbmp,
            &item.title,
            item.kind == MediaKind::Video,
            theme,
            fonts,
        );

        // Title
        let title_rect = UiRect::new(p_x, prev_rect.y + prev_h + 12, p_w, 22);
        draw::draw_text(
            dc,
            &item.title,
            &title_rect,
            theme.text_primary,
            fonts.section,
            f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE | f::DT_END_ELLIPSIS,
        );

        // Specs Grid
        let mut row_y = title_rect.y + 30;
        let draw_row = |dc: f::HDC, label: &str, val: &str, y: i32| {
            let l_rect = UiRect::new(p_x, y, 90, 16);
            draw::draw_text(
                dc,
                label,
                &l_rect,
                theme.text_tertiary,
                fonts.caption,
                f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
            );
            let v_rect = UiRect::new(p_x + 95, y, p_w - 95, 16);
            draw::draw_text(
                dc,
                val,
                &v_rect,
                theme.text_primary,
                fonts.caption,
                f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE | f::DT_END_ELLIPSIS,
            );
        };

        draw_row(
            dc,
            "Dimensions:",
            &format!("{} × {}", item.width, item.height),
            row_y,
        );
        row_y += 20;
        draw_row(dc, "Framerate:", &format!("{:.2} FPS", item.fps), row_y);
        row_y += 20;
        draw_row(dc, "Duration:", &format!("{:.1}s", item.duration), row_y);
        row_y += 20;
        draw_row(dc, "Profile:", item.profile.text(), row_y);
        row_y += 20;
        draw_row(
            dc,
            "Cache Size:",
            &format!("{:.1} MiB", item.bytes as f64 / 1_048_576.0),
            row_y,
        );
        row_y += 20;

        // Target Monitor Selector
        let mon_label_rect = UiRect::new(p_x, row_y + 8, p_w, 16);
        draw::draw_text(
            dc,
            "TARGET MONITOR:",
            &mon_label_rect,
            theme.text_tertiary,
            fonts.caption,
            f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
        );

        row_y += 28;
        let sel_mon = monitors
            .get(ui.selected_monitor_idx)
            .or_else(|| monitors.first());
        let mon_str = sel_mon
            .map(|m| format!("{} ({}×{})", m.device, m.rect.width(), m.rect.height()))
            .unwrap_or_else(|| "Primary Display".into());

        let mon_btn = UiRect::new(p_x, row_y, p_w, 28);
        let is_mon_hover = ui.hovered_btn_id == Some("lib_cycle_monitor");
        draw::draw_button(
            dc,
            &mon_btn,
            &mon_str,
            false,
            is_mon_hover,
            false,
            theme,
            fonts,
        );

        // Action Buttons
        row_y += 36;
        let btn_apply = UiRect::new(p_x, row_y, p_w, 34);
        let is_apply_hover = ui.hovered_btn_id == Some("lib_apply");
        draw::draw_button(
            dc,
            &btn_apply,
            "▶ Apply to Display",
            true,
            is_apply_hover,
            false,
            theme,
            fonts,
        );

        row_y += 40;
        let btn_reopt = UiRect::new(p_x, row_y, (p_w - 8) / 2, 28);
        let is_reopt_hover = ui.hovered_btn_id == Some("lib_reoptimize");
        draw::draw_button(
            dc,
            &btn_reopt,
            "⚡ Re-optimize",
            false,
            is_reopt_hover,
            false,
            theme,
            fonts,
        );

        let btn_reveal = UiRect::new(p_x + (p_w - 8) / 2 + 8, row_y, (p_w - 8) / 2, 28);
        let is_reveal_hover = ui.hovered_btn_id == Some("lib_reveal");
        draw::draw_button(
            dc,
            &btn_reveal,
            "📁 Show File",
            false,
            is_reveal_hover,
            false,
            theme,
            fonts,
        );

        row_y += 34;
        let btn_remove = UiRect::new(p_x, row_y, p_w, 28);
        let is_remove_hover = ui.hovered_btn_id == Some("lib_remove");
        draw::draw_button(
            dc,
            &btn_remove,
            "🗑️ Remove Wallpaper",
            false,
            is_remove_hover,
            false,
            theme,
            fonts,
        );
    } else {
        let no_sel_rect = UiRect::new(
            detail_rect.x,
            detail_rect.y + detail_h(detail_rect.h) / 2 - 20,
            detail_rect.w,
            40,
        );
        draw::draw_text(
            dc,
            "Select a wallpaper\nto view properties",
            &no_sel_rect,
            theme.text_tertiary,
            fonts.caption,
            f::DT_CENTER | f::DT_VCENTER | f::DT_WORDBREAK,
        );
    }
}

fn detail_h(h: i32) -> i32 {
    h
}

fn render_displays(
    dc: f::HDC,
    ui: &mut UiState,
    items: &[LibraryItem],
    monitors: &[Monitor],
    assignments: &std::collections::BTreeMap<String, String>,
    bounds: &UiRect,
    ctx: &ViewContext,
) {
    let theme = &ctx.theme;
    let fonts = ctx.fonts;

    let desc_rect = UiRect::new(bounds.x, bounds.y, bounds.w, 20);
    draw::draw_text(
        dc,
        "Connected physical monitors and current WorkerW desktop wallpaper assignments:",
        &desc_rect,
        theme.text_secondary,
        fonts.body,
        f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
    );

    let cards_top = bounds.y + 32;
    let card_w = (bounds.w - 18).clamp(320, 480);
    let card_h = 160;

    for (idx, mon) in monitors.iter().enumerate() {
        let card_y = cards_top + (idx as i32) * (card_h + 16);
        if card_y + card_h > bounds.y + bounds.h {
            break;
        }

        let is_sel = ui.selected_monitor_idx == idx;
        let is_hover = ui.hovered_item_idx == Some(idx);
        let card_rect = UiRect::new(bounds.x, card_y, card_w, card_h);
        draw::draw_card(dc, &card_rect, is_hover, is_sel, theme);

        // Monitor Icon & Name
        let mon_icon = UiRect::new(card_rect.x + 16, card_rect.y + 16, 28, 28);
        draw::fill_round_rect(dc, &mon_icon, 6, theme.surface_elevated, None);
        draw::draw_text(
            dc,
            "🖥",
            &mon_icon,
            theme.accent,
            fonts.heading,
            f::DT_CENTER | f::DT_VCENTER | f::DT_SINGLELINE,
        );

        let name_rect = UiRect::new(card_rect.x + 52, card_rect.y + 14, 180, 20);
        draw::draw_text(
            dc,
            &mon.device,
            &name_rect,
            theme.text_primary,
            fonts.body_bold,
            f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
        );

        if mon.primary {
            let prim_rect = UiRect::new(card_rect.x + 190, card_rect.y + 15, 68, 18);
            draw::draw_pill(
                dc,
                &prim_rect,
                "PRIMARY",
                theme.accent_subtle,
                theme.accent,
                None,
                fonts.caption,
            );
        }

        let geom_rect = UiRect::new(card_rect.x + 52, card_rect.y + 34, 220, 16);
        let geom_text = format!(
            "{} × {} @ ({}, {})",
            mon.rect.width(),
            mon.rect.height(),
            mon.rect.left,
            mon.rect.top
        );
        draw::draw_text(
            dc,
            &geom_text,
            &geom_rect,
            theme.text_tertiary,
            fonts.caption,
            f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
        );

        // Current Assigned Wallpaper Info
        let assigned_id = assignments.get(&mon.device);
        let assigned_item = assigned_id.and_then(|id| items.iter().find(|i| &i.id == id));

        let info_rect = UiRect::new(card_rect.x + 16, card_rect.y + 64, card_rect.w - 32, 42);
        draw::fill_round_rect(
            dc,
            &info_rect,
            4,
            theme.surface_elevated,
            Some(theme.border_subtle),
        );

        if let Some(item) = assigned_item {
            let wall_title = UiRect::new(info_rect.x + 12, info_rect.y + 4, info_rect.w - 110, 18);
            draw::draw_text(
                dc,
                &item.title,
                &wall_title,
                theme.text_primary,
                fonts.caption,
                f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE | f::DT_END_ELLIPSIS,
            );

            let wall_sub = UiRect::new(info_rect.x + 12, info_rect.y + 22, info_rect.w - 110, 14);
            draw::draw_text(
                dc,
                &format!("{} • {:.0} FPS", item.profile.text(), item.fps),
                &wall_sub,
                theme.text_secondary,
                fonts.caption,
                f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
            );

            let status_rect = UiRect::new(info_rect.x + info_rect.w - 85, info_rect.y + 11, 75, 20);
            draw::draw_pill(
                dc,
                &status_rect,
                "● Playing",
                theme.success_subtle,
                theme.success,
                None,
                fonts.caption,
            );
        } else {
            let empty_text = UiRect::new(info_rect.x + 12, info_rect.y + 12, info_rect.w - 24, 18);
            draw::draw_text(
                dc,
                "No wallpaper assigned to this display",
                &empty_text,
                theme.text_tertiary,
                fonts.caption,
                f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
            );
        }

        // Action Buttons Row
        let btn_y = card_rect.y + 116;
        let btn_apply = UiRect::new(card_rect.x + 16, btn_y, 160, 30);
        let apply_id = if idx == 0 {
            "disp_apply_0"
        } else {
            "disp_apply_1"
        };
        let is_apply_hover = ui.hovered_btn_id == Some(apply_id);
        draw::draw_button(
            dc,
            &btn_apply,
            "Apply Selected Wallpaper",
            true,
            is_apply_hover,
            false,
            theme,
            fonts,
        );

        let btn_clear = UiRect::new(card_rect.x + 184, btn_y, 110, 30);
        let clear_id = if idx == 0 {
            "disp_clear_0"
        } else {
            "disp_clear_1"
        };
        let is_clear_hover = ui.hovered_btn_id == Some(clear_id);
        draw::draw_button(
            dc,
            &btn_clear,
            "Clear Display",
            false,
            is_clear_hover,
            false,
            theme,
            fonts,
        );
    }
}

fn render_optimization(
    dc: f::HDC,
    ui: &mut UiState,
    _items: &[LibraryItem],
    _monitors: &[Monitor],
    bounds: &UiRect,
    ctx: &ViewContext,
) {
    let theme = &ctx.theme;
    let fonts = ctx.fonts;

    let desc_rect = UiRect::new(bounds.x, bounds.y, bounds.w, 20);
    draw::draw_text(
        dc,
        "Select the performance profile used for media conversion and wallpaper playback:",
        &desc_rect,
        theme.text_secondary,
        fonts.body,
        f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
    );

    // 4 Profile Cards (2x2 grid)
    let grid_y = bounds.y + 32;
    let card_gap = 16;
    let card_w = (bounds.w - card_gap) / 2;
    let card_h = 120;

    let profiles = [
        (
            Profile::UltraEco,
            "Ultra Eco",
            "Lowest CPU & battery load",
            "~15 FPS • 720p/1080p • High compression (CRF 27)",
        ),
        (
            Profile::Balanced,
            "Balanced (Recommended)",
            "Recommended default profile",
            "30 FPS • 1080p • Optimal efficiency & clarity (CRF 23)",
        ),
        (
            Profile::Quality,
            "Quality",
            "Maximum visual fidelity",
            "Up to 60 FPS • 4K (3840×2160) • Crisp detail (CRF 19)",
        ),
        (
            Profile::Automatic,
            "Automatic",
            "Dynamic power-aware switching",
            "Runs Ultra Eco on battery, Balanced on AC power",
        ),
    ];

    for (idx, (prof, title, subtitle, details)) in profiles.iter().enumerate() {
        let col = (idx as i32) % 2;
        let row = (idx as i32) / 2;
        let card_x = bounds.x + col * (card_w + card_gap);
        let card_y = grid_y + row * (card_h + card_gap);
        let card_rect = UiRect::new(card_x, card_y, card_w, card_h);

        let is_active = true; // Placeholder for current profile matching
        let prof_btn_id = match prof {
            Profile::UltraEco => "prof_eco",
            Profile::Balanced => "prof_balanced",
            Profile::Quality => "prof_quality",
            Profile::Automatic => "prof_auto",
        };
        let is_hover = ui.hovered_btn_id == Some(prof_btn_id);

        draw::draw_card(dc, &card_rect, is_hover, is_active, theme);

        // Radio indicator
        let radio_box = UiRect::new(card_rect.x + 16, card_rect.y + 16, 18, 18);
        draw::fill_round_rect(
            dc,
            &radio_box,
            9,
            if is_active {
                theme.accent
            } else {
                theme.surface_active
            },
            Some(theme.border),
        );
        if is_active {
            let dot = UiRect::new(radio_box.x + 5, radio_box.y + 5, 8, 8);
            draw::fill_round_rect(dc, &dot, 4, theme.text_on_accent, None);
        }

        // Title & Badges
        let title_rect = UiRect::new(card_rect.x + 44, card_rect.y + 14, card_w - 60, 20);
        draw::draw_text(
            dc,
            title,
            &title_rect,
            theme.text_primary,
            fonts.body_bold,
            f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
        );

        let sub_rect = UiRect::new(card_rect.x + 44, card_rect.y + 36, card_w - 60, 16);
        draw::draw_text(
            dc,
            subtitle,
            &sub_rect,
            theme.text_secondary,
            fonts.caption,
            f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
        );

        let det_rect = UiRect::new(card_rect.x + 44, card_rect.y + 60, card_w - 60, 42);
        draw::draw_text(
            dc,
            details,
            &det_rect,
            theme.text_tertiary,
            fonts.caption,
            f::DT_LEFT | f::DT_TOP | f::DT_WORDBREAK,
        );
    }

    // Tuning & Manual Controls Card
    let tune_y = grid_y + card_h * 2 + card_gap + 16;
    let tune_h = 135;
    let tune_rect = UiRect::new(bounds.x, tune_y, bounds.w, tune_h);
    draw::draw_card(dc, &tune_rect, false, false, theme);

    let tune_title = UiRect::new(tune_rect.x + 18, tune_rect.y + 14, 300, 20);
    draw::draw_text(
        dc,
        "MANUAL OVERRIDES & RE-OPTIMIZATION",
        &tune_title,
        theme.text_primary,
        fonts.body_bold,
        f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
    );

    let tune_sub = UiRect::new(tune_rect.x + 18, tune_rect.y + 34, bounds.w - 36, 16);
    draw::draw_text(
        dc,
        "Override framerate and resolution limits for new imports and re-optimization (0 = profile default).",
        &tune_sub,
        theme.text_secondary,
        fonts.caption,
        f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
    );

    // Overrides row
    let row_y = tune_rect.y + 64;
    let l1 = UiRect::new(tune_rect.x + 18, row_y, 110, 24);
    draw::draw_text(
        dc,
        "Custom FPS Limit:",
        &l1,
        theme.text_secondary,
        fonts.body,
        f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
    );

    let fps_box = UiRect::new(tune_rect.x + 130, row_y, 60, 26);
    draw::fill_round_rect(dc, &fps_box, 4, theme.surface_elevated, Some(theme.border));
    draw::draw_text(
        dc,
        "0",
        &fps_box,
        theme.text_primary,
        fonts.mono,
        f::DT_CENTER | f::DT_VCENTER | f::DT_SINGLELINE,
    );

    let l2 = UiRect::new(tune_rect.x + 220, row_y, 130, 24);
    draw::draw_text(
        dc,
        "Max Resolution (px):",
        &l2,
        theme.text_secondary,
        fonts.body,
        f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
    );

    let cap_box = UiRect::new(tune_rect.x + 355, row_y, 70, 26);
    draw::fill_round_rect(dc, &cap_box, 4, theme.surface_elevated, Some(theme.border));
    draw::draw_text(
        dc,
        "0",
        &cap_box,
        theme.text_primary,
        fonts.mono,
        f::DT_CENTER | f::DT_VCENTER | f::DT_SINGLELINE,
    );

    // Re-optimize button
    let reopt_rect = UiRect::new(tune_rect.x + tune_rect.w - 230, row_y - 2, 210, 32);
    let is_reopt_hover = ui.hovered_btn_id == Some("opt_reoptimize");
    draw::draw_button(
        dc,
        &reopt_rect,
        "⚡ Re-optimize Selected Item",
        true,
        is_reopt_hover,
        false,
        theme,
        fonts,
    );
}

fn render_performance(dc: f::HDC, ui: &mut UiState, bounds: &UiRect, ctx: &ViewContext) {
    let theme = &ctx.theme;
    let fonts = ctx.fonts;

    // 4 Top Metric Cards
    let card_gap = 14;
    let card_w = (bounds.w - card_gap * 3) / 4;
    let card_h = 88;

    let c1 = UiRect::new(bounds.x, bounds.y, card_w, card_h);
    let cpu_str = format!("{:.2}%", ctx.cpu_pct);
    draw::draw_stat_card(
        dc,
        &c1,
        "FEATHERWALL CPU LOAD",
        &cpu_str,
        "Overall CPU capacity",
        Some(theme.chart_cpu),
        theme,
        fonts,
    );

    let c2 = UiRect::new(bounds.x + card_w + card_gap, bounds.y, card_w, card_h);
    let ram_str = format!("{:.1} MiB", ctx.working_set_mb);
    draw::draw_stat_card(
        dc,
        &c2,
        "PHYSICAL WORKING SET",
        &ram_str,
        "Process RAM usage",
        Some(theme.chart_mem),
        theme,
        fonts,
    );

    let c3 = UiRect::new(bounds.x + (card_w + card_gap) * 2, bounds.y, card_w, card_h);
    let priv_str = format!("{:.1} MiB", ctx.private_commit_mb);
    draw::draw_stat_card(
        dc,
        &c3,
        "PRIVATE COMMITTED",
        &priv_str,
        "Private memory pages",
        Some(theme.accent),
        theme,
        fonts,
    );

    let c4 = UiRect::new(bounds.x + (card_w + card_gap) * 3, bounds.y, card_w, card_h);
    let dec_str = format!("{} Active", ctx.playing_count);
    draw::draw_stat_card(
        dc,
        &c4,
        "MEDIA FOUNDATION EVR",
        &dec_str,
        "Hardware D3D decoder",
        Some(theme.warning),
        theme,
        fonts,
    );

    // Real-Time Historical Sparkline Chart
    let chart_y = bounds.y + card_h + 16;
    let chart_h = 165;
    let chart_rect = UiRect::new(bounds.x, chart_y, bounds.w, chart_h);
    draw::draw_history_chart(
        dc,
        &chart_rect,
        &ui.cpu_history,
        &ui.mem_history,
        theme,
        fonts,
    );

    // Diagnostics & Recent Logs Card
    let diag_y = chart_y + chart_h + 16;
    let diag_h = bounds.h - (diag_y - bounds.y);
    let diag_rect = UiRect::new(bounds.x, diag_y, bounds.w, diag_h);
    draw::draw_card(dc, &diag_rect, false, false, theme);

    let diag_title = UiRect::new(diag_rect.x + 16, diag_rect.y + 12, 300, 20);
    draw::draw_text(
        dc,
        "RECENT APPLICATION LOGS & DIAGNOSTICS",
        &diag_title,
        theme.text_primary,
        fonts.body_bold,
        f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
    );

    let btn_open_log = UiRect::new(diag_rect.x + diag_rect.w - 145, diag_rect.y + 10, 130, 26);
    let is_log_hover = ui.hovered_btn_id == Some("perf_open_log");
    draw::draw_button(
        dc,
        &btn_open_log,
        "📄 Open Log File",
        false,
        is_log_hover,
        false,
        theme,
        fonts,
    );

    // Log display box
    let log_box = UiRect::new(
        diag_rect.x + 16,
        diag_rect.y + 40,
        diag_rect.w - 32,
        diag_h - 52,
    );
    draw::fill_round_rect(
        dc,
        &log_box,
        4,
        theme.surface_elevated,
        Some(theme.border_subtle),
    );

    let mut log_y = log_box.y + 6;
    for line in ui.recent_logs.iter().rev().take(7) {
        if log_y + 16 > log_box.y + log_box.h {
            break;
        }
        let line_rect = UiRect::new(log_box.x + 10, log_y, log_box.w - 20, 16);
        draw::draw_text(
            dc,
            line,
            &line_rect,
            theme.text_secondary,
            fonts.caption,
            f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE | f::DT_END_ELLIPSIS,
        );
        log_y += 18;
    }
}

fn render_settings(
    dc: f::HDC,
    ui: &mut UiState,
    _monitors: &[Monitor],
    bounds: &UiRect,
    ctx: &ViewContext,
) {
    let theme = &ctx.theme;
    let fonts = ctx.fonts;

    // Two column layout for Settings cards
    let card_gap = 16;
    let card_w = (bounds.w - card_gap) / 2;
    let card_h = 240;

    // 1. General Preferences Card
    let c1 = UiRect::new(bounds.x, bounds.y, card_w, card_h);
    draw::draw_card(dc, &c1, false, false, theme);

    let t1 = UiRect::new(c1.x + 18, c1.y + 14, card_w - 36, 20);
    draw::draw_text(
        dc,
        "GENERAL PREFERENCES",
        &t1,
        theme.text_primary,
        fonts.body_bold,
        f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
    );

    let mut y1 = c1.y + 44;
    draw::draw_checkbox(
        dc,
        c1.x + 18,
        y1,
        card_w - 36,
        false,
        "Start FeatherWall with Windows",
        ui.hovered_btn_id == Some("set_autostart"),
        theme,
        fonts,
    );
    y1 += 34;

    let theme_lbl = UiRect::new(c1.x + 18, y1, 140, 24);
    draw::draw_text(
        dc,
        "Interface Appearance:",
        &theme_lbl,
        theme.text_secondary,
        fonts.body,
        f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
    );

    let theme_toggle_btn = UiRect::new(c1.x + 160, y1, 120, 28);
    let theme_text = if theme.is_dark {
        "🌙 Fluent Dark"
    } else {
        "☀️ Fluent Light"
    };
    draw::draw_button(
        dc,
        &theme_toggle_btn,
        theme_text,
        false,
        ui.hovered_btn_id == Some("set_theme_toggle"),
        false,
        theme,
        fonts,
    );
    y1 += 38;

    let def_prof_lbl = UiRect::new(c1.x + 18, y1, card_w - 36, 16);
    draw::draw_text(
        dc,
        "Default Import Profile: Balanced (Recommended)",
        &def_prof_lbl,
        theme.text_tertiary,
        fonts.caption,
        f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
    );

    // 2. Playback & Battery Policies Card
    let c2 = UiRect::new(bounds.x + card_w + card_gap, bounds.y, card_w, card_h);
    draw::draw_card(dc, &c2, false, false, theme);

    let t2 = UiRect::new(c2.x + 18, c2.y + 14, card_w - 36, 20);
    draw::draw_text(
        dc,
        "PLAYBACK & SUSPENSION POLICIES",
        &t2,
        theme.text_primary,
        fonts.body_bold,
        f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
    );

    let mut y2 = c2.y + 44;
    draw::draw_checkbox(
        dc,
        c2.x + 18,
        y2,
        card_w - 36,
        true,
        "Pause when fullscreen app or game is active",
        ui.hovered_btn_id == Some("set_pause_fs"),
        theme,
        fonts,
    );
    y2 += 28;
    draw::draw_checkbox(
        dc,
        c2.x + 18,
        y2,
        card_w - 36,
        false,
        "Pause behind maximized windows",
        ui.hovered_btn_id == Some("set_pause_max"),
        theme,
        fonts,
    );
    y2 += 28;
    draw::draw_checkbox(
        dc,
        c2.x + 18,
        y2,
        card_w - 36,
        false,
        "Pause when running on battery power",
        ui.hovered_btn_id == Some("set_pause_bat"),
        theme,
        fonts,
    );
    y2 += 28;
    draw::draw_checkbox(
        dc,
        c2.x + 18,
        y2,
        card_w - 36,
        true,
        "Pause in Windows Battery Saver mode",
        ui.hovered_btn_id == Some("set_pause_sav"),
        theme,
        fonts,
    );
    y2 += 28;
    draw::draw_checkbox(
        dc,
        c2.x + 18,
        y2,
        card_w - 36,
        true,
        "Pause during Remote Desktop sessions",
        ui.hovered_btn_id == Some("set_pause_rdp"),
        theme,
        fonts,
    );
    y2 += 28;
    draw::draw_checkbox(
        dc,
        c2.x + 18,
        y2,
        card_w - 36,
        true,
        "Adaptive battery policy (Ultra Eco on battery)",
        ui.hovered_btn_id == Some("set_adaptive"),
        theme,
        fonts,
    );

    // 3. Storage & Cache Management Card
    let row2_y = bounds.y + card_h + card_gap;
    let c3 = UiRect::new(bounds.x, row2_y, card_w, 180);
    draw::draw_card(dc, &c3, false, false, theme);

    let t3 = UiRect::new(c3.x + 18, c3.y + 14, card_w - 36, 20);
    draw::draw_text(
        dc,
        "STORAGE & CACHE",
        &t3,
        theme.text_primary,
        fonts.body_bold,
        f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
    );

    let c_dir = UiRect::new(c3.x + 18, c3.y + 38, card_w - 36, 16);
    draw::draw_text(
        dc,
        "%LOCALAPPDATA%\\FeatherWall\\cache",
        &c_dir,
        theme.text_secondary,
        fonts.caption,
        f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
    );

    let c_sz = UiRect::new(c3.x + 18, c3.y + 58, card_w - 36, 22);
    draw::draw_text(
        dc,
        &format!("Cache size: {}", ui.cache_size_str),
        &c_sz,
        theme.text_primary,
        fonts.section,
        f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
    );

    let btn_clean = UiRect::new(c3.x + 18, c3.y + 120, 140, 30);
    let is_clean_hover = ui.hovered_btn_id == Some("set_clean_cache");
    draw::draw_button(
        dc,
        &btn_clean,
        "🧹 Clean Cache",
        false,
        is_clean_hover,
        false,
        theme,
        fonts,
    );

    let btn_folder = UiRect::new(c3.x + 168, c3.y + 120, 150, 30);
    let is_folder_hover = ui.hovered_btn_id == Some("set_open_folder");
    draw::draw_button(
        dc,
        &btn_folder,
        "📁 Open Folder",
        false,
        is_folder_hover,
        false,
        theme,
        fonts,
    );

    // 4. Advanced & About Card
    let c4 = UiRect::new(bounds.x + card_w + card_gap, row2_y, card_w, 180);
    draw::draw_card(dc, &c4, false, false, theme);

    let t4 = UiRect::new(c4.x + 18, c4.y + 14, card_w - 36, 20);
    draw::draw_text(
        dc,
        "ABOUT FEATHERWALL",
        &t4,
        theme.text_primary,
        fonts.body_bold,
        f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
    );

    let ab1 = UiRect::new(c4.x + 18, c4.y + 40, card_w - 36, 18);
    draw::draw_text(
        dc,
        "FeatherWall Desktop Wallpaper Engine v0.2.0",
        &ab1,
        theme.text_primary,
        fonts.body_bold,
        f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
    );

    let ab2 = UiRect::new(c4.x + 18, c4.y + 60, card_w - 36, 16);
    draw::draw_text(
        dc,
        "Engine: Rust • Win32 GDI • Windows Media Foundation EVR",
        &ab2,
        theme.text_secondary,
        fonts.caption,
        f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
    );

    let ab3 = UiRect::new(c4.x + 18, c4.y + 78, card_w - 36, 16);
    draw::draw_text(
        dc,
        "100% Offline • Zero Telemetry • MIT License",
        &ab3,
        theme.accent,
        fonts.caption,
        f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
    );

    let btn_log_about = UiRect::new(c4.x + 18, c4.y + 120, 140, 30);
    let is_log_about_hover = ui.hovered_btn_id == Some("set_open_log");
    draw::draw_button(
        dc,
        &btn_log_about,
        "📄 View Logs",
        false,
        is_log_about_hover,
        false,
        theme,
        fonts,
    );
}

fn render_confirm_modal(
    dc: f::HDC,
    win_w: i32,
    win_h: i32,
    item_title: &str,
    ui: &UiState,
    theme: &Theme,
    fonts: &Fonts,
) {
    // Backdrop overlay
    let overlay_rect = UiRect::new(0, 0, win_w, win_h);
    draw::fill_rect(dc, &overlay_rect, f::rgb(0, 0, 0));

    // Dialog card
    let modal_w = 420;
    let modal_h = 190;
    let modal_x = (win_w - modal_w) / 2;
    let modal_y = (win_h - modal_h) / 2;
    let modal_rect = UiRect::new(modal_x, modal_y, modal_w, modal_h);

    draw::fill_round_rect(dc, &modal_rect, 8, theme.surface_card, Some(theme.border));

    let m_title = UiRect::new(modal_x + 24, modal_y + 20, modal_w - 48, 24);
    draw::draw_text(
        dc,
        "Remove Wallpaper From Library?",
        &m_title,
        theme.text_primary,
        fonts.section,
        f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
    );

    let m_desc = UiRect::new(modal_x + 24, modal_y + 52, modal_w - 48, 44);
    let msg = format!(
        "Are you sure you want to remove \"{}\" from your wallpaper library?\nYour original media file will NOT be deleted.",
        item_title
    );
    draw::draw_text(
        dc,
        &msg,
        &m_desc,
        theme.text_secondary,
        fonts.body,
        f::DT_LEFT | f::DT_TOP | f::DT_WORDBREAK,
    );

    let btn_cancel = UiRect::new(modal_x + modal_w - 220, modal_y + modal_h - 48, 95, 32);
    let is_can_hover = ui.hovered_btn_id == Some("modal_cancel");
    draw::draw_button(
        dc,
        &btn_cancel,
        "Cancel",
        false,
        is_can_hover,
        false,
        theme,
        fonts,
    );

    let btn_delete = UiRect::new(modal_x + modal_w - 115, modal_y + modal_h - 48, 95, 32);
    let is_del_hover = ui.hovered_btn_id == Some("modal_confirm_delete");
    draw::draw_button(
        dc,
        &btn_delete,
        "Remove",
        true,
        is_del_hover,
        false,
        theme,
        fonts,
    );
}

pub fn render_widgets_view(
    dc: f::HDC,
    ui: &mut UiState,
    _monitors: &[Monitor],
    bounds: &UiRect,
    ctx: &ViewContext,
) {
    let theme = &ctx.theme;
    let fonts = ctx.fonts;
    let w = ctx.widgets;

    // 1. Live Interactive Desktop Simulation Card (Top banner)
    let preview_h = 135;
    let preview_rect = UiRect::new(bounds.x, bounds.y, bounds.w, preview_h);
    draw::draw_card(dc, &preview_rect, false, false, theme);

    // Dark frosted wallpaper preview container
    let prev_inner = UiRect::new(preview_rect.x + 8, preview_rect.y + 8, preview_rect.w - 16, preview_h - 16);
    draw::fill_round_rect(dc, &prev_inner, 6, f::rgb(14, 16, 20), Some(theme.border_subtle));

    // Badge: "✦ LIVE DESKTOP SIMULATION"
    let badge_rect = UiRect::new(prev_inner.x + 14, prev_inner.y + 10, 168, 20);
    draw::draw_pill(dc, &badge_rect, "✦ DESKTOP SIMULATION", theme.accent_subtle, theme.accent, Some(theme.accent), fonts.caption);

    // Preview Clock & Date on right side
    let prev_clock_str = if w.clock_24h {
        if w.clock_seconds { "20:29:45" } else { "20:29" }
    } else if w.clock_seconds {
        "8:29:45 PM"
    } else {
        "8:29 PM"
    };
    let prev_date_str = w.date_format.format(2026, 10, 21, 3);
    let prev_greet_str = if !w.greeting_name.is_empty() {
        format!("Good evening, {}!", w.greeting_name)
    } else {
        "Good evening, Alex!".into()
    };

    let p_clock_rect = UiRect::new(prev_inner.x + prev_inner.w - 290, prev_inner.y + 12, 270, 42);
    draw::draw_text(
        dc,
        prev_clock_str,
        &p_clock_rect,
        w.clock_color,
        fonts.title,
        f::DT_RIGHT | f::DT_VCENTER | f::DT_SINGLELINE,
    );

    let p_date_rect = UiRect::new(prev_inner.x + prev_inner.w - 290, prev_inner.y + 56, 270, 18);
    draw::draw_text(
        dc,
        &prev_date_str,
        &p_date_rect,
        0x00E2E8F0,
        fonts.body_bold,
        f::DT_RIGHT | f::DT_VCENTER | f::DT_SINGLELINE,
    );

    let p_greet_rect = UiRect::new(prev_inner.x + prev_inner.w - 290, prev_inner.y + 76, 270, 16);
    draw::draw_text(
        dc,
        &prev_greet_str,
        &p_greet_rect,
        0x00CBD5E1,
        fonts.caption,
        f::DT_RIGHT | f::DT_VCENTER | f::DT_SINGLELINE,
    );

    // Preview Music capsule (Left side of preview)
    let p_music_rect = UiRect::new(prev_inner.x + 14, prev_inner.y + 44, 290, 48);
    draw::fill_round_rect(dc, &p_music_rect, 24, f::rgb(22, 24, 30), Some(f::rgb(50, 54, 66)));
    let p_music_icon = UiRect::new(p_music_rect.x + 10, p_music_rect.y, 24, 48);
    draw::draw_text(dc, "♫", &p_music_icon, theme.accent, fonts.heading, f::DT_CENTER | f::DT_VCENTER | f::DT_SINGLELINE);
    let p_m_title = UiRect::new(p_music_rect.x + 38, p_music_rect.y + 8, 140, 16);
    draw::draw_text(dc, "Daft Punk — Get Lucky", &p_m_title, 0x00FFFFFF, fonts.caption, f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE);
    let p_m_artist = UiRect::new(p_music_rect.x + 38, p_music_rect.y + 24, 140, 14);
    draw::draw_text(dc, "Random Access Memories", &p_m_artist, 0x0094A3B8, fonts.caption, f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE);
    let p_m_ctrls = UiRect::new(p_music_rect.x + 184, p_music_rect.y, 96, 48);
    draw::draw_text(dc, "⏮  ▶  ⏭", &p_m_ctrls, 0x00E2E8F0, fonts.body_bold, f::DT_CENTER | f::DT_VCENTER | f::DT_SINGLELINE);

    // 2. Two-Column Settings Grid
    let grid_y = bounds.y + preview_h + 14;
    let card_gap = 14;
    let card_w = (bounds.w - card_gap) / 2;
    let card_h = 325;

    // LEFT CARD: CLOCK & DATE CONFIGURATION
    let c_left = UiRect::new(bounds.x, grid_y, card_w, card_h);
    draw::draw_card(dc, &c_left, false, false, theme);

    let t_clock = UiRect::new(c_left.x + 18, c_left.y + 14, card_w - 36, 20);
    draw::draw_text(
        dc,
        "DIGITAL CLOCK & DATE",
        &t_clock,
        theme.text_primary,
        fonts.body_bold,
        f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
    );

    let mut ly = c_left.y + 42;

    // Master Overlay Toggle
    draw::draw_checkbox(
        dc,
        c_left.x + 18,
        ly,
        card_w - 36,
        w.enabled,
        "Enable Desktop Widgets Master Switch",
        ui.hovered_btn_id == Some("w_master_toggle"),
        theme,
        fonts,
    );
    ly += 26;

    // Clock Enable
    draw::draw_checkbox(
        dc,
        c_left.x + 18,
        ly,
        card_w - 36,
        w.clock_enabled,
        "Show Large Floating Clock",
        ui.hovered_btn_id == Some("w_clock_toggle"),
        theme,
        fonts,
    );
    ly += 28;

    // 12h vs 24h & Seconds
    let btn_24h = UiRect::new(c_left.x + 18, ly, 100, 26);
    draw::draw_button(dc, &btn_24h, "24-Hour", w.clock_24h, ui.hovered_btn_id == Some("w_clock_24h"), false, theme, fonts);
    let btn_12h = UiRect::new(c_left.x + 124, ly, 105, 26);
    draw::draw_button(dc, &btn_12h, "12-Hour AM/PM", !w.clock_24h, ui.hovered_btn_id == Some("w_clock_12h"), false, theme, fonts);
    let btn_sec = UiRect::new(c_left.x + 235, ly, 105, 26);
    let sec_lbl = if w.clock_seconds { "Seconds: ON" } else { "Seconds: OFF" };
    draw::draw_button(dc, &btn_sec, sec_lbl, w.clock_seconds, ui.hovered_btn_id == Some("w_clock_sec_toggle"), false, theme, fonts);
    ly += 32;

    // Clock Font Size
    let sz_lbl = UiRect::new(c_left.x + 18, ly, 75, 24);
    draw::draw_text(dc, "Font Size:", &sz_lbl, theme.text_secondary, fonts.body, f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE);
    let s48 = UiRect::new(c_left.x + 95, ly, 55, 24);
    draw::draw_button(dc, &s48, "48pt", w.clock_size == 48, ui.hovered_btn_id == Some("w_size_48"), false, theme, fonts);
    let s64 = UiRect::new(c_left.x + 155, ly, 55, 24);
    draw::draw_button(dc, &s64, "64pt", w.clock_size == 64, ui.hovered_btn_id == Some("w_size_64"), false, theme, fonts);
    let s72 = UiRect::new(c_left.x + 215, ly, 55, 24);
    draw::draw_button(dc, &s72, "72pt", w.clock_size == 72, ui.hovered_btn_id == Some("w_size_72"), false, theme, fonts);
    let s96 = UiRect::new(c_left.x + 275, ly, 55, 24);
    draw::draw_button(dc, &s96, "96pt", w.clock_size == 96, ui.hovered_btn_id == Some("w_size_96"), false, theme, fonts);
    ly += 30;

    // Clock Text Color & Opacity
    let col_lbl = UiRect::new(c_left.x + 18, ly, 75, 24);
    draw::draw_text(dc, "Color/Opac:", &col_lbl, theme.text_secondary, fonts.body, f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE);
    let c_white = UiRect::new(c_left.x + 95, ly, 55, 24);
    draw::draw_button(dc, &c_white, "White", w.clock_color == 0x00FFFFFF, ui.hovered_btn_id == Some("w_color_white"), false, theme, fonts);
    let c_silver = UiRect::new(c_left.x + 155, ly, 55, 24);
    draw::draw_button(dc, &c_silver, "Silver", w.clock_color == 0x00E2E8F0, ui.hovered_btn_id == Some("w_color_silver"), false, theme, fonts);
    let c_gold = UiRect::new(c_left.x + 215, ly, 55, 24);
    draw::draw_button(dc, &c_gold, "Gold", w.clock_color == 0x00FBBF24, ui.hovered_btn_id == Some("w_color_gold"), false, theme, fonts);
    let c_indigo = UiRect::new(c_left.x + 275, ly, 55, 24);
    draw::draw_button(dc, &c_indigo, "Indigo", w.clock_color == 0x00818CF8, ui.hovered_btn_id == Some("w_color_indigo"), false, theme, fonts);
    ly += 30;

    // Date Enable
    draw::draw_checkbox(
        dc,
        c_left.x + 18,
        ly,
        card_w - 36,
        w.date_enabled,
        "Show Date Display",
        ui.hovered_btn_id == Some("w_date_toggle"),
        theme,
        fonts,
    );
    ly += 26;

    // Date Formats
    let df_full = UiRect::new(c_left.x + 18, ly, 75, 24);
    draw::draw_button(dc, &df_full, "Full", w.date_format == DateFormat::Full, ui.hovered_btn_id == Some("w_date_full"), false, theme, fonts);
    let df_md = UiRect::new(c_left.x + 98, ly, 80, 24);
    draw::draw_button(dc, &df_md, "Day, Date", w.date_format == DateFormat::MonthDay, ui.hovered_btn_id == Some("w_date_monthday"), false, theme, fonts);
    let df_dot = UiRect::new(c_left.x + 183, ly, 75, 24);
    draw::draw_button(dc, &df_dot, "Dot ·", w.date_format == DateFormat::Dot, ui.hovered_btn_id == Some("w_date_dot"), false, theme, fonts);
    let df_num = UiRect::new(c_left.x + 263, ly, 68, 24);
    draw::draw_button(dc, &df_num, "21/10", w.date_format == DateFormat::Numeric, ui.hovered_btn_id == Some("w_date_numeric"), false, theme, fonts);

    // RIGHT CARD: GREETING, MUSIC & POSITION CONFIGURATION
    let c_right = UiRect::new(bounds.x + card_w + card_gap, grid_y, card_w, card_h);
    draw::draw_card(dc, &c_right, false, false, theme);

    let t_right = UiRect::new(c_right.x + 18, c_right.y + 14, card_w - 36, 20);
    draw::draw_text(
        dc,
        "GREETINGS, MUSIC & POSITION",
        &t_right,
        theme.text_primary,
        fonts.body_bold,
        f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE,
    );

    let mut ry = c_right.y + 42;

    // Greeting Toggle
    draw::draw_checkbox(
        dc,
        c_right.x + 18,
        ry,
        card_w - 36,
        w.greeting_enabled,
        "Show Intelligent Personalized Greetings",
        ui.hovered_btn_id == Some("w_greet_toggle"),
        theme,
        fonts,
    );
    ry += 26;

    // Greeting Mode: Time / Rotating / Fixed
    let gm_time = UiRect::new(c_right.x + 18, ry, 100, 24);
    draw::draw_button(dc, &gm_time, "Time of Day", w.greeting_mode == GreetingMode::TimeBased, ui.hovered_btn_id == Some("w_greet_time"), false, theme, fonts);
    let gm_rot = UiRect::new(c_right.x + 124, ry, 100, 24);
    draw::draw_button(dc, &gm_rot, "Rotating", w.greeting_mode == GreetingMode::Rotating, ui.hovered_btn_id == Some("w_greet_rotating"), false, theme, fonts);
    let gm_fix = UiRect::new(c_right.x + 230, ry, 95, 24);
    draw::draw_button(dc, &gm_fix, "Fixed", w.greeting_mode == GreetingMode::Fixed, ui.hovered_btn_id == Some("w_greet_fixed"), false, theme, fonts);
    ry += 30;

    // Music Toggle
    draw::draw_checkbox(
        dc,
        c_right.x + 18,
        ry,
        card_w - 36,
        w.music_enabled,
        "Show Floating Music Controller",
        ui.hovered_btn_id == Some("w_music_toggle"),
        theme,
        fonts,
    );
    ry += 26;

    // Music Mode: Spotify / Universal & Launch Button
    let btn_spot = UiRect::new(c_right.x + 18, ry, 110, 26);
    draw::draw_button(dc, &btn_spot, "Prefer Spotify", w.music_spotify_preferred, ui.hovered_btn_id == Some("w_music_spotify"), false, theme, fonts);
    let btn_gen = UiRect::new(c_right.x + 134, ry, 110, 26);
    draw::draw_button(dc, &btn_gen, "General Media", w.music_general_media, ui.hovered_btn_id == Some("w_music_general"), false, theme, fonts);
    let btn_launch = UiRect::new(c_right.x + 250, ry, 90, 26);
    draw::draw_button(dc, &btn_launch, "▶ Spotify", false, ui.hovered_btn_id == Some("w_open_spotify"), false, theme, fonts);
    ry += 32;

    // Anchor Position Selectors
    let pos_lbl = UiRect::new(c_right.x + 18, ry, 75, 24);
    draw::draw_text(dc, "Position:", &pos_lbl, theme.text_secondary, fonts.body, f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE);
    let p_tr = UiRect::new(c_right.x + 95, ry, 75, 24);
    draw::draw_button(dc, &p_tr, "Top Right", w.position == WidgetPosition::TopRight, ui.hovered_btn_id == Some("w_pos_tr"), false, theme, fonts);
    let p_tl = UiRect::new(c_right.x + 175, ry, 75, 24);
    draw::draw_button(dc, &p_tl, "Top Left", w.position == WidgetPosition::TopLeft, ui.hovered_btn_id == Some("w_pos_tl"), false, theme, fonts);
    let p_c = UiRect::new(c_right.x + 255, ry, 75, 24);
    draw::draw_button(dc, &p_c, "Center", w.position == WidgetPosition::Center, ui.hovered_btn_id == Some("w_pos_c"), false, theme, fonts);
    ry += 28;

    // Fine Offset Adjusters & Text Shadow
    let off_lbl = UiRect::new(c_right.x + 18, ry, 75, 24);
    draw::draw_text(dc, "Offset:", &off_lbl, theme.text_secondary, fonts.body, f::DT_LEFT | f::DT_VCENTER | f::DT_SINGLELINE);
    let btn_xm = UiRect::new(c_right.x + 95, ry, 34, 24);
    draw::draw_button(dc, &btn_xm, "X-", false, ui.hovered_btn_id == Some("w_offset_x_minus"), false, theme, fonts);
    let btn_xp = UiRect::new(c_right.x + 133, ry, 34, 24);
    draw::draw_button(dc, &btn_xp, "X+", false, ui.hovered_btn_id == Some("w_offset_x_plus"), false, theme, fonts);
    let btn_ym = UiRect::new(c_right.x + 175, ry, 34, 24);
    draw::draw_button(dc, &btn_ym, "Y-", false, ui.hovered_btn_id == Some("w_offset_y_minus"), false, theme, fonts);
    let btn_yp = UiRect::new(c_right.x + 213, ry, 34, 24);
    draw::draw_button(dc, &btn_yp, "Y+", false, ui.hovered_btn_id == Some("w_offset_y_plus"), false, theme, fonts);
    let btn_sh = UiRect::new(c_right.x + 255, ry, 75, 24);
    draw::draw_button(dc, &btn_sh, "Shadow", w.text_shadow, ui.hovered_btn_id == Some("w_shadow_toggle"), false, theme, fonts);
    ry += 30;

    // Action Button: "Reposition on Desktop (Edit Mode)"
    let edit_btn = UiRect::new(c_right.x + 18, ry, card_w - 36, 30);
    let edit_text = if w.edit_mode {
        "✓ Done Repositioning (Exit Edit Mode)"
    } else {
        "✥ Interactive Desktop Reposition Mode"
    };
    draw::draw_button(
        dc,
        &edit_btn,
        edit_text,
        w.edit_mode,
        ui.hovered_btn_id == Some("w_edit_mode"),
        false,
        theme,
        fonts,
    );
}

