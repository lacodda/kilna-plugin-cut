//! A short, cut: every stretch of its donors laid out vertically - the
//! picture over its own blur, the channel and the song above, a call below -
//! spliced in order, with the cover standing at the end. One ffmpeg pass,
//! so the file runs exactly as long as its stretches and its cover.

use std::path::{Path, PathBuf};

use crate::ffmpeg::{self, Tools};
use crate::protocol::{self, Subject};
use crate::settings::Settings;

/// The frame of a short.
const WIDTH: u32 = 1080;
const HEIGHT: u32 = 1920;
/// Where the sharp 16:9 picture stands: full width, its top here.
const PICTURE_TOP: u32 = 536;
/// The bars over the blur: the top one's height, and where the bottom one
/// starts.
const TOP_BAR: u32 = 280;
const BOTTOM_BAR: u32 = 1400;

/// The pointing hand under the call, rendered from Noto Color Emoji.
const HAND: &[u8] = include_bytes!("../assets/emoji-point-down.png");

/// One stretch, as it will be cut.
#[derive(Debug, Clone, PartialEq)]
pub struct Stretch {
    pub path: PathBuf,
    pub starts_at: f64,
    pub seconds: f64,
    pub has_sound: bool,
}

/// Everything a cut is made of, settled before ffmpeg runs.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    pub stretches: Vec<Stretch>,
    /// The picture's rate, the first donor's: every stretch is laid out at it.
    pub rate: String,
    /// The song's title, as the top bar letters it.
    pub title: String,
    pub cover: Option<PathBuf>,
    /// The file it ends as, and the one it is written to until it is done.
    pub output: PathBuf,
    pub partial: PathBuf,
}

impl Plan {
    /// How long the cut runs, cover and all.
    pub fn seconds(&self, settings: &Settings) -> f64 {
        let stretches: f64 = self.stretches.iter().map(|one| one.seconds).sum();
        stretches
            + if self.cover.is_some() {
                settings.cover_seconds
            } else {
                0.0
            }
    }
}

/// Settle what to cut, and say plainly what stops it.
pub fn plan(subject: &Subject, settings: &Settings, tools: &Tools) -> Result<Plan, String> {
    if subject.shots.is_empty() {
        return Err("The short has no stretches to cut yet.".into());
    }
    let folder = subject
        .folder
        .as_deref()
        .ok_or("The short has no folder on disk to cut into: set the media folder, or name the folder of its kind.")?;
    let mut stretches = Vec::new();
    let mut rate = None;
    for shot in &subject.shots {
        let path = shot.path.as_deref().ok_or_else(|| {
            format!(
                "\"{}\", which a stretch is cut from, has no finished file.",
                shot.source_title
            )
        })?;
        let path = PathBuf::from(path);
        let probe = ffmpeg::probe(tools, &path)?;
        if rate.is_none() {
            rate = Some(probe.rate.clone());
        }
        if shot.seconds() <= 0.0 {
            return Err(format!(
                "A stretch of \"{}\" runs no time.",
                shot.source_title
            ));
        }
        stretches.push(Stretch {
            path,
            starts_at: shot.starts_at,
            seconds: shot.seconds(),
            has_sound: probe.has_sound,
        });
    }
    let folder = Path::new(folder);
    let out_dir = folder.join(&settings.output);
    let output = next_output(&out_dir);
    let partial = partial_of(&output);
    let cover = if settings.cover_seconds > 0.0 {
        subject
            .cover
            .as_ref()
            .map(PathBuf::from)
            .filter(|path| path.is_file())
            .or_else(|| newest_picture(&folder.join(&settings.covers)))
    } else {
        None
    };
    let title = subject
        .origin
        .as_ref()
        .map_or_else(|| subject.title.clone(), |origin| origin.title.clone());
    Ok(Plan {
        stretches,
        rate: rate.unwrap_or_else(|| "30".into()),
        title: if settings.title_upper {
            title.to_uppercase()
        } else {
            title
        },
        cover,
        output,
        partial,
    })
}

/// The next free `v{k}.mp4` in `dir`: a cut never overwrites the one before
/// it, it lies down beside it.
pub fn next_output(dir: &Path) -> PathBuf {
    let greatest = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            name.strip_prefix('v')?
                .strip_suffix(".mp4")?
                .parse::<u32>()
                .ok()
        })
        .max()
        .unwrap_or(0);
    dir.join(format!("v{}.mp4", greatest + 1))
}

/// Where a cut is written until it is whole: a name no rule for `*.mp4`
/// matches, so a cut stopped halfway is never taken for the short's file.
fn partial_of(output: &Path) -> PathBuf {
    let mut name = output.file_name().unwrap_or_default().to_os_string();
    name.push(".part");
    output.with_file_name(name)
}

/// The newest picture in a folder, when there is one.
pub fn newest_picture(dir: &Path) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .map(|ending| ending.to_string_lossy().to_lowercase())
                    .is_some_and(|ending| {
                        matches!(ending.as_str(), "png" | "jpg" | "jpeg" | "webp")
                    })
        })
        .max_by_key(|path| {
            std::fs::metadata(path)
                .and_then(|metadata| metadata.modified())
                .ok()
        })
}

/// The size the song's title is lettered at so it fits the width: a short
/// title large, a long one smaller, never past either end.
pub fn title_size(title: &str) -> u32 {
    let letters = title.chars().count().max(1) as f64;
    let estimated = (980.0 / (letters * 0.62)) as u32;
    estimated.clamp(42, 80)
}

/// A path as a value inside a filter's quotes: forward slashes, the drive's
/// colon escaped, a quote closed and reopened around itself.
pub fn quoted(path: &Path) -> String {
    let path = path.to_string_lossy().replace('\\', "/");
    format!("'{}'", path.replace(':', "\\:").replace('\'', "'\\''"))
}

/// The files the words are read from - written, not escaped into the graph,
/// so a title with a colon or a quote is lettered as it is.
pub struct Words {
    pub channel: Option<PathBuf>,
    pub title: PathBuf,
    pub call: Option<PathBuf>,
    pub hand: Option<PathBuf>,
}

/// Write the words and the hand into `dir`.
pub fn words(plan: &Plan, settings: &Settings, dir: &Path) -> Result<Words, String> {
    let write = |name: &str, bytes: &[u8]| -> Result<PathBuf, String> {
        let path = dir.join(name);
        std::fs::write(&path, bytes)
            .map_err(|cause| format!("could not write {}: {cause}", path.display()))?;
        Ok(path)
    };
    let channel = settings.channel.trim();
    let call = settings.call.trim();
    Ok(Words {
        channel: (!channel.is_empty())
            .then(|| write("channel.txt", channel.as_bytes()))
            .transpose()?,
        title: write("title.txt", plan.title.as_bytes())?,
        call: (!call.is_empty())
            .then(|| write("call.txt", call.as_bytes()))
            .transpose()?,
        hand: (!call.is_empty() && settings.hand)
            .then(|| write("hand.png", HAND))
            .transpose()?,
    })
}

/// The arguments ffmpeg is run with: the stretches as trimmed inputs, the
/// hand and the cover after them, one graph laying each stretch out and
/// splicing them, and the encoding.
pub fn arguments(plan: &Plan, settings: &Settings, words: &Words) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();
    for stretch in &plan.stretches {
        args.extend([
            "-ss".into(),
            format!("{:.3}", stretch.starts_at),
            "-t".into(),
            format!("{:.3}", stretch.seconds),
            "-i".into(),
            stretch.path.to_string_lossy().into_owned(),
        ]);
    }
    let count = plan.stretches.len();
    let mut next_input = count;
    let hand_input = words.hand.as_ref().map(|hand| {
        args.extend(["-i".into(), hand.to_string_lossy().into_owned()]);
        next_input += 1;
        next_input - 1
    });
    let cover_input = plan.cover.as_ref().map(|cover| {
        args.extend([
            "-loop".into(),
            "1".into(),
            "-framerate".into(),
            plan.rate.clone(),
            "-t".into(),
            format!("{:.3}", settings.cover_seconds),
            "-i".into(),
            cover.to_string_lossy().into_owned(),
        ]);
        next_input += 1;
        next_input - 1
    });

    let mut graph: Vec<String> = Vec::new();
    if let Some(hand) = hand_input {
        let outs: String = (0..count).map(|i| format!("[h{i}]")).collect();
        graph.push(format!("[{hand}:v]split={count}{outs}"));
    }
    let shade = settings.bar_shade;
    let title_y = if words.channel.is_some() { 148 } else { 110 };
    for (i, stretch) in plan.stretches.iter().enumerate() {
        let seconds = format!("{:.3}", stretch.seconds);
        let mut chain = vec![
            format!("[{i}:v]fps={},split=2[bg{i}][fg{i}]", plan.rate),
            format!(
                "[bg{i}]scale={WIDTH}:{HEIGHT}:force_original_aspect_ratio=increase,crop={WIDTH}:{HEIGHT},boxblur=30:1,eq=brightness=-0.18[blur{i}]"
            ),
            format!("[fg{i}]scale={WIDTH}:-2[sharp{i}]"),
            format!("[blur{i}][sharp{i}]overlay=0:{PICTURE_TOP}[base{i}]"),
            format!("color=black@{shade:.2}:size={WIDTH}x{TOP_BAR}:d={seconds}[top{i}]"),
            format!("[base{i}][top{i}]overlay=0:0[up{i}]"),
            format!(
                "color=black@{:.2}:size={WIDTH}x{}:d={seconds}[bottom{i}]",
                (shade + 0.05).min(1.0),
                HEIGHT - BOTTOM_BAR
            ),
            format!("[up{i}][bottom{i}]overlay=0:{BOTTOM_BAR}[bars{i}]"),
        ];
        let mut last = format!("bars{i}");
        let ink = &settings.ink;
        if let Some(channel) = &words.channel {
            chain.push(format!(
                "[{last}]drawtext=fontfile={}:textfile={}:expansion=none:fontcolor={ink}@0.85:fontsize=44:x=(w-text_w)/2:y=72:borderw=2:bordercolor=black@0.6[chan{i}]",
                quoted(Path::new(&settings.font)),
                quoted(channel)
            ));
            last = format!("chan{i}");
        }
        chain.push(format!(
            "[{last}]drawtext=fontfile={}:textfile={}:expansion=none:fontcolor={ink}:fontsize={}:x=(w-text_w)/2:y={title_y}:borderw=3:bordercolor=black@0.75:shadowx=2:shadowy=2:shadowcolor=black@0.6[title{i}]",
            quoted(Path::new(&settings.font_bold)),
            quoted(&words.title),
            title_size(&plan.title)
        ));
        last = format!("title{i}");
        if let Some(call) = &words.call {
            chain.push(format!(
                "[{last}]drawtext=fontfile={}:textfile={}:expansion=none:fontcolor={ink}:fontsize=92:x=(w-text_w)/2:y=1500:borderw=4:bordercolor=black@0.8:shadowx=3:shadowy=3:shadowcolor=black@0.7[call{i}]",
                quoted(Path::new(&settings.font_bold)),
                quoted(call)
            ));
            last = format!("call{i}");
        }
        if hand_input.is_some() {
            // Bobbing at 0.8 Hz, 25 px either side of its place.
            chain.push(format!(
                "[{last}][h{i}]overlay=x=(W-w)/2:y=1680+25*sin(2*PI*t*0.8)[hand{i}]"
            ));
            last = format!("hand{i}");
        }
        chain.push(format!("[{last}]setsar=1,format=yuv420p[v{i}]"));
        chain.push(if stretch.has_sound {
            format!("[{i}:a]aresample=44100,aformat=channel_layouts=stereo[a{i}]")
        } else {
            format!("anullsrc=r=44100:cl=stereo,atrim=duration={seconds}[a{i}]")
        });
        graph.extend(chain);
    }
    let mut parts: String = (0..count).map(|i| format!("[v{i}][a{i}]")).collect();
    let mut segments = count;
    if let Some(cover) = cover_input {
        graph.push(format!(
            "[{cover}:v]scale={WIDTH}:{HEIGHT}:force_original_aspect_ratio=decrease,pad={WIDTH}:{HEIGHT}:(ow-iw)/2:(oh-ih)/2:color=black,setsar=1,fps={},format=yuv420p[vcover]",
            plan.rate
        ));
        graph.push(format!(
            "anullsrc=r=44100:cl=stereo,atrim=duration={:.3}[acover]",
            settings.cover_seconds
        ));
        parts.push_str("[vcover][acover]");
        segments += 1;
    }
    graph.push(format!("{parts}concat=n={segments}:v=1:a=1[outv][outa]"));

    args.extend([
        "-filter_complex".into(),
        graph.join(";"),
        "-map".into(),
        "[outv]".into(),
        "-map".into(),
        "[outa]".into(),
        "-c:v".into(),
        "libx264".into(),
        "-preset".into(),
        "medium".into(),
        "-crf".into(),
        settings.crf.to_string(),
        "-pix_fmt".into(),
        "yuv420p".into(),
        "-c:a".into(),
        "aac".into(),
        "-b:a".into(),
        "192k".into(),
        "-movflags".into(),
        "+faststart".into(),
        "-f".into(),
        "mp4".into(),
        plan.partial.to_string_lossy().into_owned(),
    ]);
    args
}

/// Cut the short: settle the plan, run ffmpeg once with progress, and put the
/// whole file under its name. Answers with the file.
pub fn cut(subject: &Subject, settings: &Settings, tools: &Tools) -> Result<PathBuf, String> {
    protocol::progress(0.0, "Reading the stretches");
    let plan = plan(subject, settings, tools)?;
    let out_dir = plan
        .output
        .parent()
        .ok_or("the cut has no folder to go into")?;
    std::fs::create_dir_all(out_dir)
        .map_err(|cause| format!("could not make {}: {cause}", out_dir.display()))?;

    let work = std::env::temp_dir().join(format!("kilna-plugin-cut-{}", std::process::id()));
    std::fs::create_dir_all(&work)
        .map_err(|cause| format!("could not make {}: {cause}", work.display()))?;
    let result = (|| {
        let words = words(&plan, settings, &work)?;
        let args = arguments(&plan, settings, &words);
        let total = plan.seconds(settings).max(0.001);
        let name = plan
            .output
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let mut said = -1.0;
        ffmpeg::run(tools, &args, &mut |seconds| {
            let share = (seconds / total).clamp(0.0, 1.0);
            // A line per whole percent: kilna draws a bar, not a log.
            if share - said >= 0.01 {
                said = share;
                protocol::progress(
                    share,
                    &format!(
                        "Cutting {name}: {:.0} of {:.0} s",
                        seconds.min(total),
                        total
                    ),
                );
            }
        })?;
        std::fs::rename(&plan.partial, &plan.output).map_err(|cause| {
            format!(
                "could not put the cut at {}: {cause}",
                plan.output.display()
            )
        })?;
        Ok(plan.output.clone())
    })();
    let _ = std::fs::remove_dir_all(&work);
    if result.is_err() {
        let _ = std::fs::remove_file(&plan.partial);
    }
    protocol::progress(1.0, "Done");
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a_plan(stretches: usize, cover: bool) -> Plan {
        Plan {
            stretches: (0..stretches)
                .map(|i| Stretch {
                    path: PathBuf::from(format!("D:/media/clip{i}.mp4")),
                    starts_at: 12.0 + i as f64 * 30.0,
                    seconds: 8.5,
                    has_sound: i == 0,
                })
                .collect(),
            rate: "30".into(),
            title: "TIDE".into(),
            cover: cover.then(|| PathBuf::from("D:/media/covers/v1.jpeg")),
            output: PathBuf::from("D:/media/shorts/v2/video/v3.mp4"),
            partial: PathBuf::from("D:/media/shorts/v2/video/v3.mp4.part"),
        }
    }

    fn words(channel: bool, call: bool) -> Words {
        Words {
            channel: channel.then(|| PathBuf::from("C:/tmp/channel.txt")),
            title: PathBuf::from("C:/tmp/title.txt"),
            call: call.then(|| PathBuf::from("C:/tmp/call.txt")),
            hand: call.then(|| PathBuf::from("C:/tmp/hand.png")),
        }
    }

    /// The next cut lies beside the last: `v3.mp4` after `v1` and `v2`, and
    /// a half-written cut does not count.
    #[test]
    fn a_cut_never_overwrites_the_one_before() {
        let dir = std::env::temp_dir().join(format!("kpc-next-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(next_output(&dir), dir.join("v1.mp4"));
        for name in ["v1.mp4", "V2.MP4", "v9.mp4.part", "notes.txt"] {
            std::fs::write(dir.join(name), b"x").unwrap();
        }
        assert_eq!(next_output(&dir), dir.join("v3.mp4"));
        assert_eq!(
            partial_of(&dir.join("v3.mp4")),
            dir.join("v3.mp4.part"),
            "a name no rule for *.mp4 matches"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Every stretch is a trimmed input, each laid out and spliced in order,
    /// the cover after them - one graph, one output.
    #[test]
    fn the_stretches_and_the_cover_are_spliced_in_one_pass() {
        let settings = Settings::default();
        let args = arguments(&a_plan(2, true), &settings, &words(true, true));
        let graph = &args[args
            .iter()
            .position(|arg| arg == "-filter_complex")
            .unwrap()
            + 1];

        assert_eq!(
            args.iter().filter(|arg| *arg == "-i").count(),
            4,
            "two stretches, the hand and the cover"
        );
        assert!(
            args.windows(2)
                .any(|pair| pair[0] == "-ss" && pair[1] == "42.000")
        );
        assert!(graph.contains("[1:v]split=2[h0][h1]") || graph.contains("[2:v]split=2[h0][h1]"));
        assert!(graph.ends_with("[v0][a0][v1][a1][vcover][acover]concat=n=3:v=1:a=1[outv][outa]"));
        assert!(
            graph.contains("anullsrc=r=44100:cl=stereo,atrim=duration=8.500[a1]"),
            "a donor with no sound plays silence for its stretch"
        );
        assert!(graph.contains("[0:a]aresample=44100"));
        assert_eq!(args.last().unwrap(), "D:/media/shorts/v2/video/v3.mp4.part");
        assert!(
            args.windows(2)
                .any(|pair| pair[0] == "-f" && pair[1] == "mp4")
        );
    }

    /// No channel and no call: the title stands alone, lower in the bar, and
    /// no hand points at nothing.
    #[test]
    fn what_the_settings_leave_out_is_not_drawn() {
        let settings = Settings::default();
        let args = arguments(&a_plan(1, false), &settings, &words(false, false));
        let graph = &args[args
            .iter()
            .position(|arg| arg == "-filter_complex")
            .unwrap()
            + 1];

        assert!(!graph.contains("chan0"));
        assert!(!graph.contains("call0"));
        assert!(!graph.contains("hand0"));
        assert!(graph.contains("y=110"), "the title takes the bar alone");
        assert!(graph.ends_with("[v0][a0]concat=n=1:v=1:a=1[outv][outa]"));
    }

    /// A Windows path inside a filter's quotes keeps its drive.
    #[test]
    fn a_path_is_quoted_for_a_filter() {
        assert_eq!(
            quoted(Path::new(r"C:\Windows\Fonts\arial.ttf")),
            r"'C\:/Windows/Fonts/arial.ttf'"
        );
        assert_eq!(quoted(Path::new("/tmp/it's.txt")), r"'/tmp/it'\''s.txt'");
    }

    /// A title fits the width: short ones large, long ones smaller, within
    /// bounds.
    #[test]
    fn a_title_is_lettered_to_fit() {
        assert_eq!(title_size("TIDE"), 80);
        assert_eq!(title_size(&"A".repeat(60)), 42);
        assert!(title_size("HARBOUR LIGHTS AT DAWN") < 80);
    }

    #[test]
    fn a_plan_runs_as_long_as_its_stretches_and_cover() {
        let settings = Settings::default();
        assert_eq!(a_plan(2, true).seconds(&settings), 17.5);
        assert_eq!(a_plan(2, false).seconds(&settings), 17.0);
    }
}
