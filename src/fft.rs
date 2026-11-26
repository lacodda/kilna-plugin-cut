//! A radix-2 FFT, enough to read which notes sound in half a second of audio
//! and to find where one recording lies inside another.
//!
//! The plugin listens to a song a few hundred windows at a time; a crate for
//! that would be the heaviest thing in it. In-place, iterative, for lengths
//! that are a power of two.

/// Transform `re` and `im` in place, forward or back. Their length must be a
/// power of two. The way back is not scaled: divide by the length for the
/// signal itself.
pub fn transform(re: &mut [f32], im: &mut [f32], inverse: bool) {
    let n = re.len();
    assert!(
        n.is_power_of_two() && im.len() == n,
        "the window is a power of two"
    );
    if n < 2 {
        return;
    }

    // Bit-reversed order, so the butterflies below can work in place.
    let bits = n.trailing_zeros();
    for i in 0..n {
        let j = i.reverse_bits() >> (usize::BITS - bits);
        if j > i {
            re.swap(i, j);
            im.swap(i, j);
        }
    }

    // The turns of the circle, once: a butterfly of any size reads every
    // `n / size`-th of them.
    let sign = if inverse { 1.0 } else { -1.0 };
    let turns: Vec<(f32, f32)> = (0..n / 2)
        .map(|k| {
            let angle = sign * 2.0 * std::f64::consts::PI * k as f64 / n as f64;
            (angle.sin() as f32, angle.cos() as f32)
        })
        .collect();

    let mut size = 2;
    while size <= n {
        let half = size / 2;
        let stride = n / size;
        for start in (0..n).step_by(size) {
            for k in 0..half {
                let (sin, cos) = turns[k * stride];
                let (a, b) = (start + k, start + k + half);
                let tr = re[b] * cos - im[b] * sin;
                let ti = re[b] * sin + im[b] * cos;
                re[b] = re[a] - tr;
                im[b] = im[a] - ti;
                re[a] += tr;
                im[a] += ti;
            }
        }
        size *= 2;
    }
}

/// The magnitudes of the first half of the spectrum of `samples`, whose
/// length must be a power of two: bin `k` is `k * rate / samples.len()` Hz.
pub fn magnitudes(samples: &[f32]) -> Vec<f32> {
    let n = samples.len();
    let mut re: Vec<f32> = samples.to_vec();
    let mut im = vec![0.0f32; n];
    transform(&mut re, &mut im, false);
    (0..n / 2).map(|k| re[k].hypot(im[k])).collect()
}

/// How many samples into `whole` the stretch `part` begins: the lag at which
/// the two agree best. `None` when either is silent or empty.
pub fn lag(whole: &[f32], part: &[f32]) -> Option<usize> {
    if whole.is_empty() || part.is_empty() {
        return None;
    }
    let n = (whole.len() + part.len()).next_power_of_two();
    let padded = |samples: &[f32]| {
        let mut re = vec![0.0f32; n];
        re[..samples.len()].copy_from_slice(samples);
        (re, vec![0.0f32; n])
    };
    let (mut wr, mut wi) = padded(whole);
    let (mut pr, mut pi) = padded(part);
    transform(&mut wr, &mut wi, false);
    transform(&mut pr, &mut pi, false);
    // The whole times the conjugate of the part: their correlation, once
    // taken back.
    for k in 0..n {
        let (a, b, c, d) = (wr[k], wi[k], pr[k], pi[k]);
        wr[k] = a * c + b * d;
        wi[k] = b * c - a * d;
    }
    transform(&mut wr, &mut wi, true);
    let (at, peak) = wr[..whole.len()]
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(b.1))?;
    (*peak > 0.0).then_some(at)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A pure tone lands in the bin of its frequency.
    #[test]
    fn a_tone_lands_in_its_bin() {
        let n = 1024;
        let rate = 8192.0;
        let tone = 440.0;
        let samples: Vec<f32> = (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * tone * i as f32 / rate).sin())
            .collect();

        let spectrum = magnitudes(&samples);

        let loudest = spectrum
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .map(|(bin, _)| bin)
            .unwrap();
        let expected = (tone * n as f32 / rate).round() as usize;
        assert!(
            loudest.abs_diff(expected) <= 1,
            "bin {loudest}, not {expected}"
        );
    }

    /// Silence has no spectrum to speak of.
    #[test]
    fn silence_is_flat() {
        assert!(magnitudes(&[0.0; 64]).iter().all(|m| *m == 0.0));
    }

    /// There and back is the signal again.
    #[test]
    fn the_way_back_returns_the_signal() {
        let signal: Vec<f32> = (0..256)
            .map(|i| ((i * 37 % 101) as f32 - 50.0) / 50.0)
            .collect();
        let mut re = signal.clone();
        let mut im = vec![0.0; 256];
        transform(&mut re, &mut im, false);
        transform(&mut re, &mut im, true);
        for (back, was) in re.iter().zip(&signal) {
            assert!((back / 256.0 - was).abs() < 1e-4);
        }
    }

    /// A stretch is found where it was taken from.
    #[test]
    fn a_stretch_is_found_inside_the_whole() {
        // Noise from a small generator: nothing in it repeats.
        let mut state = 451u32;
        let whole: Vec<f32> = (0..6000)
            .map(|_| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                (state >> 8) as f32 / 8_388_608.0 - 1.0
            })
            .collect();
        assert_eq!(lag(&whole, &whole[1234..2234]), Some(1234));
        assert_eq!(lag(&whole, &[0.0; 100]), None, "silence lies nowhere");
        assert_eq!(lag(&[], &whole), None);
    }
}
