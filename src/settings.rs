//! Where a short goes and how well it is encoded.
//!
//! What a short looks like is not set here: the look is the style of the
//! space, kept and edited in kilna and handed over with every run. Three
//! layers, each over the last: the defaults; `kilna-plugin-cut.json` beside
//! the executable, optional; and what was set in kilna's Settings, handed
//! over with every run. The manifest offers the file's values as the defaults
//! of its settings, so what the form in kilna shows is what a build does.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// The settings file's name, beside the executable.
pub const FILE_NAME: &str = "kilna-plugin-cut.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Where the short goes inside its folder.
    pub output: String,
    /// Where the short's cover is looked for inside its folder when kilna
    /// hands none: a folder, the newest picture in it wins.
    pub covers: String,
    /// The quality the picture is encoded at: x264's CRF, lower is better.
    pub crf: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            output: "video".into(),
            covers: "covers".into(),
            crf: 23,
        }
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
/// is an error said by name, not a reason to build with the defaults: a short
/// laid in the wrong folder is worse than none.
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
        if self.crf > 51 {
            return Err(format!(
                "crf is {}, and x264 counts it from 0 to 51",
                self.crf
            ));
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
        assert_eq!(settings.output, "video");
        assert_eq!(settings.crf, 23);
    }

    /// A file names only what it changes; the rest stays at the defaults.
    #[test]
    fn a_file_changes_only_what_it_names() {
        let dir = std::env::temp_dir().join(format!("kpc-settings-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join(FILE_NAME);
        std::fs::write(&file, r#"{"output": "cuts"}"#).unwrap();

        let settings = load(Some(&file)).unwrap();
        assert_eq!(settings.output, "cuts");
        assert_eq!(settings.crf, 23, "what it does not name is the default");

        std::fs::write(&file, r#"{"crf": 80}"#).unwrap();
        assert!(
            load(Some(&file)).is_err(),
            "a quality x264 has not got is said"
        );
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
            output: "cuts".into(),
            ..Settings::default()
        };

        let settings = file.with(&given(serde_json::json!({ "crf": 18 }))).unwrap();

        assert_eq!(settings.crf, 18);
        assert_eq!(settings.output, "cuts", "from the file");
        assert_eq!(settings.covers, "covers", "from the defaults");
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
                .with(&given(serde_json::json!({"crf": 80})))
                .unwrap_err()
                .contains("crf")
        );
        assert!(
            Settings::default()
                .with(&given(serde_json::json!({"output": 3})))
                .unwrap_err()
                .contains("do not read")
        );
    }

    /// A key this version does not know is left alone: a newer kilna's, or
    /// what an older plugin was set - the words of the layout before 0.4.
    #[test]
    fn a_key_this_plugin_does_not_know_is_no_reason_to_stop() {
        let settings = Settings::default()
            .with(&given(
                serde_json::json!({"from_the_future": 1, "channel": "Tide Radio"}),
            ))
            .unwrap();
        assert_eq!(settings.crf, 23);
    }
}
