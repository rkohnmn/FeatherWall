//! Windows desktop host discovery, display enumeration, and efficient still-image presentation.
use super::ffi as f;
use crate::core::Rect;
use std::ffi::c_void;
use std::path::Path;
use std::ptr;

#[derive(Clone, Debug)]
pub struct Monitor {
    pub device: String,
    pub rect: Rect,
    pub primary: bool,
}
unsafe extern "system" fn enum_monitor(
    monitor: f::HMONITOR,
    _dc: f::HDC,
    _rect: *mut f::RECT,
    data: isize,
) -> i32 {
    let output = &mut *(data as *mut Vec<Monitor>);
    let mut info: f::MONITORINFOEXW = f::zeroed();
    info.cbSize = std::mem::size_of::<f::MONITORINFOEXW>() as u32;
    if f::GetMonitorInfoW(monitor, &mut info) == 0 {
        return 1;
    }
    output.push(Monitor {
        device: f::wchar_str(&info.szDevice),
        rect: Rect {
            left: info.rcMonitor.left,
            top: info.rcMonitor.top,
            right: info.rcMonitor.right,
            bottom: info.rcMonitor.bottom,
        },
        primary: (info.dwFlags & 1) != 0,
    });
    1
}
pub fn monitors() -> Vec<Monitor> {
    let mut result = Vec::new();
    unsafe {
        f::EnumDisplayMonitors(
            0,
            ptr::null(),
            Some(enum_monitor),
            (&mut result as *mut Vec<Monitor>) as isize,
        );
    }
    result.sort_by_key(|m: &Monitor| (!m.primary, m.device.clone()));
    result
}
unsafe extern "system" fn enum_host(hwnd: f::HWND, data: isize) -> i32 {
    let cls = f::wide("SHELLDLL_DefView");
    let defview = f::FindWindowExW(hwnd, 0, cls.as_ptr(), ptr::null());
    if defview != 0 {
        let worker = f::wide("WorkerW");
        let behind = f::FindWindowExW(0, hwnd, worker.as_ptr(), ptr::null());
        if behind != 0 && behind != hwnd {
            *(data as *mut f::HWND) = behind;
            return 0;
        }
    }
    1
}
pub fn desktop_host() -> Result<f::HWND, String> {
    let mut progman = unsafe { f::FindWindowW(f::wide("Progman").as_ptr(), ptr::null()) };
    if progman == 0 {
        let desk = unsafe { f::OpenDesktopW(f::wide("default").as_ptr(), 0, 0, 0x01FF) };
        if desk != 0 {
            unsafe {
                let _ = f::SetThreadDesktop(desk);
            }
            progman = unsafe { f::FindWindowW(f::wide("Progman").as_ptr(), ptr::null()) };
        }
    }
    if progman == 0 {
        return Err("Explorer's Progman window was not found".into());
    }
    // Undocumented Explorer message used by WorkerW wallpaper hosts. SMTO_ABORTIFHUNG=2
    // limits the worst-case UI stall if Explorer is unresponsive.
    let mut outcome = 0usize;
    unsafe {
        let _ = f::SendMessageTimeoutW(progman, 0x052c, 0, 0, 2, 1000, &mut outcome);
    }
    let mut result = 0;
    unsafe {
        f::EnumWindows(Some(enum_host), (&mut result as *mut f::HWND) as isize);
    }
    if result == 0 {
        return Err(
            "Explorer did not expose a usable WorkerW host (unsupported shell layout)".into(),
        );
    }
    Ok(result)
}
pub fn child_rect(monitor: &Monitor, host: f::HWND) -> Result<Rect, String> {
    let mut parent: f::RECT = f::zeroed();
    if unsafe { f::GetWindowRect(host, &mut parent) } == 0 {
        return Err("Cannot obtain WorkerW bounds".into());
    }
    Ok(Rect {
        left: monitor.rect.left - parent.left,
        top: monitor.rect.top - parent.top,
        right: monitor.rect.right - parent.left,
        bottom: monitor.rect.bottom - parent.top,
    })
}
pub fn covers_window(window: f::HWND, rect: &Rect, require_exact: bool) -> bool {
    if window == 0 {
        return false;
    }
    unsafe {
        if f::IsWindowVisible(window) == 0 || f::IsIconic(window) != 0 {
            return false;
        }
        let mut class_buf = [0u16; 64];
        let len = f::GetClassNameW(window, class_buf.as_mut_ptr(), class_buf.len() as i32);
        if len > 0 {
            let class_name = String::from_utf16_lossy(&class_buf[..len as usize]);
            if matches!(
                class_name.as_str(),
                "Progman"
                    | "WorkerW"
                    | "SHELLDLL_DefView"
                    | "Shell_TrayWnd"
                    | "Shell_SecondaryTrayWnd"
                    | "FeatherWallControlWindow"
                    | "FeatherWallSurfaceWindow"
            ) {
                return false;
            }
        }
        let mut bounds: f::RECT = f::zeroed();
        if f::GetWindowRect(window, &mut bounds) == 0 {
            return false;
        }
        let current = Rect {
            left: bounds.left,
            top: bounds.top,
            right: bounds.right,
            bottom: bounds.bottom,
        };
        let tol = if require_exact { 4 } else { 32 };
        current.covers(rect, tol)
    }
}
/// Static images are imported as BMP, then loaded as a DIB bitmap exactly once.
pub fn load_still(path: &Path) -> Result<f::HBITMAP, String> {
    use std::os::windows::ffi::OsStrExt;
    let path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let bitmap = unsafe {
        f::LoadImageW(
            0,
            path.as_ptr(),
            f::IMAGE_BITMAP,
            0,
            0,
            f::LR_LOADFROMFILE | f::LR_CREATEDIBSECTION,
        )
    };
    if bitmap == 0 {
        Err(format!(
            "Unable to load still wallpaper (GDI error {})",
            unsafe { f::GetLastError() }
        ))
    } else {
        Ok(bitmap)
    }
}
pub fn paint_bitmap(hwnd: f::HWND, bitmap: f::HBITMAP) {
    unsafe {
        let mut ps: f::PAINTSTRUCT = f::zeroed();
        let dest = f::BeginPaint(hwnd, &mut ps);
        if dest != 0 && bitmap != 0 {
            let src = f::CreateCompatibleDC(dest);
            if src != 0 {
                let previous = f::SelectObject(src, bitmap);
                let mut bmp: f::BITMAP = f::zeroed();
                let ok = f::GetObjectW(
                    bitmap,
                    std::mem::size_of::<f::BITMAP>() as i32,
                    (&mut bmp as *mut f::BITMAP).cast::<c_void>(),
                );
                let mut box_rect: f::RECT = f::zeroed();
                if ok != 0 && f::GetClientRect(hwnd, &mut box_rect) != 0 {
                    f::SetStretchBltMode(dest, f::HALFTONE);
                    let _ = f::StretchBlt(
                        dest,
                        0,
                        0,
                        box_rect.right - box_rect.left,
                        box_rect.bottom - box_rect.top,
                        src,
                        0,
                        0,
                        bmp.bmWidth,
                        bmp.bmHeight,
                        f::SRCCOPY,
                    );
                }
                f::SelectObject(src, previous);
                f::DeleteDC(src);
            }
        }
        f::EndPaint(hwnd, &ps);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monitors_enumeration() {
        let mons = monitors();
        println!("Monitors found: {:?}", mons);
        assert!(!mons.is_empty(), "Should detect at least one monitor");
    }

    #[test]
    fn test_desktop_host_discovery() {
        let host = desktop_host();
        println!("Desktop host: {:?}", host);
        if let Ok(hwnd) = host {
            assert_ne!(hwnd, 0);
        }
    }
}
