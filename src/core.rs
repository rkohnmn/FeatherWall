//! Configuration and library persistence: no native dependencies and no network access.
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const FORMAT_VERSION: u32 = 2;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    UltraEco,
    Balanced,
    Quality,
    Automatic,
}
impl Profile {
    pub const ALL: [Self; 4] = [
        Self::UltraEco,
        Self::Balanced,
        Self::Quality,
        Self::Automatic,
    ];
    pub fn text(self) -> &'static str {
        match self {
            Self::UltraEco => "Ultra Eco",
            Self::Balanced => "Balanced",
            Self::Quality => "Quality",
            Self::Automatic => "Automatic",
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Self::UltraEco => "eco",
            Self::Balanced => "balanced",
            Self::Quality => "quality",
            Self::Automatic => "automatic",
        }
    }
    pub fn parse(v: &str) -> Self {
        match v {
            "eco" => Self::UltraEco,
            "quality" => Self::Quality,
            "automatic" => Self::Automatic,
            _ => Self::Balanced,
        }
    }
    pub fn preset(
        self,
        source_width: u32,
        source_height: u32,
        source_fps: f64,
        display_width: u32,
        display_height: u32,
        battery: bool,
    ) -> Plan {
        let selected = if self == Self::Automatic {
            if battery || source_fps <= 18.0 {
                Self::UltraEco
            } else {
                Self::Balanced
            }
        } else {
            self
        };
        let (cap, fps, crf, preset): (u32, f64, u8, &'static str) = match selected {
            Self::UltraEco => (1280, 15.0, 27, "medium"),
            Self::Balanced => (1920, 30.0, 23, "medium"),
            Self::Quality => (3840, 60.0, 19, "medium"),
            Self::Automatic => unreachable!(),
        };
        let source_width = source_width.max(2);
        let source_height = source_height.max(2);
        let monitor_width = display_width.max(2);
        let monitor_height = display_height.max(2);
        let factor = (cap as f64 / source_width.max(source_height) as f64)
            .min(monitor_width as f64 / source_width as f64)
            .min(monitor_height as f64 / source_height as f64)
            .min(1.0);
        let even = |n: u32| (n.max(2) / 2) * 2;
        Plan {
            width: even(((source_width as f64 * factor).round() as u32).max(2)),
            height: even(((source_height as f64 * factor).round() as u32).max(2)),
            fps: if source_fps > 0.0 {
                fps.min(source_fps)
            } else {
                fps
            },
            crf,
            encoder_preset: preset,
            resolved: selected,
        }
    }
}
#[derive(Clone, Debug)]
pub struct Plan {
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub crf: u8,
    pub encoder_preset: &'static str,
    pub resolved: Profile,
}
impl Plan {
    pub fn key(&self) -> String {
        format!(
            "{}-{}x{}-{:.3}-crf{}-{}-v2",
            self.resolved.key(),
            self.width,
            self.height,
            self.fps,
            self.crf,
            self.encoder_preset
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaKind {
    Video,
    Still,
}
impl MediaKind {
    pub fn text(self) -> &'static str {
        match self {
            Self::Video => "video",
            Self::Still => "still",
        }
    }
    pub fn parse(s: &str) -> Self {
        if s == "still" {
            Self::Still
        } else {
            Self::Video
        }
    }
}
#[derive(Clone, Debug)]
pub struct LibraryItem {
    pub id: String,
    pub title: String,
    pub original: PathBuf,
    pub asset: PathBuf,
    pub kind: MediaKind,
    pub profile: Profile,
    pub source_width: u32,
    pub source_height: u32,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub duration: f64,
    pub bytes: u64,
    pub imported_at: u64,
    pub note: String,
}
impl LibraryItem {
    pub fn thumb_path(&self) -> PathBuf {
        let parent = self.asset.parent().unwrap_or_else(|| Path::new(""));
        parent.join(format!("{}.thumb.bmp", self.id))
    }
}
#[derive(Clone, Debug)]
pub struct Settings {
    pub profile: Profile,
    pub pause_fullscreen: bool,
    pub pause_maximized: bool,
    pub pause_battery: bool,
    pub pause_saver: bool,
    pub pause_remote: bool,
    pub start_windows: bool,
    pub fit: bool,
    pub adaptive: bool,
    pub custom_fps: u32,
    pub custom_cap: u32,
    pub dark_theme: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            profile: Profile::Automatic,
            pause_fullscreen: true,
            pause_maximized: false,
            pause_battery: false,
            pause_saver: true,
            pause_remote: true,
            start_windows: false,
            fit: false,
            adaptive: true,
            custom_fps: 0,
            custom_cap: 0,
            dark_theme: true,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WidgetPosition {
    TopLeft,
    TopCenter,
    TopRight,
    Center,
    BottomLeft,
    BottomCenter,
    BottomRight,
}
impl WidgetPosition {
    pub const ALL: [Self; 7] = [
        Self::TopLeft,
        Self::TopCenter,
        Self::TopRight,
        Self::Center,
        Self::BottomLeft,
        Self::BottomCenter,
        Self::BottomRight,
    ];
    pub fn key(self) -> &'static str {
        match self {
            Self::TopLeft => "top_left",
            Self::TopCenter => "top_center",
            Self::TopRight => "top_right",
            Self::Center => "center",
            Self::BottomLeft => "bottom_left",
            Self::BottomCenter => "bottom_center",
            Self::BottomRight => "bottom_right",
        }
    }
    pub fn text(self) -> &'static str {
        match self {
            Self::TopLeft => "Top Left",
            Self::TopCenter => "Top Center",
            Self::TopRight => "Top Right",
            Self::Center => "Center",
            Self::BottomLeft => "Bottom Left",
            Self::BottomCenter => "Bottom Center",
            Self::BottomRight => "Bottom Right",
        }
    }
    pub fn parse(s: &str) -> Self {
        match s {
            "top_left" => Self::TopLeft,
            "top_center" => Self::TopCenter,
            "top_right" => Self::TopRight,
            "center" => Self::Center,
            "bottom_left" => Self::BottomLeft,
            "bottom_center" => Self::BottomCenter,
            "bottom_right" => Self::BottomRight,
            _ => Self::TopRight,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DateFormat {
    Full,
    MonthDay,
    DateYear,
    Numeric,
    Dot,
    Iso,
}
impl DateFormat {
    pub const ALL: [Self; 6] = [
        Self::Full,
        Self::MonthDay,
        Self::DateYear,
        Self::Numeric,
        Self::Dot,
        Self::Iso,
    ];
    pub fn key(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::MonthDay => "month_day",
            Self::DateYear => "date_year",
            Self::Numeric => "numeric",
            Self::Dot => "dot",
            Self::Iso => "iso",
        }
    }
    pub fn text(self) -> &'static str {
        match self {
            Self::Full => "Wednesday, October 21, 2026",
            Self::MonthDay => "Wednesday, October 21",
            Self::DateYear => "October 21, 2026",
            Self::Numeric => "21/10/2026",
            Self::Dot => "Wednesday · October 21",
            Self::Iso => "2026-10-21",
        }
    }
    pub fn parse(s: &str) -> Self {
        match s {
            "month_day" => Self::MonthDay,
            "date_year" => Self::DateYear,
            "numeric" => Self::Numeric,
            "dot" => Self::Dot,
            "iso" => Self::Iso,
            _ => Self::Full,
        }
    }
    pub fn format(self, year: u32, month: u32, day: u32, weekday: u32) -> String {
        let weekday_name = match weekday {
            0 => "Sunday",
            1 => "Monday",
            2 => "Tuesday",
            3 => "Wednesday",
            4 => "Thursday",
            5 => "Friday",
            6 => "Saturday",
            _ => "Sunday",
        };
        let month_name = match month {
            1 => "January",
            2 => "February",
            3 => "March",
            4 => "April",
            5 => "May",
            6 => "June",
            7 => "July",
            8 => "August",
            9 => "September",
            10 => "October",
            11 => "November",
            12 => "December",
            _ => "October",
        };
        match self {
            Self::Full => format!("{weekday_name}, {month_name} {day}, {year}"),
            Self::MonthDay => format!("{weekday_name}, {month_name} {day}"),
            Self::DateYear => format!("{month_name} {day}, {year}"),
            Self::Numeric => format!("{day:02}/{month:02}/{year}"),
            Self::Dot => format!("{weekday_name} · {month_name} {day}"),
            Self::Iso => format!("{year}-{month:02}-{day:02}"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GreetingMode {
    TimeBased,
    Rotating,
    Fixed,
    Disabled,
}
impl GreetingMode {
    pub const ALL: [Self; 4] = [
        Self::TimeBased,
        Self::Rotating,
        Self::Fixed,
        Self::Disabled,
    ];
    pub fn key(self) -> &'static str {
        match self {
            Self::TimeBased => "time",
            Self::Rotating => "rotating",
            Self::Fixed => "fixed",
            Self::Disabled => "disabled",
        }
    }
    pub fn text(self) -> &'static str {
        match self {
            Self::TimeBased => "Time of Day",
            Self::Rotating => "Rotating Phrases",
            Self::Fixed => "Fixed Phrase",
            Self::Disabled => "Disabled",
        }
    }
    pub fn parse(s: &str) -> Self {
        match s {
            "rotating" => Self::Rotating,
            "fixed" => Self::Fixed,
            "disabled" => Self::Disabled,
            _ => Self::TimeBased,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WidgetDisplayTarget {
    Primary,
    All,
    Selected,
}
impl WidgetDisplayTarget {
    pub const ALL: [Self; 3] = [Self::Primary, Self::All, Self::Selected];
    pub fn key(self) -> &'static str {
        match self {
            Self::Primary => "primary",
            Self::All => "all",
            Self::Selected => "selected",
        }
    }
    pub fn text(self) -> &'static str {
        match self {
            Self::Primary => "Primary Display",
            Self::All => "All Displays",
            Self::Selected => "Selected Display",
        }
    }
    pub fn parse(s: &str) -> Self {
        match s {
            "all" => Self::All,
            "selected" => Self::Selected,
            _ => Self::Primary,
        }
    }
}

pub fn resolve_greeting(
    mode: GreetingMode,
    hour: u32,
    minute: u32,
    name: &str,
    interval_mins: u32,
    custom: &str,
) -> String {
    if mode == GreetingMode::Disabled {
        return String::new();
    }
    let display_name = if name.trim().is_empty() {
        "Alex"
    } else {
        name.trim()
    };
    if mode == GreetingMode::Fixed && !custom.trim().is_empty() {
        return custom.replace("{name}", display_name);
    }
    let (_period, phrases): (&str, &[&str]) = match hour {
        5..=11 => (
            "morning",
            &[
                "Good morning, {name}!",
                "Have a wonderful morning!",
                "Ready for a fresh start?",
                "Make today a good one.",
            ],
        ),
        12..=17 => (
            "afternoon",
            &[
                "Good afternoon, {name}!",
                "Hope your day is going well!",
                "Keep up the good work!",
                "Enjoy your afternoon.",
            ],
        ),
        18..=21 => (
            "evening",
            &[
                "Good evening, {name}!",
                "Welcome back!",
                "Time to unwind.",
                "Hope you had a great day.",
            ],
        ),
        _ => (
            "night",
            &[
                "Good night, {name}!",
                "Enjoy the quiet hours.",
                "Time to relax.",
                "Have a peaceful night.",
            ],
        ),
    };
    let idx = if mode == GreetingMode::Rotating {
        let interval = interval_mins.max(5);
        let tick = (hour * 60 + minute) / interval;
        (tick as usize) % phrases.len()
    } else {
        0
    };
    phrases[idx].replace("{name}", display_name)
}

#[derive(Clone, Debug)]
pub struct WidgetSettings {
    pub enabled: bool,
    pub clock_enabled: bool,
    pub clock_24h: bool,
    pub clock_seconds: bool,
    pub clock_size: u32,
    pub clock_color: u32,
    pub opacity: u8,
    pub date_enabled: bool,
    pub date_format: DateFormat,
    pub date_size: u32,
    pub greeting_enabled: bool,
    pub greeting_name: String,
    pub greeting_mode: GreetingMode,
    pub greeting_interval_mins: u32,
    pub custom_greeting: String,
    pub music_enabled: bool,
    pub music_spotify_preferred: bool,
    pub music_general_media: bool,
    pub music_position: WidgetPosition,
    pub music_offset_x: i32,
    pub music_offset_y: i32,
    pub accent_color: u32,
    pub text_shadow: bool,
    pub position: WidgetPosition,
    pub offset_x: i32,
    pub offset_y: i32,
    pub display_target: WidgetDisplayTarget,
    pub edit_mode: bool,
}
impl Default for WidgetSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            clock_enabled: true,
            clock_24h: true,
            clock_seconds: false,
            clock_size: 72,
            clock_color: 0x00FFFFFF,
            opacity: 95,
            date_enabled: true,
            date_format: DateFormat::Full,
            date_size: 18,
            greeting_enabled: true,
            greeting_name: String::new(),
            greeting_mode: GreetingMode::TimeBased,
            greeting_interval_mins: 30,
            custom_greeting: String::new(),
            music_enabled: true,
            music_spotify_preferred: true,
            music_general_media: true,
            music_position: WidgetPosition::BottomCenter,
            music_offset_x: 0,
            music_offset_y: -40,
            accent_color: 0x006366F1,
            text_shadow: true,
            position: WidgetPosition::TopRight,
            offset_x: -60,
            offset_y: 60,
            display_target: WidgetDisplayTarget::Primary,
            edit_mode: false,
        }
    }
}

#[derive(Default, Clone, Debug)]
pub struct Library {
    pub settings: Settings,
    pub widgets: WidgetSettings,
    pub items: Vec<LibraryItem>,
    /// Stable monitor device string, e.g. \\.\DISPLAY1, mapped to library item id.
    pub assignments: BTreeMap<String, String>,
}
impl Library {
    pub fn item(&self, id: &str) -> Option<&LibraryItem> {
        self.items.iter().find(|i| i.id == id)
    }
    pub fn remove(&mut self, id: &str) {
        self.items.retain(|i| i.id != id);
        self.assignments.retain(|_, value| value != id);
    }
}
fn escape(v: &str) -> String {
    let mut s = String::with_capacity(v.len() * 2);
    for b in v.as_bytes() {
        s.push_str(&format!("{b:02x}"));
    }
    s
}
fn unescape(s: &str) -> Option<String> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    let bytes: Option<Vec<u8>> = s
        .as_bytes()
        .chunks(2)
        .map(|pair| {
            let pair = std::str::from_utf8(pair).ok()?;
            u8::from_str_radix(pair, 16).ok()
        })
        .collect();
    String::from_utf8(bytes?).ok()
}
fn enc_path(path: &Path) -> String {
    escape(&path.to_string_lossy())
}
fn flag(value: bool) -> &'static str {
    if value {
        "1"
    } else {
        "0"
    }
}
fn yes(value: &str) -> bool {
    value == "1" || value == "true"
}
fn opt_num<T: std::str::FromStr>(s: Option<&&str>, default: T) -> T {
    s.and_then(|s| s.parse().ok()).unwrap_or(default)
}
fn str_field<'a>(fields: &[&'a str], i: usize) -> &'a str {
    fields.get(i).copied().unwrap_or("")
}
fn encoded(fields: &[&str], i: usize) -> String {
    unescape(str_field(fields, i)).unwrap_or_default()
}

pub fn load(path: &Path) -> Library {
    let Ok(contents) = fs::read_to_string(path) else {
        return Library::default();
    };
    let mut result = Library::default();
    for line in contents.lines() {
        if line.len() > 100_000 {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        match f.first().copied() {
            Some("settings") if f.len() >= 13 => {
                let s = &mut result.settings;
                s.profile = Profile::parse(f[1]);
                s.pause_fullscreen = yes(f[2]);
                s.pause_maximized = yes(f[3]);
                s.pause_battery = yes(f[4]);
                s.pause_saver = yes(f[5]);
                s.pause_remote = yes(f[6]);
                s.start_windows = yes(f[7]);
                s.fit = yes(f[8]);
                s.adaptive = yes(f[9]);
                s.custom_fps = opt_num(f.get(10), 0u32).min(120);
                s.custom_cap = opt_num(f.get(11), 0u32).min(8192);
                s.dark_theme = if let Some(v) = f.get(13) {
                    yes(v)
                } else {
                    true
                };
            }
            Some("entry") if f.len() >= 17 => {
                let item = LibraryItem {
                    id: str_field(&f, 1).to_owned(),
                    title: encoded(&f, 2),
                    original: PathBuf::from(encoded(&f, 3)),
                    asset: PathBuf::from(encoded(&f, 4)),
                    kind: MediaKind::parse(f[5]),
                    profile: Profile::parse(f[6]),
                    source_width: opt_num(f.get(7), 0),
                    source_height: opt_num(f.get(8), 0),
                    width: opt_num(f.get(9), 0),
                    height: opt_num(f.get(10), 0),
                    fps: opt_num(f.get(11), 0.0f64),
                    duration: opt_num(f.get(12), 0.0f64),
                    bytes: opt_num(f.get(13), 0u64),
                    imported_at: opt_num(f.get(14), 0u64),
                    note: encoded(&f, 15),
                };
                if !item.id.is_empty()
                    && !item.asset.as_os_str().is_empty()
                    && result.items.len() < 20000
                {
                    result.items.push(item);
                }
            }
            Some("assign") if f.len() >= 3 => {
                result.assignments.insert(encoded(&f, 1), f[2].to_string());
            }
            Some("widgets") if f.len() >= 28 => {
                let w = &mut result.widgets;
                w.enabled = yes(f[1]);
                w.clock_enabled = yes(f[2]);
                w.clock_24h = yes(f[3]);
                w.clock_seconds = yes(f[4]);
                w.clock_size = opt_num(f.get(5), 72u32).clamp(24, 180);
                w.clock_color = opt_num(f.get(6), 0x00FFFFFFu32);
                w.opacity = opt_num(f.get(7), 95u8).clamp(10, 100);
                w.date_enabled = yes(f[8]);
                w.date_format = DateFormat::parse(f[9]);
                w.date_size = opt_num(f.get(10), 18u32).clamp(10, 60);
                w.greeting_enabled = yes(f[11]);
                w.greeting_name = unescape(f[12]).unwrap_or_default();
                w.greeting_mode = GreetingMode::parse(f[13]);
                w.greeting_interval_mins = opt_num(f.get(14), 30u32).clamp(5, 360);
                w.custom_greeting = unescape(f[15]).unwrap_or_default();
                w.music_enabled = yes(f[16]);
                w.music_spotify_preferred = yes(f[17]);
                w.music_general_media = yes(f[18]);
                w.music_position = WidgetPosition::parse(f[19]);
                w.music_offset_x = opt_num(f.get(20), 0i32);
                w.music_offset_y = opt_num(f.get(21), -40i32);
                w.accent_color = opt_num(f.get(22), 0x006366F1u32);
                w.text_shadow = yes(f[23]);
                w.position = WidgetPosition::parse(f[24]);
                w.offset_x = opt_num(f.get(25), -60i32);
                w.offset_y = opt_num(f.get(26), 60i32);
                w.display_target = WidgetDisplayTarget::parse(f[27]);
            }
            _ => {}
        }
    }
    result
}
pub fn save(path: &Path, lib: &Library) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let s = &lib.settings;
    let mut output = format!(
        "FEATHERWALL\t{FORMAT_VERSION}\nsettings\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t0\t{}\n",
        s.profile.key(),
        flag(s.pause_fullscreen),
        flag(s.pause_maximized),
        flag(s.pause_battery),
        flag(s.pause_saver),
        flag(s.pause_remote),
        flag(s.start_windows),
        flag(s.fit),
        flag(s.adaptive),
        s.custom_fps,
        s.custom_cap,
        flag(s.dark_theme)
    );
    let w = &lib.widgets;
    output.push_str(&format!(
        "widgets\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
        flag(w.enabled),
        flag(w.clock_enabled),
        flag(w.clock_24h),
        flag(w.clock_seconds),
        w.clock_size,
        w.clock_color,
        w.opacity,
        flag(w.date_enabled),
        w.date_format.key(),
        w.date_size,
        flag(w.greeting_enabled),
        escape(&w.greeting_name),
        w.greeting_mode.key(),
        w.greeting_interval_mins,
        escape(&w.custom_greeting),
        flag(w.music_enabled),
        flag(w.music_spotify_preferred),
        flag(w.music_general_media),
        w.music_position.key(),
        w.music_offset_x,
        w.music_offset_y,
        w.accent_color,
        flag(w.text_shadow),
        w.position.key(),
        w.offset_x,
        w.offset_y,
        w.display_target.key(),
    ));
    for item in &lib.items {
        output.push_str(&format!(
            "entry\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.4}\t{:.4}\t{}\t{}\t{}\t0\n",
            item.id,
            escape(&item.title),
            enc_path(&item.original),
            enc_path(&item.asset),
            item.kind.text(),
            item.profile.key(),
            item.source_width,
            item.source_height,
            item.width,
            item.height,
            item.fps,
            item.duration,
            item.bytes,
            item.imported_at,
            escape(&item.note)
        ));
    }
    for (device, item) in &lib.assignments {
        output.push_str(&format!("assign\t{}\t{}\n", escape(device), item));
    }
    // Atomic replacement: interrupted saves never overwrite the last valid file.
    let temp = path.with_extension(format!("tmp-{}", std::process::id()));
    let mut file = fs::File::create(&temp)?;
    file.write_all(output.as_bytes())?;
    file.sync_all()?;
    drop(file);
    if let Err(e) = fs::rename(&temp, path) {
        let _ = fs::remove_file(&temp);
        return Err(e);
    }
    Ok(())
}

pub fn data_root() -> io::Result<PathBuf> {
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        Ok(PathBuf::from(local).join("FeatherWall"))
    } else {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "LOCALAPPDATA is missing",
        ))
    }
}
pub fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Two independent 64-bit FNV streams plus source length. Not a cryptographic hash.
/// The cache is untrusted and outputs are still probed before use.
pub fn content_key(path: &Path) -> io::Result<String> {
    content_key_cancellable(path, || false)
}

/// Streaming hash that can be canceled between 64 KiB file reads.
pub fn content_key_cancellable(path: &Path, cancel: impl Fn() -> bool) -> io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut left = 0xcbf29ce484222325u64;
    let mut right = 0x100000001b3u64 ^ 0x9e3779b97f4a7c15;
    let mut total = 0u64;
    let mut buf = [0u8; 64 * 1024];
    loop {
        if cancel() {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "Import cancelled during hashing",
            ));
        }
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        total += n as u64;
        for &b in &buf[..n] {
            left = (left ^ b as u64).wrapping_mul(0x100000001b3);
            right = (right ^ b as u64)
                .wrapping_mul(0x100000001b3)
                .rotate_left(13);
        }
    }
    Ok(format!("{left:016x}{right:016x}-{total:x}"))
}

pub fn supported_extension(path: &Path) -> bool {
    let extension = path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    matches!(
        extension.as_str(),
        "mp4" | "gif" | "webm" | "mov" | "mkv" | "avi" | "jpg" | "jpeg" | "png" | "bmp"
    )
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaybackState {
    Stopped,
    Loading,
    Playing,
    Paused,
    Recovering,
    Failed,
}
impl PlaybackState {
    pub fn can_transition(self, to: Self) -> bool {
        use PlaybackState::*;
        self == to
            || matches!(
                (self, to),
                (Stopped, Loading)
                    | (Loading, Playing)
                    | (Loading, Failed)
                    | (Loading, Stopped)
                    | (Playing, Paused)
                    | (Playing, Stopped)
                    | (Playing, Failed)
                    | (Playing, Recovering)
                    | (Paused, Playing)
                    | (Paused, Stopped)
                    | (Paused, Recovering)
                    | (Recovering, Playing)
                    | (Recovering, Failed)
                    | (Recovering, Stopped)
                    | (Failed, Stopped)
                    | (Failed, Loading)
            )
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}
impl Rect {
    pub fn width(&self) -> i32 {
        self.right.saturating_sub(self.left).max(0)
    }
    pub fn height(&self) -> i32 {
        self.bottom.saturating_sub(self.top).max(0)
    }
    pub fn covers(&self, other: &Self, tolerance: i32) -> bool {
        self.left <= other.left + tolerance
            && self.top <= other.top + tolerance
            && self.right >= other.right - tolerance
            && self.bottom >= other.bottom - tolerance
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn profile_caps() {
        let eco = Profile::UltraEco.preset(8000, 4000, 60., 3840, 2160, false);
        assert!(eco.width <= 1280 && eco.fps <= 15.);
        assert_eq!(eco.width % 2, 0);
    }
    #[test]
    fn automatic_battery() {
        let a = Profile::Automatic.preset(1920, 1080, 30., 1920, 1080, true);
        assert_eq!(a.resolved, Profile::UltraEco);
    }
    #[test]
    fn paths_and_unicode() {
        let s = "你好\t💕 C:\\foo";
        assert_eq!(unescape(&escape(s)).unwrap(), s);
    }
    #[test]
    fn geometry_negative_coords() {
        let a = Rect {
            left: -1920,
            top: 0,
            right: 0,
            bottom: 1080,
        };
        assert!(a.covers(&a, 0));
        assert_eq!(a.width(), 1920);
    }
    #[test]
    fn playback_transitions() {
        assert!(PlaybackState::Loading.can_transition(PlaybackState::Playing));
        assert!(!PlaybackState::Stopped.can_transition(PlaybackState::Playing));
    }
    #[test]
    fn settings_roundtrip() {
        let path = std::env::temp_dir().join(format!(
            "featherwall-test-{}-{}.cfg",
            std::process::id(),
            unix_seconds()
        ));
        let mut lib = Library::default();
        lib.settings.profile = Profile::Quality;
        lib.items.push(LibraryItem {
            id: "abcdef".into(),
            title: "壁纸\tX".into(),
            original: "C:\\foo bar.gif".into(),
            asset: "C:\\temp.mp4".into(),
            kind: MediaKind::Video,
            profile: Profile::Quality,
            source_width: 1440,
            source_height: 900,
            width: 1280,
            height: 800,
            fps: 30.,
            duration: 1.0,
            bytes: 123,
            imported_at: 1,
            note: "ok".into(),
        });
        lib.assignments
            .insert("\\\\.\\DISPLAY2".into(), "abcdef".into());
        save(&path, &lib).unwrap();
        let decoded = load(&path);
        let _ = fs::remove_file(path);
        assert_eq!(decoded.settings.profile, Profile::Quality);
        assert_eq!(decoded.items[0].title, "壁纸\tX");
        assert_eq!(decoded.assignments.len(), 1);
    }
    #[test]
    fn extension_filter() {
        assert!(supported_extension(Path::new("x.MP4")));
        assert!(!supported_extension(Path::new("script.exe")));
    }
    #[test]
    fn malformed_falls_back() {
        let path = std::env::temp_dir().join(format!("fw-broken-{}", std::process::id()));
        fs::write(&path, b"entry\tgarbage\nsettings\tbroken\n").unwrap();
        assert!(load(&path).items.is_empty());
        let _ = fs::remove_file(path);
    }
    #[test]
    fn widgets_roundtrip() {
        let path = std::env::temp_dir().join(format!(
            "featherwall-widget-test-{}-{}.cfg",
            std::process::id(),
            unix_seconds()
        ));
        let mut lib = Library::default();
        lib.widgets.clock_24h = false;
        lib.widgets.clock_size = 80;
        lib.widgets.date_format = DateFormat::Dot;
        lib.widgets.greeting_name = "Alex".into();
        lib.widgets.position = WidgetPosition::TopLeft;
        lib.widgets.offset_x = 42;
        lib.widgets.offset_y = 84;
        lib.widgets.music_offset_y = -50;
        save(&path, &lib).unwrap();
        let decoded = load(&path);
        let _ = fs::remove_file(path);
        assert_eq!(decoded.widgets.clock_24h, false);
        assert_eq!(decoded.widgets.clock_size, 80);
        assert_eq!(decoded.widgets.date_format, DateFormat::Dot);
        assert_eq!(decoded.widgets.greeting_name, "Alex");
        assert_eq!(decoded.widgets.position, WidgetPosition::TopLeft);
        assert_eq!(decoded.widgets.offset_x, 42);
        assert_eq!(decoded.widgets.offset_y, 84);
        assert_eq!(decoded.widgets.music_offset_y, -50);
    }
    #[test]
    fn test_greeting_resolution() {
        let evening = resolve_greeting(GreetingMode::TimeBased, 20, 29, "Alex", 30, "");
        assert_eq!(evening, "Good evening, Alex!");

        let morning = resolve_greeting(GreetingMode::TimeBased, 9, 0, "Alex", 30, "");
        assert_eq!(morning, "Good morning, Alex!");

        let fixed = resolve_greeting(GreetingMode::Fixed, 14, 0, "Alex", 30, "Welcome, {name}!");
        assert_eq!(fixed, "Welcome, Alex!");

        let disabled = resolve_greeting(GreetingMode::Disabled, 20, 0, "Alex", 30, "");
        assert!(disabled.is_empty());
    }

    #[test]
    fn test_date_formats() {
        // Wednesday, October 21, 2026 (day_of_week 3 = Wednesday)
        assert_eq!(
            DateFormat::Full.format(2026, 10, 21, 3),
            "Wednesday, October 21, 2026"
        );
        assert_eq!(
            DateFormat::MonthDay.format(2026, 10, 21, 3),
            "Wednesday, October 21"
        );
        assert_eq!(
            DateFormat::DateYear.format(2026, 10, 21, 3),
            "October 21, 2026"
        );
        assert_eq!(
            DateFormat::Numeric.format(2026, 10, 21, 3),
            "21/10/2026"
        );
        assert_eq!(
            DateFormat::Dot.format(2026, 10, 21, 3),
            "Wednesday · October 21"
        );
        assert_eq!(
            DateFormat::Iso.format(2026, 10, 21, 3),
            "2026-10-21"
        );
    }

    #[test]
    fn test_widget_enums() {
        for pos in WidgetPosition::ALL {
            assert_eq!(WidgetPosition::parse(pos.key()), pos);
        }
        for target in WidgetDisplayTarget::ALL {
            assert_eq!(WidgetDisplayTarget::parse(target.key()), target);
        }
        for mode in GreetingMode::ALL {
            assert_eq!(GreetingMode::parse(mode.key()), mode);
        }
    }
}

