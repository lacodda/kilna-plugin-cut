//! The words under a short, lit as they are sung.
//!
//! kilna hands over every line of the song with the second each word is sung
//! at, in the time of the recording they were timed on. What is done here is
//! the cutting: which lines fall inside a stretch, where a long one breaks in
//! two, and the subtitle script ffmpeg letters them from.

use serde::Deserialize;

/// One word of the song and when it is sung, in seconds of the recording the
/// words were timed on.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Word {
    pub word: String,
    pub at: f64,
    pub end: f64,
}

/// A line as it is shown under one stretch.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub words: Vec<Word>,
    /// The word its second row begins with, when it is set in two.
    pub breaks_at: Option<usize>,
    /// When its last word is sung out, never past the end of the stretch.
    pub end: f64,
}

/// Where to break a line in two: near the middle, at a comma or a dash when
/// there is one. `None` for a line of one or two words.
pub fn two_rows(words: &[Word]) -> Option<usize> {
    if words.len() < 3 {
        return None;
    }
    let length = |word: &Word| word.word.chars().count() as i64;
    let total: i64 = words.iter().map(length).sum::<i64>() + words.len() as i64 - 1;
    (1..words.len()).min_by_key(|at| {
        let left: i64 = words[..*at].iter().map(length).sum::<i64>() + *at as i64 - 1;
        let pause = words[at - 1]
            .word
            .chars()
            .last()
            .is_some_and(|last| ",;:-".contains(last));
        (2 * left - total).abs() - if pause { 6 } else { 0 }
    })
}

/// The lines shown under the stretch that begins `start` seconds into the
/// recording and runs `seconds`.
///
/// A line that runs past the end of the stretch: words held back by a long
/// pause are dropped; otherwise the line stays whole when at least half of it
/// is sung inside, and goes when less. A line the stretch opens in the middle
/// of is shown whole, its sung words already lit.
pub fn fragment(lines: &[Vec<Word>], start: f64, seconds: f64) -> Vec<Line> {
    let close = start + seconds - 0.3;
    let mut out = Vec::new();
    for line in lines {
        let mut row = line.clone();
        while row.len() > 1
            && row[row.len() - 1].at >= close
            && row[row.len() - 1].at - row[row.len() - 2].at > 3.0
        {
            row.pop();
        }
        let Some(last) = row.last() else {
            continue;
        };
        let inside = row.iter().filter(|word| word.at < close).count();
        if inside * 2 < row.len() {
            continue;
        }
        let end = last.end.min(start + seconds);
        if end < start + 0.6 {
            continue;
        }
        out.push(Line {
            breaks_at: two_rows(&row),
            words: row,
            end,
        });
    }
    out
}

/// A colour as a subtitle script writes one: `#FF2E63` is `&H00632EFF`.
/// White for anything that is not six hex digits.
pub fn colour(hex: &str) -> String {
    let digits = hex.trim_start_matches('#');
    if digits.len() != 6 || !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return "&H00FFFFFF".into();
    }
    format!("&H00{}{}{}", &digits[4..6], &digits[2..4], &digits[0..2]).to_uppercase()
}

/// Seconds as a subtitle script's clock: `0:00:07.25`.
fn clock(seconds: f64) -> String {
    let seconds = seconds.max(0.0);
    let hours = (seconds / 3600.0).floor();
    let minutes = ((seconds % 3600.0) / 60.0).floor();
    format!("{hours:.0}:{minutes:02.0}:{:05.2}", seconds % 60.0)
}

/// How the words are lettered.
#[derive(Debug, Clone, PartialEq)]
pub struct Lettering {
    pub font: String,
    pub size: u32,
    /// The colour a word takes as it is sung.
    pub sung: String,
    /// How far from the top of the frame the lines stand.
    pub top: u32,
}

/// The lines of one stretch, with where it lies in both times: `shift` is
/// the second of the recording the stretch opens at, `base` the second of
/// the short it opens at.
pub struct Stretch {
    pub lines: Vec<Line>,
    pub shift: f64,
    pub base: f64,
}

/// A word with nothing in it a subtitle script would read as its own.
fn plain(word: &str) -> String {
    word.chars()
        .filter(|c| !matches!(c, '{' | '}' | '\\' | '\n' | '\r'))
        .collect()
}

/// The subtitle script of a short: each line fades in a third of a second
/// before its first word and fills word by word.
pub fn script(stretches: &[Stretch], lettering: &Lettering) -> String {
    let mut out = format!(
        "[Script Info]\nScriptType: v4.00+\nPlayResX: 1080\nPlayResY: 1920\nWrapStyle: 2\n\n\
         [V4+ Styles]\nFormat: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, \
         Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\n\
         Style: K,{},{},{},&H00FFFFFF,&H00101010,&H00000000,-1,0,0,0,100,100,0,0,1,4,0,8,30,30,{},1\n\n\
         [Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n",
        lettering.font.replace(',', " "),
        lettering.size,
        colour(&lettering.sung),
        lettering.top
    );
    let hundredths = |seconds: f64| (seconds * 100.0).round().max(0.0) as u64;
    for stretch in stretches {
        let shift = stretch.shift;
        for (number, line) in stretch.lines.iter().enumerate() {
            let Some(opening) = line.words.first() else {
                continue;
            };
            let end = line.end - shift;
            let first = opening.at - shift;
            let begin = (first - 0.35).max(0.0);
            let mut text = format!(
                "{{\\fad(180,180)}}{{\\k{}}}",
                hundredths(first.max(0.0) - begin)
            );
            for (at, word) in line.words.iter().enumerate() {
                if line.breaks_at == Some(at) {
                    text.truncate(text.trim_end().len());
                    text.push_str("\\N");
                }
                let until = line.words.get(at + 1).map_or(line.end, |next| next.at);
                // A word sung before the stretch opens is lit from the first frame.
                text.push_str(&format!(
                    "{{\\kf{}}}{} ",
                    hundredths(until.max(shift) - word.at.max(shift)),
                    plain(&word.word)
                ));
            }
            // A line leaves before the next one comes, or the newcomer is
            // pushed down under it.
            let following = stretch
                .lines
                .get(number + 1)
                .and_then(|next| next.words.first())
                .map_or(end + 0.45, |next| next.at - shift - 0.4);
            out.push_str(&format!(
                "Dialogue: 0,{},{},K,,0,0,0,,{}\n",
                clock(stretch.base + begin),
                clock(stretch.base + (end + 0.45).min(following)),
                text.trim_end()
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(words: &[(&str, f64)]) -> Vec<Word> {
        words
            .iter()
            .map(|(word, at)| Word {
                word: (*word).to_owned(),
                at: *at,
                end: at + 0.4,
            })
            .collect()
    }

    #[test]
    fn a_long_line_breaks_near_the_middle_at_a_pause() {
        let words = line(&[
            ("the", 0.0),
            ("harbour,", 0.5),
            ("lights", 1.0),
            ("at", 1.5),
            ("dawn", 2.0),
        ]);
        assert_eq!(two_rows(&words), Some(2), "after the comma");
        assert_eq!(two_rows(&line(&[("two", 0.0), ("words", 0.5)])), None);
    }

    /// Lines before the stretch and after it are left out; one cut by its end
    /// stays when half of it is sung inside, and one the stretch opens in the
    /// middle of is shown whole.
    #[test]
    fn a_stretch_shows_the_lines_sung_in_it() {
        let lines = vec![
            line(&[("long", 2.0), ("before", 3.0)]),
            line(&[
                ("opened", 9.5),
                ("in", 10.2),
                ("the", 10.6),
                ("middle", 11.0),
            ]),
            line(&[("sung", 14.0), ("inside", 15.0)]),
            line(&[
                ("half", 18.0),
                ("of", 19.0),
                ("it", 19.9),
                ("outside", 20.5),
            ]),
            line(&[
                ("mostly", 19.8),
                ("after", 21.0),
                ("the", 22.0),
                ("end", 23.0),
            ]),
        ];

        let shown = fragment(&lines, 10.0, 10.0);

        let firsts: Vec<&str> = shown.iter().map(|l| l.words[0].word.as_str()).collect();
        assert_eq!(firsts, vec!["opened", "sung", "half"]);
        assert_eq!(shown[2].end, 20.0, "never past the end of the stretch");
        assert_eq!(shown[0].breaks_at, Some(2));
    }

    /// A word held back by a long pause past the end of the stretch is
    /// dropped, and the rest of its line stays.
    #[test]
    fn a_word_after_a_long_pause_is_dropped() {
        let lines = vec![line(&[("stays", 5.0), ("too", 6.0), ("later", 12.0)])];
        let shown = fragment(&lines, 4.0, 6.0);
        assert_eq!(shown[0].words.len(), 2);
    }

    #[test]
    fn a_colour_is_written_blue_first() {
        assert_eq!(colour("#FF2E63"), "&H00632EFF");
        assert_eq!(colour("red"), "&H00FFFFFF");
    }

    /// One line of a stretch that opens 30 s into the recording and 4 s into
    /// the short: it fades in before its first word, fills word by word, and
    /// a word sung before the stretch is lit from the start.
    #[test]
    fn a_script_lights_the_words_as_they_are_sung() {
        let lines = vec![
            line(&[("before", 29.5), ("the{}", 30.5), ("cut", 31.0)]),
            line(&[("next", 33.0), ("line", 33.5)]),
        ];
        let stretches = [Stretch {
            lines: fragment(&lines, 30.0, 10.0),
            shift: 30.0,
            base: 4.0,
        }];
        let lettering = Lettering {
            font: "Arial".into(),
            size: 60,
            sung: "#FF2E63".into(),
            top: 1412,
        };

        let script = script(&stretches, &lettering);

        assert!(script.contains("Style: K,Arial,60,&H00632EFF,&H00FFFFFF"));
        assert!(script.contains(",8,30,30,1412,1\n"), "set from the top");
        let rows: Vec<&str> = script
            .lines()
            .filter(|l| l.starts_with("Dialogue"))
            .collect();
        assert_eq!(rows.len(), 2);
        // "before" was sung half a second before the stretch: from nought to
        // 0.5 s is its fill, the brace of a word is not the script's.
        assert_eq!(
            rows[0],
            "Dialogue: 0,0:00:04.00,0:00:05.85,K,,0,0,0,,{\\fad(180,180)}{\\k0}{\\kf50}before\\N{\\kf50}the {\\kf40}cut"
        );
        assert!(rows[1].starts_with("Dialogue: 0,0:00:06.65,0:00:08.35,K,,"));
        assert!(rows[1].ends_with("{\\k35}{\\kf50}next {\\kf40}line"));
    }
}
