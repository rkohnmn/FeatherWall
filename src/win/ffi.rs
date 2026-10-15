//! Minimal, auditable Win32 ABI surface. All pointers/handles are native 64-bit on x64.
#![allow(non_snake_case, non_camel_case_types, dead_code)]
use std::ffi::c_void;
pub type HWND = isize;
pub type HANDLE = isize;
pub type HMONITOR = isize;
pub type HDC = isize;
pub type HBITMAP = isize;
pub type HGDIOBJ = isize;
pub type HINSTANCE = isize;
pub type HMENU = isize;
pub type HICON = isize;
pub type HCURSOR = isize;
pub type HKEY = isize;
pub type HHOOK = isize;
pub type HPOWERNOTIFY = isize;
pub type HDESK = isize;
pub type HRESULT = i32;
#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct RECT {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}
#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct POINT {
    pub x: i32,
    pub y: i32,
}
#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct FILETIME {
    pub lo: u32,
    pub hi: u32,
}
#[repr(C)]
pub struct MSG {
    pub hwnd: HWND,
    pub message: u32,
    pub wParam: usize,
    pub lParam: isize,
    pub time: u32,
    pub pt: POINT,
    pub lPrivate: u32,
}
#[repr(C)]
pub struct WNDCLASSEXW {
    pub cbSize: u32,
    pub style: u32,
    pub lpfnWndProc: Option<unsafe extern "system" fn(HWND, u32, usize, isize) -> isize>,
    pub cbClsExtra: i32,
    pub cbWndExtra: i32,
    pub hInstance: HINSTANCE,
    pub hIcon: HICON,
    pub hCursor: HCURSOR,
    pub hbrBackground: isize,
    pub lpszMenuName: *const u16,
    pub lpszClassName: *const u16,
    pub hIconSm: HICON,
}
#[repr(C)]
pub struct PAINTSTRUCT {
    pub hdc: HDC,
    pub fErase: i32,
    pub rcPaint: RECT,
    pub fRestore: i32,
    pub fIncUpdate: i32,
    pub rgbReserved: [u8; 32],
}
#[repr(C)]
pub struct BITMAP {
    pub bmType: i32,
    pub bmWidth: i32,
    pub bmHeight: i32,
    pub bmWidthBytes: i32,
    pub bmPlanes: u16,
    pub bmBitsPixel: u16,
    pub bmBits: *mut c_void,
}
#[repr(C)]
pub struct MONITORINFOEXW {
    pub cbSize: u32,
    pub rcMonitor: RECT,
    pub rcWork: RECT,
    pub dwFlags: u32,
    pub szDevice: [u16; 32],
}
#[repr(C)]
pub struct OPENFILENAMEW {
    pub lStructSize: u32,
    pub hwndOwner: HWND,
    pub hInstance: HINSTANCE,
    pub lpstrFilter: *const u16,
    pub lpstrCustomFilter: *mut u16,
    pub nMaxCustFilter: u32,
    pub nFilterIndex: u32,
    pub lpstrFile: *mut u16,
    pub nMaxFile: u32,
    pub lpstrFileTitle: *mut u16,
    pub nMaxFileTitle: u32,
    pub lpstrInitialDir: *const u16,
    pub lpstrTitle: *const u16,
    pub Flags: u32,
    pub nFileOffset: u16,
    pub nFileExtension: u16,
    pub lpstrDefExt: *const u16,
    pub lCustData: isize,
    pub lpfnHook: *mut c_void,
    pub lpTemplateName: *const u16,
    pub pvReserved: *mut c_void,
    pub dwReserved: u32,
    pub FlagsEx: u32,
}
#[repr(C)]
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct GUID {
    pub a: u32,
    pub b: u16,
    pub c: u16,
    pub d: [u8; 8],
}
#[repr(C)]
pub struct NOTIFYICONDATAW {
    pub cbSize: u32,
    pub hWnd: HWND,
    pub uID: u32,
    pub uFlags: u32,
    pub uCallbackMessage: u32,
    pub hIcon: HICON,
    pub szTip: [u16; 128],
    pub dwState: u32,
    pub dwStateMask: u32,
    pub szInfo: [u16; 256],
    pub uTimeoutOrVersion: u32,
    pub szInfoTitle: [u16; 64],
    pub dwInfoFlags: u32,
    pub guidItem: GUID,
    pub hBalloonIcon: HICON,
}
#[repr(C)]
pub struct COPYDATASTRUCT {
    pub dwData: usize,
    pub cbData: u32,
    pub lpData: *const c_void,
}
#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct SYSTEMTIME {
    pub wYear: u16,
    pub wMonth: u16,
    pub wDayOfWeek: u16,
    pub wDay: u16,
    pub wHour: u16,
    pub wMinute: u16,
    pub wSecond: u16,
    pub wMilliseconds: u16,
}
#[repr(C)]
#[derive(Default)]
pub struct SYSTEM_POWER_STATUS {
    pub ACLineStatus: u8,
    pub BatteryFlag: u8,
    pub BatteryLifePercent: u8,
    pub SystemStatusFlag: u8,
    pub BatteryLifeTime: u32,
    pub BatteryFullLifeTime: u32,
}
#[repr(C)]
pub struct POWERBROADCAST_SETTING {
    pub PowerSetting: GUID,
    pub DataLength: u32,
    pub Data: [u8; 1],
}
#[repr(C)]
#[derive(Default)]
pub struct PROCESS_MEMORY_COUNTERS {
    pub cb: u32,
    pub PageFaultCount: u32,
    pub PeakWorkingSetSize: usize,
    pub WorkingSetSize: usize,
    pub QuotaPeakPagedPoolUsage: usize,
    pub QuotaPagedPoolUsage: usize,
    pub QuotaPeakNonPagedPoolUsage: usize,
    pub QuotaNonPagedPoolUsage: usize,
    pub PagefileUsage: usize,
    pub PeakPagefileUsage: usize,
}
#[repr(C)]
#[derive(Default)]
pub struct PROCESS_MEMORY_COUNTERS_EX {
    pub cb: u32,
    pub PageFaultCount: u32,
    pub PeakWorkingSetSize: usize,
    pub WorkingSetSize: usize,
    pub QuotaPeakPagedPoolUsage: usize,
    pub QuotaPagedPoolUsage: usize,
    pub QuotaPeakNonPagedPoolUsage: usize,
    pub QuotaNonPagedPoolUsage: usize,
    pub PagefileUsage: usize,
    pub PeakPagefileUsage: usize,
    pub PrivateUsage: usize,
}
#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct BLENDFUNCTION {
    pub BlendOp: u8,
    pub BlendFlags: u8,
    pub SourceConstantAlpha: u8,
    pub AlphaFormat: u8,
}
#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct SIZE {
    pub cx: i32,
    pub cy: i32,
}
#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct BITMAPINFOHEADER {
    pub biSize: u32,
    pub biWidth: i32,
    pub biHeight: i32,
    pub biPlanes: u16,
    pub biBitCount: u16,
    pub biCompression: u32,
    pub biSizeImage: u32,
    pub biXPelsPerMeter: i32,
    pub biYPelsPerMeter: i32,
    pub biClrUsed: u32,
    pub biClrImportant: u32,
}
#[repr(C)]
pub struct BITMAPINFO {
    pub bmiHeader: BITMAPINFOHEADER,
    pub bmiColors: [u32; 1],
}
#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct WINDOWPOS {
    pub hwnd: HWND,
    pub hwndInsertAfter: HWND,
    pub x: i32,
    pub y: i32,
    pub cx: i32,
    pub cy: i32,
    pub flags: u32,
}
pub const WS_POPUP: u32 = 0x80000000;
pub const WS_OVERLAPPEDWINDOW: u32 = 0x00cf0000;
pub const WS_CHILD: u32 = 0x40000000;
pub const WS_VISIBLE: u32 = 0x10000000;
pub const WS_CLIPCHILDREN: u32 = 0x02000000;
pub const WS_CLIPSIBLINGS: u32 = 0x04000000;
pub const WS_TABSTOP: u32 = 0x00010000;
pub const WS_BORDER: u32 = 0x00800000;
pub const ES_NUMBER: u32 = 0x00002000;
pub const WS_VSCROLL: u32 = 0x00200000;
pub const WS_EX_TOOLWINDOW: u32 = 0x00000080;
pub const WS_EX_NOACTIVATE: u32 = 0x08000000;
pub const WS_EX_LAYERED: u32 = 0x00080000;
pub const WS_EX_TRANSPARENT: u32 = 0x00000020;
pub const ULW_ALPHA: u32 = 0x00000002;
pub const AC_SRC_OVER: u8 = 0x00;
pub const AC_SRC_ALPHA: u8 = 0x01;
pub const WM_NCHITTEST: u32 = 0x0084;
pub const HTTRANSPARENT: isize = -1;
pub const HTCLIENT: isize = 1;
pub const WM_MOUSEACTIVATE: u32 = 0x0021;
pub const MA_NOACTIVATE: isize = 3;
pub const WM_WINDOWPOSCHANGING: u32 = 0x0046;
pub const HWND_BOTTOM: HWND = 1;
pub const HWND_TOP: HWND = 0;
pub const HWND_TOPMOST: HWND = -1;
pub const HWND_NOTOPMOST: HWND = -2;
pub const WM_TIMECHANGE: u32 = 0x001E;
pub const DIB_RGB_COLORS: u32 = 0;
pub const BI_RGB: u32 = 0;
pub const APPCOMMAND_MEDIA_PLAY_PAUSE: u32 = 14;
pub const APPCOMMAND_MEDIA_STOP: u32 = 13;
pub const APPCOMMAND_MEDIA_NEXTTRACK: u32 = 11;
pub const APPCOMMAND_MEDIA_PREVIOUSTRACK: u32 = 12;
pub const WM_APPCOMMAND: u32 = 0x0319;
pub const VK_MEDIA_NEXT_TRACK: u8 = 0xB0;
pub const VK_MEDIA_PREV_TRACK: u8 = 0xB1;
pub const VK_MEDIA_STOP: u8 = 0xB2;
pub const VK_MEDIA_PLAY_PAUSE: u8 = 0xB3;
pub const KEYEVENTF_KEYUP: u32 = 0x0002;
pub const ANTIALIASED_QUALITY: u32 = 4;
pub const BS_AUTOCHECKBOX: u32 = 0x00000003;
pub const CBS_DROPDOWNLIST: u32 = 0x0003;
pub const LBS_NOTIFY: u32 = 1;
pub const LBS_NOINTEGRALHEIGHT: u32 = 0x0100;
pub const SW_HIDE: i32 = 0;
pub const SW_SHOW: i32 = 5;
pub const SW_RESTORE: i32 = 9;
pub const SWP_NOSIZE: u32 = 0x0001;
pub const SWP_NOMOVE: u32 = 0x0002;
pub const SWP_NOZORDER: u32 = 0x0004;
pub const SWP_NOACTIVATE: u32 = 0x0010;
pub const SWP_SHOWWINDOW: u32 = 0x0040;
pub const WM_CREATE: u32 = 1;
pub const WM_DESTROY: u32 = 2;
pub const WM_SIZE: u32 = 5;
pub const WM_PAINT: u32 = 15;
pub const WM_CLOSE: u32 = 16;
pub const WM_ERASEBKGND: u32 = 0x14;
pub const WM_SETCURSOR: u32 = 0x20;
pub const WM_GETMINMAXINFO: u32 = 0x24;
pub const WM_COPYDATA: u32 = 0x4a;
pub const WM_DISPLAYCHANGE: u32 = 0x7e;
pub const WM_KEYDOWN: u32 = 0x100;
pub const WM_KEYUP: u32 = 0x101;
pub const WM_CHAR: u32 = 0x102;
pub const WM_COMMAND: u32 = 0x111;
pub const WM_TIMER: u32 = 0x113;
pub const WM_MOUSEMOVE: u32 = 0x200;
pub const WM_LBUTTONDOWN: u32 = 0x201;
pub const WM_LBUTTONUP: u32 = 0x202;
pub const WM_LBUTTONDBLCLK: u32 = 0x203;
pub const WM_RBUTTONDOWN: u32 = 0x204;
pub const WM_RBUTTONUP: u32 = 0x205;
pub const WM_MOUSEWHEEL: u32 = 0x20a;
pub const WM_POWERBROADCAST: u32 = 0x218;
pub const WM_DROPFILES: u32 = 0x233;
pub const WM_MOUSELEAVE: u32 = 0x2a3;
pub const WM_WTSSESSION_CHANGE: u32 = 0x2b1;
pub const WM_APP: u32 = 0x8000;
pub const BM_SETCHECK: u32 = 0xf1;
pub const BM_GETCHECK: u32 = 0xf0;
pub const BST_CHECKED: usize = 1;
pub const CB_ADDSTRING: u32 = 0x143;
pub const CB_GETCURSEL: u32 = 0x147;
pub const CB_SETCURSEL: u32 = 0x14e;
pub const CB_RESETCONTENT: u32 = 0x14b;
pub const LB_ADDSTRING: u32 = 0x180;
pub const LB_GETCURSEL: u32 = 0x188;
pub const LB_SETCURSEL: u32 = 0x186;
pub const LB_RESETCONTENT: u32 = 0x184;
pub const LBN_DBLCLK: usize = 2;
pub const CBN_SELCHANGE: usize = 1;
pub const MOUSE_RBUTTONUP: usize = 0x205;
pub const MOUSE_LBUTTONDBLCLK: usize = 0x203;
pub const MF_STRING: u32 = 0;
pub const MF_SEPARATOR: u32 = 0x0800;
pub const MF_POPUP: u32 = 0x0010;
pub const TPM_RIGHTBUTTON: u32 = 2;
pub const NIF_MESSAGE: u32 = 1;
pub const NIF_ICON: u32 = 2;
pub const NIF_TIP: u32 = 4;
pub const NIM_ADD: u32 = 0;
pub const NIM_DELETE: u32 = 2;
pub const IMAGE_BITMAP: u32 = 0;
pub const LR_LOADFROMFILE: u32 = 0x10;
pub const LR_CREATEDIBSECTION: u32 = 0x2000;
pub const SRCCOPY: u32 = 0x00cc0020;
pub const HALFTONE: i32 = 4;
pub const OFN_FILEMUSTEXIST: u32 = 0x00001000;
pub const OFN_PATHMUSTEXIST: u32 = 0x00000800;
pub const OFN_EXPLORER: u32 = 0x00080000;
pub const OFN_NOCHANGEDIR: u32 = 0x00000008;
pub const PBT_APMSUSPEND: usize = 4;
pub const PBT_APMRESUMEAUTOMATIC: usize = 18;
pub const PBT_POWERSETTINGCHANGE: usize = 0x8013;
pub const WTS_SESSION_LOCK: usize = 7;
pub const WTS_SESSION_UNLOCK: usize = 8;
pub const EVENT_SYSTEM_FOREGROUND: u32 = 3;
pub const WINEVENT_OUTOFCONTEXT: u32 = 0;
pub const WINEVENT_SKIPOWNPROCESS: u32 = 2;
pub const ERROR_ALREADY_EXISTS: u32 = 183;
pub const REG_SZ: u32 = 1;
pub const KEY_SET_VALUE: u32 = 2;
pub const KEY_QUERY_VALUE: u32 = 1;
pub const HKEY_CURRENT_USER: HKEY = 0x80000001u32 as i32 as isize;
pub const SW_SHOWNORMAL: i32 = 1;
pub const SM_REMOTESESSION: i32 = 0x1000;
pub const GUID_CONSOLE_DISPLAY_STATE: GUID = GUID {
    a: 0x6fe69556,
    b: 0x704a,
    c: 0x47a0,
    d: [0x8f, 0x24, 0xc2, 0x8d, 0x93, 0x6f, 0xda, 0x47],
};

pub const DT_TOP: u32 = 0x00000000;
pub const DT_LEFT: u32 = 0x00000000;
pub const DT_CENTER: u32 = 0x00000001;
pub const DT_RIGHT: u32 = 0x00000002;
pub const DT_VCENTER: u32 = 0x00000004;
pub const DT_BOTTOM: u32 = 0x00000008;
pub const DT_WORDBREAK: u32 = 0x00000010;
pub const DT_SINGLELINE: u32 = 0x00000020;
pub const DT_NOPREFIX: u32 = 0x00000800;
pub const DT_END_ELLIPSIS: u32 = 0x00008000;

pub const PS_SOLID: i32 = 0;
pub const PS_NULL: i32 = 5;
pub const TRANSPARENT: i32 = 1;
pub const OPAQUE: i32 = 2;

pub const TME_LEAVE: u32 = 0x00000002;
pub const DWMWA_USE_IMMERSIVE_DARK_MODE: u32 = 20;

pub const IDC_ARROW: usize = 32512;
pub const IDC_IBEAM: usize = 32513;
pub const IDC_HAND: usize = 32649;
pub const IDC_SIZEALL: usize = 32646;

#[repr(C)]
pub struct TRACKMOUSEEVENT {
    pub cbSize: u32,
    pub dwFlags: u32,
    pub hwndTrack: HWND,
    pub dwHoverTime: u32,
}

#[repr(C)]
pub struct MINMAXINFO {
    pub ptReserved: POINT,
    pub ptMaxSize: POINT,
    pub ptMaxPosition: POINT,
    pub ptMinTrackSize: POINT,
    pub ptMaxTrackSize: POINT,
}

pub const fn rgb(r: u8, g: u8, b: u8) -> u32 {
    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16)
}

pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
pub fn wchar_str(ptr: &[u16]) -> String {
    let end = ptr.iter().position(|&x| x == 0).unwrap_or(ptr.len());
    String::from_utf16_lossy(&ptr[..end])
}
pub fn zeroed<T>() -> T {
    unsafe { std::mem::zeroed() }
}
pub fn low(value: usize) -> u16 {
    value as u16
}
pub fn high(value: usize) -> u16 {
    (value >> 16) as u16
}
#[link(name = "user32")]
extern "system" {
    pub fn MessageBoxW(owner: HWND, text: *const u16, caption: *const u16, flags: u32) -> i32;
    pub fn FindWindowW(cls: *const u16, name: *const u16) -> HWND;
    pub fn FindWindowExW(parent: HWND, after: HWND, cls: *const u16, name: *const u16) -> HWND;
    pub fn OpenDesktopW(
        lpszDesktop: *const u16,
        dwFlags: u32,
        fInherit: i32,
        dwDesiredAccess: u32,
    ) -> HDESK;
    pub fn CloseDesktop(hDesktop: HDESK) -> i32;
    pub fn SetThreadDesktop(hDesktop: HDESK) -> i32;
    pub fn SendMessageTimeoutW(
        hwnd: HWND,
        msg: u32,
        w: usize,
        l: isize,
        flags: u32,
        timeout: u32,
        result: *mut usize,
    ) -> isize;
    pub fn SendMessageW(hwnd: HWND, msg: u32, w: usize, l: isize) -> isize;
    pub fn PostMessageW(hwnd: HWND, msg: u32, w: usize, l: isize) -> i32;
    pub fn EnumWindows(
        cb: Option<unsafe extern "system" fn(HWND, isize) -> i32>,
        data: isize,
    ) -> i32;
    pub fn EnumDisplayMonitors(
        dc: HDC,
        rect: *const RECT,
        cb: Option<unsafe extern "system" fn(HMONITOR, HDC, *mut RECT, isize) -> i32>,
        data: isize,
    ) -> i32;
    pub fn GetMonitorInfoW(monitor: HMONITOR, info: *mut MONITORINFOEXW) -> i32;
    pub fn GetWindowRect(hwnd: HWND, rect: *mut RECT) -> i32;
    pub fn GetClientRect(hwnd: HWND, rect: *mut RECT) -> i32;
    pub fn GetParent(hwnd: HWND) -> HWND;
    pub fn SetParent(hwnd: HWND, new_parent: HWND) -> HWND;
    pub fn IsWindow(hwnd: HWND) -> i32;
    pub fn IsWindowVisible(hwnd: HWND) -> i32;
    pub fn IsIconic(hwnd: HWND) -> i32;
    pub fn GetForegroundWindow() -> HWND;
    pub fn GetClassNameW(hwnd: HWND, name: *mut u16, max: i32) -> i32;
    pub fn GetWindowThreadProcessId(hwnd: HWND, id: *mut u32) -> u32;
    pub fn GetSystemMetrics(index: i32) -> i32;
    pub fn GetCursorPos(point: *mut POINT) -> i32;
    pub fn SetForegroundWindow(hwnd: HWND) -> i32;
    pub fn CreateWindowExW(
        ex: u32,
        class: *const u16,
        name: *const u16,
        style: u32,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        parent: HWND,
        menu: HMENU,
        instance: HINSTANCE,
        param: *mut c_void,
    ) -> HWND;
    pub fn RegisterClassExW(cls: *const WNDCLASSEXW) -> u16;
    pub fn DefWindowProcW(hwnd: HWND, msg: u32, w: usize, l: isize) -> isize;
    pub fn DestroyWindow(hwnd: HWND) -> i32;
    pub fn ShowWindow(hwnd: HWND, command: i32) -> i32;
    pub fn SetWindowPos(
        hwnd: HWND,
        after: HWND,
        x: i32,
        y: i32,
        cx: i32,
        cy: i32,
        flags: u32,
    ) -> i32;
    pub fn SetWindowTextW(hwnd: HWND, text: *const u16) -> i32;
    pub fn GetWindowTextW(hwnd: HWND, text: *mut u16, capacity: i32) -> i32;
    pub fn GetMessageW(msg: *mut MSG, hwnd: HWND, min: u32, max: u32) -> i32;
    pub fn TranslateMessage(msg: *const MSG) -> i32;
    pub fn DispatchMessageW(msg: *const MSG) -> isize;
    pub fn PostQuitMessage(code: i32);
    pub fn RegisterWindowMessageW(name: *const u16) -> u32;
    pub fn LoadIconW(instance: HINSTANCE, name: *const u16) -> HICON;
    pub fn LoadCursorW(instance: HINSTANCE, name: *const u16) -> HCURSOR;
    pub fn SetTimer(hwnd: HWND, id: usize, elapse: u32, proc: *const c_void) -> usize;
    pub fn KillTimer(hwnd: HWND, id: usize) -> i32;
    pub fn BeginPaint(hwnd: HWND, ps: *mut PAINTSTRUCT) -> HDC;
    pub fn EndPaint(hwnd: HWND, ps: *const PAINTSTRUCT) -> i32;
    pub fn InvalidateRect(hwnd: HWND, rect: *const RECT, erase: i32) -> i32;
    pub fn LoadImageW(
        instance: HINSTANCE,
        name: *const u16,
        kind: u32,
        cx: i32,
        cy: i32,
        flags: u32,
    ) -> isize;
    pub fn SetWinEventHook(
        min: u32,
        max: u32,
        module: HINSTANCE,
        callback: Option<unsafe extern "system" fn(HHOOK, u32, HWND, i32, i32, u32, u32)>,
        pid: u32,
        tid: u32,
        flags: u32,
    ) -> HHOOK;
    pub fn UnhookWinEvent(hook: HHOOK) -> i32;
    pub fn RegisterPowerSettingNotification(
        hwnd: HWND,
        guid: *const GUID,
        flags: u32,
    ) -> HPOWERNOTIFY;
    pub fn UnregisterPowerSettingNotification(handle: HPOWERNOTIFY) -> i32;
    pub fn CreatePopupMenu() -> HMENU;
    pub fn AppendMenuW(menu: HMENU, flags: u32, id: usize, text: *const u16) -> i32;
    pub fn TrackPopupMenu(
        menu: HMENU,
        flags: u32,
        x: i32,
        y: i32,
        reserved: i32,
        hwnd: HWND,
        rect: *const RECT,
    ) -> i32;
    pub fn DestroyMenu(menu: HMENU) -> i32;
    pub fn GetDC(hwnd: HWND) -> HDC;
    pub fn ReleaseDC(hwnd: HWND, hdc: HDC) -> i32;
    pub fn SetCursor(cursor: HCURSOR) -> HCURSOR;
    pub fn TrackMouseEvent(event: *mut TRACKMOUSEEVENT) -> i32;
    pub fn FillRect(hdc: HDC, rect: *const RECT, brush: HGDIOBJ) -> i32;
    pub fn FrameRect(hdc: HDC, rect: *const RECT, brush: HGDIOBJ) -> i32;
    pub fn DrawTextW(hdc: HDC, text: *const u16, len: i32, rect: *mut RECT, format: u32) -> i32;
    pub fn UpdateLayeredWindow(
        hwnd: HWND,
        hdcDst: HDC,
        pptDst: *const POINT,
        psize: *const SIZE,
        hdcSrc: HDC,
        pptSrc: *const POINT,
        crKey: u32,
        pblend: *const BLENDFUNCTION,
        dwFlags: u32,
    ) -> i32;
    pub fn keybd_event(bVk: u8, bScan: u8, dwFlags: u32, dwExtraInfo: usize);
    pub fn GetDpiForWindow(hwnd: HWND) -> u32;
}
#[link(name = "kernel32")]
extern "system" {
    pub fn GetModuleHandleW(name: *const u16) -> HINSTANCE;
    pub fn GetLastError() -> u32;
    pub fn CreateMutexW(attr: *mut c_void, initial: i32, name: *const u16) -> HANDLE;
    pub fn CloseHandle(handle: HANDLE) -> i32;
    pub fn GetSystemPowerStatus(status: *mut SYSTEM_POWER_STATUS) -> i32;
    pub fn GetCurrentProcess() -> HANDLE;
    pub fn GetProcessTimes(
        h: HANDLE,
        create: *mut FILETIME,
        exit: *mut FILETIME,
        kernel: *mut FILETIME,
        user: *mut FILETIME,
    ) -> i32;
    pub fn GetLocalTime(time: *mut SYSTEMTIME);
}
#[link(name = "gdi32")]
extern "system" {
    pub fn CreateCompatibleDC(dc: HDC) -> HDC;
    pub fn DeleteDC(dc: HDC) -> i32;
    pub fn SelectObject(dc: HDC, obj: HGDIOBJ) -> HGDIOBJ;
    pub fn DeleteObject(obj: HGDIOBJ) -> i32;
    pub fn GetObjectW(obj: HGDIOBJ, cb: i32, ptr: *mut c_void) -> i32;
    pub fn StretchBlt(
        dest: HDC,
        dx: i32,
        dy: i32,
        dw: i32,
        dh: i32,
        src: HDC,
        sx: i32,
        sy: i32,
        sw: i32,
        sh: i32,
        rop: u32,
    ) -> i32;
    pub fn SetStretchBltMode(dc: HDC, mode: i32) -> i32;
    pub fn CreateSolidBrush(color: u32) -> HGDIOBJ;
    pub fn CreatePen(style: i32, width: i32, color: u32) -> HGDIOBJ;
    pub fn RoundRect(
        hdc: HDC,
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
        width: i32,
        height: i32,
    ) -> i32;
    pub fn Rectangle(hdc: HDC, left: i32, top: i32, right: i32, bottom: i32) -> i32;
    pub fn Ellipse(hdc: HDC, left: i32, top: i32, right: i32, bottom: i32) -> i32;
    pub fn Polyline(hdc: HDC, points: *const POINT, count: i32) -> i32;
    pub fn SetTextColor(hdc: HDC, color: u32) -> u32;
    pub fn SetBkMode(hdc: HDC, mode: i32) -> i32;
    pub fn SetBkColor(hdc: HDC, color: u32) -> u32;
    pub fn CreateFontW(
        height: i32,
        width: i32,
        escapement: i32,
        orientation: i32,
        weight: i32,
        italic: u32,
        underline: u32,
        strike_out: u32,
        char_set: u32,
        out_precision: u32,
        clip_precision: u32,
        quality: u32,
        pitch_and_family: u32,
        face_name: *const u16,
    ) -> HGDIOBJ;
    pub fn CreateCompatibleBitmap(hdc: HDC, width: i32, height: i32) -> HBITMAP;
    pub fn CreateDIBSection(
        hdc: HDC,
        pbmi: *const BITMAPINFO,
        usage: u32,
        ppvBits: *mut *mut c_void,
        hSection: HANDLE,
        offset: u32,
    ) -> HBITMAP;
    pub fn BitBlt(
        dest: HDC,
        dx: i32,
        dy: i32,
        dw: i32,
        dh: i32,
        src: HDC,
        sx: i32,
        sy: i32,
        rop: u32,
    ) -> i32;
}
#[link(name = "shell32")]
extern "system" {
    pub fn Shell_NotifyIconW(msg: u32, icon: *mut NOTIFYICONDATAW) -> i32;
    pub fn ShellExecuteW(
        parent: HWND,
        operation: *const u16,
        file: *const u16,
        params: *const u16,
        dir: *const u16,
        show: i32,
    ) -> isize;
    pub fn DragAcceptFiles(hwnd: HWND, accept: i32);
    pub fn DragQueryFileW(hdrop: isize, file_idx: u32, file: *mut u16, max_chars: u32) -> u32;
    pub fn DragFinish(hdrop: isize);
}
#[link(name = "dwmapi")]
extern "system" {
    pub fn DwmSetWindowAttribute(hwnd: HWND, attr: u32, value: *const c_void, size: u32) -> i32;
}
#[link(name = "comdlg32")]
extern "system" {
    pub fn GetOpenFileNameW(ofn: *mut OPENFILENAMEW) -> i32;
}
#[link(name = "wtsapi32")]
extern "system" {
    pub fn WTSRegisterSessionNotification(hwnd: HWND, flags: u32) -> i32;
    pub fn WTSUnRegisterSessionNotification(hwnd: HWND) -> i32;
}
#[link(name = "advapi32")]
extern "system" {
    pub fn RegCreateKeyExW(
        key: HKEY,
        sub: *const u16,
        reserved: u32,
        class: *mut u16,
        options: u32,
        sam: u32,
        security: *mut c_void,
        result: *mut HKEY,
        disposition: *mut u32,
    ) -> u32;
    pub fn RegOpenKeyExW(
        key: HKEY,
        sub: *const u16,
        options: u32,
        sam: u32,
        result: *mut HKEY,
    ) -> u32;
    pub fn RegQueryValueExW(
        key: HKEY,
        name: *const u16,
        reserved: *mut u32,
        kind: *mut u32,
        data: *mut u8,
        size: *mut u32,
    ) -> u32;
    pub fn RegSetValueExW(
        key: HKEY,
        name: *const u16,
        reserved: u32,
        kind: u32,
        data: *const u8,
        size: u32,
    ) -> u32;
    pub fn RegDeleteValueW(key: HKEY, name: *const u16) -> u32;
    pub fn RegCloseKey(key: HKEY) -> u32;
    pub fn GetUserNameW(buffer: *mut u16, size: *mut u32) -> i32;
}

pub fn current_windows_username() -> Option<String> {
    let mut buf = [0u16; 256];
    let mut size = buf.len() as u32;
    let ok = unsafe { GetUserNameW(buf.as_mut_ptr(), &mut size) };
    if ok != 0 && size > 1 {
        let len = (size as usize - 1).min(buf.len());
        Some(String::from_utf16_lossy(&buf[..len]))
    } else {
        None
    }
}
#[link(name = "ole32")]
extern "system" {
    pub fn CoInitializeEx(reserved: *mut c_void, mode: u32) -> HRESULT;
    pub fn CoUninitialize();
}
#[link(name = "psapi")]
extern "system" {
    pub fn GetProcessMemoryInfo(
        process: HANDLE,
        counters: *mut PROCESS_MEMORY_COUNTERS,
        size: u32,
    ) -> i32;
}

#[link(name = "shcore")]
extern "system" {
    pub fn SetProcessDpiAwareness(level: i32) -> HRESULT;
}
