//! The plugin as kilna runs it, against a real ffmpeg.
//!
//! Ignored by default: it needs ffmpeg on the PATH and a font, which CI's
//! `cut` job installs. A clip, a cover and the layers of a look are made by
//! ffmpeg itself, the plugin is run with an invocation shaped as kilna v0.168
//! writes it, and the file it names is measured.

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

/// A picture of one colour, as a layer kilna would have drawn.
fn layer(dir: &Path, name: &str, colour: &str, size: &str) {
    ffmpeg(&[
        "-f",
        "lavfi",
        "-i",
        &format!("color=c={colour}:size={size},format=rgba"),
        "-frames:v",
        "1",
        dir.join(name).to_str().unwrap(),
    ]);
}

/// Two stretches of a clip become one vertical file under a whole look - the
/// ground with a blot, the header, the stamp, bars and a sign down the pipe,
/// the words, the cover at the end - in the short's folder, as long as the
/// stretches and the cover, beside the build before it; and the progress is
/// told as it goes.
#[test]
#[ignore]
fn a_short_is_built_from_its_stretches_under_its_look() {
    let dir = std::env::temp_dir().join(format!("kpc-it-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let short = dir.join("songs").join("Tide").join("shorts").join("v2");
    let layers = dir.join("layers");
    std::fs::create_dir_all(short.join("covers")).unwrap();
    std::fs::create_dir_all(short.join("video")).unwrap();
    std::fs::create_dir_all(&layers).unwrap();
    std::fs::write(short.join("video").join("v1.mp4"), b"the build before").unwrap();
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
    layer(&short.join("covers"), "v1.png", "orange", "1080x1920");
    layer(&layers, "ground.png", "0x121114", "1080x1920");
    layer(&layers, "header.png", "black@0.0", "1080x1920");
    layer(&layers, "blot-0.png", "red@0.3", "300x300");
    layer(&layers, "stamp.png", "red", "400x120");
    layer(&layers, "stamp-ghost.png", "yellow@0.7", "400x120");
    layer(&layers, "sign.png", "white", "420x420");
    let settings: PathBuf = dir.join("kilna-plugin-cut.json");
    std::fs::write(&settings, "{}").unwrap();

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
            "cover": null,
            "short": {
                "look": {
                    "accent": "#FF2E63", "second": "#D3EC3F",
                    "sign": {"on": true, "size": 310, "pulse": 0.24, "sway": 5},
                    "ground": {"motion": true},
                    "wave": {"on": true, "kind": "bars", "strip": true},
                    "karaoke": {"on": true, "font": "DejaVu Sans", "size": 60},
                    "effects": {"zoom": 0.1, "grain": 7, "vignette": true, "glitch": true},
                    "tail": {"on": true, "seconds": 0.5}
                },
                "layers": {
                    "dir": layers.to_string_lossy(), "ground": "ground.png", "header": "header.png",
                    "blots": [{"file": "blot-0.png", "x": 40, "y": 60, "ax": 20, "ay": 20,
                               "px": 8, "py": 11, "phase": 1, "kick": -40}],
                    "stamp": {"file": "stamp.png", "ghost": "stamp-ghost.png", "x": 340, "y": 430},
                    "sign": {"open": "sign.png"}
                },
                "karaoke": {
                    "lines": [
                        [{"word": "the", "at": 2.5, "end": 3.0}, {"word": "tide", "at": 3.0, "end": 4.0},
                         {"word": "comes", "at": 4.0, "end": 4.5}, {"word": "in", "at": 4.5, "end": 5.0}],
                        [{"word": "again", "at": 13.0, "end": 14.0}]
                    ],
                    "offset": 0
                }
            }
        }
    });

    let (outcome, stderr) = run(&invocation, &settings);

    assert_eq!(outcome["error"], serde_json::Value::Null, "{outcome}");
    let file = PathBuf::from(outcome["files"][0].as_str().unwrap());
    assert_eq!(
        file,
        short.join("video").join("v2.mp4"),
        "beside the build before"
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

    // What kilna hands over reaches the build: a quality x264 has not got is
    // said by name, and nothing is built.
    let mut unfit = invocation.clone();
    unfit["settings"] = serde_json::json!({ "crf": 80 });
    let (outcome, _) = run(&unfit, &settings);
    assert!(
        outcome["error"].as_str().unwrap().contains("crf"),
        "{outcome}"
    );
    assert!(!short.join("video").join("v3.mp4").exists());

    // A look with everything off still builds: the stretches over the ground.
    let mut bare = invocation.clone();
    bare["settings"] = serde_json::json!({ "crf": 30 });
    bare["subject"]["short"]["look"] = serde_json::json!({});
    bare["subject"]["short"]["layers"] =
        serde_json::json!({ "dir": layers.to_string_lossy(), "ground": "ground.png" });
    bare["subject"]["short"]["karaoke"] = serde_json::Value::Null;
    let (outcome, _) = run(&bare, &settings);
    assert!(outcome["error"].is_null(), "{outcome}");
    let made = PathBuf::from(outcome["files"][0].as_str().unwrap());
    assert!(made.ends_with("v3.mp4"), "{made:?}");
    assert_eq!(size(&made), (1080, 1920));
    assert!(
        (length(&made) - 6.5).abs() < 0.2,
        "no cover where the look has no tail"
    );
    let _ = std::fs::remove_file(&made);

    // A layer that is not there is said, and nothing is built.
    let mut torn = invocation.clone();
    torn["subject"]["short"]["layers"]["stamp"]["file"] = "missing.png".into();
    let (outcome, _) = run(&torn, &settings);
    assert!(
        outcome["error"].as_str().unwrap().contains("missing.png"),
        "{outcome}"
    );

    // A kilna that hands over no look is told which kilna this needs.
    let mut old = invocation.clone();
    old["subject"]["short"] = serde_json::Value::Null;
    let (outcome, _) = run(&old, &settings);
    assert!(
        outcome["error"].as_str().unwrap().contains("0.168"),
        "{outcome}"
    );

    // A stretch whose donor has no file is said, and nothing is built.
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
