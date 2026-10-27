//! ffmpeg and ffprobe, found on the PATH and asked what a file is.
//!
//! The plugin carries no video library of its own: the two tools are what
//! every cutter on every system already has, and a missing one is said when
//! kilna asks the plugin to describe itself, so it stands in kilna's list
//! with the reason rather than failing at the first cut.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The two tools a cut needs.
pub struct Tools {
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
}

/// Find both on the PATH, or say which is missing.
pub fn find() -> Result<Tools, String> {
    let ffmpeg =
        on_path("ffmpeg").ok_or("ffmpeg is not on the PATH; install it and start kilna again")?;
    let ffprobe = on_path("ffprobe").ok_or("ffprobe is not on the PATH; it comes with ffmpeg")?;
    Ok(Tools { ffmpeg, ffprobe })
}

/// The first `name` on the PATH that is a file - `name.exe` on Windows.
fn on_path(name: &str) -> Option<PathBuf> {
    let file = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_owned()
    };
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|dir| dir.join(&file))
        .find(|path| path.is_file())
}

/// A command with no console window flashing up on Windows.
pub fn command(program: &Path) -> Command {
    #[cfg_attr(not(windows), allow(unused_mut))]
    let mut command = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

/// What a cut needs to know of a donor: its picture's rate and whether it
/// has a sound.
#[derive(Debug, Clone, PartialEq)]
pub struct Probe {
    /// Frames a second, as a fraction ffmpeg reads: `30000/1001`.
    pub rate: String,
    pub has_sound: bool,
}

/// Ask ffprobe about a file.
pub fn probe(tools: &Tools, path: &Path) -> Result<Probe, String> {
    let output = command(&tools.ffprobe)
        .args([
            "-v",
            "error",
            "-show_entries",
            "stream=codec_type,avg_frame_rate",
            "-of",
            "json",
        ])
        .arg(path)
        .output()
        .map_err(|cause| format!("could not run ffprobe: {cause}"))?;
    if !output.status.success() {
        return Err(format!(
            "ffprobe could not read {}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let report: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|cause| format!("ffprobe's answer does not read: {cause}"))?;
    let streams = report["streams"].as_array().cloned().unwrap_or_default();
    let rate = streams
        .iter()
        .find(|stream| stream["codec_type"] == "video")
        .and_then(|stream| stream["avg_frame_rate"].as_str())
        .filter(|rate| !rate.starts_with('0'))
        .ok_or_else(|| format!("{} has no picture", path.display()))?
        .to_owned();
    let has_sound = streams.iter().any(|stream| stream["codec_type"] == "audio");
    Ok(Probe { rate, has_sound })
}

/// How long a file runs, in seconds.
pub fn length(tools: &Tools, path: &Path) -> Result<f64, String> {
    let output = command(&tools.ffprobe)
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "csv=p=0",
        ])
        .arg(path)
        .output()
        .map_err(|cause| format!("could not run ffprobe: {cause}"))?;
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .map_err(|_| format!("ffprobe could not say how long {} runs", path.display()))
}

/// Run ffmpeg with `args`, telling `on_time` how many seconds of the output
/// are written as it goes. The tail of what ffmpeg said is the error when it
/// fails.
pub fn run(tools: &Tools, args: &[String], on_time: &mut dyn FnMut(f64)) -> Result<(), String> {
    let mut child = command(&tools.ffmpeg)
        .args(["-hide_banner", "-nostats", "-progress", "pipe:1", "-y"])
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|cause| format!("could not run ffmpeg: {cause}"))?;

    // What ffmpeg says on stderr, read beside its progress so neither pipe
    // fills and stalls it; only the tail is kept.
    let stderr = child.stderr.take();
    let said = std::thread::spawn(move || {
        let mut tail: Vec<String> = Vec::new();
        if let Some(pipe) = stderr {
            for line in BufReader::new(pipe).lines().map_while(Result::ok) {
                tail.push(line);
                if tail.len() > 20 {
                    tail.remove(0);
                }
            }
        }
        tail.join("\n")
    });
    if let Some(stdout) = child.stdout.take() {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if let Some(micros) = line
                .strip_prefix("out_time_us=")
                .and_then(|value| value.trim().parse::<f64>().ok())
            {
                on_time(micros / 1_000_000.0);
            }
        }
    }
    let status = child
        .wait()
        .map_err(|cause| format!("ffmpeg could not be waited on: {cause}"))?;
    let said = said.join().unwrap_or_default();
    if status.success() {
        Ok(())
    } else {
        Err(format!("ffmpeg failed: {}", said.trim()))
    }
}
