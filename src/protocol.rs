//! What kilna hands the plugin, and what the plugin hands back.
//!
//! kilna's plugin protocol 1: `--manifest` prints what the plugin offers,
//! `run` reads one invocation from stdin and prints one outcome to stdout.
//! Since kilna v0.101 a work travels with everything a cutter needs - the
//! shot list beside each donor's finished file, the work's folder, the chain
//! of titles up to the song, its cover - and a command may take longer than
//! two minutes, report progress on stderr and name the files it made.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The protocol this plugin speaks.
pub const PROTOCOL_VERSION: u32 = 1;

/// How long cutting may take: a short of a minute re-encoded twice, on a slow
/// machine, with room to spare.
pub const TIMEOUT_SECONDS: u64 = 1800;

/// What `--manifest` prints.
pub fn manifest() -> Value {
    serde_json::json!({
        "protocol_version": PROTOCOL_VERSION,
        "name": "cut",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "Cuts a short out of its stretches: a vertical 1080x1920 video with the picture over its own blur, the channel and the song above, a call below, and the cover at the end.",
        "commands": [{
            "key": "cut",
            "label": "Cut the short",
            "description": "Cuts the stretches in order into a new file in the short's folder; the file before it stays.",
            "target": "cuts",
            "timeout": TIMEOUT_SECONDS
        }]
    })
}

/// One invocation, as kilna writes it to stdin.
#[derive(Debug, Deserialize)]
pub struct Invocation {
    pub command: String,
    pub subject: Subject,
}

/// The work a command runs on - only what a cutter reads of it.
#[derive(Debug, Default, Deserialize)]
pub struct Subject {
    #[serde(default)]
    pub title: String,
    /// The work's folder on disk, whole, when its kind names one.
    #[serde(default)]
    pub folder: Option<String>,
    /// The stretches it is spliced from, in order.
    #[serde(default)]
    pub shots: Vec<Shot>,
    /// What it is all made from - the song, for a short.
    #[serde(default)]
    pub origin: Option<Named>,
    /// The picture it goes out under, whole, when it has one.
    #[serde(default)]
    pub cover: Option<String>,
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

/// A work as the chain names it.
#[derive(Debug, Clone, Deserialize)]
pub struct Named {
    pub title: String,
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

    /// The manifest kilna reads: protocol 1, one command on the Cuts tab
    /// with an allowance longer than the default two minutes.
    #[test]
    fn the_manifest_offers_one_cutter_on_the_cuts_tab() {
        let manifest = manifest();
        assert_eq!(manifest["protocol_version"], 1);
        assert_eq!(manifest["commands"][0]["target"], "cuts");
        assert!(manifest["commands"][0]["timeout"].as_u64().unwrap() > 120);
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
                "donors": [], "chain": [{"id": "v1", "title": "Tide (video)", "kind": "video"}],
                "origin": {"id": "s1", "title": "Tide", "kind": "song"},
                "cover": null
            }
        }"#;
        let invocation: Invocation = serde_json::from_str(raw).unwrap();
        assert_eq!(invocation.command, "cut");
        assert_eq!(invocation.subject.shots[0].seconds(), 8.5);
        assert_eq!(invocation.subject.origin.unwrap().title, "Tide");
        assert_eq!(invocation.subject.cover, None);
    }
}
