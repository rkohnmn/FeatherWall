//! Native MFPlay/EVR playback. It uses the Windows system media pipeline, not an FFmpeg process.
//! The exact video decoder and GPU copy path are selected internally by Windows; hardware acceleration
//! is possible but NOT guaranteed and cannot be truthfully measured through the MFPlay API.
use super::ffi::{self as f, GUID, HWND};
use std::ffi::c_void;
use std::path::Path;
use std::ptr;
use std::sync::atomic::{AtomicU32, Ordering};

pub const WM_PLAY_EVENT: u32 = f::WM_APP + 4;
const IID_IUNKNOWN: GUID = GUID {
    a: 0x00000000,
    b: 0x0000,
    c: 0x0000,
    d: [0xc0, 0, 0, 0, 0, 0, 0, 0x46],
};
const IID_CALLBACK: GUID = GUID {
    a: 0x766c8ffb,
    b: 0x5fdb,
    c: 0x4fea,
    d: [0xa2, 0x8d, 0xb9, 0x12, 0x99, 0x6f, 0x51, 0xbd],
};

#[repr(C)]
struct Header {
    kind: i32,
    hr: i32,
    player: *mut c_void,
    state: i32,
    properties: *mut c_void,
}
#[repr(C)]
struct Callback {
    vtable: *const CallbackVTable,
    refs: AtomicU32,
    window: HWND,
}
#[repr(C)]
struct CallbackVTable {
    query: unsafe extern "system" fn(*mut Callback, *const GUID, *mut *mut c_void) -> i32,
    add: unsafe extern "system" fn(*mut Callback) -> u32,
    release: unsafe extern "system" fn(*mut Callback) -> u32,
    event: unsafe extern "system" fn(*mut Callback, *const Header),
}
unsafe extern "system" fn query(
    this: *mut Callback,
    id: *const GUID,
    result: *mut *mut c_void,
) -> i32 {
    if this.is_null() || id.is_null() || result.is_null() {
        return 0x80004003u32 as i32;
    }
    *result = ptr::null_mut();
    if *id == IID_IUNKNOWN || *id == IID_CALLBACK {
        *result = this.cast();
        add(this);
        0
    } else {
        0x80004002u32 as i32
    }
}
unsafe extern "system" fn add(this: *mut Callback) -> u32 {
    (*this).refs.fetch_add(1, Ordering::Relaxed) + 1
}
unsafe extern "system" fn release(this: *mut Callback) -> u32 {
    let remaining = (*this).refs.fetch_sub(1, Ordering::AcqRel) - 1;
    if remaining == 0 {
        drop(Box::from_raw(this));
    }
    remaining
}
unsafe extern "system" fn on_event(this: *mut Callback, header: *const Header) {
    if this.is_null() || header.is_null() {
        return;
    }
    let evt = &*header;
    // Native COM may call this on an internal media thread. No GUI or player access here.
    let _ = f::PostMessageW(
        (*this).window,
        WM_PLAY_EVENT,
        evt.kind as usize,
        evt.hr as isize,
    );
}
static CALLBACK_VTABLE: CallbackVTable = CallbackVTable {
    query,
    add,
    release,
    event: on_event,
};
#[link(name = "mfplay")]
extern "system" {
    fn MFPCreateMediaPlayer(
        url: *const u16,
        start: i32,
        options: u32,
        callback: *mut Callback,
        hwnd: HWND,
        player: *mut *mut c_void,
    ) -> i32;
}
/// Methods selected from the documented IMFPMediaPlayer vtable (IUnknown is slots 0..2).
/// The interface pointer is kept only on the Win32 message thread.
unsafe fn call0(obj: *mut c_void, slot: usize) -> i32 {
    let methods = *(obj as *const *const usize);
    let func: unsafe extern "system" fn(*mut c_void) -> i32 =
        std::mem::transmute(*methods.add(slot));
    func(obj)
}
unsafe fn call_setmute(obj: *mut c_void, mute: i32) -> i32 {
    let methods = *(obj as *const *const usize);
    // 3 IUnknown + Play/Pause/Stop/FrameStep + SetPosition/GetPosition/GetDuration +
    // SetRate/GetRate/GetSupportedRates/GetState + CreateItemURL/Object/Set/Clear/Get +
    // GetVolume/SetVolume/GetBalance/SetBalance/GetMute = 23; SetMute = 24.
    let func: unsafe extern "system" fn(*mut c_void, i32) -> i32 =
        std::mem::transmute(*methods.add(24));
    func(obj, mute)
}

pub struct Player {
    ptr: *mut c_void,
    callback: *mut Callback,
}
impl Player {
    pub fn open(path: &Path, window: HWND) -> Result<Self, String> {
        use std::os::windows::ffi::OsStrExt;
        let url: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let cb = Box::into_raw(Box::new(Callback {
            vtable: &CALLBACK_VTABLE,
            refs: AtomicU32::new(1),
            window,
        }));
        let mut ptr = ptr::null_mut();
        // Safety: all raw pointers remain live through the call. COM owns an extra callback
        // reference if it retains the callback; we release our reference in Player::drop.
        let hr = unsafe { MFPCreateMediaPlayer(url.as_ptr(), 1, 0, cb, window, &mut ptr) };
        if hr < 0 || ptr.is_null() {
            unsafe { release(cb) };
            return Err(format!(
                "Media Foundation player creation failed (HRESULT {hr:#x})"
            ));
        }
        let player = Self { ptr, callback: cb };
        // The output asset should already be silent, but also mute here defensively.
        let _ = unsafe { call_setmute(player.ptr, 1) };
        Ok(player)
    }
    pub fn pause(&mut self) -> Result<(), String> {
        self.command(4, "pause")
    }
    pub fn play(&mut self) -> Result<(), String> {
        self.command(3, "play")
    }
    pub fn stop(&mut self) -> Result<(), String> {
        self.command(5, "stop")
    }
    pub fn restart(&mut self) -> Result<(), String> {
        // MFPlay Stop resets timeline. Some sources can still have a visible boundary frame;
        // seamless *content* looping must be authored/prepared in the media itself.
        self.stop()?;
        self.play()
    }
    fn command(&self, slot: usize, name: &str) -> Result<(), String> {
        let hr = unsafe { call0(self.ptr, slot) };
        if hr < 0 {
            Err(format!("MFPlay {name} failed: HRESULT {hr:#x}"))
        } else {
            Ok(())
        }
    }
}
impl Drop for Player {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe {
                let _ = call0(self.ptr, 5); // Stop
                let _ = call0(self.ptr, 38); // Shutdown (last IMFPMediaPlayer method)
                                             // Slot 2 is IUnknown::Release (returns a reference count, not HRESULT).
                let _ = call0(self.ptr, 2);
            }
        }
        // Callback receives no GUI references; MFPlay releases its own callback when shut down.
        if !self.callback.is_null() {
            unsafe { release(self.callback) };
        }
    }
}
