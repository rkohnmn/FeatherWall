//! FeatherWall native Windows runtime. The Windows message thread owns all HWNDs and COM players.
//! Importing is performed by a short-lived worker; the steady-state loop blocks on GetMessageW.
mod desktop;
pub mod ffi;
pub mod media;
pub mod overlay;
mod playback;
pub mod ui;
use self::ffi as f;
use self::ui::{NavTab, UiAction, UiState};
use crate::core::{self, Library, LibraryItem, MediaKind, PlaybackState, Profile};
use crate::importer::{self, CancelToken, ImportOptions, Progress};
use std::cell::RefCell;
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::{
    atomic::{AtomicIsize, Ordering},
    mpsc::{self, Receiver},
};
use std::thread::{self, JoinHandle};
use std::time::Instant;

const MAIN_CLASS: &str = "FeatherWallControlWindow";
const SURFACE_CLASS: &str = "FeatherWallSurfaceWindow";
const WM_TRAY: u32 = f::WM_APP + 1;
const WM_FOREGROUND: u32 = f::WM_APP + 2;
const WM_SHOW: u32 = f::WM_APP + 3;
#[allow(dead_code)]
const WM_IMPORT_FILE: u32 = f::WM_APP + 5;
const WM_CLIENT_PAUSE: u32 = f::WM_APP + 6;
const WM_CLIENT_RESUME: u32 = f::WM_APP + 7;
const WM_CLIENT_STOP: u32 = f::WM_APP + 8;
const WM_CLIENT_STATUS: u32 = f::WM_APP + 9;
const ID_IMPORT: u16 = 100;
const ID_APPLY: u16 = 101;
const ID_REMOVE: u16 = 102;
const ID_REOPT: u16 = 103;
const ID_PAUSE: u16 = 104;
const ID_RESUME: u16 = 105;
const ID_STOP: u16 = 106;
const ID_CLEAN: u16 = 115;
const ID_FOLDER: u16 = 116;
const ID_CANCEL: u16 = 117;
const ID_EXIT: u16 = 200;
const ID_TRAY_OPEN: u16 = 201;
const ID_TRAY_SELECT: u16 = 202;
const ID_TRAY_PROFILE_BASE: u16 = 220;
const ID_TRAY_TOGGLE_START: u16 = 240;
const ID_TRAY_SETTINGS: u16 = 241;
const ID_TRAY_WIDGETS: u16 = 242;
static WINDOW: AtomicIsize = AtomicIsize::new(0);
thread_local! {static APPLICATION:RefCell<Option<App>>=const{RefCell::new(None)};}

struct Surface {
    hwnd: f::HWND,
    device: String,
    #[allow(dead_code)]
    id: String,
    content: Content,
    state: PlaybackState,
    suspended: bool,
}
enum Content {
    Video(playback::Player),
    Still(f::HBITMAP),
}
impl Content {
    fn pause(&mut self) -> Result<(), String> {
        match self {
            Self::Video(p) => p.pause(),
            Self::Still(_) => Ok(()),
        }
    }
    fn play(&mut self) -> Result<(), String> {
        match self {
            Self::Video(p) => p.play(),
            Self::Still(_) => Ok(()),
        }
    }
    fn stop(&mut self) -> Result<(), String> {
        match self {
            Self::Video(p) => p.stop(),
            Self::Still(_) => Ok(()),
        }
    }
}
impl Drop for Content {
    fn drop(&mut self) {
        if let Self::Still(bmp) = self {
            if *bmp != 0 {
                unsafe {
                    f::DeleteObject(*bmp);
                }
            }
        }
    }
}
enum WorkerEvent {
    Progress(Progress),
    Done(Result<LibraryItem, String>),
}
struct Worker {
    cancel: CancelToken,
    receiver: Receiver<WorkerEvent>,
    handle: JoinHandle<()>,
}
struct App {
    hwnd: f::HWND,
    instance: f::HINSTANCE,
    root: PathBuf,
    config: PathBuf,
    library: Library,
    monitor_list: Vec<desktop::Monitor>,
    surfaces: Vec<Surface>,
    host: f::HWND,
    ui: UiState,
    widgets_overlay: Option<overlay::WidgetOverlayManager>,
    worker: Option<Worker>,
    worker_status: String,
    worker_progress: u8,
    manual_pause: bool,
    screen_off: bool,
    session_locked: bool,
    sleep: bool,
    exiting: bool,
    tray_msg: u32,
    tray_icon: f::HICON,
    taskbar_message: u32,
    hook: f::HHOOK,
    power_notify: f::HPOWERNOTIFY,
    have_wts: bool,
    last_power_battery: Option<bool>,
    last_perf: Option<(Instant, u64)>,
    perf_cpu: f64,
    perf_working_set: f64,
    perf_private: f64,
}
fn logfile(root: &Path, message: &str) {
    let path = root.join("featherwall.log");
    if fs::metadata(&path)
        .map(|m| m.len() > 2_000_000)
        .unwrap_or(false)
    {
        let backup = root.join("featherwall.previous.log");
        let _ = fs::remove_file(&backup);
        let _ = fs::rename(&path, backup);
    }
    if let Ok(mut file) = OpenOptions::new().append(true).create(true).open(path) {
        let sanitized: String = message
            .chars()
            .filter(|c| *c != '\r' && *c != '\n')
            .take(1400)
            .collect();
        let _ = writeln!(file, "{}: {}", core::unix_seconds(), sanitized);
    }
}
fn reg_autostart(enabled: bool) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    let key = f::wide("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
    let value = f::wide("FeatherWall");
    let mut handle = 0;
    unsafe {
        let rc = f::RegCreateKeyExW(
            f::HKEY_CURRENT_USER,
            key.as_ptr(),
            0,
            ptr::null_mut(),
            0,
            f::KEY_SET_VALUE,
            ptr::null_mut(),
            &mut handle,
            ptr::null_mut(),
        );
        if rc != 0 {
            return Err(format!("Cannot open per-user startup key: Win32 {rc}"));
        }
        let rc = if enabled {
            let exe = match std::env::current_exe() {
                Ok(exe) => exe,
                Err(e) => {
                    f::RegCloseKey(handle);
                    return Err(e.to_string());
                }
            };
            let command = format!("\"{}\"", exe.to_string_lossy());
            let wide: Vec<u16> = OsString::from(command)
                .encode_wide()
                .chain(Some(0))
                .collect();
            f::RegSetValueExW(
                handle,
                value.as_ptr(),
                0,
                f::REG_SZ,
                wide.as_ptr().cast::<u8>(),
                (wide.len() * 2) as u32,
            )
        } else {
            let rc = f::RegDeleteValueW(handle, value.as_ptr());
            if rc == 2 {
                0
            } else {
                rc
            }
        };
        f::RegCloseKey(handle);
        if rc != 0 {
            Err(format!("Startup registry update failed: Win32 {rc}"))
        } else {
            Ok(())
        }
    }
}
fn query_autostart() -> bool {
    let key = f::wide("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
    let value = f::wide("FeatherWall");
    let mut h = 0;
    unsafe {
        if f::RegOpenKeyExW(
            f::HKEY_CURRENT_USER,
            key.as_ptr(),
            0,
            f::KEY_QUERY_VALUE,
            &mut h,
        ) != 0
        {
            return false;
        }
        let mut len = 0u32;
        let rc = f::RegQueryValueExW(
            h,
            value.as_ptr(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            &mut len,
        );
        f::RegCloseKey(h);
        rc == 0 && len > 2
    }
}
unsafe extern "system" fn on_foreground(
    _hook: f::HHOOK,
    _event: u32,
    _window: f::HWND,
    _object: i32,
    _child: i32,
    _thread: u32,
    _time: u32,
) {
    let hwnd = WINDOW.load(Ordering::Acquire);
    if hwnd != 0 {
        let _ = f::PostMessageW(hwnd, WM_FOREGROUND, 0, 0);
    }
}
unsafe extern "system" fn main_proc(hwnd: f::HWND, msg: u32, w: usize, l: isize) -> isize {
    let mut response = None;
    APPLICATION.with(|cell| {
        if let Ok(mut active) = cell.try_borrow_mut() {
            if let Some(app) = active.as_mut() {
                if app.hwnd == hwnd {
                    response = app.main_message(msg, w, l);
                }
            }
        }
    });
    response.unwrap_or_else(|| f::DefWindowProcW(hwnd, msg, w, l))
}
unsafe extern "system" fn surface_proc(hwnd: f::HWND, msg: u32, w: usize, l: isize) -> isize {
    match msg {
        f::WM_PAINT => {
            let mut bitmap = 0;
            APPLICATION.with(|cell| {
                if let Ok(app) = cell.try_borrow() {
                    if let Some(app) = app.as_ref() {
                        for surface in &app.surfaces {
                            if surface.hwnd == hwnd {
                                if let Content::Still(value) = &surface.content {
                                    bitmap = *value;
                                }
                                break;
                            }
                        }
                    }
                }
            });
            desktop::paint_bitmap(hwnd, bitmap);
            0
        }
        playback::WM_PLAY_EVENT => {
            APPLICATION.with(|cell| {
                if let Ok(mut app) = cell.try_borrow_mut() {
                    if let Some(app) = app.as_mut() {
                        app.play_event(hwnd, w, l);
                    }
                }
            });
            0
        }
        _ => f::DefWindowProcW(hwnd, msg, w, l),
    }
}
fn register_class(
    name: &str,
    proc: unsafe extern "system" fn(f::HWND, u32, usize, isize) -> isize,
    instance: f::HINSTANCE,
) -> Result<(), String> {
    let name = f::wide(name);
    let class = f::WNDCLASSEXW {
        cbSize: std::mem::size_of::<f::WNDCLASSEXW>() as u32,
        style: 0,
        lpfnWndProc: Some(proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: instance,
        hIcon: unsafe { f::LoadIconW(0, 32512usize as *const u16) },
        hCursor: unsafe { f::LoadCursorW(0, 32512usize as *const u16) },
        hbrBackground: 0,
        lpszMenuName: ptr::null(),
        lpszClassName: name.as_ptr(),
        hIconSm: 0,
    };
    if unsafe { f::RegisterClassExW(&class) } == 0 {
        return Err(format!("Cannot register window class (Win32 {})", unsafe {
            f::GetLastError()
        }));
    }
    Ok(())
}

pub fn run() -> Result<(), String> {
    // PROCESS_PER_MONITOR_DPI_AWARE (Windows 8.1+) keeps WorkerW and monitor geometry in physical pixels.
    unsafe {
        let _ = f::SetProcessDpiAwareness(2);
        let progman = f::FindWindowW(f::wide("Progman").as_ptr(), ptr::null());
        if progman == 0 {
            let desk = f::OpenDesktopW(f::wide("default").as_ptr(), 0, 0, 0x01FF);
            if desk != 0 {
                let _ = f::SetThreadDesktop(desk);
            }
        }
    }
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    if !args.is_empty() {
        return command_line(&args);
    }
    let root = core::data_root().map_err(|e| e.to_string())?;
    fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let mutex_name = f::wide("Local\\FeatherWall_Rust_User_Instance");
    let mutex = unsafe { f::CreateMutexW(ptr::null_mut(), 0, mutex_name.as_ptr()) };
    if mutex == 0 {
        return Err("Unable to establish single-instance mutex".into());
    }
    if unsafe { f::GetLastError() } == f::ERROR_ALREADY_EXISTS {
        let existing = unsafe { f::FindWindowW(f::wide(MAIN_CLASS).as_ptr(), ptr::null()) };
        if existing != 0 {
            unsafe {
                f::PostMessageW(existing, WM_SHOW, 0, 0);
            }
        }
        unsafe {
            f::CloseHandle(mutex);
        }
        return Ok(());
    }
    let hr = unsafe { f::CoInitializeEx(ptr::null_mut(), 2) };
    if hr < 0 {
        unsafe { f::CloseHandle(mutex) };
        return Err(format!("COM initialization failed: {hr:#x}"));
    }
    let outcome = launch_gui(root);
    unsafe {
        f::CoUninitialize();
        f::CloseHandle(mutex);
    }
    outcome
}
fn launch_gui(root: PathBuf) -> Result<(), String> {
    let instance = unsafe { f::GetModuleHandleW(ptr::null()) };
    register_class(MAIN_CLASS, main_proc, instance)?;
    register_class(SURFACE_CLASS, surface_proc, instance)?;
    let class = f::wide(MAIN_CLASS);
    let title = f::wide("FeatherWall — Native Animated Wallpapers");
    let hwnd = unsafe {
        f::CreateWindowExW(
            0,
            class.as_ptr(),
            title.as_ptr(),
            f::WS_OVERLAPPEDWINDOW | f::WS_CLIPCHILDREN,
            120,
            60,
            1040,
            720,
            0,
            0,
            instance,
            ptr::null_mut(),
        )
    };
    if hwnd == 0 {
        return Err("Main Win32 window creation failed".into());
    }
    unsafe {
        f::DragAcceptFiles(hwnd, 1);
    }
    WINDOW.store(hwnd, Ordering::Release);
    let mut library = core::load(&root.join("library.fwcfg"));
    library.settings.start_windows = query_autostart();
    let ui_state = ui::UiState::new(library.settings.dark_theme);
    ui_state.theme.apply_to_window(hwnd);

    let monitor_list = desktop::monitors();
    let widgets_overlay = match overlay::WidgetOverlayManager::new(instance, library.widgets.clone()) {
        Ok(mut mgr) => {
            mgr.register_global();
            mgr.sync_monitors(&monitor_list);
            Some(mgr)
        }
        Err(err) => {
            logfile(&root, &format!("Failed to initialize widget overlay: {err}"));
            None
        }
    };

    let app = App {
        hwnd,
        instance,
        root: root.clone(),
        config: root.join("library.fwcfg"),
        library,
        monitor_list,
        surfaces: Vec::new(),
        host: 0,
        ui: ui_state,
        widgets_overlay,
        worker: None,
        worker_status: String::new(),
        worker_progress: 0,
        manual_pause: false,
        screen_off: false,
        session_locked: false,
        sleep: false,
        exiting: false,
        tray_msg: WM_TRAY,
        tray_icon: unsafe { f::LoadIconW(0, 32512usize as *const u16) },
        taskbar_message: unsafe { f::RegisterWindowMessageW(f::wide("TaskbarCreated").as_ptr()) },
        hook: 0,
        power_notify: 0,
        have_wts: false,
        last_power_battery: None,
        last_perf: None,
        perf_cpu: 0.0,
        perf_working_set: 0.0,
        perf_private: 0.0,
    };
    APPLICATION.with(|cell| cell.replace(Some(app)));
    APPLICATION.with(|cell| {
        if let Some(app) = cell.borrow_mut().as_mut() {
            app.initialize_ui();
            app.register_notifications();
            app.install_tray();
            app.rebuild_surfaces();
            if app.surfaces.is_empty() {
                unsafe {
                    f::ShowWindow(hwnd, f::SW_SHOW);
                }
            }
            app.log("Startup complete");
        }
    });
    let mut msg: f::MSG = f::zeroed();
    loop {
        let rc = unsafe { f::GetMessageW(&mut msg, 0, 0, 0) };
        if rc <= 0 {
            break;
        }
        unsafe {
            f::TranslateMessage(&msg);
            f::DispatchMessageW(&msg);
        }
    }
    APPLICATION.with(|cell| {
        if let Some(mut app) = cell.borrow_mut().take() {
            app.cleanup();
        }
    });
    WINDOW.store(0, Ordering::Release);
    Ok(())
}

fn selected_item(app: &App) -> Option<LibraryItem> {
    app.ui
        .selected_item_id
        .as_ref()
        .and_then(|id| app.library.item(id).cloned())
        .or_else(|| app.library.items.first().cloned())
}
fn selected_monitor(app: &App) -> Option<desktop::Monitor> {
    app.monitor_list
        .get(app.ui.selected_monitor_idx)
        .cloned()
        .or_else(|| app.monitor_list.first().cloned())
}

impl App {
    fn log(&self, message: &str) {
        logfile(&self.root, message);
    }
    fn status(&mut self, message: &str) {
        self.ui.show_toast(message, true);
        unsafe {
            f::InvalidateRect(self.hwnd, ptr::null(), 0);
        }
    }
    fn persist(&self) {
        if let Err(e) = core::save(&self.config, &self.library) {
            self.log(&format!("Settings save error: {e}"));
        }
    }
    fn update_cache_size(&mut self) {
        let cache_dir = self.root.join("cache");
        if let Ok(entries) = fs::read_dir(cache_dir) {
            let mut total_bytes = 0u64;
            let mut count = 0;
            for entry in entries.flatten() {
                if let Ok(m) = entry.metadata() {
                    if m.is_file() {
                        total_bytes += m.len();
                        count += 1;
                    }
                }
            }
            self.ui.cache_size_str = format!(
                "{:.1} MiB in {} files",
                total_bytes as f64 / 1_048_576.0,
                count
            );
        }
    }
    fn refresh_recent_logs(&mut self) {
        let log_path = self.root.join("featherwall.log");
        if let Ok(contents) = fs::read_to_string(log_path) {
            let lines: Vec<String> = contents
                .lines()
                .rev()
                .take(15)
                .map(|s| s.to_string())
                .collect();
            self.ui.recent_logs = lines;
        }
    }
    fn initialize_ui(&mut self) {
        if self.ui.selected_item_id.is_none() {
            self.ui.selected_item_id = self.library.items.first().map(|i| i.id.clone());
        }
        for item in &self.library.items {
            let thumb = item.thumb_path();
            if !thumb.is_file() && item.original.is_file() {
                let _ = importer::generate_thumbnail(
                    &item.original,
                    &thumb,
                    item.kind == MediaKind::Video,
                );
            }
        }
        self.update_cache_size();
        self.refresh_recent_logs();
    }
    fn register_notifications(&mut self) {
        unsafe {
            self.have_wts = f::WTSRegisterSessionNotification(self.hwnd, 0) != 0;
            self.power_notify =
                f::RegisterPowerSettingNotification(self.hwnd, &f::GUID_CONSOLE_DISPLAY_STATE, 0);
            self.hook = f::SetWinEventHook(
                f::EVENT_SYSTEM_FOREGROUND,
                f::EVENT_SYSTEM_FOREGROUND,
                0,
                Some(on_foreground),
                0,
                0,
                f::WINEVENT_OUTOFCONTEXT | f::WINEVENT_SKIPOWNPROCESS,
            );
            f::SetTimer(self.hwnd, 1, 3000, ptr::null());
        }
    }
    fn tray_data(&self) -> f::NOTIFYICONDATAW {
        let mut info: f::NOTIFYICONDATAW = f::zeroed();
        info.cbSize = std::mem::size_of::<f::NOTIFYICONDATAW>() as u32;
        info.hWnd = self.hwnd;
        info.uID = 1;
        info.uFlags = f::NIF_ICON | f::NIF_MESSAGE | f::NIF_TIP;
        info.uCallbackMessage = self.tray_msg;
        info.hIcon = self.tray_icon;
        let msg = f::wide("FeatherWall - low overhead animated wallpaper");
        info.szTip[..msg.len().min(128)].copy_from_slice(&msg[..msg.len().min(128)]);
        info
    }
    fn install_tray(&self) {
        let mut info = self.tray_data();
        unsafe {
            f::Shell_NotifyIconW(f::NIM_ADD, &mut info);
        }
    }
    fn remove_tray(&self) {
        let mut info = self.tray_data();
        unsafe {
            f::Shell_NotifyIconW(f::NIM_DELETE, &mut info);
        }
    }
    fn show_ui(&self) {
        unsafe {
            f::ShowWindow(self.hwnd, f::SW_RESTORE);
            f::SetForegroundWindow(self.hwnd);
            f::InvalidateRect(self.hwnd, ptr::null(), 0);
        }
    }
    fn tray_menu(&mut self) {
        unsafe {
            let menu = f::CreatePopupMenu();
            let profiles = f::CreatePopupMenu();
            let add = |menu: f::HMENU, label: &str, id: usize| {
                f::AppendMenuW(menu, f::MF_STRING, id, f::wide(label).as_ptr());
            };
            add(menu, "Open FeatherWall", ID_TRAY_OPEN as usize);
            add(menu, "Import Wallpaper...", ID_IMPORT as usize);
            add(menu, "Wallpaper Library...", ID_TRAY_SELECT as usize);
            f::AppendMenuW(menu, f::MF_SEPARATOR, 0, ptr::null());
            if self.manual_pause
                || self
                    .surfaces
                    .iter()
                    .all(|s| s.state == PlaybackState::Paused)
            {
                add(menu, "▶ Resume Playback", ID_RESUME as usize);
            } else {
                add(menu, "❚❚ Pause Playback", ID_PAUSE as usize);
            }
            add(menu, "■ Stop Wallpaper", ID_STOP as usize);
            f::AppendMenuW(menu, f::MF_SEPARATOR, 0, ptr::null());
            for (i, p) in Profile::ALL.iter().enumerate() {
                let label = if *p == self.library.settings.profile {
                    format!("● {}", p.text())
                } else {
                    format!("   {}", p.text())
                };
                add(profiles, &label, ID_TRAY_PROFILE_BASE as usize + i);
            }
            f::AppendMenuW(
                menu,
                f::MF_POPUP,
                profiles as usize,
                f::wide("Performance Profile").as_ptr(),
            );
            f::AppendMenuW(menu, f::MF_SEPARATOR, 0, ptr::null());
            add(menu, "⚙ Settings", ID_TRAY_SETTINGS as usize);
            add(
                menu,
                if self.library.widgets.enabled {
                    "✓ Desktop Widgets"
                } else {
                    "   Desktop Widgets"
                },
                ID_TRAY_WIDGETS as usize,
            );
            add(menu, "📁 Open Wallpaper Folder", ID_FOLDER as usize);
            add(
                menu,
                if self.library.settings.start_windows {
                    "✓ Start with Windows"
                } else {
                    "   Start with Windows"
                },
                ID_TRAY_TOGGLE_START as usize,
            );
            f::AppendMenuW(menu, f::MF_SEPARATOR, 0, ptr::null());
            add(menu, "Exit FeatherWall", ID_EXIT as usize);
            let mut cursor: f::POINT = f::zeroed();
            f::GetCursorPos(&mut cursor);
            f::SetForegroundWindow(self.hwnd);
            f::TrackPopupMenu(
                menu,
                f::TPM_RIGHTBUTTON,
                cursor.x,
                cursor.y,
                0,
                self.hwnd,
                ptr::null(),
            );
            f::DestroyMenu(menu);
            f::PostMessageW(self.hwnd, 0, 0, 0);
        }
    }
}
impl App {
    fn clear_surfaces(&mut self) {
        for mut surface in self.surfaces.drain(..) {
            let _ = surface.content.stop();
            drop(surface.content); // Release COM decoder before the target window is destroyed.
            unsafe {
                if f::IsWindow(surface.hwnd) != 0 {
                    f::DestroyWindow(surface.hwnd);
                }
            }
        }
        self.host = 0;
    }
    fn system_battery() -> (bool, bool) {
        let mut status: f::SYSTEM_POWER_STATUS = f::zeroed();
        if unsafe { f::GetSystemPowerStatus(&mut status) } == 0 {
            return (false, false);
        }
        (status.ACLineStatus == 0, status.SystemStatusFlag == 1)
    }
    fn choose_item(&self, id: &str, battery: bool) -> Option<LibraryItem> {
        let selected = self.library.item(id)?.clone();
        if self.library.settings.adaptive
            && battery
            && selected.kind == MediaKind::Video
            && selected.profile != Profile::UltraEco
        {
            if let Some(low) = self.library.items.iter().find(|item| {
                item.original == selected.original
                    && item.profile == Profile::UltraEco
                    && item.asset.is_file()
            }) {
                return Some(low.clone());
            }
        }
        Some(selected)
    }
    fn rebuild_surfaces(&mut self) {
        self.clear_surfaces();
        let assignments = self.library.assignments.clone();
        if assignments.is_empty() {
            self.update_timer();
            return;
        }
        self.monitor_list = desktop::monitors();
        let host = match desktop::desktop_host() {
            Ok(host) => host,
            Err(err) => {
                self.status(&format!("Wallpaper host unavailable: {err}"));
                self.log(&err);
                self.update_timer();
                return;
            }
        };
        self.host = host;
        let (battery, _saver) = Self::system_battery();
        let monitors = self.monitor_list.clone();
        for m in &monitors {
            let Some(id) = assignments.get(&m.device) else {
                continue;
            };
            let Some(item) = self.choose_item(id, battery) else {
                continue;
            };
            if !item.asset.is_file() {
                self.log(&format!(
                    "Missing cached wallpaper: {}",
                    item.asset.display()
                ));
                continue;
            }
            let placement = match desktop::child_rect(m, host) {
                Ok(r) => r,
                Err(err) => {
                    self.log(&err);
                    continue;
                }
            };
            let child_class = f::wide(SURFACE_CLASS);
            let hwnd = unsafe {
                f::CreateWindowExW(
                    f::WS_EX_TOOLWINDOW | f::WS_EX_NOACTIVATE,
                    child_class.as_ptr(),
                    f::wide("FeatherWall desktop content").as_ptr(),
                    f::WS_CHILD | f::WS_VISIBLE | f::WS_CLIPCHILDREN | f::WS_CLIPSIBLINGS,
                    placement.left,
                    placement.top,
                    placement.width(),
                    placement.height(),
                    host,
                    0,
                    self.instance,
                    ptr::null_mut(),
                )
            };
            if hwnd == 0 {
                self.log(&format!("Wallpaper window failed for {}", m.device));
                continue;
            }
            let content = match item.kind {
                MediaKind::Still => desktop::load_still(&item.asset).map(Content::Still),
                MediaKind::Video => playback::Player::open(&item.asset, hwnd).map(Content::Video),
            };
            match content {
                Ok(content) => {
                    let state = if item.kind == MediaKind::Still {
                        PlaybackState::Playing
                    } else {
                        PlaybackState::Loading
                    };
                    self.surfaces.push(Surface {
                        hwnd,
                        device: m.device.clone(),
                        id: id.clone(),
                        content,
                        state,
                        suspended: false,
                    });
                    unsafe {
                        f::InvalidateRect(hwnd, ptr::null(), 1);
                    }
                }
                Err(err) => {
                    unsafe {
                        f::DestroyWindow(hwnd);
                    }
                    self.log(&format!("{}: {err}", m.device));
                    self.status(&format!("Cannot play {}: {err}", item.title));
                }
            }
        }
        self.evaluate_suspension();
        self.update_timer();
        if !self.surfaces.is_empty() {
            self.log(&format!(
                "Started {} monitor wallpaper(s)",
                self.surfaces.len()
            ));
        }
    }
    fn evaluate_suspension(&mut self) {
        let foreground = unsafe { f::GetForegroundWindow() };
        let (battery, saver) = Self::system_battery();
        if let Some(previous) = self.last_power_battery {
            if previous != battery && self.library.settings.adaptive {
                self.last_power_battery = Some(battery);
                self.rebuild_surfaces();
                return;
            }
        }
        self.last_power_battery = Some(battery);
        let remote = unsafe { f::GetSystemMetrics(f::SM_REMOTESESSION) } != 0;
        let all_suspended = self.manual_pause
            || self.screen_off
            || self.session_locked
            || self.sleep
            || (self.library.settings.pause_battery && battery)
            || (self.library.settings.pause_saver && saver)
            || (self.library.settings.pause_remote && remote);
        let mut warnings = Vec::new();
        self.log(&format!(
            "evaluate_suspension: manual={} screen_off={} locked={} sleep={} battery={} saver={} remote={} fg={:#x}",
            self.manual_pause, self.screen_off, self.session_locked, self.sleep,
            self.library.settings.pause_battery && battery,
            self.library.settings.pause_saver && saver,
            self.library.settings.pause_remote && remote,
            foreground
        ));
        for surface in &mut self.surfaces {
            let rect = self
                .monitor_list
                .iter()
                .find(|m| m.device == surface.device)
                .map(|m| m.rect);
            let obscured = if let Some(monitor) = rect {
                (self.library.settings.pause_fullscreen
                    && desktop::covers_window(foreground, &monitor, true))
                    || (self.library.settings.pause_maximized
                        && desktop::covers_window(foreground, &monitor, false))
            } else {
                true
            };
            let should_pause = all_suspended || obscured;
            warnings.push(format!(
                "surface {} rect={:?} obscured={} should_pause={}",
                surface.device, rect, obscured, should_pause
            ));
            if should_pause && !surface.suspended {
                if surface.state == PlaybackState::Loading {
                    surface.suspended = true;
                    surface.state = PlaybackState::Paused;
                    continue;
                }
                match surface.content.pause() {
                    Ok(()) => {
                        surface.state = PlaybackState::Paused;
                        surface.suspended = true;
                    }
                    Err(err) => {
                        surface.state = PlaybackState::Failed;
                        warnings.push(format!("Pause failed: {err}"));
                    }
                }
            } else if !should_pause && surface.suspended {
                match surface.content.play() {
                    Ok(()) => {
                        surface.state = PlaybackState::Playing;
                        surface.suspended = false;
                    }
                    Err(err) => {
                        surface.state = PlaybackState::Failed;
                        warnings.push(format!("Resume failed: {err}"));
                    }
                }
            }
        }
        for warning in warnings {
            self.log(&warning);
        }
        if let Some(mgr) = &mut self.widgets_overlay {
            let overlay_suspended = all_suspended
                || (!self.surfaces.is_empty() && self.surfaces.iter().all(|s| s.suspended));
            if overlay_suspended && !mgr.suspended {
                mgr.suspend();
            } else if !overlay_suspended && mgr.suspended {
                mgr.resume();
            }
        }
        self.update_timer();
    }
    fn play_event(&mut self, hwnd: f::HWND, kind: usize, hr: isize) {
        let Some(device) = self
            .surfaces
            .iter()
            .find(|s| s.hwnd == hwnd)
            .map(|s| s.device.clone())
        else {
            return;
        };
        self.log(&format!("play_event: {device} kind={kind} hr={hr:#x}"));
        let mut error = None;
        if let Some(surface) = self.surfaces.iter_mut().find(|s| s.hwnd == hwnd) {
            if hr < 0 || kind == 10 {
                surface.state = PlaybackState::Failed;
                error = Some(format!(
                    "Media Foundation event {kind} failed with {hr:#x} on {}",
                    surface.device
                ));
            } else {
                if kind == 0 {
                    if surface.suspended {
                        if let Err(e) = surface.content.pause() {
                            surface.state = PlaybackState::Failed;
                            error = Some(e);
                        }
                    } else {
                        surface.state = PlaybackState::Playing;
                    }
                }
                if kind == 11 && !surface.suspended && !self.manual_pause {
                    if let Content::Video(ref mut player) = surface.content {
                        if let Err(e) = player.restart() {
                            surface.state = PlaybackState::Failed;
                            error = Some(e);
                        }
                    }
                }
            }
        }
        if let Some(error) = error {
            self.log(&error);
            self.status(&error);
        }
    }
    fn update_timer(&self) {
        let every = if self.worker.is_some() {
            250
        } else if unsafe { f::IsWindowVisible(self.hwnd) } != 0 {
            1500
        } else if self
            .surfaces
            .iter()
            .any(|s| matches!(&s.content, Content::Video(_)) && !s.suspended)
        {
            4000
        } else {
            0
        };
        unsafe {
            f::KillTimer(self.hwnd, 1);
            if every > 0 {
                f::SetTimer(self.hwnd, 1, every, ptr::null());
            }
        }
    }
    fn on_timer(&mut self) {
        self.poll_import();
        if self.host != 0 && unsafe { f::IsWindow(self.host) } == 0 {
            self.log("Explorer wallpaper host invalidated; attempting recovery");
            self.rebuild_surfaces();
        } else if self.host == 0 && !self.library.assignments.is_empty() && self.worker.is_none() {
            // A previous Explorer attach failed. An explicit timer (when UI visible) allows retry.
            self.rebuild_surfaces();
        }
        self.evaluate_suspension();
        if unsafe { f::IsWindowVisible(self.hwnd) } != 0 {
            self.update_diagnostics();
        }
    }
    fn update_diagnostics(&mut self) {
        let mut memory: f::PROCESS_MEMORY_COUNTERS_EX = f::zeroed();
        memory.cb = std::mem::size_of::<f::PROCESS_MEMORY_COUNTERS_EX>() as u32;
        if unsafe {
            f::GetProcessMemoryInfo(
                f::GetCurrentProcess(),
                &mut memory as *mut _ as *mut _,
                memory.cb,
            )
        } != 0
        {
            self.perf_working_set = memory.WorkingSetSize as f64 / (1024.0 * 1024.0);
            self.perf_private = memory.PrivateUsage as f64 / (1024.0 * 1024.0);
        }

        let mut create: f::FILETIME = f::zeroed();
        let mut end: f::FILETIME = f::zeroed();
        let mut kernel: f::FILETIME = f::zeroed();
        let mut user: f::FILETIME = f::zeroed();
        if unsafe {
            f::GetProcessTimes(
                f::GetCurrentProcess(),
                &mut create,
                &mut end,
                &mut kernel,
                &mut user,
            )
        } != 0
        {
            let to64 = |x: f::FILETIME| ((x.hi as u64) << 32) | x.lo as u64;
            let now = Instant::now();
            let ticks = to64(kernel).saturating_add(to64(user));
            if let Some((prior, previous_ticks)) = self.last_perf {
                let elapsed = now.duration_since(prior).as_secs_f64();
                if elapsed > 0.0 {
                    let processors = thread::available_parallelism()
                        .map(|n| n.get())
                        .unwrap_or(1);
                    let percent = ((ticks.saturating_sub(previous_ticks) as f64 / 10_000_000.)
                        / elapsed)
                        * 100.
                        / processors as f64;
                    self.perf_cpu = percent.max(0.0);
                }
            }
            self.last_perf = Some((now, ticks));
        }

        // Collect performance chart history only when on the Performance tab to avoid unnecessary work
        if self.ui.current_tab == NavTab::Performance {
            self.ui.cpu_history.push(self.perf_cpu);
            if self.ui.cpu_history.len() > 60 {
                self.ui.cpu_history.remove(0);
            }
            self.ui.mem_history.push(self.perf_working_set);
            if self.ui.mem_history.len() > 60 {
                self.ui.mem_history.remove(0);
            }
            self.refresh_recent_logs();
        }

        unsafe {
            f::InvalidateRect(self.hwnd, ptr::null(), 0);
        }
    }
    fn refresh_choices(&mut self) {
        unsafe {
            f::InvalidateRect(self.hwnd, ptr::null(), 0);
        }
    }
    fn apply_selection(&mut self) {
        let Some(item) = selected_item(self) else {
            self.status("Select a wallpaper first.");
            return;
        };
        let Some(monitor) = selected_monitor(self) else {
            self.status("No connected monitor detected.");
            return;
        };
        self.library
            .assignments
            .insert(monitor.device.clone(), item.id.clone());
        self.persist();
        self.manual_pause = false;
        self.rebuild_surfaces();
        self.status(&format!("Assigned {} to {}", item.title, monitor.device));
        self.refresh_choices();
    }
    fn remove_selection(&mut self) {
        let Some(item) = selected_item(self) else {
            return;
        };
        let id = item.id.clone();
        self.library.remove(&id);
        self.library.assignments.retain(|_, v| v != &id);
        if self.ui.selected_item_id.as_deref() == Some(&id) {
            self.ui.selected_item_id = self.library.items.first().map(|i| i.id.clone());
        }
        self.persist();
        self.rebuild_surfaces();
        self.status("Library entry removed.");
        self.refresh_choices();
    }
    fn pause(&mut self) {
        self.manual_pause = true;
        self.evaluate_suspension();
        self.status("Paused (engine suspended; windows preserved).");
        self.refresh_choices();
    }
    fn resume(&mut self) {
        self.manual_pause = false;
        if self.surfaces.is_empty() {
            self.rebuild_surfaces();
        }
        self.evaluate_suspension();
        self.status("Resumed visible wallpapers.");
        self.refresh_choices();
    }
    fn stop(&mut self) {
        self.clear_surfaces();
        self.manual_pause = false;
        self.update_timer();
        self.status("Stopped; video decoders and resources released.");
        self.refresh_choices();
    }
    fn cleanup_cache(&mut self) {
        if self.worker.is_some() {
            self.status("Finish or cancel the active import before cleaning the cache.");
            return;
        }
        match importer::cleanup(
            &self.root,
            self.library.items.iter().map(|i| i.asset.clone()),
        ) {
            Ok((count, bytes)) => {
                self.status(&format!(
                    "Removed {count} unused cache assets ({:.1} MiB)",
                    bytes as f64 / 1048576.
                ));
                self.update_cache_size();
                self.refresh_choices();
            }
            Err(e) => self.status(&format!("Cache cleanup failed: {e}")),
        }
    }
    fn pick_file(&self) -> Option<PathBuf> {
        let mut file = [0u16; 32768];
        let filter: Vec<u16> = "Wallpapers\0*.mp4;*.gif;*.webm;*.mov;*.mkv;*.avi;*.jpg;*.jpeg;*.png;*.bmp\0All files\0*.*\0\0".encode_utf16().collect();
        let title = f::wide("Import an animated wallpaper or still image");
        let mut dialog: f::OPENFILENAMEW = f::zeroed();
        dialog.lStructSize = std::mem::size_of::<f::OPENFILENAMEW>() as u32;
        dialog.hwndOwner = self.hwnd;
        dialog.lpstrFilter = filter.as_ptr();
        dialog.lpstrFile = file.as_mut_ptr();
        dialog.nMaxFile = file.len() as u32;
        dialog.lpstrTitle = title.as_ptr();
        dialog.Flags =
            f::OFN_FILEMUSTEXIST | f::OFN_PATHMUSTEXIST | f::OFN_EXPLORER | f::OFN_NOCHANGEDIR;
        if unsafe { f::GetOpenFileNameW(&mut dialog) } == 0 {
            return None;
        }
        use std::os::windows::ffi::OsStringExt;
        let len = file.iter().position(|&x| x == 0).unwrap_or(file.len());
        Some(OsString::from_wide(&file[..len]).into())
    }
    fn start_import(&mut self, source: PathBuf) {
        if self.worker.is_some() {
            self.status("One import is already running. Cancel it first.");
            return;
        }
        if !source.is_file() {
            self.status("The selected source file does not exist.");
            return;
        }
        let monitor = selected_monitor(self).or_else(|| self.monitor_list.first().cloned());
        let (width, height) = monitor
            .map(|m| (m.rect.width().max(2) as u32, m.rect.height().max(2) as u32))
            .unwrap_or((1920, 1080));
        let (battery, _) = Self::system_battery();
        let options = ImportOptions {
            profile: self.library.settings.profile,
            monitor_width: width,
            monitor_height: height,
            fps: self.library.settings.custom_fps,
            cap: self.library.settings.custom_cap,
            fit: self.library.settings.fit,
            battery,
        };
        let root = self.root.clone();
        let cancel = CancelToken::new();
        let worker_cancel = cancel.clone();
        let (tx, rx) = mpsc::channel();
        let name = source
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let handle = thread::spawn(move || {
            let finished = tx.clone();
            let result = importer::import(&source, &root, &options, &worker_cancel, |state| {
                let _ = tx.send(WorkerEvent::Progress(state));
            });
            let _ = finished.send(WorkerEvent::Done(result));
        });
        self.worker = Some(Worker {
            cancel,
            receiver: rx,
            handle,
        });
        self.worker_status = format!("Analyzing {name}...");
        self.worker_progress = 5;
        self.status(&format!("Analyzing {name}..."));
        self.log(&format!("Import started: {name}"));
        self.update_timer();
    }
    fn poll_import(&mut self) {
        let mut completed = None;
        let mut events = Vec::new();
        if let Some(worker) = &self.worker {
            while let Ok(event) = worker.receiver.try_recv() {
                events.push(event);
            }
        }
        for event in events {
            match event {
                WorkerEvent::Progress(progress) => match progress {
                    Progress::Analyze => {
                        self.worker_status = "Probing media with FFprobe...".into();
                        self.worker_progress = 10;
                        self.status("Probing input media with FFprobe...");
                    }
                    Progress::Hashing => {
                        self.worker_status = "Checking cache integrity...".into();
                        self.worker_progress = 25;
                        self.status("Checking cache integrity...");
                    }
                    Progress::Reusing => {
                        self.worker_status = "Reusing existing optimized cache...".into();
                        self.worker_progress = 90;
                        self.status("Reusing existing optimized cache...");
                    }
                    Progress::Converting(percent) => {
                        self.worker_status = format!("Optimizing wallpaper: {percent}%");
                        self.worker_progress = percent;
                        self.status(&format!("Optimizing wallpaper: {percent}%"));
                    }
                    Progress::Finished => {
                        self.worker_status = "Validating optimized stream...".into();
                        self.worker_progress = 98;
                        self.status("Validating optimized output...");
                    }
                    Progress::Failed(error) => {
                        self.worker_status = error.clone();
                        self.status(&error);
                    }
                },
                WorkerEvent::Done(result) => completed = Some(result),
            }
        }
        if let Some(result) = completed {
            if let Some(worker) = self.worker.take() {
                let _ = worker.handle.join();
            }
            self.worker_progress = 0;
            self.worker_status.clear();
            match result {
                Ok(item) => {
                    let title = item.title.clone();
                    let id = item.id.clone();
                    let bytes = item.bytes;
                    let thumb = item.thumb_path();
                    if !thumb.is_file() && item.original.is_file() {
                        let _ = importer::generate_thumbnail(
                            &item.original,
                            &thumb,
                            item.kind == MediaKind::Video,
                        );
                    }
                    self.ui.selected_item_id = Some(id.clone());
                    self.library.items.retain(|existing| existing.id != id);
                    self.library.items.push(item);
                    if let Some(monitor) =
                        selected_monitor(self).or_else(|| self.monitor_list.first().cloned())
                    {
                        self.library.assignments.insert(monitor.device, id);
                    }
                    self.persist();
                    self.rebuild_surfaces();
                    self.status(&format!(
                        "Imported {title} ({:.1} MiB). Applied to display.",
                        bytes as f64 / 1048576.
                    ));
                    self.log(&format!("Import completed: {title}"));
                }
                Err(err) => {
                    self.status(&format!("Import failed: {err}"));
                    self.log(&format!("Import failed: {err}"));
                }
            }
            self.update_cache_size();
            self.refresh_choices();
            self.update_timer();
        }
    }
    fn reoptimize(&mut self) {
        let Some(item) = selected_item(self) else {
            return;
        };
        if !item.original.exists() {
            self.status("Source file is missing; current optimized wallpaper remains usable.");
            return;
        }
        self.start_import(item.original);
    }
    fn set_profile(&mut self, index: usize) {
        if let Some(profile) = Profile::ALL.get(index).copied() {
            self.library.settings.profile = profile;
            self.persist();
            self.status(&format!(
                "Profile changed to {}. Re-optimize a wallpaper to apply it.",
                profile.text()
            ));
            self.refresh_choices();
        }
    }
    fn open_folder(&self) {
        let path = f::wide(&self.root.join("cache").to_string_lossy());
        unsafe {
            f::ShellExecuteW(
                self.hwnd,
                f::wide("open").as_ptr(),
                path.as_ptr(),
                ptr::null(),
                ptr::null(),
                f::SW_SHOWNORMAL,
            );
        }
    }
    fn paint(&mut self) {
        let mut ps: f::PAINTSTRUCT = f::zeroed();
        let hdc = unsafe { f::BeginPaint(self.hwnd, &mut ps) };
        if hdc == 0 {
            return;
        }
        let mut rc: f::RECT = f::zeroed();
        unsafe { f::GetClientRect(self.hwnd, &mut rc) };
        let width = rc.right - rc.left;
        let height = rc.bottom - rc.top;

        if width > 0 && height > 0 {
            if let Some(mem_dc) = ui::draw::MemDc::create(hdc, width, height) {
                let playing_count = self
                    .surfaces
                    .iter()
                    .filter(|s| s.state == PlaybackState::Playing)
                    .count();
                let paused_count = self
                    .surfaces
                    .iter()
                    .filter(|s| s.state == PlaybackState::Paused)
                    .count();
                let worker_active = self.worker.is_some();
                let fonts = self.ui.fonts.clone();
                let theme = self.ui.theme;

                let ctx = ui::views::ViewContext {
                    theme,
                    fonts: &fonts,
                    win_w: width,
                    win_h: height,
                    active_tab: self.ui.current_tab,
                    cpu_pct: self.perf_cpu,
                    working_set_mb: self.perf_working_set,
                    private_commit_mb: self.perf_private,
                    playing_count,
                    paused_count,
                    worker_active,
                    worker_progress: self.worker_progress,
                    worker_status: &self.worker_status,
                    widgets: &self.library.widgets,
                };

                ui::views::render_shell(
                    mem_dc.hdc,
                    &mut self.ui,
                    &self.library.items,
                    &self.monitor_list,
                    &self.library.assignments,
                    &ctx,
                );

                mem_dc.flip(hdc);
            }
        }

        unsafe { f::EndPaint(self.hwnd, &ps) };
    }
    fn execute_ui_action(&mut self, action: UiAction) {
        match action {
            UiAction::None => {}
            UiAction::ImportFile => {
                if let Some(file) = self.pick_file() {
                    self.start_import(file);
                }
            }
            UiAction::TogglePause => {
                if self.manual_pause
                    || self
                        .surfaces
                        .iter()
                        .all(|s| s.state == PlaybackState::Paused)
                {
                    self.resume();
                } else {
                    self.pause();
                }
            }
            UiAction::Resume => self.resume(),
            UiAction::Pause => self.pause(),
            UiAction::Stop => self.stop(),
            UiAction::ApplySelected => self.apply_selection(),
            UiAction::ApplyToMonitor(device) => {
                if let Some(item) = selected_item(self) {
                    self.library
                        .assignments
                        .insert(device.clone(), item.id.clone());
                    self.persist();
                    self.manual_pause = false;
                    self.rebuild_surfaces();
                    self.status(&format!("Assigned {} to {}", item.title, device));
                } else {
                    self.status("Select a wallpaper from the library first.");
                }
            }
            UiAction::ClearMonitor(device) => {
                self.library.assignments.remove(&device);
                self.persist();
                self.rebuild_surfaces();
                self.status(&format!("Cleared wallpaper from {device}."));
            }
            UiAction::ReoptimizeSelected => self.reoptimize(),
            UiAction::ShowFileInExplorer(path) => {
                if path.is_file() {
                    let param = format!("/select,\"{}\"", path.display());
                    let param_w = f::wide(&param);
                    unsafe {
                        f::ShellExecuteW(
                            self.hwnd,
                            f::wide("open").as_ptr(),
                            f::wide("explorer.exe").as_ptr(),
                            param_w.as_ptr(),
                            ptr::null(),
                            f::SW_SHOWNORMAL,
                        );
                    }
                } else {
                    self.status("Source file is not available on disk.");
                }
            }
            UiAction::RemoveItem(id) => {
                self.library.remove(&id);
                self.library.assignments.retain(|_, v| v != &id);
                if self.ui.selected_item_id.as_deref() == Some(&id) {
                    self.ui.selected_item_id = self.library.items.first().map(|i| i.id.clone());
                }
                self.persist();
                self.rebuild_surfaces();
                self.status("Library item removed.");
            }
            UiAction::CancelWorker => {
                if let Some(worker) = &self.worker {
                    worker.cancel.cancel();
                    self.status("Cancelling import operation...");
                }
            }
            UiAction::SetProfile(profile) => {
                self.library.settings.profile = profile;
                self.persist();
                self.status(&format!("Profile set to {}.", profile.text()));
            }
            UiAction::ToggleAutostart => {
                let target = !self.library.settings.start_windows;
                match reg_autostart(target) {
                    Ok(()) => {
                        self.library.settings.start_windows = target;
                        self.persist();
                        self.status(if target {
                            "FeatherWall will start with Windows."
                        } else {
                            "Start with Windows disabled."
                        });
                    }
                    Err(err) => self.status(&err),
                }
            }
            UiAction::ToggleSetting(key) => {
                match key {
                    "fullscreen" => {
                        self.library.settings.pause_fullscreen =
                            !self.library.settings.pause_fullscreen;
                        self.evaluate_suspension();
                    }
                    "maximized" => {
                        self.library.settings.pause_maximized =
                            !self.library.settings.pause_maximized;
                        self.evaluate_suspension();
                    }
                    "battery" => {
                        self.library.settings.pause_battery = !self.library.settings.pause_battery;
                        self.evaluate_suspension();
                    }
                    "saver" => {
                        self.library.settings.pause_saver = !self.library.settings.pause_saver;
                        self.evaluate_suspension();
                    }
                    "remote" => {
                        self.library.settings.pause_remote = !self.library.settings.pause_remote;
                        self.evaluate_suspension();
                    }
                    "adaptive" => {
                        self.library.settings.adaptive = !self.library.settings.adaptive;
                        self.rebuild_surfaces();
                    }
                    "fit" => {
                        self.library.settings.fit = !self.library.settings.fit;
                    }
                    _ => {}
                }
                self.persist();
            }
            UiAction::CleanCache => {
                self.cleanup_cache();
                self.update_cache_size();
            }
            UiAction::OpenCacheFolder => self.open_folder(),
            UiAction::OpenLogFile => {
                let log_path = self.root.join("featherwall.log");
                if !log_path.exists() {
                    let _ = fs::write(&log_path, "FeatherWall log initialized\n");
                }
                let path_w = f::wide(&log_path.to_string_lossy());
                unsafe {
                    f::ShellExecuteW(
                        self.hwnd,
                        f::wide("open").as_ptr(),
                        path_w.as_ptr(),
                        ptr::null(),
                        ptr::null(),
                        f::SW_SHOWNORMAL,
                    );
                }
            }
            UiAction::ToggleTheme => {
                self.ui.theme = if self.ui.theme.is_dark {
                    ui::theme::Theme::light()
                } else {
                    ui::theme::Theme::dark()
                };
                self.library.settings.dark_theme = self.ui.theme.is_dark;
                self.ui.theme.apply_to_window(self.hwnd);
                self.persist();
            }
            UiAction::ToggleWidgetsMaster => {
                self.library.widgets.enabled = !self.library.widgets.enabled;
                if let Some(mgr) = &mut self.widgets_overlay {
                    mgr.update_settings(&self.library.widgets);
                    mgr.sync_monitors(&self.monitor_list);
                }
                self.persist();
            }
            UiAction::ToggleClock => {
                self.library.widgets.clock_enabled = !self.library.widgets.clock_enabled;
                if let Some(mgr) = &mut self.widgets_overlay {
                    mgr.update_settings(&self.library.widgets);
                }
                self.persist();
            }
            UiAction::ToggleClock24h => {
                self.library.widgets.clock_24h = !self.library.widgets.clock_24h;
                if let Some(mgr) = &mut self.widgets_overlay {
                    mgr.update_settings(&self.library.widgets);
                }
                self.persist();
            }
            UiAction::ToggleClockSeconds => {
                self.library.widgets.clock_seconds = !self.library.widgets.clock_seconds;
                if let Some(mgr) = &mut self.widgets_overlay {
                    mgr.update_settings(&self.library.widgets);
                }
                self.persist();
            }
            UiAction::SetClockSize(size) => {
                self.library.widgets.clock_size = size;
                if let Some(mgr) = &mut self.widgets_overlay {
                    mgr.update_settings(&self.library.widgets);
                }
                self.persist();
            }
            UiAction::SetClockColor(color) => {
                self.library.widgets.clock_color = color;
                if let Some(mgr) = &mut self.widgets_overlay {
                    mgr.update_settings(&self.library.widgets);
                }
                self.persist();
            }
            UiAction::SetClockOpacity(opacity) => {
                self.library.widgets.opacity = opacity;
                if let Some(mgr) = &mut self.widgets_overlay {
                    mgr.update_settings(&self.library.widgets);
                }
                self.persist();
            }
            UiAction::ToggleDate => {
                self.library.widgets.date_enabled = !self.library.widgets.date_enabled;
                if let Some(mgr) = &mut self.widgets_overlay {
                    mgr.update_settings(&self.library.widgets);
                }
                self.persist();
            }
            UiAction::SetDateFormat(df) => {
                self.library.widgets.date_format = df;
                if let Some(mgr) = &mut self.widgets_overlay {
                    mgr.update_settings(&self.library.widgets);
                }
                self.persist();
            }
            UiAction::SetDateSize(size) => {
                self.library.widgets.date_size = size;
                if let Some(mgr) = &mut self.widgets_overlay {
                    mgr.update_settings(&self.library.widgets);
                }
                self.persist();
            }
            UiAction::ToggleGreeting => {
                self.library.widgets.greeting_enabled = !self.library.widgets.greeting_enabled;
                if let Some(mgr) = &mut self.widgets_overlay {
                    mgr.update_settings(&self.library.widgets);
                }
                self.persist();
            }
            UiAction::SetGreetingMode(mode) => {
                self.library.widgets.greeting_mode = mode;
                if let Some(mgr) = &mut self.widgets_overlay {
                    mgr.update_settings(&self.library.widgets);
                }
                self.persist();
            }
            UiAction::SetGreetingInterval(interval) => {
                self.library.widgets.greeting_interval_mins = interval;
                if let Some(mgr) = &mut self.widgets_overlay {
                    mgr.update_settings(&self.library.widgets);
                }
                self.persist();
            }
            UiAction::ToggleMusic => {
                self.library.widgets.music_enabled = !self.library.widgets.music_enabled;
                if let Some(mgr) = &mut self.widgets_overlay {
                    mgr.update_settings(&self.library.widgets);
                }
                self.persist();
            }
            UiAction::ToggleMusicSpotifyPreferred => {
                self.library.widgets.music_spotify_preferred = !self.library.widgets.music_spotify_preferred;
                if let Some(mgr) = &mut self.widgets_overlay {
                    mgr.update_settings(&self.library.widgets);
                }
                self.persist();
            }
            UiAction::ToggleMusicGeneralMedia => {
                self.library.widgets.music_general_media = !self.library.widgets.music_general_media;
                if let Some(mgr) = &mut self.widgets_overlay {
                    mgr.update_settings(&self.library.widgets);
                }
                self.persist();
            }
            UiAction::SetWidgetPosition(pos) => {
                self.library.widgets.position = pos;
                if let Some(mgr) = &mut self.widgets_overlay {
                    mgr.update_settings(&self.library.widgets);
                }
                self.persist();
            }
            UiAction::AdjustWidgetOffsetX(delta) => {
                self.library.widgets.offset_x += delta;
                if let Some(mgr) = &mut self.widgets_overlay {
                    mgr.update_settings(&self.library.widgets);
                }
                self.persist();
            }
            UiAction::AdjustWidgetOffsetY(delta) => {
                self.library.widgets.offset_y += delta;
                if let Some(mgr) = &mut self.widgets_overlay {
                    mgr.update_settings(&self.library.widgets);
                }
                self.persist();
            }
            UiAction::ToggleTextShadow => {
                self.library.widgets.text_shadow = !self.library.widgets.text_shadow;
                if let Some(mgr) = &mut self.widgets_overlay {
                    mgr.update_settings(&self.library.widgets);
                }
                self.persist();
            }
            UiAction::SetWidgetDisplayTarget(target) => {
                self.library.widgets.display_target = target;
                if let Some(mgr) = &mut self.widgets_overlay {
                    mgr.update_settings(&self.library.widgets);
                    mgr.sync_monitors(&self.monitor_list);
                }
                self.persist();
            }
            UiAction::ToggleWidgetEditMode => {
                self.library.widgets.edit_mode = !self.library.widgets.edit_mode;
                if let Some(mgr) = &mut self.widgets_overlay {
                    mgr.update_settings(&self.library.widgets);
                }
                self.persist();
            }
            UiAction::OpenSpotifyApp => {
                media::open_spotify();
            }
        }
        self.refresh_choices();
    }
    fn dispatch_command(&mut self, id: u16, _notification: u16) {
        if (ID_TRAY_PROFILE_BASE..ID_TRAY_PROFILE_BASE + 4).contains(&id) {
            self.set_profile((id - ID_TRAY_PROFILE_BASE) as usize);
            return;
        }
        match id {
            ID_TRAY_WIDGETS => {
                self.execute_ui_action(UiAction::ToggleWidgetsMaster);
            }
            ID_IMPORT => {
                if let Some(file) = self.pick_file() {
                    self.start_import(file);
                }
            }
            ID_APPLY => self.apply_selection(),
            ID_REMOVE => self.remove_selection(),
            ID_REOPT => self.reoptimize(),
            ID_CLEAN => self.cleanup_cache(),
            ID_PAUSE => self.pause(),
            ID_RESUME => self.resume(),
            ID_STOP => self.stop(),
            ID_CANCEL => {
                if let Some(worker) = &self.worker {
                    worker.cancel.cancel();
                    self.status("Cancelling the active operation...");
                }
            }
            ID_FOLDER => self.open_folder(),
            ID_TRAY_OPEN => self.show_ui(),
            ID_TRAY_SELECT => {
                self.ui.current_tab = NavTab::Library;
                self.show_ui();
            }
            ID_TRAY_SETTINGS => {
                self.ui.current_tab = NavTab::Settings;
                self.show_ui();
            }
            ID_TRAY_TOGGLE_START => {
                self.execute_ui_action(UiAction::ToggleAutostart);
            }
            ID_EXIT => {
                self.exiting = true;
                self.cleanup();
                unsafe {
                    f::DestroyWindow(self.hwnd);
                    f::PostQuitMessage(0);
                }
            }
            _ => {}
        }
    }
    fn main_message(&mut self, msg: u32, w: usize, l: isize) -> Option<isize> {
        if msg == self.taskbar_message && msg != 0 {
            self.log("Explorer taskbar restart observed; re-registering notification icon and WorkerW host");
            self.install_tray();
            self.rebuild_surfaces();
            return Some(0);
        }
        match msg {
            f::WM_ERASEBKGND => Some(1),
            f::WM_PAINT => {
                self.paint();
                Some(0)
            }
            f::WM_GETMINMAXINFO => {
                let info = unsafe { &mut *(l as *mut f::MINMAXINFO) };
                info.ptMinTrackSize.x = 880;
                info.ptMinTrackSize.y = 560;
                Some(0)
            }
            f::WM_SIZE => {
                unsafe {
                    f::InvalidateRect(self.hwnd, ptr::null(), 0);
                }
                Some(0)
            }
            f::WM_SETCURSOR => {
                if f::low(l as usize) == 1 {
                    let cursor_id = if self.ui.hovered_btn_id.is_some()
                        || self.ui.hovered_nav.is_some()
                        || self.ui.hovered_item_idx.is_some()
                    {
                        f::IDC_HAND
                    } else {
                        f::IDC_ARROW
                    };
                    unsafe {
                        f::SetCursor(f::LoadCursorW(0, cursor_id as *const u16));
                    }
                    return Some(1);
                }
                None
            }
            f::WM_MOUSEMOVE => {
                let px = f::low(l as usize) as i16 as i32;
                let py = f::high(l as usize) as i16 as i32;
                let mut rc: f::RECT = f::zeroed();
                unsafe { f::GetClientRect(self.hwnd, &mut rc) };
                let win_w = rc.right - rc.left;
                let win_h = rc.bottom - rc.top;

                let mut tme = f::TRACKMOUSEEVENT {
                    cbSize: std::mem::size_of::<f::TRACKMOUSEEVENT>() as u32,
                    dwFlags: f::TME_LEAVE,
                    hwndTrack: self.hwnd,
                    dwHoverTime: 0,
                };
                unsafe { f::TrackMouseEvent(&mut tme) };

                let changed = ui::events::handle_mouse_move(
                    &mut self.ui,
                    px,
                    py,
                    win_w,
                    win_h,
                    &self.library.items,
                    &self.monitor_list,
                );
                if changed {
                    unsafe { f::InvalidateRect(self.hwnd, ptr::null(), 0) };
                }
                Some(0)
            }
            f::WM_MOUSELEAVE => {
                self.ui.hovered_nav = None;
                self.ui.hovered_btn_id = None;
                self.ui.hovered_item_idx = None;
                unsafe { f::InvalidateRect(self.hwnd, ptr::null(), 0) };
                Some(0)
            }
            f::WM_LBUTTONDOWN => {
                let px = f::low(l as usize) as i16 as i32;
                let py = f::high(l as usize) as i16 as i32;
                let mut rc: f::RECT = f::zeroed();
                unsafe { f::GetClientRect(self.hwnd, &mut rc) };
                let win_w = rc.right - rc.left;
                let win_h = rc.bottom - rc.top;

                let action = ui::events::handle_click(
                    &mut self.ui,
                    px,
                    py,
                    win_w,
                    win_h,
                    &self.library.items,
                    &self.monitor_list,
                );
                self.execute_ui_action(action);
                Some(0)
            }
            f::WM_LBUTTONDBLCLK => {
                let px = f::low(l as usize) as i16 as i32;
                let py = f::high(l as usize) as i16 as i32;
                let mut rc: f::RECT = f::zeroed();
                unsafe { f::GetClientRect(self.hwnd, &mut rc) };
                let win_w = rc.right - rc.left;
                let win_h = rc.bottom - rc.top;

                let action = ui::events::handle_double_click(
                    &mut self.ui,
                    px,
                    py,
                    win_w,
                    win_h,
                    &self.library.items,
                );
                self.execute_ui_action(action);
                Some(0)
            }
            f::WM_MOUSEWHEEL => {
                let delta = (w >> 16) as i16 as i32;
                if ui::events::handle_scroll(&mut self.ui, delta) {
                    unsafe { f::InvalidateRect(self.hwnd, ptr::null(), 0) };
                }
                Some(0)
            }
            f::WM_CHAR => {
                let ch = w as u16;
                if self.ui.current_tab == NavTab::Library {
                    if ch == 8 {
                        self.ui.search_query.pop();
                        unsafe { f::InvalidateRect(self.hwnd, ptr::null(), 0) };
                        return Some(0);
                    } else if ch >= 32 {
                        if let Some(c) = char::from_u32(ch as u32) {
                            if self.ui.search_query.len() < 50 {
                                self.ui.search_query.push(c);
                                unsafe { f::InvalidateRect(self.hwnd, ptr::null(), 0) };
                                return Some(0);
                            }
                        }
                    }
                }
                Some(0)
            }
            f::WM_KEYDOWN => {
                if w == 27 {
                    if self.ui.confirm_delete.is_some() {
                        self.ui.confirm_delete = None;
                        unsafe { f::InvalidateRect(self.hwnd, ptr::null(), 0) };
                        return Some(0);
                    } else if !self.ui.search_query.is_empty() {
                        self.ui.search_query.clear();
                        unsafe { f::InvalidateRect(self.hwnd, ptr::null(), 0) };
                        return Some(0);
                    }
                }
                Some(0)
            }
            f::WM_DROPFILES => {
                let hdrop = w as isize;
                let count = unsafe { f::DragQueryFileW(hdrop, 0xFFFFFFFF, ptr::null_mut(), 0) };
                if count > 0 {
                    let mut buffer = [0u16; 1024];
                    let len = unsafe {
                        f::DragQueryFileW(hdrop, 0, buffer.as_mut_ptr(), buffer.len() as u32)
                    };
                    if len > 0 {
                        use std::os::windows::ffi::OsStringExt;
                        let path: PathBuf = OsString::from_wide(&buffer[..len as usize]).into();
                        if path.is_file() {
                            self.start_import(path);
                        }
                    }
                }
                unsafe { f::DragFinish(hdrop) };
                Some(0)
            }
            f::WM_CLOSE => {
                unsafe {
                    f::ShowWindow(self.hwnd, f::SW_HIDE);
                }
                self.update_timer();
                Some(0)
            }
            f::WM_COMMAND => {
                self.dispatch_command(f::low(w), f::high(w));
                Some(0)
            }
            f::WM_TIMER => {
                if w == 1 {
                    self.on_timer();
                }
                Some(0)
            }
            f::WM_DISPLAYCHANGE => {
                self.log("Display configuration changed");
                self.monitor_list = desktop::monitors();
                if let Some(mgr) = &mut self.widgets_overlay {
                    mgr.sync_monitors(&self.monitor_list);
                }
                self.refresh_choices();
                self.rebuild_surfaces();
                Some(0)
            }
            f::WM_TIMECHANGE => {
                self.log("System time or timezone changed");
                if let Some(mgr) = &mut self.widgets_overlay {
                    mgr.schedule_next_tick();
                    mgr.render_all();
                }
                Some(0)
            }
            f::WM_WTSSESSION_CHANGE => {
                match w {
                    f::WTS_SESSION_LOCK => self.session_locked = true,
                    f::WTS_SESSION_UNLOCK => self.session_locked = false,
                    _ => {}
                }
                self.evaluate_suspension();
                Some(0)
            }
            f::WM_POWERBROADCAST => {
                match w {
                    f::PBT_APMSUSPEND => self.sleep = true,
                    10 => {}
                    f::PBT_APMRESUMEAUTOMATIC => {
                        self.sleep = false;
                        if let Some(mgr) = &mut self.widgets_overlay {
                            mgr.schedule_next_tick();
                            mgr.render_all();
                        }
                        self.rebuild_surfaces();
                    }
                    f::PBT_POWERSETTINGCHANGE if l != 0 => {
                        let setting = unsafe { &*(l as *const f::POWERBROADCAST_SETTING) };
                        if setting.PowerSetting == f::GUID_CONSOLE_DISPLAY_STATE
                            && setting.DataLength >= 4
                        {
                            let value =
                                unsafe { ptr::read_unaligned(setting.Data.as_ptr().cast::<u32>()) };
                            let was_off = self.screen_off;
                            self.screen_off = value == 0;
                            if was_off && !self.screen_off {
                                self.rebuild_surfaces();
                            }
                        }
                    }
                    _ => {}
                }
                self.evaluate_suspension();
                Some(1)
            }
            WM_FOREGROUND => {
                self.evaluate_suspension();
                Some(0)
            }
            WM_SHOW => {
                self.show_ui();
                Some(0)
            }
            WM_CLIENT_PAUSE => {
                self.pause();
                Some(0)
            }
            WM_CLIENT_RESUME => {
                self.resume();
                Some(0)
            }
            WM_CLIENT_STOP => {
                self.stop();
                Some(0)
            }
            WM_CLIENT_STATUS => Some(
                self.surfaces
                    .iter()
                    .filter(|s| s.state == PlaybackState::Playing)
                    .count() as isize,
            ),
            f::WM_COPYDATA if l != 0 => {
                let data = unsafe { &*(l as *const f::COPYDATASTRUCT) };
                if data.dwData == 0x46575449
                    && !data.lpData.is_null()
                    && data.cbData >= 2
                    && data.cbData < 65536
                    && data.cbData % 2 == 0
                {
                    let slice = unsafe {
                        std::slice::from_raw_parts(
                            data.lpData.cast::<u16>(),
                            data.cbData as usize / 2,
                        )
                    };
                    let text = f::wchar_str(slice);
                    if !text.is_empty() {
                        self.start_import(PathBuf::from(text));
                        return Some(1);
                    }
                }
                Some(0)
            }
            WM_TRAY => {
                match l as usize {
                    f::MOUSE_LBUTTONDBLCLK => self.show_ui(),
                    f::MOUSE_RBUTTONUP => self.tray_menu(),
                    _ => {}
                }
                Some(0)
            }
            _ => None,
        }
    }
    fn cleanup(&mut self) {
        self.exiting = true;
        if let Some(worker) = self.worker.take() {
            worker.cancel.cancel();
            let _ = worker.handle.join();
        }
        self.clear_surfaces();
        if let Some(mut mgr) = self.widgets_overlay.take() {
            mgr.destroy_all();
        }
        self.remove_tray();
        unsafe {
            f::KillTimer(self.hwnd, 1);
            if self.have_wts {
                f::WTSUnRegisterSessionNotification(self.hwnd);
                self.have_wts = false;
            }
            if self.power_notify != 0 {
                f::UnregisterPowerSettingNotification(self.power_notify);
                self.power_notify = 0;
            }
            if self.hook != 0 {
                f::UnhookWinEvent(self.hook);
                self.hook = 0;
            }
        }
        self.log("Shut down cleanly");
    }
}

fn command_line(args: &[OsString]) -> Result<(), String> {
    let command = args[0].to_string_lossy().to_ascii_lowercase();
    let root = core::data_root().map_err(|e| e.to_string())?;
    match command.as_str() {
        "--help" | "help" => {
            println!("FeatherWall commands: import <media> [--profile eco|balanced|quality|automatic], list, pause, resume, stop, status, gui");
            Ok(())
        }
        "list" => {
            for item in core::load(&root.join("library.fwcfg")).items {
                println!("{} | {} | {}", item.id, item.title, item.asset.display());
            }
            Ok(())
        }
        "pause" | "resume" | "stop" | "status" | "gui" => {
            let hwnd = unsafe { f::FindWindowW(f::wide(MAIN_CLASS).as_ptr(), ptr::null()) };
            if hwnd == 0 {
                return Err("FeatherWall is not running".into());
            }
            let msg = match command.as_str() {
                "pause" => WM_CLIENT_PAUSE,
                "resume" => WM_CLIENT_RESUME,
                "stop" => WM_CLIENT_STOP,
                "gui" => WM_SHOW,
                _ => WM_CLIENT_STATUS,
            };
            let mut result = 0usize;
            if unsafe { f::SendMessageTimeoutW(hwnd, msg, 0, 0, 2, 3000, &mut result) } == 0 {
                return Err("FeatherWall did not respond".into());
            }
            if command == "status" {
                println!("Running; {result} monitor(s) playing");
            }
            Ok(())
        }
        "import" => {
            if args.len() < 2 {
                return Err(
                    "Usage: featherwall import <file> [--profile eco|balanced|quality|automatic]"
                        .into(),
                );
            }
            let file = PathBuf::from(&args[1]);
            if !file.is_file() {
                return Err("Input file not found".into());
            }
            let hwnd = unsafe { f::FindWindowW(f::wide(MAIN_CLASS).as_ptr(), ptr::null()) };
            if hwnd != 0 {
                use std::os::windows::ffi::OsStrExt;
                let source: Vec<u16> = file.as_os_str().encode_wide().chain(Some(0)).collect();
                let message = f::COPYDATASTRUCT {
                    dwData: 0x46575449,
                    cbData: (source.len() * 2) as u32,
                    lpData: source.as_ptr().cast(),
                };
                let mut result = 0usize;
                if unsafe {
                    f::SendMessageTimeoutW(
                        hwnd,
                        f::WM_COPYDATA,
                        0,
                        &message as *const _ as isize,
                        2,
                        5000,
                        &mut result,
                    )
                } == 0
                {
                    return Err("Could not forward import to running FeatherWall".into());
                }
                return Ok(());
            }
            let profile = if args.len() >= 4 && args[2].to_string_lossy() == "--profile" {
                Profile::parse(&args[3].to_string_lossy())
            } else {
                Profile::Automatic
            };
            let monitor = desktop::monitors().into_iter().next();
            let (w, h) = monitor
                .map(|m| (m.rect.width().max(2) as u32, m.rect.height().max(2) as u32))
                .unwrap_or((1920, 1080));
            let token = CancelToken::new();
            let item = importer::import(
                &file,
                &root,
                &ImportOptions {
                    profile,
                    monitor_width: w,
                    monitor_height: h,
                    cap: 0,
                    fps: 0,
                    fit: false,
                    battery: false,
                },
                &token,
                |p| println!("{p:?}"),
            )?;
            let mut lib = core::load(&root.join("library.fwcfg"));
            lib.items.retain(|existing| existing.id != item.id);
            lib.items.push(item);
            core::save(&root.join("library.fwcfg"), &lib).map_err(|e| e.to_string())?;
            Ok(())
        }
        _ => Err(format!("Unknown command: {command}")),
    }
}
