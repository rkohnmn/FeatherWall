#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(windows)]
fn main() {
    if let Err(error) = featherwall::win::run() {
        eprintln!("FeatherWall: {error}");
        let message = featherwall::win::ffi::wide(&error);
        let caption = featherwall::win::ffi::wide("FeatherWall cannot start");
        unsafe {
            featherwall::win::ffi::MessageBoxW(0, message.as_ptr(), caption.as_ptr(), 0x10);
        }
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("FeatherWall playback requires Windows 10 or later. Run `cargo test` for platform-independent tests.");
}
