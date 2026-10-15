//! FFprobe/FFmpeg run only while explicitly importing. Runtime playback has no FFmpeg dependency.
use crate::core::{self, LibraryItem, MediaKind, Plan, Profile};
use std::ffi::OsString;
use std::fs;
use std::io::{self, BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc,
};
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, Default)]
pub struct Probe {
    pub format: String,
    pub codec: String,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub duration: f64,
    pub bitrate: u64,
    pub pix_fmt: String,
    pub bits: u32,
    pub has_audio: bool,
    pub frames: u64,
    pub rotation: String,
    pub color: String,
    pub vfr: bool,
    pub kind: Option<MediaKind>,
}
fn fraction(s: &str) -> f64 {
    if let Some((n, d)) = s.split_once('/') {
        let n = n.parse::<f64>().unwrap_or(0.0);
        let d = d.parse::<f64>().unwrap_or(0.0);
        if d == 0.0 {
            0.0
        } else {
            n / d
        }
    } else {
        s.parse().unwrap_or(0.0)
    }
}
fn numeric(s: &str) -> u64 {
    s.trim().parse().unwrap_or(0)
}

pub fn parse_probe(text: &str) -> Result<Probe, String> {
    let mut result = Probe::default();
    let mut video_seen = false;
    let mut avg_fps = 0.0;
    let mut nominal_fps = 0.0;
    let mut in_stream = false;
    let mut stream_fields: Vec<(String, String)> = Vec::new();
    for line in text.lines() {
        match line {
            "[STREAM]" => {
                in_stream = true;
                stream_fields.clear();
                continue;
            }
            "[/STREAM]" => {
                in_stream = false;
                let get = |key: &str| {
                    stream_fields
                        .iter()
                        .find(|(k, _)| k == key)
                        .map(|(_, v)| v.as_str())
                        .unwrap_or("")
                };
                match get("codec_type") {
                    "audio" => result.has_audio = true,
                    "video" if !video_seen => {
                        video_seen = true;
                        result.codec = get("codec_name").into();
                        result.width = numeric(get("width")) as u32;
                        result.height = numeric(get("height")) as u32;
                        result.pix_fmt = get("pix_fmt").into();
                        result.bits = numeric(get("bits_per_raw_sample")) as u32;
                        result.frames = numeric(get("nb_frames"));
                        result.rotation = get("TAG:rotate").into();
                        result.color = get("color_space").into();
                        nominal_fps = fraction(get("r_frame_rate"));
                        avg_fps = fraction(get("avg_frame_rate"));
                    }
                    _ => {}
                }
                continue;
            }
            "[FORMAT]" | "[/FORMAT]" => continue,
            _ => {}
        }
        if let Some((key, value)) = line.split_once('=') {
            if in_stream {
                stream_fields.push((key.to_string(), value.to_string()));
            } else {
                match key {
                    "format_name" => result.format = value.into(),
                    "duration" => result.duration = fraction(value),
                    "bit_rate" => result.bitrate = numeric(value),
                    _ => {}
                }
            }
        }
    }
    if !video_seen || result.width == 0 || result.height == 0 {
        return Err("No usable image or video stream was found".into());
    }
    result.fps = if avg_fps > 0.0 { avg_fps } else { nominal_fps };
    result.vfr = avg_fps > 0.0 && nominal_fps > 0.0 && (avg_fps - nominal_fps).abs() > 0.5;
    let format = result.format.as_str();
    let still = result.frames <= 1
        && (format.contains("image2")
            || matches!(result.codec.as_str(), "png" | "mjpeg" | "bmp") && result.duration <= 0.0);
    result.kind = Some(if still {
        MediaKind::Still
    } else {
        MediaKind::Video
    });
    Ok(result)
}

fn program(name: &str) -> PathBuf {
    // Explicit override; adjacent portable binary; lastly the user's PATH.
    let env_name = if name == "ffmpeg" {
        "FEATHERWALL_FFMPEG"
    } else {
        "FEATHERWALL_FFPROBE"
    };
    if let Some(overridden) = std::env::var_os(env_name) {
        return PathBuf::from(overridden);
    }
    if let Ok(me) = std::env::current_exe() {
        let sibling = me.with_file_name(format!("{name}.exe"));
        if sibling.is_file() {
            return sibling;
        }
    }
    PathBuf::from(name)
}
pub fn probe(path: &Path) -> Result<Probe, String> {
    probe_with_cancel(path, None)
}
fn probe_with_cancel(path: &Path, cancel: Option<&CancelToken>) -> Result<Probe, String> {
    use std::time::Instant;
    let mut child = Command::new(program("ffprobe"))
        .arg("-v").arg("error")
        .arg("-show_entries")
        .arg("stream=codec_type,codec_name,width,height,avg_frame_rate,r_frame_rate,nb_frames,pix_fmt,bits_per_raw_sample,color_space,color_transfer,color_primaries:stream_tags=rotate:format=format_name,duration,bit_rate")
        .arg("-of").arg("default=noprint_wrappers=0")
        .arg(path).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn().map_err(|e|format!("ffprobe could not start: {e}"))?;
    let stdout = child.stdout.take().ok_or("ffprobe stdout missing")?;
    let stderr = child.stderr.take().ok_or("ffprobe stderr missing")?;
    // Both pipes are drained concurrently, so malformed files cannot deadlock the process on a full pipe.
    let reading = thread::spawn(move || {
        let mut buffer = Vec::new();
        let _ = stdout.take(256 * 1024).read_to_end(&mut buffer);
        buffer
    });
    let errors = thread::spawn(move || {
        let mut buffer = Vec::new();
        let _ = stderr.take(64 * 1024).read_to_end(&mut buffer);
        buffer
    });
    let started = Instant::now();
    let status = loop {
        if cancel.is_some_and(|flag| flag.requested())
            || started.elapsed() > Duration::from_secs(30)
        {
            let _ = child.kill();
            let _ = child.wait();
            let _ = reading.join();
            let _ = errors.join();
            return Err("Media analysis cancelled or exceeded a 30-second timeout".into());
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => thread::sleep(Duration::from_millis(100)),
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = reading.join();
                let _ = errors.join();
                return Err(format!("Media probing failed: {e}"));
            }
        }
    };
    let output = reading.join().unwrap_or_default();
    let error = errors.join().unwrap_or_default();
    if !status.success() {
        return Err(format!("Media probing failed: {}", last_error(&error)));
    }
    parse_probe(&String::from_utf8_lossy(&output))
}
fn last_error(bytes: &[u8]) -> String {
    let message = String::from_utf8_lossy(bytes);
    let final_lines = message
        .lines()
        .rev()
        .take(5)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join(" | ");
    final_lines.chars().take(700).collect()
}
#[derive(Clone, Debug)]
pub struct ImportOptions {
    pub profile: Profile,
    pub monitor_width: u32,
    pub monitor_height: u32,
    pub cap: u32,
    pub fps: u32,
    pub fit: bool,
    pub battery: bool,
}
#[derive(Clone, Debug)]
pub enum Progress {
    Analyze,
    Hashing,
    Reusing,
    Converting(u8),
    Finished,
    Failed(String),
}
#[derive(Clone)]
pub struct CancelToken(Arc<AtomicBool>);
impl Default for CancelToken {
    fn default() -> Self {
        Self::new()
    }
}
impl CancelToken {
    pub fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
    fn requested(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}
fn sanitize_dimension(n: u32) -> u32 {
    (n.clamp(2, 8192) / 2) * 2
}
fn adapt_plan(mut plan: Plan, options: &ImportOptions, probe: &Probe) -> Plan {
    if options.fps > 0 {
        plan.fps =
            (options.fps.min(120) as f64).min(if probe.fps > 0.0 { probe.fps } else { 120. });
    }
    if options.cap > 0 {
        let max = options.cap.clamp(64, 8192) as f64;
        let scale = (max / (plan.width.max(plan.height) as f64)).min(1.0);
        plan.width = sanitize_dimension((plan.width as f64 * scale) as u32);
        plan.height = sanitize_dimension((plan.height as f64 * scale) as u32);
    }
    // In Fill mode a video can be cropped once at import, avoiding a runtime scaler.
    if !options.fit {
        let w = options.monitor_width.max(2) as f64;
        let h = options.monitor_height.max(2) as f64;
        let long = (plan.width.max(plan.height)) as f64;
        let scale = (long / w.max(h)).min(1.0);
        plan.width = sanitize_dimension((w * scale).round() as u32);
        plan.height = sanitize_dimension((h * scale).round() as u32);
    }
    plan
}

fn compatible_copy(p: &Probe, plan: &Plan) -> bool {
    p.kind == Some(MediaKind::Video)
        && p.format.contains("mp4")
        && p.codec == "h264"
        && !p.has_audio
        && p.pix_fmt == "yuv420p"
        && !p.vfr
        && p.rotation.is_empty()
        && p.width == plan.width
        && p.height == plan.height
        && p.fps > 0.0
        && p.fps <= plan.fps + 0.05
}

fn spawn_encoder(
    source: &Path,
    output: &Path,
    p: &Probe,
    plan: &Plan,
    fit: bool,
    cancel: &CancelToken,
    callback: &impl Fn(Progress),
) -> Result<(), String> {
    let mut args: Vec<OsString> = ["-hide_banner", "-nostdin", "-loglevel", "error", "-y"]
        .iter()
        .map(|s| OsString::from(*s))
        .collect();
    if p.format == "gif" {
        args.extend(["-ignore_loop".into(), "1".into()]);
    }
    args.push("-i".into());
    args.push(source.as_os_str().to_os_string());
    args.extend(
        ["-map", "0:v:0", "-an", "-sn", "-dn"]
            .iter()
            .map(|s| OsString::from(*s)),
    );
    let video = p.kind == Some(MediaKind::Video);
    let dimensions = format!("{}:{}", plan.width, plan.height);
    let filt = if fit {
        format!("scale={dimensions}:force_original_aspect_ratio=decrease,pad={dimensions}:(ow-iw)/2:(oh-ih)/2")
    } else {
        format!("scale={dimensions}:force_original_aspect_ratio=increase,crop={dimensions}")
    };
    args.push(OsString::from("-vf"));
    args.push(OsString::from(filt));
    if video {
        args.extend([
            OsString::from("-r"),
            OsString::from(format!("{:.3}", plan.fps.max(1.0))),
            OsString::from("-fps_mode"),
            OsString::from("cfr"),
            OsString::from("-c:v"),
            OsString::from("libx264"),
            OsString::from("-pix_fmt"),
            OsString::from("yuv420p"),
            OsString::from("-preset"),
            OsString::from(plan.encoder_preset),
            OsString::from("-crf"),
            OsString::from(plan.crf.to_string()),
            OsString::from("-movflags"),
            OsString::from("+faststart"),
            OsString::from("-f"),
            OsString::from("mp4"),
        ]);
    } else {
        args.extend([
            "-frames:v".into(),
            "1".into(),
            "-pix_fmt".into(),
            "bgr24".into(),
            "-c:v".into(),
            "bmp".into(),
            "-f".into(),
            "image2".into(),
        ]);
    }
    args.extend(["-progress".into(), "pipe:1".into(), "-nostats".into()]);
    args.push(output.as_os_str().to_os_string());
    let mut child = Command::new(program("ffmpeg"))
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Could not start FFmpeg: {e}"))?;
    let (tx, rx) = mpsc::channel();
    let stdout = child.stdout.take().ok_or("No FFmpeg progress pipe")?;
    let reader = thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if tx.send(line).is_err() {
                break;
            }
        }
    });
    let stderr = child.stderr.take().ok_or("No FFmpeg diagnostics pipe")?;
    let diagnostics = thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stderr.take(64 * 1024).read_to_end(&mut buf);
        buf
    });
    let mut last_percent = 0;
    let result = loop {
        if cancel.requested() {
            let _ = child.kill();
            let _ = child.wait();
            break Err("Import cancelled".into());
        }
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(line) => {
                if let Some(ms) = line.strip_prefix("out_time_ms=") {
                    if p.duration > 0.0 {
                        // FFmpeg out_time_ms is microseconds for historical compatibility.
                        let percent =
                            (ms.parse::<f64>().unwrap_or(0.0) / (p.duration * 1_000_000.0) * 100.0)
                                .clamp(0.0, 99.0) as u8;
                        if percent > last_percent {
                            last_percent = percent;
                            callback(Progress::Converting(percent));
                        }
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {}
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                break if status.success() {
                    Ok(())
                } else {
                    Err(format!("FFmpeg returned {status}"))
                }
            }
            Ok(None) => {}
            Err(e) => break Err(format!("FFmpeg process error: {e}")),
        }
    };
    let _ = reader.join();
    let errors = diagnostics.join().unwrap_or_default();
    result.map_err(|e| format!("{e}. {}", last_error(&errors)))
}

pub fn import(
    input: &Path,
    root: &Path,
    options: &ImportOptions,
    cancel: &CancelToken,
    callback: impl Fn(Progress),
) -> Result<LibraryItem, String> {
    callback(Progress::Analyze);
    if cancel.requested() {
        return Err("Import cancelled".into());
    }
    if !input.is_file() {
        return Err("File does not exist".into());
    }
    let bytes = fs::metadata(input).map_err(|e| e.to_string())?.len();
    if bytes == 0 {
        return Err("Empty media file".into());
    }
    let p = probe_with_cancel(input, Some(cancel))?;
    if cancel.requested() {
        return Err("Import cancelled".into());
    }
    callback(Progress::Hashing);
    let digest =
        core::content_key_cancellable(input, || cancel.requested()).map_err(|e| e.to_string())?;
    let selected = options.profile.preset(
        p.width,
        p.height,
        p.fps,
        options.monitor_width,
        options.monitor_height,
        options.battery,
    );
    let plan = adapt_plan(selected, options, &p);
    let static_image = p.kind == Some(MediaKind::Still);
    let copied = compatible_copy(&p, &plan);
    let id = format!(
        "{}-{}-{}",
        digest,
        plan.key(),
        if options.fit { "fit" } else { "fill" }
    );
    let extension = if static_image { "bmp" } else { "mp4" };
    let directory = root.join("cache");
    fs::create_dir_all(&directory).map_err(|e| format!("Cannot create cache directory: {e}"))?;
    let asset = directory.join(format!("{id}.{extension}"));
    if asset.is_file()
        && fs::metadata(&asset)
            .map(|m| m.len() > 1024)
            .unwrap_or(false)
        && probe_with_cancel(&asset, Some(cancel)).is_ok()
    {
        callback(Progress::Reusing);
    } else {
        let temporary = directory.join(format!("{id}-{}.partial.{extension}", std::process::id()));
        struct Temp(PathBuf);
        impl Drop for Temp {
            fn drop(&mut self) {
                let _ = fs::remove_file(&self.0);
            }
        }
        let _cleanup = Temp(temporary.clone());
        if copied {
            callback(Progress::Reusing);
            fs::copy(input, &temporary).map_err(|e| format!("Cache copy failed: {e}"))?;
        } else {
            callback(Progress::Converting(0));
            spawn_encoder(input, &temporary, &p, &plan, options.fit, cancel, &callback)?;
        }
        if cancel.requested() {
            return Err("Import cancelled".into());
        }
        let validation = probe_with_cancel(&temporary, Some(cancel))?;
        if validation.kind != p.kind {
            return Err("FFmpeg output changed the media type".into());
        }
        if asset.exists() {
            fs::remove_file(&asset).map_err(|e| e.to_string())?;
        }
        fs::rename(&temporary, &asset).map_err(|e| format!("Cache finalization failed: {e}"))?;
    }
    let thumb = directory.join(format!("{id}.thumb.bmp"));
    if !thumb.is_file() {
        let _ = generate_thumbnail(input, &thumb, !static_image);
    }
    let optimized = probe_with_cancel(&asset, Some(cancel))?;
    let result = LibraryItem {
        id,
        title: input.file_stem().unwrap_or_default().to_string_lossy().to_string(),
        original: input.to_path_buf(),asset:asset.clone(),kind:if static_image {MediaKind::Still}else{MediaKind::Video},
        profile:plan.resolved,source_width:p.width,source_height:p.height,
        width:optimized.width,height:optimized.height,fps:optimized.fps,
        duration:optimized.duration,bytes:fs::metadata(&asset).map_err(|e|e.to_string())?.len(),
        imported_at:core::unix_seconds(),
        note:format!("{} → {} {}×{} at {:.1} FPS; {}. Source: {} / {} / {}; audio stripped: {}. Decoder acceleration is OS-dependent.",
            options.profile.text(),plan.resolved.text(),plan.width,plan.height,plan.fps,
            if copied {"Direct-copy H.264"}else if static_image{"Native BMP still"}else{"H.264 conversion"},
            p.format,p.codec,p.pix_fmt,p.has_audio),
    };
    callback(Progress::Finished);
    Ok(result)
}

pub fn generate_thumbnail(input: &Path, output: &Path, is_video: bool) -> Result<(), String> {
    let mut cmd = Command::new(program("ffmpeg"));
    cmd.arg("-y");
    if is_video {
        cmd.arg("-ss").arg("00:00:00.500");
    }
    cmd.arg("-i").arg(input.as_os_str());
    cmd.arg("-vf").arg("scale=320:180:force_original_aspect_ratio=decrease,pad=320:180:(ow-iw)/2:(oh-ih)/2:color=black");
    cmd.arg("-vframes").arg("1");
    cmd.arg("-pix_fmt").arg("bgr24");
    cmd.arg("-c:v").arg("bmp");
    cmd.arg(output.as_os_str());
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let status = match cmd.status() {
        Ok(s) => s,
        Err(e) => return Err(e.to_string()),
    };
    if !status.success() && is_video {
        let mut retry = Command::new(program("ffmpeg"));
        retry
            .arg("-y")
            .arg("-ss")
            .arg("0")
            .arg("-i")
            .arg(input.as_os_str());
        retry.arg("-vf").arg("scale=320:180:force_original_aspect_ratio=decrease,pad=320:180:(ow-iw)/2:(oh-ih)/2:color=black");
        retry
            .arg("-vframes")
            .arg("1")
            .arg("-pix_fmt")
            .arg("bgr24")
            .arg("-c:v")
            .arg("bmp")
            .arg(output.as_os_str());
        retry
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let retry_status = retry.status().map_err(|e| e.to_string())?;
        if !retry_status.success() {
            return Err("Thumbnail extraction failed".into());
        }
    }
    Ok(())
}

/// Orphan files are deleted only if they are direct, regular files in the dedicated cache directory.
pub fn cleanup(root: &Path, referenced: impl Iterator<Item = PathBuf>) -> io::Result<(u64, u64)> {
    let directory = root.join("cache");
    let mut keep: std::collections::HashSet<PathBuf> = referenced.collect();
    for asset in keep.clone() {
        if let Some(stem) = asset.file_stem() {
            keep.insert(directory.join(format!("{}.thumb.bmp", stem.to_string_lossy())));
        }
    }
    let mut removed = 0;
    let mut bytes = 0;
    if !directory.is_dir() {
        return Ok((0, 0));
    }
    for entry in fs::read_dir(&directory)? {
        let entry = entry?;
        let path = entry.path();
        if !entry.file_type()?.is_file() || keep.contains(&path) {
            continue;
        }
        let metadata = entry.metadata()?;
        if fs::remove_file(path).is_ok() {
            removed += 1;
            bytes += metadata.len();
        }
    }
    Ok((removed, bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fractions() {
        assert_eq!(fraction("30000/1001").round() as i32, 30);
        assert_eq!(fraction("0/0"), 0.0);
    }
    #[test]
    fn sample_probe_video() {
        let text="[STREAM]\ncodec_name=h264\ncodec_type=video\nwidth=1920\nheight=1080\nr_frame_rate=30/1\navg_frame_rate=30/1\npix_fmt=yuv420p\n[/STREAM]\n[STREAM]\ncodec_type=audio\n[/STREAM]\n[FORMAT]\nformat_name=mov,mp4,m4a,3gp,3g2,mj2\nduration=5.0\nbit_rate=2300000\n[/FORMAT]";
        let p = parse_probe(text).unwrap();
        assert_eq!(p.width, 1920);
        assert!(p.has_audio);
        assert_eq!(p.kind, Some(MediaKind::Video));
    }
    #[test]
    fn static_detection() {
        let p=parse_probe("[STREAM]\ncodec_type=video\ncodec_name=png\nwidth=100\nheight=200\n[/STREAM]\n[FORMAT]\nformat_name=png_pipe\n[/FORMAT]").unwrap();
        assert_eq!(p.kind, Some(MediaKind::Still));
    }
    #[test]
    fn reject_invalid() {
        assert!(parse_probe("[FORMAT]\nformat_name=mp3\n[/FORMAT]").is_err());
    }
    #[test]
    fn safe_cleanup() {
        let root = std::env::temp_dir().join(format!("featherwall-clean-{}", std::process::id()));
        let cache = root.join("cache");
        let _ = fs::create_dir_all(&cache);
        let a = cache.join("used.mp4");
        let b = cache.join("orphan.mp4");
        fs::write(&a, b"use").unwrap();
        fs::write(&b, b"orphan").unwrap();
        let (count, bytes) = cleanup(&root, std::iter::once(a.clone())).unwrap();
        assert_eq!(count, 1);
        assert_eq!(bytes, 6);
        assert!(a.exists());
        assert!(!b.exists());
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn early_import_cancellation() {
        let token = CancelToken::new();
        token.cancel();
        let root = std::env::temp_dir();
        let options = ImportOptions {
            profile: Profile::Automatic,
            monitor_width: 1920,
            monitor_height: 1080,
            cap: 0,
            fps: 0,
            fit: false,
            battery: false,
        };
        let e = import(Path::new("never.exists"), &root, &options, &token, |_| {}).unwrap_err();
        assert!(e.contains("cancelled"));
    }
    #[test]
    fn plan_cfr() {
        let p = Probe {
            fps: 60.,
            width: 3840,
            height: 2160,
            ..Probe::default()
        };
        let o = ImportOptions {
            profile: Profile::UltraEco,
            monitor_width: 1920,
            monitor_height: 1080,
            cap: 0,
            fps: 0,
            fit: false,
            battery: false,
        };
        let a = adapt_plan(
            o.profile
                .preset(p.width, p.height, p.fps, 1920, 1080, false),
            &o,
            &p,
        );
        assert!(a.fps <= 15.);
    }
}
