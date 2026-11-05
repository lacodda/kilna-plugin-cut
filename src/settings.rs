//! What a cut looks like: the words on it, its colours, its fonts and where
//! the file goes.
//!
//! Nothing a channel says is written into the code. Three layers, each over
//! the last: the defaults, which letter nothing in any one person's language;
//! `kilna-plugin-cut.json` beside the executable, optional; and what was set in
//! kilna's Settings, handed over with every run. The manifest offers the file's
//! values as the defaults of its settings, so what the form in kilna shows is
//! what a cut does.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// The settings file's name, beside the executable.
pub const FILE_NAME: &str = "kilna-plugin-cut.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    settings
        .check()
        .map_err(|reason| format!("{}: {reason}", path.display()))?;
    Ok(settings)
}

impl Settings {
    /// The values no type can hold to their bounds.
    fn check(&self) -> Result<(), String> {
        if !(0.0..=1.0).contains(&self.bar_shade) {
            return Err(format!(
                "bar_shade is {}, and a shade runs from 0 to 1",
                self.bar_shade
            ));
        }
        if self.cover_seconds < 0.0 {
            return Err("cover_seconds is below nought".into());
        }
        Ok(())
    }

    /// These settings with what kilna handed over laid on top.
    ///
    /// kilna holds a value to what the manifest declares before it sends it, so
    /// a value that does not read here is a kilna older or newer than this
    /// plugin: said by name rather than cut around, for the reason a file that
    /// does not read is. A number box can hold 18.5 for a setting that counts
    /// whole steps, so `crf` is taken to the nearest.
    pub fn with(self, given: &Map<String, Value>) -> Result<Self, String> {
        let mut merged = match serde_json::to_value(&self) {
            Ok(Value::Object(fields)) => fields,
            _ => return Err("the settings could not be read back".into()),
        };
        for (key, value) in given {
            merged.insert(key.clone(), value.clone());
        }
        if let Some(crf) = merged.get("crf").and_then(Value::as_f64) {
            merged.insert("crf".into(), Value::from(crf.round().max(0.0) as u64));
        }
        let settings: Self = serde_json::from_value(Value::Object(merged))
            .map_err(|cause| format!("the settings kilna handed over do not read: {cause}"))?;
        settings
            .check()
            .map_err(|reason| format!("the settings kilna handed over: {reason}"))?;
        Ok(settings)
    }
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

    fn given(raw: Value) -> Map<String, Value> {
        raw.as_object().unwrap().clone()
    }

    /// What was set in kilna wins over the file, and what it does not name
    /// stays as the file or the defaults have it.
    #[test]
    fn what_kilna_hands_over_lies_on_top() {
        let file = Settings {
            channel: "Harbour Radio".into(),
            call: "WHOLE SONG".into(),
            ..Settings::default()
        };

        let settings = file
            .with(&given(serde_json::json!({
                "channel": "Tide Radio",
                "hand": false,
                "crf": 18,
            })))
            .unwrap();

        assert_eq!(settings.channel, "Tide Radio");
        assert!(!settings.hand);
        assert_eq!(settings.crf, 18);
        assert_eq!(settings.call, "WHOLE SONG", "from the file");
        assert_eq!(settings.output, "video", "from the defaults");
    }

    /// A number box can hold what x264 cannot; the rest is said by name.
    #[test]
    fn what_kilna_hands_over_is_held_to_the_same_bounds() {
        let round = Settings::default()
            .with(&given(serde_json::json!({"crf": 18.6})))
            .unwrap();
        assert_eq!(round.crf, 19);

        assert!(
            Settings::default()
                .with(&given(serde_json::json!({"bar_shade": 3})))
                .unwrap_err()
                .contains("bar_shade")
        );
        assert!(
            Settings::default()
                .with(&given(serde_json::json!({"cover_seconds": -1})))
                .is_err()
        );
        assert!(
            Settings::default()
                .with(&given(serde_json::json!({"channel": 3})))
                .unwrap_err()
                .contains("do not read")
        );
    }

    /// A key this version does not know is left alone: a newer kilna's, or an
    /// older plugin's.
    #[test]
    fn a_key_this_plugin_does_not_know_is_no_reason_to_stop() {
        let settings = Settings::default()
            .with(&given(serde_json::json!({"from_the_future": 1})))
            .unwrap();
        assert_eq!(settings.crf, 20);
    }
}
