//! Where a song's parts begin, and which of them are worth a short.
//!
//! A person picking a stretch by ear listens for two things: where a part
//! starts - a short that opens mid-phrase sounds cut, not made - and which
//! part is the one that carries the song. The first is heard as a change: the
//! notes that sound turn into other notes. The second, most often, is the
//! part that is loud and comes back - the chorus.
//!
//! So the song is heard as half-second frames, each with its loudness and its
//! chroma - how much of each of the twelve notes sounds, whatever the octave.
//! A part begins where the frames before a moment are alike among themselves
//! and unlike the frames after it (novelty on the self-similarity of the
//! chroma). A part is worth a short when it is loud and something like it
//! sounds again elsewhere. Nothing here decides: kilna draws the beginnings on
//! the track for edges to land on and offers the parts, each with its reason,
//! and the person picks.

use std::io::Read;
use std::path::Path;
use std::process::Stdio;

use serde::Serialize;

use crate::ffmpeg::{self, Tools};
use crate::fft;

/// The rate a song is heard at: the notes that make a chord are below it.
pub const RATE: u32 = 11025;
/// Seconds a frame stands for.
const FRAME: f64 = 0.5;
/// Samples a frame's spectrum is read from: a third of a second, enough to
/// tell neighbouring notes apart in the low octaves.
const WINDOW: usize = 4096;
/// How long a part offered for a short may run, and what it aims at: the
/// stretches people take run 20 to 40 seconds.
const SHORTEST: f64 = 18.0;
const LONGEST: f64 = 45.0;
const TARGET: f64 = 30.0;
/// How many parts are offered for one video.
const MOST: usize = 5;
/// How far apart two beginnings must be: a part shorter than this is a fill.
const SPACING: f64 = 4.0;
/// How far back something must lie to count as the part coming back rather
/// than the part going on.
const LEAST_LAG: f64 = 10.0;
/// How strong a turn must be, as a share of the strongest in the song, to be
/// a beginning: below it is a held chord heard through rounding, not a part.
const FAINTEST: f32 = 0.1;

/// A part of a song offered for a short.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Part {
    pub starts_at: f64,
    pub ends_at: f64,
    /// Why it is offered, as a key kilna words in the person's language:
    /// `chorus` (loud, and it comes back), `loudest`, `repeats`, `lift`
    /// (louder at once after a quieter stretch), `part`.
    pub reason: &'static str,
    /// How strongly, from 0 to 1, for the order they are offered in.
    pub score: f64,
}

/// What is heard in one video: where its parts begin, and the parts worth a
/// short, best first.
#[derive(Debug, Clone, PartialEq)]
pub struct Map {
    pub boundaries: Vec<f64>,
    pub parts: Vec<Part>,
}

/// The song as frames.
pub struct Frames {
    /// Loudness of each frame, in dB below full scale.
    pub loud: Vec<f32>,
    /// The twelve notes of each frame, as a unit vector.
    pub chroma: Vec<[f32; 12]>,
}

/// The sound of a file as mono samples at [`RATE`], read through ffmpeg.
pub fn decode(tools: &Tools, path: &Path) -> Result<Vec<f32>, String> {
    let mut child = ffmpeg::command(&tools.ffmpeg)
        .args(["-v", "error", "-nostdin", "-i"])
        .arg(path)
        .args([
            "-vn",
            "-ac",
            "1",
            "-ar",
            &RATE.to_string(),
            "-f",
            "f32le",
            "-",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|cause| format!("could not start ffmpeg: {cause}"))?;
    let mut bytes = Vec::new();
    if let Some(mut out) = child.stdout.take() {
        out.read_to_end(&mut bytes)
            .map_err(|cause| format!("could not read the sound of {}: {cause}", path.display()))?;
    }
    let output = child
        .wait_with_output()
        .map_err(|cause| format!("ffmpeg did not finish: {cause}"))?;
    if !output.status.success() {
        let said = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "ffmpeg could not read the sound of {}: {}",
            path.display(),
            said.trim()
        ));
    }
    Ok(bytes
        .chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect())
}

/// Hear samples at [`RATE`] as frames of [`FRAME`] seconds.
pub fn frames(samples: &[f32]) -> Frames {
    let hop = (f64::from(RATE) * FRAME) as usize;
    let count = samples.len().saturating_sub(WINDOW) / hop;
    let hann: Vec<f32> = (0..WINDOW)
        .map(|i| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / (WINDOW - 1) as f32).cos())
        .collect();
    // Which note each bin of the spectrum is, for the bins a chord sounds in.
    let notes: Vec<Option<usize>> = (0..WINDOW / 2)
        .map(|bin| {
            let hz = bin as f64 * f64::from(RATE) / WINDOW as f64;
            (80.0..5000.0).contains(&hz).then(|| {
                let semitones = (12.0 * (hz / 440.0).log2()).round() as i64;
                semitones.rem_euclid(12) as usize
            })
        })
        .collect();

    let mut loud = Vec::with_capacity(count);
    let mut chroma = Vec::with_capacity(count);
    for i in 0..count {
        let chunk = &samples[i * hop..i * hop + WINDOW];
        let power: f32 = chunk.iter().map(|s| s * s).sum::<f32>() / WINDOW as f32;
        loud.push(10.0 * (power + 1e-12).log10());
        let windowed: Vec<f32> = chunk.iter().zip(&hann).map(|(s, w)| s * w).collect();
        let mut notes_here = [0.0f32; 12];
        for (bin, magnitude) in fft::magnitudes(&windowed).into_iter().enumerate() {
            if let Some(note) = notes[bin] {
                notes_here[note] += magnitude;
            }
        }
        let norm = notes_here.iter().map(|v| v * v).sum::<f32>().sqrt() + 1e-9;
        chroma.push(notes_here.map(|v| v / norm));
    }
    Frames { loud, chroma }
}

/// Where the parts begin and which are worth a short.
pub fn map(frames: &Frames) -> Map {
    let n = frames.loud.len();
    if n < frames_of(SHORTEST) {
        return Map {
            boundaries: Vec::new(),
            parts: Vec::new(),
        };
    }
    let alike = similarity(&frames.chroma);
    let change = novelty(&alike, frames_of(SPACING));
    let mut bounds = vec![0];
    bounds.extend(peaks(&change, frames_of(SPACING), faintest()));
    bounds.push(n);
    bounds.sort_unstable();
    bounds.dedup();

    let target = frames_of(TARGET);
    let (quiet, loudest) = (
        percentile(&frames.loud, 0.10),
        percentile(&frames.loud, 0.95),
    );
    let loudness = |from: usize, to: usize| -> f32 {
        let to = to.min(n);
        if to <= from {
            return quiet;
        }
        frames.loud[from..to].iter().sum::<f32>() / (to - from) as f32
    };

    // Every beginning, run to the beginning nearest the length aimed at.
    let mut found: Vec<(Part, f32, f32, f32)> = Vec::new();
    for &start in &bounds[..bounds.len() - 1] {
        let ends: Vec<usize> = bounds
            .iter()
            .copied()
            .filter(|end| *end > start)
            .filter(|end| (frames_of(SHORTEST)..=frames_of(LONGEST)).contains(&(end - start)))
            .collect();
        let end = ends
            .into_iter()
            .min_by_key(|end| (*end as i64 - start as i64 - target as i64).abs())
            .unwrap_or((start + target).min(n));
        if end - start < frames_of(SHORTEST) {
            continue;
        }
        let loud = ((loudness(start, end) - quiet) / (loudest - quiet + 1e-6)).clamp(0.0, 1.0);
        let comes_back = repetition(&alike, start, end, frames_of(LEAST_LAG));
        // A part that fades out ends a short on nothing.
        let tail = frames_of(4.0).min(end - start);
        let fades = loudness(end - tail, end) < loudness(start, end) - 6.0;
        let before = loudness(start.saturating_sub(frames_of(6.0)), start);
        let lift =
            start >= frames_of(6.0) && loudness(start, start + frames_of(6.0)) - before >= 6.0;
        found.push((
            Part {
                starts_at: seconds_of(start),
                ends_at: seconds_of(end),
                reason: if lift { "lift" } else { "part" },
                score: 0.0,
            },
            loud,
            comes_back,
            if fades { 0.3 } else { 0.0 },
        ));
    }
    // How much each comes back, against the others of the same song: chroma
    // is alike across most of any song, so only the difference says.
    let (least_back, most_back) = found.iter().fold((f32::MAX, f32::MIN), |(lo, hi), c| {
        (lo.min(c.2), hi.max(c.2))
    });
    for candidate in &mut found {
        candidate.2 = if most_back > least_back {
            (candidate.2 - least_back) / (most_back - least_back)
        } else {
            0.0
        };
        let score = 0.5 * candidate.1 + 0.5 * candidate.2 - candidate.3;
        candidate.0.score = f64::from(score.clamp(0.0, 1.0));
        candidate.3 = score;
    }
    found.sort_by(|a, b| b.3.total_cmp(&a.3));

    // The best that do not lie over one another, beyond a few seconds.
    let mut chosen: Vec<(Part, f32, f32, f32)> = Vec::new();
    for candidate in found {
        let apart = chosen.iter().all(|(other, ..)| {
            candidate.0.ends_at <= other.starts_at + 5.0
                || candidate.0.starts_at >= other.ends_at - 5.0
        });
        if apart {
            chosen.push(candidate);
        }
        if chosen.len() == MOST {
            break;
        }
    }
    let loudest_chosen = chosen.iter().map(|c| c.1).fold(f32::MIN, f32::max);
    let parts = chosen
        .into_iter()
        .map(|(mut part, loud, comes_back, _)| {
            part.reason = if comes_back >= 0.7 && loud >= 0.6 {
                "chorus"
            } else if loud >= loudest_chosen {
                "loudest"
            } else if comes_back >= 0.7 {
                "repeats"
            } else {
                part.reason
            };
            part
        })
        .collect();
    Map {
        boundaries: bounds.iter().map(|b| seconds_of(*b)).collect(),
        parts,
    }
}

fn frames_of(seconds: f64) -> usize {
    (seconds / FRAME).round() as usize
}

fn seconds_of(frame: usize) -> f64 {
    frame as f64 * FRAME
}

fn percentile(values: &[f32], share: f64) -> f32 {
    let mut sorted = values.to_vec();
    sorted.sort_by(f32::total_cmp);
    sorted[((sorted.len() - 1) as f64 * share).round() as usize]
}

/// How alike each frame is to each other frame: the cosine of their notes.
fn similarity(chroma: &[[f32; 12]]) -> Vec<Vec<f32>> {
    chroma
        .iter()
        .map(|a| {
            chroma
                .iter()
                .map(|b| a.iter().zip(b).map(|(x, y)| x * y).sum())
                .collect()
        })
        .collect()
}

/// How much each frame is a turn: the frames before it alike among
/// themselves and unlike the frames after it. A checkerboard laid along the
/// diagonal of the self-similarity, softened at its corners.
fn novelty(alike: &[Vec<f32>], half: usize) -> Vec<f32> {
    let n = alike.len();
    let size = 2 * half;
    let taper: Vec<f32> = (0..size)
        .map(|i| {
            let x = (i as f32 - half as f32 + 0.5) / (half as f32 / 2.0);
            (-0.5 * x * x).exp()
        })
        .collect();
    let at = |i: isize, j: isize| -> f32 {
        let clamp = |v: isize| v.clamp(0, n as isize - 1) as usize;
        alike[clamp(i)][clamp(j)]
    };
    (0..n)
        .map(|centre| {
            let mut sum = 0.0;
            for a in 0..size {
                for b in 0..size {
                    let sign = if (a < half) == (b < half) { 1.0 } else { -1.0 };
                    let i = centre as isize - half as isize + a as isize;
                    let j = centre as isize - half as isize + b as isize;
                    sum += sign * taper[a] * taper[b] * at(i, j);
                }
            }
            sum
        })
        .collect()
}

/// The share of the strongest turn a beginning must reach;
/// `KILNA_PLUGIN_CUT_FAINTEST` overrides it, for measuring.
fn faintest() -> f32 {
    std::env::var("KILNA_PLUGIN_CUT_FAINTEST")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(FAINTEST)
}

/// The frames where the curve peaks above `share` of its highest, strongest
/// first kept, no two closer than `spacing` frames; in order.
fn peaks(curve: &[f32], spacing: usize, share: f32) -> Vec<usize> {
    let highest = curve.iter().copied().fold(0.0f32, f32::max);
    let floor = highest * share;
    let mut found: Vec<usize> = (1..curve.len().saturating_sub(1))
        .filter(|&i| {
            curve[i] > floor.max(0.0) && curve[i] >= curve[i - 1] && curve[i] >= curve[i + 1]
        })
        .collect();
    found.sort_by(|a, b| curve[*b].total_cmp(&curve[*a]));
    let mut kept: Vec<usize> = Vec::new();
    for i in found {
        if kept.iter().all(|j| i.abs_diff(*j) >= spacing) {
            kept.push(i);
        }
    }
    kept.sort_unstable();
    kept
}

/// How closely the frames from `start` to `end` sound again elsewhere in the
/// song: the best mean similarity along a diagonal of the self-similarity,
/// at least the part's own length away - so a held chord does not come back
/// as itself a few seconds on - and never less than `least_lag`.
fn repetition(alike: &[Vec<f32>], start: usize, end: usize, least_lag: usize) -> f32 {
    let n = alike.len();
    let length = end.saturating_sub(start);
    if length == 0 || n < length {
        return 0.0;
    }
    let away = least_lag.max(length);
    let mut best = 0.0f32;
    for j in 0..=n - length {
        if start.abs_diff(j) < away {
            continue;
        }
        let mut sum = 0.0;
        let mut count = 0;
        for k in (0..length).step_by(2) {
            sum += alike[start + k][j + k];
            count += 1;
        }
        best = best.max(sum / count as f32);
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A song made of frames: each section a chord and a loudness, for so
    /// many seconds.
    fn song(sections: &[([usize; 3], f32, f64)]) -> Frames {
        let mut loud = Vec::new();
        let mut chroma = Vec::new();
        for (chord, level, seconds) in sections {
            for _ in 0..frames_of(*seconds) {
                let mut notes = [0.02f32; 12];
                for note in chord {
                    notes[*note] = 1.0;
                }
                let norm = notes.iter().map(|v| v * v).sum::<f32>().sqrt();
                chroma.push(notes.map(|v| v / norm));
                loud.push(*level);
            }
        }
        Frames { loud, chroma }
    }

    const VERSE: [usize; 3] = [0, 4, 7];
    const CHORUS: [usize; 3] = [5, 9, 0];
    const BRIDGE: [usize; 3] = [2, 6, 9];

    /// Parts begin where the chords turn, and the loud part that comes back
    /// is offered first, as the chorus.
    #[test]
    fn the_loud_part_that_comes_back_is_offered_as_the_chorus() {
        let frames = song(&[
            (VERSE, -30.0, 30.0),
            (CHORUS, -14.0, 30.0),
            (VERSE, -30.0, 30.0),
            (CHORUS, -14.0, 30.0),
            (BRIDGE, -26.0, 20.0),
        ]);

        let map = map(&frames);

        for turn in [30.0, 60.0, 90.0, 120.0] {
            assert!(
                map.boundaries.iter().any(|b| (b - turn).abs() <= 1.0),
                "a part begins at {turn}: {:?}",
                map.boundaries
            );
        }
        let best = &map.parts[0];
        assert_eq!(best.reason, "chorus", "{:?}", map.parts);
        assert!(
            [30.0, 90.0]
                .iter()
                .any(|start| (best.starts_at - start).abs() <= 1.0),
            "the best part is a chorus: {best:?}"
        );
        assert!(
            (SHORTEST..=LONGEST).contains(&(best.ends_at - best.starts_at)),
            "{best:?}"
        );
    }

    /// No two parts offered lie over one another.
    #[test]
    fn parts_offered_lie_apart() {
        let frames = song(&[
            (VERSE, -30.0, 25.0),
            (CHORUS, -14.0, 30.0),
            (BRIDGE, -22.0, 25.0),
            (CHORUS, -14.0, 30.0),
            (VERSE, -28.0, 25.0),
        ]);

        let parts = map(&frames).parts;

        assert!(!parts.is_empty());
        assert!(parts.len() <= MOST);
        for (i, a) in parts.iter().enumerate() {
            for b in &parts[i + 1..] {
                assert!(
                    a.ends_at <= b.starts_at + 5.0 || a.starts_at >= b.ends_at - 5.0,
                    "{a:?} and {b:?}"
                );
            }
        }
    }

    /// A song shorter than the shortest part offers nothing.
    #[test]
    fn a_song_too_short_offers_nothing() {
        let map = map(&song(&[(VERSE, -20.0, 10.0)]));
        assert!(map.parts.is_empty());
        assert!(map.boundaries.is_empty());
    }

    /// A tone is heard as its note.
    #[test]
    fn a_tone_is_heard_as_its_note() {
        let samples: Vec<f32> = (0..RATE as usize * 3)
            .map(|i| 0.5 * (2.0 * std::f32::consts::PI * 440.0 * i as f32 / RATE as f32).sin())
            .collect();

        let heard = frames(&samples);

        assert!(!heard.chroma.is_empty());
        let a = 0; // 440 Hz is the note the scale is counted from
        let strongest = heard.chroma[0]
            .iter()
            .enumerate()
            .max_by(|x, y| x.1.total_cmp(y.1))
            .map(|(note, _)| note)
            .unwrap();
        assert_eq!(strongest, a);
        assert!(
            heard.loud[0] > -12.0 && heard.loud[0] < -6.0,
            "{}",
            heard.loud[0]
        );
    }
}
