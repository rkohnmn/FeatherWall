//! Lightweight native Windows media detection and playback control.
//! Integrates directly with Spotify Desktop and general Windows media sessions
//! without web API polling, Electron, or browser dependencies.
use super::ffi as f;
use std::ptr;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MediaState {
    pub is_active: bool,
    pub is_playing: bool,
    pub is_spotify: bool,
    pub title: String,
    pub artist: String,
    pub app_name: String,
    pub target_hwnd: f::HWND,
}

impl Default for MediaState {
    fn default() -> Self {
        Self {
            is_active: false,
            is_playing: false,
            is_spotify: false,
            title: "No Media Playing".into(),
            artist: "Spotify / Media Offline".into(),
            app_name: "Spotify".into(),
            target_hwnd: 0,
        }
    }
}

struct EnumMediaContext {
    spotify_preferred: bool,
    general_media: bool,
    found_state: Option<MediaState>,
}

unsafe extern "system" fn enum_media_proc(hwnd: f::HWND, data: isize) -> i32 {
    let ctx = &mut *(data as *mut EnumMediaContext);

    if f::IsWindowVisible(hwnd) == 0 {
        return 1;
    }

    let mut title_buf = [0u16; 512];
    let title_len = f::GetWindowTextW(hwnd, title_buf.as_mut_ptr(), title_buf.len() as i32);
    if title_len <= 0 {
        return 1;
    }
    let title_str = String::from_utf16_lossy(&title_buf[..title_len as usize]);

    let mut class_buf = [0u16; 128];
    let class_len = f::GetClassNameW(hwnd, class_buf.as_mut_ptr(), class_buf.len() as i32);
    let class_str = if class_len > 0 {
        String::from_utf16_lossy(&class_buf[..class_len as usize])
    } else {
        String::new()
    };

    // 1. Spotify Detection
    // Spotify's main window uses "Chrome_WidgetWin_0" class.
    // When playing, the title is "Artist - Track" or "Artist – Track".
    // When paused/stopped/idle, title is "Spotify Free", "Spotify Premium", or "Spotify".
    if class_str == "Chrome_WidgetWin_0" || title_str.starts_with("Spotify") {
        if title_str.contains(" - ") || title_str.contains(" – ") || title_str.contains(" — ") {
            let (artist, track) = parse_artist_track(&title_str);
            ctx.found_state = Some(MediaState {
                is_active: true,
                is_playing: true,
                is_spotify: true,
                title: track,
                artist,
                app_name: "Spotify".into(),
                target_hwnd: hwnd,
            });
            // Found active Spotify track!
            return 0;
        } else if title_str.starts_with("Spotify") {
            // Spotify is running but idle/paused
            if ctx.found_state.is_none() {
                ctx.found_state = Some(MediaState {
                    is_active: false,
                    is_playing: false,
                    is_spotify: true,
                    title: "Spotify Ready".into(),
                    artist: "Playback Paused".into(),
                    app_name: "Spotify".into(),
                    target_hwnd: hwnd,
                });
            }
        }
    }

    // 2. General Media Detection (if enabled and no Spotify track found yet)
    if ctx.general_media && ctx.found_state.as_ref().map(|s| !s.is_active).unwrap_or(true) {
        // Check for Foobar2000, AIMP, VLC, MusicBee, etc.
        if class_str.contains("foobar2000")
            || class_str.contains("AIMP")
            || class_str.contains("VLC")
            || class_str.contains("MusicBee")
        {
            if title_str.contains(" - ") || title_str.contains(" – ") {
                let (artist, track) = parse_artist_track(&title_str);
                ctx.found_state = Some(MediaState {
                    is_active: true,
                    is_playing: true,
                    is_spotify: false,
                    title: track,
                    artist,
                    app_name: "Media Player".into(),
                    target_hwnd: hwnd,
                });
                if !ctx.spotify_preferred {
                    return 0;
                }
            }
        }
    }

    1
}

fn parse_artist_track(raw: &str) -> (String, String) {
    let raw = raw.trim();
    let separators = [" - ", " – ", " — "];
    for sep in separators {
        if let Some(pos) = raw.find(sep) {
            let artist = raw[..pos].trim().to_string();
            let track = raw[pos + sep.len()..].trim().to_string();
            return (artist, track);
        }
    }
    ("Unknown Artist".into(), raw.to_string())
}

pub fn query_media_session(spotify_preferred: bool, general_media: bool) -> MediaState {
    let mut ctx = EnumMediaContext {
        spotify_preferred,
        general_media,
        found_state: None,
    };

    unsafe {
        f::EnumWindows(
            Some(enum_media_proc),
            (&mut ctx as *mut EnumMediaContext) as isize,
        );
    }

    ctx.found_state.unwrap_or_default()
}

pub fn play_pause(state: &MediaState) {
    if state.target_hwnd != 0 && unsafe { f::IsWindow(state.target_hwnd) } != 0 {
        let cmd = (f::APPCOMMAND_MEDIA_PLAY_PAUSE << 16) as isize;
        unsafe {
            let _ = f::SendMessageTimeoutW(
                state.target_hwnd,
                f::WM_APPCOMMAND,
                0,
                cmd,
                2, // SMTO_ABORTIFHUNG
                500,
                ptr::null_mut(),
            );
        }
    } else {
        // Universal fallback: Windows hardware media key
        send_media_key(f::VK_MEDIA_PLAY_PAUSE);
    }
}

pub fn next_track(state: &MediaState) {
    if state.target_hwnd != 0 && unsafe { f::IsWindow(state.target_hwnd) } != 0 {
        let cmd = (f::APPCOMMAND_MEDIA_NEXTTRACK << 16) as isize;
        unsafe {
            let _ = f::SendMessageTimeoutW(
                state.target_hwnd,
                f::WM_APPCOMMAND,
                0,
                cmd,
                2,
                500,
                ptr::null_mut(),
            );
        }
    } else {
        send_media_key(f::VK_MEDIA_NEXT_TRACK);
    }
}

pub fn prev_track(state: &MediaState) {
    if state.target_hwnd != 0 && unsafe { f::IsWindow(state.target_hwnd) } != 0 {
        let cmd = (f::APPCOMMAND_MEDIA_PREVIOUSTRACK << 16) as isize;
        unsafe {
            let _ = f::SendMessageTimeoutW(
                state.target_hwnd,
                f::WM_APPCOMMAND,
                0,
                cmd,
                2,
                500,
                ptr::null_mut(),
            );
        }
    } else {
        send_media_key(f::VK_MEDIA_PREV_TRACK);
    }
}

fn send_media_key(vk: u8) {
    unsafe {
        f::keybd_event(vk, 0, 0, 0);
        f::keybd_event(vk, 0, f::KEYEVENTF_KEYUP, 0);
    }
}

pub fn open_spotify() {
    let uri = f::wide("spotify:");
    unsafe {
        f::ShellExecuteW(0, f::wide("open").as_ptr(), uri.as_ptr(), ptr::null(), ptr::null(), 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_media() {
        let s = query_media_session(true, true);
        println!("Media state: {:?}", s);
    }
}
