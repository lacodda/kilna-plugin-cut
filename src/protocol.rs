//! What kilna hands the plugin, and what the plugin hands back.
//!
//! kilna's plugin protocol 1: `--manifest` prints what the plugin offers,
//! `run` reads one invocation from stdin and prints one outcome to stdout.
//! Since kilna v0.101 a work travels with everything a cutter needs - the
//! shot list beside each donor's finished file, the work's folder, the chain
//! of titles up to the song, its cover - and a command may take longer than
//! two minutes, report progress on stderr and name the files it made. Since
//! kilna v0.108 the manifest declares what can be set, kilna draws it in its
//! Settings and hands the values over with every run, and a word can be given
//! by language. Since kilna v0.168 a short travels with its look.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::settings::Settings;

/// The protocol this plugin speaks.
pub const PROTOCOL_VERSION: u32 = 1;

/// How long a build may take: a short of a minute, every frame of it laid
/// out, on a slow machine, with room to spare.
pub const TIMEOUT_SECONDS: u64 = 1800;

/// What can be set for a build, as kilna draws it. Each default is what a
/// build does now, so what the form shows is the truth before anything was set.
fn declared(now: &Settings) -> Value {
    let text = |key: &str, label: &str, ru: &str, about: &str, about_ru: &str, default: &str| {
        json!({
            "key": key, "type": "text", "default": default,
            "label": label, "labels": { "ru": ru },
            "description": about, "descriptions": { "ru": about_ru },
        })
    };
    json!([
        text(
            "output",
            "Where the short goes",
            "Куда кладётся шортс",
            "A folder inside the short's folder.",
            "Папка внутри папки шортса.",
            &now.output,
        ),
        text(
            "covers",
            "Where covers are looked for",
            "Где искать обложки",
            "A folder inside the short's folder, for when kilna hands none.",
            "Папка внутри папки шортса, на случай когда kilna её не передала.",
            &now.covers,
        ),
        {
            "key": "crf", "type": "number", "min": 0, "max": 51, "step": 1,
            "default": now.crf,
            "label": "Quality", "labels": { "ru": "Качество" },
            "description": "x264's CRF: lower is better, and larger.",
            "descriptions": { "ru": "CRF x264: меньше - лучше и тяжелее." },
        },
    ])
}

/// What `--manifest` prints.
pub fn manifest(now: &Settings) -> Value {
    json!({
        "settings": declared(now),
        "protocol_version": PROTOCOL_VERSION,
        "name": "cut",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "Builds a short out of its stretches under the look of its channel: a vertical 1080x1920 video with the picture, the ground, the stamp, the sign that moves with the sound, the words lit as they are sung, and the cover at the end.",
        "descriptions": {
            "ru": "Собирает шортс из его отрезков в оформлении канала: вертикальное видео 1080x1920 с картинкой, подложкой, печатью, знаком, который движется под звук, словами, загорающимися по мере пения, и обложкой в конце."
        },
        "commands": [{
            "key": "cut",
            "label": "Build the short",
            "labels": { "ru": "Собрать шортс" },
            "description": "Splices the stretches in order under the short's look into a new file in its folder; the file before it stays.",
            "descriptions": {
                "ru": "Склеивает отрезки по порядку в оформлении шортса в новый файл в его папке; прежний файл остаётся."
            },
            "target": "cuts",
            "timeout": TIMEOUT_SECONDS
        }, {
            "key": "suggest",
            "label": "Find the parts",
            "labels": { "ru": "Найти части" },
            "description": "Listens to each video the short is cut from: where its parts begin, and the parts worth a short. Nothing is cut.",
            "descriptions": {
                "ru": "Слушает каждое видео, из которого режется шортс: где начинаются части и какие из них годятся. Ничего не режет."
            },
            "target": "cuts",
            "timeout": LISTEN_SECONDS
        }]
    })
}

/// How long listening may take: a few videos of four minutes, heard twice.
pub const LISTEN_SECONDS: u64 = 600;

/// One invocation, as kilna writes it to stdin.
#[derive(Debug, Deserialize)]
pub struct Invocation {
    pub command: String,
    pub subject: Subject,
    /// What was set for this plugin in kilna (v0.108), or what the manifest
    /// offered as its defaults. Absent from a kilna older than that.
    #[serde(default)]
    pub settings: Map<String, Value>,
}

/// The work a command runs on - only what a cutter reads of it.
#[derive(Debug, Default, Deserialize)]
pub struct Subject {
    /// The work's folder on disk, whole, when its kind names one.
    #[serde(default)]
    pub folder: Option<String>,
    /// The stretches it is spliced from, in order.
    #[serde(default)]
    pub shots: Vec<Shot>,
    /// The picture it goes out under, whole, when it has one.
    #[serde(default)]
    pub cover: Option<String>,
    /// The works its stretches come out of, each with its finished file.
    #[serde(default)]
    pub donors: Vec<Donor>,
    /// What the short looks like (kilna v0.168): the look, the still layers
    /// kilna drew for it, and the words of the song with their times.
    #[serde(default)]
    pub short: Option<crate::short::Short>,
}

/// A work a short is cut from.
#[derive(Debug, Clone, Deserialize)]
pub struct Donor {
    pub id: String,
    #[serde(default)]
    pub title: String,
    /// Its finished file, as kilna describes it: how it was found and, when
    /// it was, its whole `path`.
    #[serde(default)]
    pub file: Option<FinishedFile>,
}

impl Donor {
    /// The donor's finished file, whole, when it has one.
    pub fn path(&self) -> Option<&str> {
        match self.file.as_ref()? {
            FinishedFile::Described { path } => path.as_deref(),
            FinishedFile::Path(path) => Some(path),
        }
    }
}

/// A finished file as kilna hands it over: an object with `state`, `rule`,
/// `relative` and `path` (kilna v0.101 and later) - or a bare path, which a
/// person writing an invocation by hand may give instead.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum FinishedFile {
    Path(String),
    Described {
        #[serde(default)]
        path: Option<String>,
    },
}

/// One stretch of a donor.
#[derive(Debug, Clone, Deserialize)]
pub struct Shot {
    pub starts_at: f64,
    pub ends_at: f64,
    #[serde(default)]
    pub source_title: String,
    /// The donor's finished file; `None` when it has none.
    #[serde(default)]
    pub path: Option<String>,
}

impl Shot {
    pub fn seconds(&self) -> f64 {
        self.ends_at - self.starts_at
    }
}

/// What the plugin prints when it is done.
#[derive(Debug, Default, Serialize)]
pub struct Outcome {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<String>,
    /// What was heard in each donor, for kilna to draw on its track.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub suggestions: Vec<Suggestion>,
}

/// What was heard in one donor's finished file.
#[derive(Debug, Clone, Serialize)]
pub struct Suggestion {
    /// The donor, as kilna named it.
    pub source_id: String,
    /// The file that was heard, whole: kilna keeps what was heard only for
    /// as long as it is the donor's finished file.
    pub file: String,
    /// Seconds where its parts begin.
    pub boundaries: Vec<f64>,
    /// The parts worth a short, best first.
    pub stretches: Vec<crate::parts::Part>,
}

/// Say how far along the cut is: one line of JSON on stderr, which kilna
/// draws as a bar under the command.
pub fn progress(share: f64, message: &str) {
    let line = serde_json::json!({
        "progress": share.clamp(0.0, 1.0),
        "message": message,
    });
    eprintln!("{line}");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The manifest kilna reads: protocol 1, a build and a listening on the Cuts tab,
    /// each with an allowance longer than the default two minutes.
    #[test]
    fn the_manifest_offers_a_cutter_and_a_listener_on_the_cuts_tab() {
        let manifest = manifest(&Settings::default());
        assert_eq!(manifest["protocol_version"], 1);
        let commands = manifest["commands"].as_array().unwrap();
        assert_eq!(
            commands
                .iter()
                .map(|command| command["key"].as_str().unwrap())
                .collect::<Vec<_>>(),
            vec!["cut", "suggest"]
        );
        for command in commands {
            assert_eq!(command["target"], "cuts");
            assert!(command["timeout"].as_u64().unwrap() > 120);
        }
    }

    /// An invocation as kilna v0.101 writes it reads, and the fields a cutter
    /// does not use are passed over.
    #[test]
    fn an_invocation_from_kilna_reads() {
        let raw = r#"{
            "command": "cut", "target": "cuts",
            "subject": {
                "id": "w1", "title": "Tide (short 2)", "kind": "short", "ordinal": 2,
                "folder": "D:/media/songs/Tide/shorts/v2",
                "file": {"state": "notYet", "rule": "video/*.mp4"},
                "shots": [
                    {"cut_id": "c1", "position": 1, "starts_at": 12.0, "ends_at": 20.5,
                     "source_id": "v1", "source_title": "Tide (video)",
                     "path": "D:/media/songs/Tide/clip/v1/video/v1.mp4", "file_state": "found"}
                ],
                "donors": [
                    {"id": "v1", "title": "Tide (video)", "kind": "video", "folder": "D:/media/songs/Tide/clip",
                     "file": {"state": "found", "rule": "v*/video/*.mp4", "relative": "v1/video/v1.mp4",
                              "path": "D:/media/songs/Tide/clip/v1/video/v1.mp4"}},
                    {"id": "a1", "title": "Tide (audio)", "kind": "audio", "folder": null,
                     "file": {"state": "noFolder"}}
                ],
                "chain": [{"id": "v1", "title": "Tide (video)", "kind": "video"}],
                "origin": {"id": "s1", "title": "Tide", "kind": "song"},
                "cover": null
            }
        }"#;
        let invocation: Invocation = serde_json::from_str(raw).unwrap();
        assert_eq!(invocation.command, "cut");
        assert_eq!(invocation.subject.shots[0].seconds(), 8.5);
        assert_eq!(invocation.subject.cover, None);
        let donors = &invocation.subject.donors;
        assert_eq!(
            donors[0].path(),
            Some("D:/media/songs/Tide/clip/v1/video/v1.mp4"),
            "a donor's file is read as kilna describes it"
        );
        assert_eq!(donors[1].path(), None, "one with no file has none");
    }

    /// What a build does now is what the form shows: the manifest's defaults
    /// are the settings as they stand, the file's among them. Every setting is
    /// offered, and each says it in Russian too.
    #[test]
    fn the_settings_are_declared_with_what_a_build_does_now() {
        let now = Settings {
            output: "cuts".into(),
            crf: 18,
            ..Settings::default()
        };

        let manifest = manifest(&now);
        let settings = manifest["settings"].as_array().unwrap();
        let of = |key: &str| {
            settings
                .iter()
                .find(|setting| setting["key"] == key)
                .unwrap_or_else(|| panic!("no setting {key}"))
        };

        assert_eq!(of("output")["default"], json!("cuts"));
        assert_eq!(of("crf")["default"], json!(18));
        assert_eq!(of("crf")["type"], json!("number"));

        let fields = serde_json::to_value(&now).unwrap();
        for key in fields.as_object().unwrap().keys() {
            let setting = of(key);
            assert_eq!(setting["default"], fields[key], "{key}");
            assert!(setting["labels"]["ru"].is_string(), "{key}");
            assert!(setting["descriptions"]["ru"].is_string(), "{key}");
        }
        assert_eq!(settings.len(), fields.as_object().unwrap().len());
    }

    /// A short's look reads as kilna v0.168 writes it, and what the plugin
    /// does not move - the words of the header, the stamp's wear - is passed
    /// over.
    #[test]
    fn a_look_from_kilna_reads() {
        let raw = r##"{"command": "cut", "subject": {"short": {
            "look": {
                "accent": "#FF2E63", "second": "#D3EC3F",
                "header": {"on": true, "call": "the whole song is on the channel"},
                "sign": {"on": true, "art": "<svg/>", "size": 310, "pulse": 0.24, "sway": 5},
                "ground": {"color": "#121114", "pattern": "blots", "motion": true},
                "stamp": {"on": true, "angle": -6, "wear": 0.6},
                "wave": {"on": true, "kind": "bars", "strip": true},
                "karaoke": {"on": true, "font": "Arial", "size": 60},
                "effects": {"zoom": 0.1, "grain": 7, "vignette": true, "glitch": true},
                "tail": {"on": true, "seconds": 1.2}
            },
            "layers": {
                "dir": "T:/layers", "ground": "ground.png", "header": "header.png",
                "blots": [{"file": "blot-0.png", "x": -565, "y": -216, "ax": 23.2, "ay": 23.1,
                           "px": 8.4, "py": 11.4, "phase": 5.1, "kick": -40.6}],
                "stamp": {"file": "stamp.png", "ghost": "stamp-ghost.png", "x": 285, "y": 430},
                "sign": {"open": "sign.png", "blink": "sign-blink.png"}
            },
            "karaoke": {"lines": [[{"word": "tide", "at": 12.5, "end": 13.0}]], "audio": null}
        }}}"##;
        let invocation: Invocation = serde_json::from_str(raw).unwrap();
        let short = invocation.subject.short.unwrap();
        assert_eq!(short.look.wave.kind, crate::short::WaveKind::Bars);
        assert!(short.look.sign.on && short.look.ground.motion);
        assert_eq!(short.layers.blots[0].kick, -40.6);
        assert_eq!(
            short.layers.sign.unwrap().blink.as_deref(),
            Some("sign-blink.png")
        );
        assert_eq!(short.karaoke.unwrap().lines[0][0].word, "tide");
    }

    /// An invocation from a kilna older than the settings still reads.
    #[test]
    fn an_invocation_without_settings_reads() {
        let invocation: Invocation =
            serde_json::from_str(r#"{"command":"cut","subject":{}}"#).unwrap();
        assert!(invocation.settings.is_empty());
    }
}
