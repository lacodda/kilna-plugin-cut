//! The plugin as kilna runs it, against a real ffmpeg.
//!
//! Ignored by default: it needs ffmpeg on the PATH and a font, which CI's
//! `cut` job installs. A clip and a cover are made by ffmpeg itself, the
//! plugin is run with an invocation shaped as kilna v0.101 writes it, and
//! the file it names is measured.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn ffmpeg(args: &[&str]) {
    let status = Command::new("ffmpeg")
        .args(["-v", "error", "-y"])
        .args(args)
        .status()
        .expect("ffmpeg runs");
    assert!(status.success(), "ffmpeg {args:?}");
}

fn length(path: &Path) -> f64 {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "csv=p=0",
        ])
        .arg(path)
        .output()
        .expect("ffprobe runs");
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .unwrap()
}

fn size(path: &Path) -> (u32, u32) {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height",
            "-of",
            "csv=p=0",
        ])
        .arg(path)
        .output()
        .expect("ffprobe runs");
    let text = String::from_utf8_lossy(&output.stdout);
    let mut parts = text
        .trim()
        .split(',')
        .map(|part| part.parse::<u32>().unwrap());
    (parts.next().unwrap(), parts.next().unwrap())
}

fn run(invocation: &serde_json::Value, settings: &Path) -> (serde_json::Value, String) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_kilna-plugin-cut"))
        .arg("run")
        .env("KILNA_PLUGIN_CUT_SETTINGS", settings)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(invocation.to_string().as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    (
        serde_json::from_slice(&output.stdout).unwrap(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// Two stretches of a clip and a cover become one vertical file in the
/// short's folder, as long as the stretches and the cover, beside the cut
/// before it - and the progress is told as it goes.
#[test]
#[ignore]
fn a_short_is_cut_from_its_stretches() {
    let dir = std::env::temp_dir().join(format!("kpc-it-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let short = dir.join("songs").join("Tide").join("shorts").join("v2");
    std::fs::create_dir_all(short.join("covers")).unwrap();
    std::fs::create_dir_all(short.join("video")).unwrap();
    std::fs::write(short.join("video").join("v1.mp4"), b"the cut before").unwrap();
    let clip = dir.join("clip.mp4");
    ffmpeg(&[
        "-f",
        "lavfi",
        "-i",
        "testsrc=size=1280x720:rate=30:duration=20",
        "-f",
        "lavfi",
        "-i",
        "sine=frequency=440:duration=20",
        "-c:v",
        "libx264",
        "-pix_fmt",
        "yuv420p",
        "-c:a",
        "aac",
        "-shortest",
        clip.to_str().unwrap(),
    ]);
    ffmpeg(&[
        "-f",
        "lavfi",
        "-i",
        "color=c=orange:size=1080x1920",
        "-frames:v",
        "1",
        short.join("covers").join("v1.png").to_str().unwrap(),
    ]);
    let settings: PathBuf = dir.join("kilna-plugin-cut.json");
    std::fs::write(
        &settings,
        r#"{"channel": "Harbour Radio", "call": "WHOLE SONG: it's here"}"#,
    )
    .unwrap();

    let invocation = serde_json::json!({
        "command": "cut", "target": "cuts",
        "subject": {
            "id": "w1", "title": "Tide (short 2)", "kind": "short", "ordinal": 2,
            "folder": short.to_string_lossy(),
            "shots": [
                {"cut_id": "c1", "position": 1, "starts_at": 2.0, "ends_at": 6.0,
                 "source_id": "v1", "source_title": "Tide (video)", "path": clip.to_string_lossy(),
                 "file_state": "found"},
                {"cut_id": "c2", "position": 2, "starts_at": 12.5, "ends_at": 15.0,
                 "source_id": "v1", "source_title": "Tide (video)", "path": clip.to_string_lossy(),
                 "file_state": "found"}
            ],
            "origin": {"id": "s1", "title": "Tide: a song", "kind": "song"},
            "cover": null
        }
    });

    let (outcome, stderr) = run(&invocation, &settings);

    assert_eq!(outcome["error"], serde_json::Value::Null, "{outcome}");
    let file = PathBuf::from(outcome["files"][0].as_str().unwrap());
    assert_eq!(
        file,
        short.join("video").join("v2.mp4"),
        "beside the cut before"
    );
    assert_eq!(size(&file), (1080, 1920));
    let seconds = length(&file);
    assert!(
        (seconds - 7.0).abs() < 0.2,
        "4 s and 2.5 s of stretches and half a second of cover, not {seconds}"
    );
    assert!(
        stderr.contains("\"progress\""),
        "progress is told: {stderr}"
    );
    assert!(
        !short.join("video").join("v2.mp4.part").exists(),
        "nothing half-written is left"
    );

    // A stretch whose donor has no file is said, and nothing is cut.
    let mut broken = invocation.clone();
    broken["subject"]["shots"][1]["path"] = serde_json::Value::Null;
    let (outcome, _) = run(&broken, &settings);
    assert!(
        outcome["error"]
            .as_str()
            .unwrap()
            .contains("no finished file")
    );
    assert!(!short.join("video").join("v3.mp4").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

/// A song of a quiet verse and a loud chorus, twice, is heard as parts that
/// begin where the chords turn, and the chorus is offered first - nothing is
/// cut.
#[test]
#[ignore]
fn the_parts_of_a_song_are_heard() {
    let dir = std::env::temp_dir().join(format!("kpc-parts-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // A verse of one chord, quiet, and a chorus of another, loud: 25 s each,
    // twice over, under a still picture.
    let chord = |notes: [f64; 3], volume: f64, seconds: u32, name: &str| {
        let path = dir.join(name);
        let sines: Vec<String> = notes
            .iter()
            .map(|hz| format!("sine=frequency={hz}:duration={seconds}"))
            .collect();
        ffmpeg(&[
            "-f",
            "lavfi",
            "-i",
            &sines[0],
            "-f",
            "lavfi",
            "-i",
            &sines[1],
            "-f",
            "lavfi",
            "-i",
            &sines[2],
            "-filter_complex",
            &format!("amix=inputs=3,volume={volume}"),
            path.to_str().unwrap(),
        ]);
        path
    };
    let verse = chord([261.6, 329.6, 392.0], 0.15, 25, "verse.wav");
    let chorus = chord([349.2, 440.0, 523.3], 1.0, 25, "chorus.wav");
    let list = dir.join("list.txt");
    let line = |path: &Path| format!("file '{}'\n", path.to_string_lossy().replace('\\', "/"));
    std::fs::write(
        &list,
        [&verse, &chorus, &verse, &chorus]
            .iter()
            .map(|path| line(path))
            .collect::<String>(),
    )
    .unwrap();
    let song = dir.join("song.wav");
    ffmpeg(&[
        "-f",
        "concat",
        "-safe",
        "0",
        "-i",
        list.to_str().unwrap(),
        song.to_str().unwrap(),
    ]);
    let clip = dir.join("clip.mp4");
    ffmpeg(&[
        "-f",
        "lavfi",
        "-i",
        "color=c=navy:size=320x180:rate=10:duration=100",
        "-i",
        song.to_str().unwrap(),
        "-c:v",
        "libx264",
        "-pix_fmt",
        "yuv420p",
        "-c:a",
        "aac",
        "-shortest",
        clip.to_str().unwrap(),
    ]);
    let settings = dir.join("kilna-plugin-cut.json");
    std::fs::write(&settings, "{}").unwrap();

    let invocation = serde_json::json!({
        "command": "suggest", "target": "cuts",
        "subject": {
            "id": "w1", "title": "Tide (short 2)", "kind": "short",
            "donors": [
                {"id": "v1", "title": "Tide (video)", "kind": "video", "folder": dir.to_string_lossy(),
                 "file": {"state": "found", "rule": "*.mp4", "relative": "clip.mp4", "path": clip.to_string_lossy()}},
                {"id": "v2", "title": "Tide (audio)", "kind": "audio", "folder": null,
                 "file": {"state": "noFolder"}}
            ],
            "shots": []
        }
    });

    let (outcome, stderr) = run(&invocation, &settings);

    assert_eq!(outcome["error"], serde_json::Value::Null, "{outcome}");
    let heard = outcome["suggestions"].as_array().unwrap();
    assert_eq!(heard.len(), 1, "the donor with no file is passed over");
    assert_eq!(heard[0]["source_id"], "v1");
    let boundaries: Vec<f64> = heard[0]["boundaries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b.as_f64().unwrap())
        .collect();
    for turn in [25.0, 50.0, 75.0] {
        assert!(
            boundaries.iter().any(|b| (b - turn).abs() <= 1.5),
            "a part begins at {turn}: {boundaries:?}"
        );
    }
    let best = &heard[0]["stretches"][0];
    let starts = best["starts_at"].as_f64().unwrap();
    assert!(
        [25.0, 75.0]
            .iter()
            .any(|chorus| (starts - chorus).abs() <= 1.5),
        "the chorus is offered first: {best}"
    );
    assert!(
        stderr.contains("\"progress\""),
        "progress is told: {stderr}"
    );
    assert!(
        outcome["message"].as_str().unwrap().contains("passed over"),
        "{outcome}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
