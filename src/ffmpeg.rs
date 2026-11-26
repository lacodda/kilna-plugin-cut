//! ffmpeg and ffprobe, found on the PATH and asked what a file is.
//!
//! The plugin carries no video library of its own: the two tools are what
//! every cutter on every system already has, and a missing one is said when
//! kilna asks the plugin to describe itself, so it stands in kilna's list
//! with the reason rather than failing at the first cut.

use std::io::{BufRead, BufReader, Read, Write};
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

/// What a cut needs to know of a donor: whether it has a picture and a sound.
#[derive(Debug, Clone, PartialEq)]
pub struct Probe {
    pub has_picture: bool,
    pub has_sound: bool,
}

/// What ffprobe says of a file's streams.
fn streams(tools: &Tools, path: &Path, entries: &str) -> Result<Vec<serde_json::Value>, String> {
    let output = command(&tools.ffprobe)
        .args(["-v", "error", "-show_entries", entries, "-of", "json"])
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
    Ok(report["streams"].as_array().cloned().unwrap_or_default())
}

/// Ask ffprobe about a file.
pub fn probe(tools: &Tools, path: &Path) -> Result<Probe, String> {
    let streams = streams(tools, path, "stream=codec_type,avg_frame_rate")?;
    // A cover held in a track's tags is a "video" stream of no frames a
    // second: a picture to look at, not one to cut.
    let has_picture = streams.iter().any(|stream| {
        stream["codec_type"] == "video"
            && stream["avg_frame_rate"]
                .as_str()
                .is_some_and(|rate| !rate.starts_with('0'))
    });
    let has_sound = streams.iter().any(|stream| stream["codec_type"] == "audio");
    Ok(Probe {
        has_picture,
        has_sound,
    })
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

/// What ffmpeg writes to its standard output for `args`, whole.
fn piped(tools: &Tools, args: &[String], what: &Path) -> Result<Vec<u8>, String> {
    let mut child = command(&tools.ffmpeg)
        .args(["-v", "error", "-nostdin"])
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|cause| format!("could not start ffmpeg: {cause}"))?;
    let mut bytes = Vec::new();
    if let Some(mut out) = child.stdout.take() {
        out.read_to_end(&mut bytes)
            .map_err(|cause| format!("could not read {}: {cause}", what.display()))?;
    }
    let output = child
        .wait_with_output()
        .map_err(|cause| format!("ffmpeg did not finish: {cause}"))?;
    if !output.status.success() {
        return Err(format!(
            "ffmpeg could not read {}: {}",
            what.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(bytes)
}

/// The sound of a file as mono samples at `rate` - the whole of it, or the
/// stretch from `from` running `seconds`.
pub fn sound(
    tools: &Tools,
    path: &Path,
    rate: u32,
    stretch: Option<(f64, f64)>,
) -> Result<Vec<f32>, String> {
    let mut args: Vec<String> = Vec::new();
    if let Some((from, seconds)) = stretch {
        args.extend([
            "-ss".into(),
            format!("{from:.3}"),
            "-t".into(),
            format!("{seconds:.3}"),
        ]);
    }
    args.extend(["-i".into(), path.to_string_lossy().into_owned()]);
    args.extend(
        ["-vn", "-ac", "1", "-ar"]
            .into_iter()
            .map(str::to_owned)
            .chain([rate.to_string(), "-f".into(), "f32le".into(), "-".into()]),
    );
    Ok(piped(tools, &args, path)?
        .chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect())
}

/// A picture as its pixels, four bytes each: red, green, blue, how solid.
#[derive(Debug, Clone, PartialEq)]
pub struct Picture {
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
}

/// Read a picture's pixels through ffmpeg.
pub fn picture(tools: &Tools, path: &Path) -> Result<Picture, String> {
    let streams = streams(tools, path, "stream=width,height")?;
    let side = |key: &str| {
        streams
            .first()
            .and_then(|stream| stream[key].as_u64())
            .map(|side| side as usize)
            .filter(|side| *side > 0)
            .ok_or_else(|| format!("{} is not a picture", path.display()))
    };
    let (width, height) = (side("width")?, side("height")?);
    let args: Vec<String> = [
        "-i",
        &path.to_string_lossy(),
        "-frames:v",
        "1",
        "-f",
        "rawvideo",
        "-pix_fmt",
        "rgba",
        "-",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    let rgba = piped(tools, &args, path)?;
    if rgba.len() != width * height * 4 {
        return Err(format!("{} did not read whole", path.display()));
    }
    Ok(Picture {
        width,
        height,
        rgba,
    })
}

/// What writes the frames ffmpeg reads from its standard input.
pub type Feed<'a> = Box<dyn FnOnce(&mut dyn Write) -> std::io::Result<()> + Send + 'a>;

/// Run ffmpeg with `args` in `dir`, telling `on_time` how many seconds of
/// the output are written as it goes. `feed`, when there is one, writes what
/// ffmpeg reads from its standard input. The tail of what ffmpeg said is the
/// error when it fails.
pub fn run(
    tools: &Tools,
    args: &[String],
    dir: &Path,
    feed: Option<Feed<'_>>,
    on_time: &mut dyn FnMut(f64),
) -> Result<(), String> {
    let mut child = command(&tools.ffmpeg)
        .args(["-hide_banner", "-nostats", "-progress", "pipe:1", "-y"])
        .args(args)
        .current_dir(dir)
        .stdin(if feed.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|cause| format!("could not run ffmpeg: {cause}"))?;

    let stderr = child.stderr.take();
    let stdin = child.stdin.take();
    let stdout = child.stdout.take();
    let said = std::thread::scope(|scope| {
        // What ffmpeg says on stderr, read beside its progress so neither
        // pipe fills and stalls it; only the tail is kept.
        let said = scope.spawn(move || {
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
        if let (Some(feed), Some(mut pipe)) = (feed, stdin) {
            // A pipe ffmpeg closed early is its failure to tell, not this
            // thread's: the status and the tail below say it.
            scope.spawn(move || {
                let _ = feed(&mut pipe);
            });
        }
        if let Some(stdout) = stdout {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if let Some(micros) = line
                    .strip_prefix("out_time_us=")
                    .and_then(|value| value.trim().parse::<f64>().ok())
                {
                    on_time(micros / 1_000_000.0);
                }
            }
        }
        said.join().unwrap_or_default()
    });
    let status = child
        .wait()
        .map_err(|cause| format!("ffmpeg could not be waited on: {cause}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("ffmpeg failed: {}", said.trim()))
    }
}
