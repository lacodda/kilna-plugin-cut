//! A radix-2 FFT, enough to read which notes sound in half a second of audio.
//!
//! The plugin listens to a song a few hundred windows at a time; a crate for
//! that would be the heaviest thing in it. In-place, iterative, for lengths
//! that are a power of two.

/// The magnitudes of the first half of the spectrum of `samples`, whose
/// length must be a power of two: bin `k` is `k * rate / samples.len()` Hz.
pub fn magnitudes(samples: &[f32]) -> Vec<f32> {
    let n = samples.len();
    assert!(n.is_power_of_two(), "the window is a power of two");
    let mut re: Vec<f32> = samples.to_vec();
    let mut im = vec![0.0f32; n];

    // Bit-reversed order, so the butterflies below can work in place.
    let bits = n.trailing_zeros();
    for i in 0..n {
        let j = i.reverse_bits() >> (usize::BITS - bits);
        if j > i {
            re.swap(i, j);
            im.swap(i, j);
        }
    }

    let mut size = 2;
    while size <= n {
        let half = size / 2;
        let step = -2.0 * std::f32::consts::PI / size as f32;
        for start in (0..n).step_by(size) {
            for k in 0..half {
                let (sin, cos) = (step * k as f32).sin_cos();
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

    (0..n / 2).map(|k| re[k].hypot(im[k])).collect()
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
}
