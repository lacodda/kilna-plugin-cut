//! What a short's own sound says of itself: how loud each frame is, where it
//! hits, which bands move - and how far the video a stretch is cut from runs
//! behind the recording its words were timed on.
//!
//! Everything a frame moves to is read off the sound once, before ffmpeg
//! runs: the sign kicks with the loudness, the picture tears on the hits,
//! the bars stand as high as their bands.

use crate::fft;

/// The rate the sound of a short is listened to at.
pub const RATE: u32 = 22_050;
/// The rate two recordings are laid over each other at: enough to place one
/// inside the other to a frame, small enough to do it at once.
pub const COARSE: u32 = 4_000;

/// The `q`-th hundredth of `values`, between the two nearest when it falls
/// between them. Nought for an empty list.
pub fn percentile(values: &[f32], q: f32) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f32::total_cmp);
    let at = (sorted.len() - 1) as f32 * q / 100.0;
    let (low, high) = (at.floor() as usize, at.ceil() as usize);
    sorted[low] + (sorted[high] - sorted[low]) * (at - low as f32)
}

/// How loud each frame is, twice over: `level` for what moves with it - 0 to
/// about 1.3, up at once with a hit and sinking back slowly, so a sign kicks
/// rather than trembles - and `rms`, the loudness as it is.
pub struct Loudness {
    pub level: Vec<f32>,
    pub rms: Vec<f32>,
}

/// Hear `samples` at `rate` as frames of a picture at `fps`.
pub fn loudness(samples: &[f32], rate: u32, fps: u32) -> Loudness {
    let hop = (rate / fps) as usize;
    let rms: Vec<f32> = samples
        .chunks_exact(hop)
        .map(|frame| (frame.iter().map(|s| s * s).sum::<f32>() / hop as f32).sqrt())
        .collect();
    let loud = percentile(&rms, 97.0) + 1e-9;
    let mut value = 0.0f32;
    let level = rms
        .iter()
        .map(|one| {
            let now = (one / loud).clamp(0.0, 1.3);
            value = if now > value {
                now
            } else {
                value * 0.86 + now * 0.14
            };
            value
        })
        .collect();
    Loudness { level, rms }
}

/// Where the sound hits: the moments its loudness rises most, no two nearer
/// than `gap` seconds, none in the first or the last 0.6 s, at most `limit`
/// of them, in order.
pub fn hits(rms: &[f32], fps: u32, seconds: f64, limit: usize, gap: f64) -> Vec<f64> {
    let rise: Vec<f32> = rms
        .iter()
        .enumerate()
        .map(|(i, now)| (now - rms[i.saturating_sub(1)]).max(0.0))
        .collect();
    let mut order: Vec<usize> = (0..rise.len()).collect();
    order.sort_by(|a, b| rise[*b].total_cmp(&rise[*a]));
    let mut picked: Vec<f64> = Vec::new();
    for frame in order {
        if picked.len() == limit || rise[frame] <= 0.0 {
            break;
        }
        let at = frame as f64 / f64::from(fps);
        if at > 0.6 && at < seconds - 0.6 && picked.iter().all(|one| (at - one).abs() >= gap) {
            picked.push(at);
        }
    }
    picked.sort_by(f64::total_cmp);
    picked
}

/// The bands of the bars, per frame, 0 to 1: every band is measured against
/// its own loud moments, so the highs move as much as the bass, and a bar
/// jumps up and falls back slower.
pub fn bands(samples: &[f32], rate: u32, fps: u32, count: usize) -> Vec<Vec<f32>> {
    const WINDOW: usize = 2048;
    let hop = (rate / fps) as usize;
    let frames = samples.len() / hop;
    let mut padded = vec![0.0f32; WINDOW / 2];
    padded.extend_from_slice(samples);
    padded.extend(std::iter::repeat_n(0.0, WINDOW));
    let hann: Vec<f32> = (0..WINDOW)
        .map(|i| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / (WINDOW - 1) as f32).cos())
        .collect();
    // From 55 Hz to 9 kHz, each band as wide as the ear hears it.
    let edges: Vec<usize> = (0..=count)
        .map(|k| {
            let hz = 55.0 * (9000.0f64 / 55.0).powf(k as f64 / count as f64);
            (hz * WINDOW as f64 / f64::from(rate)) as usize
        })
        .collect();
    let mut out: Vec<Vec<f32>> = (0..frames)
        .map(|i| {
            let window: Vec<f32> = padded[i * hop..i * hop + WINDOW]
                .iter()
                .zip(&hann)
                .map(|(sample, weight)| sample * weight)
                .collect();
            let spectrum = fft::magnitudes(&window);
            (0..count)
                .map(|k| {
                    let low = edges[k].min(spectrum.len() - 1);
                    let high = edges[k + 1].max(low + 1).min(spectrum.len());
                    spectrum[low..high].iter().sum::<f32>() / (high - low) as f32
                })
                .collect()
        })
        .collect();
    for k in 0..count {
        let column: Vec<f32> = out.iter().map(|frame| frame[k]).collect();
        let loud = percentile(&column, 96.0) + 1e-9;
        let mut before = 0.0f32;
        for frame in &mut out {
            let now = (frame[k] / loud).clamp(0.0, 1.0).powf(0.8);
            before = now.max(before * 0.78);
            frame[k] = before;
        }
    }
    out
}

/// How many seconds of `whole` come before the first second of the file
/// `part` was taken from, when `part` is that file's sound from `part_from`
/// seconds in. Both at [`COARSE`]. `None` when they cannot be laid over each
/// other.
pub fn offset(whole: &[f32], part: &[f32], part_from: f64) -> Option<f64> {
    let lag = fft::lag(whole, part)?;
    Some(lag as f64 / f64::from(COARSE) - part_from)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn noise(count: usize, seed: u32) -> Vec<f32> {
        let mut state = seed;
        (0..count)
            .map(|_| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                (state >> 8) as f32 / 8_388_608.0 - 1.0
            })
            .collect()
    }

    #[test]
    fn a_percentile_falls_between_its_neighbours() {
        assert_eq!(percentile(&[4.0, 1.0, 3.0, 2.0], 50.0), 2.5);
        assert_eq!(percentile(&[1.0, 2.0, 3.0], 100.0), 3.0);
        assert_eq!(percentile(&[], 97.0), 0.0);
    }

    /// A second of quiet, a burst, quiet again: the level jumps with the
    /// burst and is still sinking a few frames after it.
    #[test]
    fn the_level_jumps_with_a_hit_and_sinks_slowly() {
        let rate = 3000;
        let mut samples = vec![0.01f32; rate as usize * 3];
        for sample in &mut samples[rate as usize..rate as usize + 600] {
            *sample = 0.9;
        }
        let heard = loudness(&samples, rate, 30);

        assert_eq!(heard.level.len(), 90);
        assert!(heard.level[29] < 0.2);
        assert!(heard.level[31] > 0.9, "up at once: {}", heard.level[31]);
        assert!(heard.level[38] > heard.level[45], "and down slowly");
        assert!(heard.level[38] > 0.2);
    }

    /// The hits are where the loudness rises most, apart from each other and
    /// away from both ends.
    #[test]
    fn hits_are_the_steepest_rises_kept_apart() {
        let mut rms = vec![0.1f32; 300];
        for at in [6, 60, 66, 150, 294] {
            rms[at] = 1.0;
        }
        rms[240] = 0.5;

        let found = hits(&rms, 30, 10.0, 10, 2.2);

        assert_eq!(found, vec![2.0, 5.0, 8.0]);
        assert_eq!(hits(&rms, 30, 10.0, 2, 2.2), vec![2.0, 5.0]);
        assert!(
            hits(&[0.3; 90], 30, 3.0, 10, 2.2).is_empty(),
            "a flat sound hits nowhere"
        );
    }

    /// A low tone moves the low bars and leaves the high ones down.
    #[test]
    fn a_tone_moves_its_own_band() {
        let samples: Vec<f32> = (0..RATE as usize)
            .map(|i| (2.0 * std::f32::consts::PI * 110.0 * i as f32 / RATE as f32).sin())
            .collect();

        let heard = bands(&samples, RATE, 30, 28);

        assert_eq!(heard.len(), 30);
        let frame = &heard[15];
        let loudest = (0..28)
            .max_by(|a, b| frame[*a].total_cmp(&frame[*b]))
            .unwrap();
        assert!(
            loudest < 8,
            "110 Hz is among the low bands, not band {loudest}"
        );
        assert!(frame.iter().all(|level| (0.0..=1.0).contains(level)));
    }

    /// A video that opens two seconds into the recording is two seconds
    /// behind it, wherever in the video the stretch was heard.
    #[test]
    fn a_video_is_placed_inside_the_recording() {
        let whole = noise(COARSE as usize * 20, 7);
        let from = COARSE as usize * 7;
        let part = &whole[from..from + COARSE as usize * 4];

        let found = offset(&whole, part, 5.0).unwrap();

        assert!((found - 2.0).abs() < 0.01, "{found}");
    }
}
