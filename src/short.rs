//! A short, built: its stretches spliced under the look of its channel.
//!
//! kilna keeps the look - the style of the space with the short's variant
//! laid over it - and draws its still layers itself: the ground, the blots,
//! the stamp, the sign, the header. This module moves them. The picture
//! pushes in, the blots drift and are thrown on a hit, the sign kicks with
//! the loudness and blinks, a wave or bars play the sound, the words light as
//! they are sung, and the cover stands at the end. One ffmpeg pass.

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::ffmpeg::{self, Picture, Tools};
use crate::karaoke::{self, Lettering, Word};
use crate::listen;
use crate::protocol::{self, Subject};
use crate::settings::Settings;

/// The frame of a short, and how many of them a second.
pub const WIDTH: u32 = 1080;
pub const HEIGHT: u32 = 1920;
pub const FPS: u32 = 30;
/// The picture: full width, 16:9, its top here.
const PICTURE_TOP: u32 = 536;
const PICTURE_HEIGHT: u32 = 608;
/// The strip that fills as the short plays, under the picture.
const STRIP_TOP: u32 = 1144;
/// The band the sound is drawn in, and how tall it is.
const WAVE_TOP: u32 = 1166;
const WAVE_HEIGHT: usize = 220;
/// Where the words stand.
const WORDS_TOP: u32 = 1412;
/// The square the sign lives in, and its top.
const SIGN_BOX: usize = 420;
const SIGN_TOP: u32 = 1505;
/// How many bars the sound is drawn as.
const BARS: usize = 28;

/// The look of a short, as kilna resolved it: the style of the space with
/// the short's variant laid over. Only what moves is read here - what stands
/// still was drawn by kilna into the layers.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Look {
    /// The two colours of the variant, `#RRGGBB`.
    pub accent: String,
    pub second: String,
    pub sign: Sign,
    pub ground: Ground,
    pub wave: Wave,
    pub karaoke: Karaoke,
    pub effects: Effects,
    pub tail: Tail,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct Sign {
    pub on: bool,
    /// Its side at rest, in pixels of the frame.
    pub size: f64,
    /// How much larger the loudest moment makes it: 0.24 is a quarter.
    pub pulse: f64,
    /// How far it sways either way, in degrees.
    pub sway: f64,
}

impl Default for Sign {
    fn default() -> Self {
        Self {
            on: false,
            size: 310.0,
            pulse: 0.24,
            sway: 5.0,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Ground {
    /// Whether the blots drift and are thrown on a hit.
    pub motion: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaveKind {
    /// One filled wave in the accent.
    #[default]
    Solid,
    /// Two lines, one in each colour.
    Lines,
    /// Mirrored bars: the bass in the middle, the highs at both edges.
    Bars,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Wave {
    pub on: bool,
    pub kind: WaveKind,
    /// The strip under the picture that fills as the short plays.
    pub strip: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct Karaoke {
    pub on: bool,
    /// The font's name, as the system knows it.
    pub font: String,
    pub size: u32,
}

impl Default for Karaoke {
    fn default() -> Self {
        Self {
            on: false,
            font: "Arial".into(),
            size: 60,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Effects {
    /// How far the picture pushes in over the short: 0.1 is a tenth.
    pub zoom: f64,
    /// Film grain, 0 to 40.
    pub grain: u32,
    pub vignette: bool,
    /// The picture tears for a moment where the sound hits.
    pub glitch: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct Tail {
    pub on: bool,
    pub seconds: f64,
}

impl Default for Tail {
    fn default() -> Self {
        Self {
            on: false,
            seconds: 1.2,
        }
    }
}

/// The still layers kilna drew, as files in one folder.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Layers {
    pub dir: String,
    /// The ground, the whole frame.
    pub ground: Option<String>,
    /// The words that stand still - the title, the call - on a clear frame.
    pub header: Option<String>,
    pub blots: Vec<Blot>,
    pub stamp: Option<Stamp>,
    pub sign: Option<SignLayer>,
}

/// One blot of the ground: where its corner stands and how it drifts.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Blot {
    pub file: String,
    pub x: f64,
    pub y: f64,
    /// How far it drifts either way, and how long one swing takes.
    pub ax: f64,
    pub ay: f64,
    pub px: f64,
    pub py: f64,
    pub phase: f64,
    /// How far it is thrown sideways on a hit.
    pub kick: f64,
}

/// The stamp, and its ghost in the second colour for a hit.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Stamp {
    pub file: String,
    pub ghost: Option<String>,
    pub x: f64,
    pub y: f64,
}

/// The sign at its largest, on a clear square - and the same with its eyes
/// shut, when it blinks.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct SignLayer {
    pub open: String,
    pub blink: Option<String>,
}

/// The words of the song, each with the second it is sung at in the time of
/// the recording they were timed on.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Words {
    pub lines: Vec<Vec<Word>>,
    /// The recording the words were timed on, when it is not the sound of
    /// the videos the short is cut from.
    pub audio: Option<String>,
    /// How many seconds of that recording come before a video's first
    /// second, when it was said rather than left to be measured.
    pub offset: Option<f64>,
}

/// Everything kilna hands over for a short.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Short {
    pub look: Look,
    pub layers: Layers,
    pub karaoke: Option<Words>,
}

/// One stretch, as it will be cut.
#[derive(Debug, Clone, PartialEq)]
pub struct Stretch {
    pub path: PathBuf,
    pub starts_at: f64,
    pub seconds: f64,
    pub has_sound: bool,
}

/// Everything a build is made of, settled before ffmpeg runs.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    pub stretches: Vec<Stretch>,
    pub look: Look,
    pub ground: PathBuf,
    pub header: Option<PathBuf>,
    pub blots: Vec<(PathBuf, Blot)>,
    pub stamp: Option<(PathBuf, Option<PathBuf>, Stamp)>,
    pub sign: Option<(PathBuf, Option<PathBuf>)>,
    pub cover: Option<PathBuf>,
    /// The file it ends as, and the one it is written to until it is done.
    pub output: PathBuf,
    pub partial: PathBuf,
}

impl Plan {
    /// How long the stretches run together.
    pub fn body(&self) -> f64 {
        self.stretches.iter().map(|one| one.seconds).sum()
    }

    /// How long the cover stands at the end.
    pub fn tail(&self) -> f64 {
        if self.cover.is_some() {
            self.look.tail.seconds
        } else {
            0.0
        }
    }

    /// Whether frames are piped in: a sign that moves, or bars.
    pub fn piped(&self) -> bool {
        self.sign.is_some() || self.bars()
    }

    fn bars(&self) -> bool {
        self.look.wave.on && self.look.wave.kind == WaveKind::Bars
    }
}

/// Settle what to build, and say plainly what stops it.
pub fn plan(subject: &Subject, settings: &Settings, tools: &Tools) -> Result<Plan, String> {
    let short = subject.short.as_ref().ok_or(
        "kilna handed over no look for the short: this plugin needs kilna 0.168 or later.",
    )?;
    if subject.shots.is_empty() {
        return Err("The short has no stretches to cut yet.".into());
    }
    let folder = subject
        .folder
        .as_deref()
        .ok_or("The short has no folder on disk to build into: set the media folder, or name the folder of its kind.")?;
    let mut stretches = Vec::new();
    for shot in &subject.shots {
        let path = shot.path.as_deref().ok_or_else(|| {
            format!(
                "\"{}\", which a stretch is cut from, has no finished file.",
                shot.source_title
            )
        })?;
        let path = PathBuf::from(path);
        let probe = ffmpeg::probe(tools, &path)?;
        if !probe.has_picture {
            return Err(format!("{} has no picture to cut from.", path.display()));
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

    let layers = &short.layers;
    let dir = Path::new(&layers.dir);
    let layer = |name: &str| -> Result<PathBuf, String> {
        let path = dir.join(name);
        if path.is_file() {
            Ok(path)
        } else {
            Err(format!(
                "the layer {} kilna drew for the short is not there",
                path.display()
            ))
        }
    };
    let ground = layer(
        layers
            .ground
            .as_deref()
            .ok_or("kilna handed over no ground for the short")?,
    )?;
    let look = short.look.clone();
    let header = layers.header.as_deref().map(layer).transpose()?;
    let blots = layers
        .blots
        .iter()
        .map(|blot| Ok((layer(&blot.file)?, blot.clone())))
        .collect::<Result<Vec<_>, String>>()?;
    let stamp = layers
        .stamp
        .as_ref()
        .map(|stamp| {
            Ok::<_, String>((
                layer(&stamp.file)?,
                stamp.ghost.as_deref().map(layer).transpose()?,
                stamp.clone(),
            ))
        })
        .transpose()?;
    let sign = match (&layers.sign, look.sign.on) {
        (Some(sign), true) => Some((
            layer(&sign.open)?,
            sign.blink.as_deref().map(layer).transpose()?,
        )),
        _ => None,
    };

    let folder = Path::new(folder);
    let output = next_output(&folder.join(&settings.output));
    let partial = partial_of(&output);
    let cover = if look.tail.on && look.tail.seconds > 0.0 {
        subject
            .cover
            .as_ref()
            .map(PathBuf::from)
            .filter(|path| path.is_file())
            .or_else(|| newest_picture(&folder.join(&settings.covers)))
    } else {
        None
    };
    Ok(Plan {
        stretches,
        look,
        ground,
        header,
        blots,
        stamp,
        sign,
        cover,
        output,
        partial,
    })
}

/// The next free `v{k}.mp4` in `dir`: a build never overwrites the one
/// before it, it lies down beside it.
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

/// Where a build is written until it is whole: a name no rule for `*.mp4`
/// matches, so one stopped halfway is never taken for the short's file.
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

/// A colour as ffmpeg reads one: `#FF2E63` is `0xFF2E63`. White for anything
/// that is not six hex digits.
fn colour(hex: &str) -> String {
    let digits = hex.trim_start_matches('#');
    if digits.len() == 6 && digits.chars().all(|c| c.is_ascii_hexdigit()) {
        format!("0x{}", digits.to_uppercase())
    } else {
        "0xFFFFFF".into()
    }
}

/// The same colour as three bytes.
fn rgb(hex: &str) -> [u8; 3] {
    let digits = colour(hex);
    let byte = |at: usize| u8::from_str_radix(&digits[at..at + 2], 16).unwrap_or(255);
    [byte(2), byte(4), byte(6)]
}

/// When the picture tears, as an expression that is 1 during a hit.
fn tear(hits: &[f64]) -> String {
    if hits.is_empty() {
        return "0".into();
    }
    hits.iter()
        .map(|at| format!("between(t\\,{at:.2}\\,{:.2})", at + 0.11))
        .collect::<Vec<_>>()
        .join("+")
}

/// A still picture as an input that lasts `seconds`.
fn still(args: &mut Vec<String>, path: &Path, seconds: f64) {
    args.extend([
        "-loop".into(),
        "1".into(),
        "-framerate".into(),
        FPS.to_string(),
        "-t".into(),
        format!("{seconds:.3}"),
        "-i".into(),
        path.to_string_lossy().into_owned(),
    ]);
}

/// The arguments ffmpeg is run with: the stretches as trimmed inputs, the
/// layers after them, and one graph that splices the stretches, lays the
/// frame out and sets the cover at the end. `hits` is where the sound hits;
/// `words` the name of the subtitle script, beside which ffmpeg is run.
pub fn arguments(
    plan: &Plan,
    settings: &Settings,
    hits: &[f64],
    words: Option<&str>,
) -> Vec<String> {
    let look = &plan.look;
    let body = plan.body();
    let tail = plan.tail();
    let count = plan.stretches.len();
    let glitch = look.effects.glitch && !hits.is_empty();
    let tear = tear(hits);
    let (a, b) = (colour(&look.accent), colour(&look.second));

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
    let mut next = count;
    let mut input = |args: &mut Vec<String>, path: &Path, seconds: f64| {
        still(args, path, seconds);
        next += 1;
        next - 1
    };
    let ground = input(&mut args, &plan.ground, body);
    let blots: Vec<usize> = plan
        .blots
        .iter()
        .map(|(path, _)| input(&mut args, path, body))
        .collect();
    let header = plan
        .header
        .as_ref()
        .map(|path| input(&mut args, path, body));
    let stamp = plan.stamp.as_ref().map(|(path, ghost, _)| {
        (
            input(&mut args, path, body),
            ghost.as_ref().map(|ghost| input(&mut args, ghost, body)),
        )
    });
    let cover = plan.cover.as_ref().map(|path| input(&mut args, path, tail));
    let pipe = plan.piped().then(|| {
        args.extend([
            "-f".into(),
            "rawvideo".into(),
            "-pix_fmt".into(),
            "rgba".into(),
            "-s".into(),
            format!("{WIDTH}x{}", WAVE_HEIGHT + SIGN_BOX),
            "-r".into(),
            FPS.to_string(),
            "-i".into(),
            "pipe:0".into(),
        ]);
        next += 1;
        next - 1
    });

    let mut graph: Vec<String> = Vec::new();
    // The stretches, each filling a doubled 16:9 frame, spliced in order.
    let mut parts = String::new();
    for (i, stretch) in plan.stretches.iter().enumerate() {
        graph.push(format!(
            "[{i}:v]fps={FPS},scale=2160:1216:force_original_aspect_ratio=increase,crop=2160:1216,setsar=1[s{i}]"
        ));
        graph.push(if stretch.has_sound {
            format!("[{i}:a]aresample=44100,aformat=channel_layouts=stereo[a{i}]")
        } else {
            format!(
                "anullsrc=r=44100:cl=stereo,atrim=duration={:.3}[a{i}]",
                stretch.seconds
            )
        });
        parts.push_str(&format!("[s{i}][a{i}]"));
    }
    graph.push(format!("{parts}concat=n={count}:v=1:a=1[src][snd]"));

    // The picture: pushed in about its centre on the doubled frame, so the
    // steps are half a pixel; then grain, the vignette and the tear.
    let mut picture = if look.effects.zoom > 0.0 {
        format!(
            "[src]zoompan=z='1+{:.3}*in/{:.1}':x='iw/2-iw/zoom/2':y='ih/2-ih/zoom/2':d=1:s={WIDTH}x{PICTURE_HEIGHT}:fps={FPS}",
            look.effects.zoom,
            body * f64::from(FPS)
        )
    } else {
        format!("[src]scale={WIDTH}:{PICTURE_HEIGHT}")
    };
    if look.effects.grain > 0 {
        picture.push_str(&format!(
            ",noise=alls={}:allf=t",
            look.effects.grain.min(40)
        ));
    }
    if look.effects.vignette {
        picture.push_str(",vignette=PI/4.6");
    }
    if glitch {
        picture.push_str(&format!(",rgbashift=rh=16:bh=-16:gv=3:enable='{tear}'"));
    }
    graph.push(format!("{picture}[fg]"));

    // The ground and its blots, which drift on slow sines and are thrown
    // sideways for the length of a hit.
    graph.push(format!("[{ground}:v]fps={FPS},format=yuv420p[g0]"));
    for (j, (blot_input, (_, blot))) in blots.iter().zip(&plan.blots).enumerate() {
        let place = if look.ground.motion {
            let kick = if glitch {
                format!("+{:.0}*({tear})", blot.kick)
            } else {
                String::new()
            };
            format!(
                "x='{:.0}+{:.1}*sin(2*PI*t/{:.2}+{:.2}){kick}':y='{:.0}+{:.1}*cos(2*PI*t/{:.2}+{:.2})'",
                blot.x,
                blot.ax,
                blot.px.max(0.1),
                blot.phase,
                blot.y,
                blot.ay,
                blot.py.max(0.1),
                blot.phase
            )
        } else {
            format!("x={:.0}:y={:.0}", blot.x, blot.y)
        };
        graph.push(format!("[{blot_input}:v]fps={FPS},format=rgba[bl{j}]"));
        graph.push(format!(
            "[g{j}][bl{j}]overlay={place}:format=auto[g{}]",
            j + 1
        ));
    }
    let grounded = format!("g{}", blots.len());
    if glitch {
        graph.push(format!(
            "[{grounded}]rgbashift=rh=22:bh=-22:enable='{tear}',format=yuv420p[bg]"
        ));
    } else {
        graph.push(format!("[{grounded}]format=yuv420p[bg]"));
    }
    graph.push(format!("[bg][fg]overlay=0:{PICTURE_TOP}[f0]"));
    let mut last = "f0".to_owned();
    let mut step = 0;
    let mut then = |graph: &mut Vec<String>, last: &mut String, filter: String| {
        step += 1;
        let out = format!("f{step}");
        graph.push(filter.replace("{in}", last).replace("{out}", &out));
        *last = out;
    };

    if look.wave.on && look.wave.strip {
        graph.push(format!("color=c={a}@0.22:size={WIDTH}x8:rate={FPS}[track]"));
        then(
            &mut graph,
            &mut last,
            format!("[{{in}}][track]overlay=0:{STRIP_TOP}:shortest=1[{{out}}]"),
        );
        graph.push(format!("color=c={a}:size={WIDTH}x8:rate={FPS}[fill]"));
        then(
            &mut graph,
            &mut last,
            format!(
                "[{{in}}][fill]overlay=x='-{WIDTH}+{WIDTH}*t/{body:.3}':y={STRIP_TOP}:shortest=1[{{out}}]"
            ),
        );
    }

    // The sound as a picture: bars come down the pipe above the sign; a wave
    // is drawn by ffmpeg from the sound itself.
    let mut sound = "snd".to_owned();
    let drawn = look.wave.on && !plan.bars();
    if let Some(pipe) = pipe {
        match (plan.bars(), plan.sign.is_some()) {
            (true, true) => graph.push(format!(
                "[{pipe}:v]split[p1][p2];[p1]crop={WIDTH}:{WAVE_HEIGHT}:0:0[wave];[p2]crop={SIGN_BOX}:{SIGN_BOX}:{}:{WAVE_HEIGHT}[sign]",
                (WIDTH as usize - SIGN_BOX) / 2
            )),
            (true, false) => graph.push(format!("[{pipe}:v]crop={WIDTH}:{WAVE_HEIGHT}:0:0[wave]")),
            (false, _) => graph.push(format!(
                "[{pipe}:v]crop={SIGN_BOX}:{SIGN_BOX}:{}:{WAVE_HEIGHT}[sign]",
                (WIDTH as usize - SIGN_BOX) / 2
            )),
        }
    }
    if drawn {
        let wave = match look.wave.kind {
            WaveKind::Lines => format!(
                "showwaves=s={WIDTH}x{WAVE_HEIGHT}:mode=p2p:rate={FPS}:colors={a}|{b}:scale=sqrt:draw=full,format=rgba"
            ),
            _ => format!(
                "aformat=channel_layouts=mono,showwaves=s={WIDTH}x{WAVE_HEIGHT}:mode=cline:rate={FPS}:colors={a}:scale=sqrt:draw=full,format=rgba"
            ),
        };
        graph.push(format!("[snd]asplit=2[heard][shown];[shown]{wave}[wave]"));
        sound = "heard".into();
    }
    if look.wave.on {
        then(
            &mut graph,
            &mut last,
            format!("[{{in}}][wave]overlay=0:{WAVE_TOP}:format=auto[{{out}}]"),
        );
    }

    if let Some(header) = header {
        graph.push(format!("[{header}:v]fps={FPS},format=rgba[hd]"));
        then(
            &mut graph,
            &mut last,
            "[{in}][hd]overlay=0:0:format=auto[{out}]".into(),
        );
    }
    if let Some(words) = words {
        then(
            &mut graph,
            &mut last,
            format!("[{{in}}]ass={words}[{{out}}]"),
        );
    }
    if let (Some((stamp_input, ghost_input)), Some((_, _, stamp))) = (stamp, &plan.stamp) {
        if let (Some(ghost), true) = (ghost_input, glitch) {
            graph.push(format!("[{ghost}:v]fps={FPS},format=rgba[gh]"));
            then(
                &mut graph,
                &mut last,
                format!(
                    "[{{in}}][gh]overlay=x={:.0}:y={:.0}:format=auto:enable='{tear}'[{{out}}]",
                    stamp.x + 20.0,
                    stamp.y - 6.0
                ),
            );
        }
        let x = if glitch {
            format!("'{:.0}-18*({tear})'", stamp.x)
        } else {
            format!("{:.0}", stamp.x)
        };
        graph.push(format!("[{stamp_input}:v]fps={FPS},format=rgba[st]"));
        then(
            &mut graph,
            &mut last,
            format!(
                "[{{in}}][st]overlay=x={x}:y={:.0}:format=auto[{{out}}]",
                stamp.y
            ),
        );
    }
    if plan.sign.is_some() {
        then(
            &mut graph,
            &mut last,
            format!("[{{in}}][sign]overlay=x=(W-w)/2:y={SIGN_TOP}:format=auto[{{out}}]"),
        );
    }
    graph.push(format!(
        "[{last}]setsar=1,trim=duration={body:.3},setpts=PTS-STARTPTS,format=yuv420p[main]"
    ));
    graph.push(format!(
        "[{sound}]atrim=duration={body:.3},asetpts=PTS-STARTPTS[said]"
    ));

    // The cover pulls back and rests whole for its last quarter of a second:
    // a title set edge to edge is never cut.
    if let Some(cover) = cover {
        graph.push(format!(
            "[{cover}:v]fps={FPS},scale=2160:3840:force_original_aspect_ratio=decrease,pad=2160:3840:(ow-iw)/2:(oh-ih)/2:color=black,\
             zoompan=z='1+0.05*pow(max(0\\,1-in/{:.1})\\,2)':x='iw/2-iw/zoom/2':y='ih/2-ih/zoom/2':d=1:s={WIDTH}x{HEIGHT}:fps={FPS},\
             setsar=1,trim=duration={tail:.3},setpts=PTS-STARTPTS,format=yuv420p[rest]",
            ((tail - 0.25).max(0.05)) * f64::from(FPS)
        ));
        graph.push(format!(
            "anullsrc=r=44100:cl=stereo,atrim=duration={tail:.3}[hush]"
        ));
        graph.push("[main][said][rest][hush]concat=n=2:v=1:a=1[outv][outa]".into());
    } else {
        graph.push("[main]null[outv];[said]anull[outa]".into());
    }

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
        "-r".into(),
        FPS.to_string(),
        "-c:a".into(),
        "aac".into(),
        "-b:a".into(),
        "192k".into(),
        "-ar".into(),
        "44100".into(),
        "-movflags".into(),
        "+faststart".into(),
        "-f".into(),
        "mp4".into(),
        plan.partial.to_string_lossy().into_owned(),
    ]);
    args
}

/// Draw `sign` into `canvas` - `width` across, the sign's square starting at
/// row `top` - `side` pixels across and turned by `degrees` about the middle
/// of its square.
fn draw_sign(canvas: &mut [u8], width: usize, top: usize, sign: &Picture, side: f64, degrees: f64) {
    let left = (width - SIGN_BOX) / 2;
    let half = SIGN_BOX as f64 / 2.0;
    let (sin, cos) = degrees.to_radians().sin_cos();
    let scale = sign.width as f64 / side.max(1.0);
    // The sign at one pixel of its picture, with the colour weighed by how
    // solid it is, so a clear neighbour does not darken an edge.
    let at = |x: i64, y: i64| -> [f64; 4] {
        if x < 0 || y < 0 || x >= sign.width as i64 || y >= sign.height as i64 {
            return [0.0; 4];
        }
        let i = (y as usize * sign.width + x as usize) * 4;
        let alpha = f64::from(sign.rgba[i + 3]) / 255.0;
        [
            f64::from(sign.rgba[i]) * alpha,
            f64::from(sign.rgba[i + 1]) * alpha,
            f64::from(sign.rgba[i + 2]) * alpha,
            alpha,
        ]
    };
    for row in 0..SIGN_BOX {
        for column in 0..SIGN_BOX {
            let (dx, dy) = (column as f64 + 0.5 - half, row as f64 + 0.5 - half);
            let sx = (dx * cos - dy * sin) * scale + sign.width as f64 / 2.0 - 0.5;
            let sy = (dx * sin + dy * cos) * scale + sign.height as f64 / 2.0 - 0.5;
            if sx < -1.0 || sy < -1.0 || sx > sign.width as f64 || sy > sign.height as f64 {
                continue;
            }
            let (x0, y0) = (sx.floor(), sy.floor());
            let (fx, fy) = (sx - x0, sy - y0);
            let (x0, y0) = (x0 as i64, y0 as i64);
            let (p00, p10, p01, p11) = (
                at(x0, y0),
                at(x0 + 1, y0),
                at(x0, y0 + 1),
                at(x0 + 1, y0 + 1),
            );
            let mut mixed = [0.0f64; 4];
            for (k, value) in mixed.iter_mut().enumerate() {
                *value = (p00[k] * (1.0 - fx) + p10[k] * fx) * (1.0 - fy)
                    + (p01[k] * (1.0 - fx) + p11[k] * fx) * fy;
            }
            if mixed[3] <= 0.0 {
                continue;
            }
            let i = ((top + row) * width + left + column) * 4;
            for k in 0..3 {
                canvas[i + k] = (mixed[k] / mixed[3]).round().clamp(0.0, 255.0) as u8;
            }
            canvas[i + 3] = (mixed[3] * 255.0).round().clamp(0.0, 255.0) as u8;
        }
    }
}

/// Draw the bars into the top band of `canvas`: the bass in the middle, the
/// highs at both edges, each bar growing up and down from the centre line.
fn draw_bars(canvas: &mut [u8], width: usize, levels: &[f32], ink: [u8; 3]) {
    const PITCH: usize = 19;
    const BAR: usize = 12;
    let middle = WAVE_HEIGHT / 2;
    for (k, level) in levels.iter().enumerate() {
        let half = (4.0 + level * (middle as f32 - 8.0)) as usize;
        for side in [-1i64, 1] {
            let x = width as i64 / 2 + side * (k * PITCH + PITCH / 2) as i64 - BAR as i64 / 2;
            if x < 0 || x as usize + BAR > width {
                continue;
            }
            for row in middle.saturating_sub(half)..(middle + half).min(WAVE_HEIGHT) {
                for column in x as usize..x as usize + BAR {
                    let i = (row * width + column) * 4;
                    canvas[i..i + 3].copy_from_slice(&ink);
                    canvas[i + 3] = 255;
                }
            }
        }
    }
}

/// What the piped frames are drawn from.
struct Moving {
    seconds: f64,
    level: Vec<f32>,
    bands: Option<Vec<Vec<f32>>>,
    ink: [u8; 3],
    sign: Option<(Picture, Option<Picture>)>,
    size: f64,
    pulse: f64,
    sway: f64,
}

impl Moving {
    /// Write every frame of the short's body to `out`.
    fn write(&self, out: &mut dyn Write) -> std::io::Result<()> {
        let width = WIDTH as usize;
        let frames = (self.seconds * f64::from(FPS)).ceil() as usize;
        let mut canvas = vec![0u8; width * (WAVE_HEIGHT + SIGN_BOX) * 4];
        for frame in 0..frames {
            canvas.fill(0);
            let t = frame as f64 / f64::from(FPS);
            if let Some(levels) = self
                .bands
                .as_ref()
                .and_then(|bands| bands.get(frame.min(bands.len().saturating_sub(1))))
            {
                draw_bars(&mut canvas, width, levels, self.ink);
            }
            if let Some((open, blink)) = &self.sign {
                let level = self
                    .level
                    .get(frame.min(self.level.len().saturating_sub(1)))
                    .copied()
                    .unwrap_or(0.0);
                let side = (self.size * (1.0 + self.pulse * f64::from(level))).min(SIGN_BOX as f64);
                let shut = t > 1.0 && t % 3.9 < 0.14;
                let face = blink.as_ref().filter(|_| shut).unwrap_or(open);
                let turn = self.sway * (2.0 * std::f64::consts::PI * 0.45 * t).sin();
                draw_sign(&mut canvas, width, WAVE_HEIGHT, face, side, turn);
            }
            out.write_all(&canvas)?;
        }
        Ok(())
    }
}

/// Lay the words of the song under the stretches: for each, how far its
/// video runs behind the recording the words were timed on, and the lines
/// sung in it.
fn worded(plan: &Plan, words: &Words, tools: &Tools) -> Result<Vec<karaoke::Stretch>, String> {
    let recording = words
        .audio
        .as_deref()
        .map(Path::new)
        .filter(|path| path.is_file());
    let whole = match (words.offset, recording) {
        (None, Some(path)) => Some(ffmpeg::sound(tools, path, listen::COARSE, None)?),
        _ => None,
    };
    let mut offsets: Vec<(PathBuf, f64)> = Vec::new();
    let mut out = Vec::new();
    let mut base = 0.0;
    for stretch in &plan.stretches {
        let offset = match (words.offset, &whole) {
            (Some(said), _) => said,
            (None, None) => 0.0,
            (None, Some(whole)) => {
                if let Some((_, known)) = offsets.iter().find(|(path, _)| *path == stretch.path) {
                    *known
                } else {
                    // A stretch from inside the song, past any opening the
                    // video adds.
                    let length = ffmpeg::length(tools, &stretch.path)?;
                    let from = if length >= 90.0 { 40.0 } else { 0.0 };
                    let part = ffmpeg::sound(
                        tools,
                        &stretch.path,
                        listen::COARSE,
                        Some((from, length.min(40.0))),
                    )?;
                    let found = listen::offset(whole, &part, from).unwrap_or(0.0);
                    offsets.push((stretch.path.clone(), found));
                    found
                }
            }
        };
        let shift = offset + stretch.starts_at;
        out.push(karaoke::Stretch {
            lines: karaoke::fragment(&words.lines, shift, stretch.seconds),
            shift,
            base,
        });
        base += stretch.seconds;
    }
    Ok(out)
}

/// Build the short: settle the plan, listen to its sound, run ffmpeg once
/// with progress, and put the whole file under its name. Answers with the
/// file.
pub fn build(subject: &Subject, settings: &Settings, tools: &Tools) -> Result<PathBuf, String> {
    protocol::progress(0.0, "Reading the stretches");
    let plan = plan(subject, settings, tools)?;
    let out_dir = plan
        .output
        .parent()
        .ok_or("the short has no folder to go into")?;
    std::fs::create_dir_all(out_dir)
        .map_err(|cause| format!("could not make {}: {cause}", out_dir.display()))?;

    let work = std::env::temp_dir().join(format!("kilna-plugin-cut-{}", std::process::id()));
    std::fs::create_dir_all(&work)
        .map_err(|cause| format!("could not make {}: {cause}", work.display()))?;
    let result = (|| {
        protocol::progress(0.02, "Listening to the short");
        let mut samples: Vec<f32> = Vec::new();
        for stretch in &plan.stretches {
            if stretch.has_sound {
                samples.extend(ffmpeg::sound(
                    tools,
                    &stretch.path,
                    listen::RATE,
                    Some((stretch.starts_at, stretch.seconds)),
                )?);
            } else {
                samples.extend(std::iter::repeat_n(
                    0.0,
                    (stretch.seconds * f64::from(listen::RATE)) as usize,
                ));
            }
        }
        let body = plan.body();
        let heard = listen::loudness(&samples, listen::RATE, FPS);
        let hits = if plan.look.effects.glitch {
            listen::hits(&heard.rms, FPS, body, 10, 2.2)
        } else {
            Vec::new()
        };

        let words = match subject
            .short
            .as_ref()
            .and_then(|short| short.karaoke.as_ref())
            .filter(|words| plan.look.karaoke.on && !words.lines.is_empty())
        {
            Some(words) => {
                protocol::progress(0.05, "Laying the words under the sound");
                let stretches = worded(&plan, words, tools)?;
                if stretches.iter().all(|one| one.lines.is_empty()) {
                    None
                } else {
                    let script = karaoke::script(
                        &stretches,
                        &Lettering {
                            font: plan.look.karaoke.font.clone(),
                            size: plan.look.karaoke.size,
                            sung: plan.look.accent.clone(),
                            top: WORDS_TOP,
                        },
                    );
                    std::fs::write(work.join("words.ass"), script)
                        .map_err(|cause| format!("could not write the words: {cause}"))?;
                    Some("words.ass")
                }
            }
            None => None,
        };

        let moving = if plan.piped() {
            let sign = match &plan.sign {
                Some((open, blink)) => Some((
                    ffmpeg::picture(tools, open)?,
                    blink
                        .as_ref()
                        .map(|blink| ffmpeg::picture(tools, blink))
                        .transpose()?,
                )),
                None => None,
            };
            Some(Moving {
                seconds: body,
                bands: plan
                    .bars()
                    .then(|| listen::bands(&samples, listen::RATE, FPS, BARS)),
                level: heard.level,
                ink: rgb(&plan.look.accent),
                sign,
                size: plan.look.sign.size,
                pulse: plan.look.sign.pulse,
                sway: plan.look.sign.sway,
            })
        } else {
            None
        };

        let args = arguments(&plan, settings, &hits, words);
        let total = (body + plan.tail()).max(0.001);
        let name = plan
            .output
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let mut said = -1.0;
        let feed: Option<ffmpeg::Feed<'_>> = moving.as_ref().map(|moving| {
            Box::new(move |out: &mut dyn Write| moving.write(out)) as ffmpeg::Feed<'_>
        });
        ffmpeg::run(tools, &args, &work, feed, &mut |seconds| {
            let share = (seconds / total).clamp(0.0, 1.0);
            // A line per whole percent: kilna draws a bar, not a log.
            if share - said >= 0.01 {
                said = share;
                protocol::progress(
                    0.08 + share * 0.92,
                    &format!(
                        "Building {name}: {:.0} of {:.0} s",
                        seconds.min(total),
                        total
                    ),
                );
            }
        })?;
        std::fs::rename(&plan.partial, &plan.output).map_err(|cause| {
            format!(
                "could not put the short at {}: {cause}",
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

    fn a_plan(stretches: usize, look: Look) -> Plan {
        Plan {
            stretches: (0..stretches)
                .map(|i| Stretch {
                    path: PathBuf::from(format!("D:/media/clip{i}.mp4")),
                    starts_at: 12.0 + i as f64 * 30.0,
                    seconds: 8.5,
                    has_sound: i == 0,
                })
                .collect(),
            look,
            ground: PathBuf::from("T:/layers/ground.png"),
            header: Some(PathBuf::from("T:/layers/header.png")),
            blots: vec![(
                PathBuf::from("T:/layers/blot-0.png"),
                Blot {
                    file: "blot-0.png".into(),
                    x: -565.0,
                    y: -216.0,
                    ax: 23.2,
                    ay: 23.1,
                    px: 8.37,
                    py: 11.39,
                    phase: 5.08,
                    kick: -40.6,
                },
            )],
            stamp: Some((
                PathBuf::from("T:/layers/stamp.png"),
                Some(PathBuf::from("T:/layers/stamp-ghost.png")),
                Stamp {
                    file: "stamp.png".into(),
                    ghost: Some("stamp-ghost.png".into()),
                    x: 285.0,
                    y: 430.0,
                },
            )),
            sign: Some((PathBuf::from("T:/layers/sign.png"), None)),
            cover: Some(PathBuf::from("D:/media/covers/v1.jpeg")),
            output: PathBuf::from("D:/media/shorts/v2/video/v3.mp4"),
            partial: PathBuf::from("D:/media/shorts/v2/video/v3.mp4.part"),
        }
    }

    fn whole_look() -> Look {
        Look {
            accent: "#FF2E63".into(),
            second: "#D3EC3F".into(),
            sign: Sign {
                on: true,
                ..Sign::default()
            },
            ground: Ground { motion: true },
            wave: Wave {
                on: true,
                kind: WaveKind::Solid,
                strip: true,
            },
            karaoke: Karaoke {
                on: true,
                ..Karaoke::default()
            },
            effects: Effects {
                zoom: 0.1,
                grain: 7,
                vignette: true,
                glitch: true,
            },
            tail: Tail {
                on: true,
                seconds: 1.2,
            },
        }
    }

    fn graph_of(args: &[String]) -> &str {
        &args[args
            .iter()
            .position(|arg| arg == "-filter_complex")
            .unwrap()
            + 1]
    }

    /// The next build lies beside the last: `v3.mp4` after `v1` and `v2`, and
    /// a half-written one does not count.
    #[test]
    fn a_build_never_overwrites_the_one_before() {
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

    /// Every block of the look is in the graph: the stretches spliced, the
    /// picture pushed in and torn on the hits, the blots drifting, the strip,
    /// the wave, the header, the words, the stamp with its ghost, the sign
    /// from the pipe, and the cover at the end.
    #[test]
    fn a_whole_look_is_laid_out_in_one_pass() {
        let plan = a_plan(2, whole_look());
        let args = arguments(&plan, &Settings::default(), &[3.0, 6.5], Some("words.ass"));
        let graph = graph_of(&args);

        assert_eq!(
            args.iter().filter(|arg| *arg == "-i").count(),
            9,
            "two stretches, the ground, a blot, the header, the stamp, its ghost, the cover, the pipe"
        );
        assert!(
            args.windows(2)
                .any(|pair| pair[0] == "-ss" && pair[1] == "42.000")
        );
        assert!(graph.contains("[s0][a0][s1][a1]concat=n=2:v=1:a=1[src][snd]"));
        assert!(
            graph.contains("anullsrc=r=44100:cl=stereo,atrim=duration=8.500[a1]"),
            "a donor with no sound plays silence for its stretch"
        );
        assert!(graph.contains("zoompan=z='1+0.100*in/510.0'"));
        assert!(graph.contains("noise=alls=7:allf=t,vignette=PI/4.6,rgbashift=rh=16:bh=-16:gv=3:enable='between(t\\,3.00\\,3.11)+between(t\\,6.50\\,6.61)'"));
        assert!(graph.contains("overlay=x='-565+23.2*sin(2*PI*t/8.37+5.08)+-41*(between("));
        assert!(graph.contains("overlay=x='-1080+1080*t/17.000':y=1144"));
        assert!(graph.contains("showwaves=s=1080x220:mode=cline"));
        assert!(graph.contains("ass=words.ass"));
        assert!(
            graph.contains("overlay=x=305:y=424:format=auto:enable='between("),
            "the ghost, on a hit"
        );
        assert!(graph.contains("overlay=x='285-18*(between("));
        assert!(graph.contains("[8:v]crop=420:420:330:220[sign]"));
        assert!(graph.contains("overlay=x=(W-w)/2:y=1505"));
        assert!(graph.ends_with("[main][said][rest][hush]concat=n=2:v=1:a=1[outv][outa]"));
        assert!(
            args.windows(2)
                .any(|pair| pair[0] == "-i" && pair[1] == "pipe:0")
        );
        assert_eq!(args.last().unwrap(), "D:/media/shorts/v2/video/v3.mp4.part");
    }

    /// A look with everything off is the stretches over the ground and
    /// nothing else: no pipe, no tear, no cover.
    #[test]
    fn what_the_look_leaves_out_is_not_drawn() {
        let mut plan = a_plan(1, Look::default());
        plan.sign = None;
        plan.stamp = None;
        plan.header = None;
        plan.cover = None;
        let args = arguments(&plan, &Settings::default(), &[3.0], None);
        let graph = graph_of(&args);

        assert!(!plan.piped());
        assert_eq!(args.iter().filter(|arg| *arg == "-i").count(), 3);
        for absent in [
            "rgbashift",
            "zoompan",
            "showwaves",
            "ass=",
            "[sign]",
            "noise",
            "sin(",
        ] {
            assert!(!graph.contains(absent), "{absent} in {graph}");
        }
        assert!(
            graph.contains("overlay=x=-565:y=-216"),
            "a blot stands still"
        );
        assert!(graph.ends_with("[main]null[outv];[said]anull[outa]"));
    }

    /// Bars come down the pipe above the sign, and no wave is drawn from the
    /// sound.
    #[test]
    fn bars_are_piped_in() {
        let mut look = whole_look();
        look.wave.kind = WaveKind::Bars;
        let mut plan = a_plan(1, look);
        plan.sign = None;
        let args = arguments(&plan, &Settings::default(), &[], None);
        let graph = graph_of(&args);

        assert!(plan.piped());
        assert!(graph.contains(":v]crop=1080:220:0:0[wave]"));
        assert!(!graph.contains("showwaves"));
        assert!(
            !graph.contains("rgbashift"),
            "nothing tears where nothing hits"
        );
    }

    /// The sign is drawn in the middle of its square, at the size asked.
    #[test]
    fn a_sign_is_drawn_at_its_size() {
        let sign = Picture {
            width: 4,
            height: 4,
            rgba: [200, 10, 10, 255].repeat(16),
        };
        let width = WIDTH as usize;
        let mut canvas = vec![0u8; width * (WAVE_HEIGHT + SIGN_BOX) * 4];

        draw_sign(&mut canvas, width, WAVE_HEIGHT, &sign, 100.0, 0.0);

        let pixel = |x: usize, y: usize| {
            let i = ((WAVE_HEIGHT + y) * width + (width - SIGN_BOX) / 2 + x) * 4;
            canvas[i..i + 4].to_vec()
        };
        assert_eq!(
            pixel(210, 210),
            vec![200, 10, 10, 255],
            "solid in the middle"
        );
        assert_eq!(pixel(210, 100)[3], 0, "clear above a sign 100 across");
        assert_eq!(pixel(180, 210)[3], 255);
        assert_eq!(pixel(130, 210)[3], 0);
    }

    /// Bars stand as high as their bands, mirrored about the middle.
    #[test]
    fn bars_grow_from_the_centre_line() {
        let width = WIDTH as usize;
        let mut canvas = vec![0u8; width * (WAVE_HEIGHT + SIGN_BOX) * 4];
        let mut levels = vec![0.0f32; BARS];
        levels[0] = 1.0;

        draw_bars(&mut canvas, width, &levels, [1, 2, 3]);

        let solid = |x: usize, y: usize| canvas[(y * width + x) * 4 + 3] == 255;
        assert!(
            solid(width / 2 + 9, 10),
            "the bass, tall, right of the middle"
        );
        assert!(solid(width / 2 - 10, 10), "and left of it");
        assert!(!solid(width / 2 + 28, 10), "the next band is low");
        assert!(solid(width / 2 + 28, WAVE_HEIGHT / 2));
    }

    #[test]
    fn a_colour_reads_for_ffmpeg_and_as_bytes() {
        assert_eq!(colour("#ff2e63"), "0xFF2E63");
        assert_eq!(colour("pink"), "0xFFFFFF");
        assert_eq!(rgb("#FF2E63"), [255, 46, 99]);
    }
}
