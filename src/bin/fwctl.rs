//! Console companion. It never starts another wallpaper playback engine.
use featherwall::{core, importer};
use std::path::PathBuf;

#[cfg(windows)]
mod windows_cli {
    use super::*;
    use featherwall::win::ffi as f;
    use std::{ffi::OsString, ptr};
    const CLASS: &str = "FeatherWallControlWindow";
    fn find_gui() -> f::HWND {
        let mut hwnd = unsafe { f::FindWindowW(f::wide(CLASS).as_ptr(), ptr::null()) };
        if hwnd == 0 {
            let desk = unsafe { f::OpenDesktopW(f::wide("default").as_ptr(), 0, 0, 0x01FF) };
            if desk != 0 {
                unsafe {
                    let _ = f::SetThreadDesktop(desk);
                }
                hwnd = unsafe { f::FindWindowW(f::wide(CLASS).as_ptr(), ptr::null()) };
            }
        }
        hwnd
    }
    pub fn run() -> Result<(), String> {
        let args: Vec<OsString> = std::env::args_os().skip(1).collect();
        if args.is_empty() {
            usage();
            return Ok(());
        }
        let cmd = args[0].to_string_lossy().to_ascii_lowercase();
        let root = core::data_root().map_err(|e| e.to_string())?;
        match cmd.as_str() {
            "--help" | "help" => {
                usage();
                Ok(())
            }
            "list" => {
                for item in core::load(&root.join("library.fwcfg")).items {
                    println!(
                        "{}\t{}\t{}\t{}",
                        item.id,
                        item.title,
                        item.profile.text(),
                        item.asset.display()
                    );
                }
                Ok(())
            }
            "pause" | "resume" | "stop" | "status" | "gui" => {
                let hwnd = find_gui();
                if hwnd == 0 {
                    return Err("FeatherWall is not running".into());
                }
                let message = match cmd.as_str() {
                    "pause" => f::WM_APP + 6,
                    "resume" => f::WM_APP + 7,
                    "stop" => f::WM_APP + 8,
                    "status" => f::WM_APP + 9,
                    _ => f::WM_APP + 3,
                };
                let mut answer = 0;
                let success =
                    unsafe { f::SendMessageTimeoutW(hwnd, message, 0, 0, 2, 3000, &mut answer) };
                if success == 0 {
                    return Err("FeatherWall did not respond to the command".into());
                }
                if cmd == "status" {
                    println!("FeatherWall is running; {} displays playing", answer);
                }
                Ok(())
            }
            "exit" | "quit" => {
                let hwnd = find_gui();
                if hwnd == 0 {
                    return Err("FeatherWall is not running".into());
                }
                unsafe {
                    f::PostMessageW(hwnd, f::WM_COMMAND, 200, 0);
                }
                println!("FeatherWall shutting down");
                Ok(())
            }
            "import" => {
                if args.len() < 2 {
                    return Err(
                        "Usage: fwctl import <filename> [--profile eco|balanced|quality|automatic]"
                            .into(),
                    );
                }
                let path = PathBuf::from(&args[1]);
                if !path.is_file() {
                    return Err(format!("File not found: {}", path.display()));
                }
                let hwnd = find_gui();
                if hwnd != 0 {
                    use std::os::windows::ffi::OsStrExt;
                    let src: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
                    let message = f::COPYDATASTRUCT {
                        dwData: 0x46575449,
                        cbData: (src.len() * 2) as u32,
                        lpData: src.as_ptr().cast(),
                    };
                    let mut ack = 0;
                    let outcome = unsafe {
                        f::SendMessageTimeoutW(
                            hwnd,
                            f::WM_COPYDATA,
                            0,
                            &message as *const _ as isize,
                            2,
                            3000,
                            &mut ack,
                        )
                    };
                    if outcome == 0 || ack == 0 {
                        return Err("Running FeatherWall did not accept the import request".into());
                    }
                    println!("Import queued in FeatherWall");
                    return Ok(());
                }
                let profile = if args.len() > 3 && args[2].to_string_lossy() == "--profile" {
                    core::Profile::parse(&args[3].to_string_lossy())
                } else {
                    core::Profile::Automatic
                };
                let resolution = (1920, 1080); // CLI import can later be assigned and re-optimized to the actual display.
                let cancel = importer::CancelToken::new();
                let item = importer::import(
                    &path,
                    &root,
                    &importer::ImportOptions {
                        profile,
                        monitor_width: resolution.0,
                        monitor_height: resolution.1,
                        fps: 0,
                        cap: 0,
                        fit: false,
                        battery: false,
                    },
                    &cancel,
                    |event| {
                        if let importer::Progress::Converting(p) = event {
                            println!("Encoding {p}%");
                        }
                    },
                )?;
                let title = item.title.clone();
                let mut library = core::load(&root.join("library.fwcfg"));
                library.items.retain(|old| old.id != item.id);
                library.items.push(item);
                core::save(&root.join("library.fwcfg"), &library).map_err(|e| e.to_string())?;
                println!("Imported {title}; open FeatherWall and assign it to a monitor.");
                Ok(())
            }
            "clean" => {
                if find_gui() != 0 {
                    return Err("Use the GUI cache cleanup while FeatherWall runs".into());
                }
                let library = core::load(&root.join("library.fwcfg"));
                let (n, bytes) =
                    importer::cleanup(&root, library.items.into_iter().map(|item| item.asset))
                        .map_err(|e| e.to_string())?;
                println!("Deleted {n} orphan cache files ({bytes} bytes)");
                Ok(())
            }
            _ => Err(format!("Unknown command: {cmd}")),
        }
    }
    fn usage() {
        println!("fwctl commands:\n  list\n  import <file> [--profile eco|balanced|quality|automatic]\n  pause | resume | stop | status | gui | clean | exit\n\nLaunch featherwall.exe for the tray application.");
    }
}
#[cfg(windows)]
fn main() {
    if let Err(e) = windows_cli::run() {
        eprintln!("Error: {e}");
        std::process::exit(1)
    }
}
#[cfg(not(windows))]
fn main() {
    println!("fwctl is available on Windows 10/11 only.");
}
