//! What a cut looks like: the words on it, its colours, its fonts and where
//! the file goes - read from `kilna-plugin-cut.json` beside the executable.
//!
//! Nothing a channel says is written into the code: a channel renamed is a
//! line changed in the file, not a new build. The file is optional; every
//! key falls back to a default, and the defaults letter nothing in any one
//! person's language.

use std::path::{Path, PathBuf};

use serde::Deserialize;

/// The settings file's name, beside the executable.
pub const FILE_NAME: &str = "kilna-plugin-cut.json";

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// The first line of the top bar - the channel's name. Empty draws none.
    pub channel: String,
    /// Whether the song's title is drawn in capitals, as a poster would.
    pub title_upper: bool,
    /// The words of the bottom bar - the call to watch the whole. Empty
    /// draws none, and no pointing hand either.
    pub call: String,
    /// Whether the pointing hand bobs under the call.
    pub hand: bool,
    /// The font of the channel's name, and the bold of the title and the
    /// call: whole paths to TrueType files.
    pub font: String,
    pub font_bold: String,
    /// The colour of the words, as ffmpeg reads a colour: `white`, `#f2e6cf`.
    pub ink: String,
    /// How dark the bars over the blur are, from 0 to 1.
    pub bar_shade: f64,
    /// Where the cut goes inside the short's folder.
    pub output: String,
    /// Where the short's cover is looked for inside its folder when kilna
    /// hands none: a folder, the newest picture in it wins.
    pub covers: String,
    /// How long the cover stands at the end, in seconds; 0 leaves it off.
    pub cover_seconds: f64,
    /// The quality the picture is encoded at: x264's CRF, lower is better.
    pub crf: u32,
}

impl Default for Settings {
    fn default() -> Self {
        let (font, font_bold) = default_fonts();
        Self {
            channel: String::new(),
            title_upper: true,
            call: "FULL VERSION".into(),
            hand: true,
            font,
            font_bold,
            ink: "white".into(),
            bar_shade: 0.5,
            output: "video".into(),
            covers: "covers".into(),
            cover_seconds: 0.5,
            crf: 20,
        }
    }
}

/// A sans-serif pair every system of its kind has.
fn default_fonts() -> (String, String) {
    if cfg!(windows) {
        (
            "C:/Windows/Fonts/arial.ttf".into(),
            "C:/Windows/Fonts/arialbd.ttf".into(),
        )
    } else if cfg!(target_os = "macos") {
        (
            "/System/Library/Fonts/Supplemental/Arial.ttf".into(),
            "/System/Library/Fonts/Supplemental/Arial Bold.ttf".into(),
        )
    } else {
        (
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf".into(),
            "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf".into(),
        )
    }
}

/// Where the settings file is: `KILNA_PLUGIN_CUT_SETTINGS` when it is set,
/// else beside the running executable.
pub fn path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("KILNA_PLUGIN_CUT_SETTINGS") {
        return Some(PathBuf::from(path));
    }
    Some(std::env::current_exe().ok()?.parent()?.join(FILE_NAME))
}

/// The settings, from the file when there is one. A file that does not read
/// is an error said by name, not a reason to cut with the defaults: a cut
/// lettered with the wrong channel is worse than none.
pub fn load(path: Option<&Path>) -> Result<Settings, String> {
    let Some(path) = path.filter(|path| path.is_file()) else {
        return Ok(Settings::default());
    };
    let text = std::fs::read_to_string(path)
        .map_err(|cause| format!("could not read {}: {cause}", path.display()))?;
    let settings: Settings = serde_json::from_str(&text)
        .map_err(|cause| format!("{} does not read: {cause}", path.display()))?;
    if !(0.0..=1.0).contains(&settings.bar_shade) {
        return Err(format!(
            "{}: bar_shade is {}, and a shade runs from 0 to 1",
            path.display(),
            settings.bar_shade
        ));
    }
    if settings.cover_seconds < 0.0 {
        return Err(format!("{}: cover_seconds is below nought", path.display()));
    }
    Ok(settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_file_is_the_defaults() {
        let settings = load(None).unwrap();
        assert_eq!(settings.channel, "");
        assert_eq!(settings.output, "video");
        assert_eq!(settings.cover_seconds, 0.5);
    }

    /// A file names only what it changes; the rest stays at the defaults.
    #[test]
    fn a_file_changes_only_what_it_names() {
        let dir = std::env::temp_dir().join(format!("kpc-settings-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join(FILE_NAME);
        std::fs::write(
            &file,
            r#"{"channel": "Harbour Radio", "call": "WHOLE SONG"}"#,
        )
        .unwrap();

        let settings = load(Some(&file)).unwrap();
        assert_eq!(settings.channel, "Harbour Radio");
        assert_eq!(settings.call, "WHOLE SONG");
        assert_eq!(settings.crf, 20, "what it does not name is the default");

        std::fs::write(&file, r#"{"bar_shade": 3}"#).unwrap();
        assert!(load(Some(&file)).is_err(), "a shade past 1 is said");
        std::fs::write(&file, "not json").unwrap();
        assert!(
            load(Some(&file)).is_err(),
            "a file that does not read is said"
        );
        let _ = std::fs::remove_dir_all(dir);
    }
}
