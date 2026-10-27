//! kilna-plugin-cut: cuts a short out of the stretches kilna keeps for it.
//!
//! A kilna plugin: an executable named `kilna-plugin-*`, run once per call.
//! `--manifest` says what it offers; `run` reads the invocation from stdin,
//! cuts the short with ffmpeg into a new file in its folder, reports progress
//! on stderr and names the file on stdout. kilna itself never opens a video.

mod cut;
mod ffmpeg;
mod protocol;
mod settings;

use std::io::Read;
use std::process::ExitCode;

use protocol::{Invocation, Outcome};

fn main() -> ExitCode {
    match std::env::args().nth(1).as_deref() {
        Some("--manifest") => describe(),
        Some("run") => {
            let outcome = run();
            match serde_json::to_string(&outcome) {
                Ok(line) => {
                    println!("{line}");
                    ExitCode::SUCCESS
                }
                Err(cause) => {
                    eprintln!("could not write the outcome: {cause}");
                    ExitCode::FAILURE
                }
            }
        }
        Some("--version") => {
            println!("kilna-plugin-cut {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!(
                "kilna-plugin-cut {}: a kilna plugin. kilna runs it as `--manifest` and `run`.",
                env!("CARGO_PKG_VERSION")
            );
            ExitCode::from(2)
        }
    }
}

/// Print the manifest - unless ffmpeg is missing, which is said on stderr so
/// kilna lists the plugin with the reason.
fn describe() -> ExitCode {
    if let Err(reason) = ffmpeg::find() {
        eprintln!("{reason}");
        return ExitCode::FAILURE;
    }
    println!("{}", protocol::manifest());
    ExitCode::SUCCESS
}

/// One call: read, cut, answer. A failure is answered as `error`, in words a
/// person can act on, rather than as a crash.
fn run() -> Outcome {
    let mut input = String::new();
    if let Err(cause) = std::io::stdin().read_to_string(&mut input) {
        return failed(format!("could not read the invocation: {cause}"));
    }
    let invocation: Invocation = match serde_json::from_str(&input) {
        Ok(invocation) => invocation,
        Err(cause) => return failed(format!("the invocation does not read: {cause}")),
    };
    if invocation.command != "cut" {
        return failed(format!(
            "there is no command \"{}\" here",
            invocation.command
        ));
    }
    let tools = match ffmpeg::find() {
        Ok(tools) => tools,
        Err(reason) => return failed(reason),
    };
    let settings = match settings::load(settings::path().as_deref()) {
        Ok(settings) => settings,
        Err(reason) => return failed(reason),
    };
    match cut::cut(&invocation.subject, &settings, &tools) {
        Ok(file) => {
            let length = ffmpeg::length(&tools, &file).ok();
            let name = file
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            Outcome {
                message: Some(match length {
                    Some(seconds) => format!("Cut {name}: {}", clock(seconds)),
                    None => format!("Cut {name}"),
                }),
                error: None,
                files: vec![file.to_string_lossy().into_owned()],
            }
        }
        Err(reason) => failed(reason),
    }
}

fn failed(reason: String) -> Outcome {
    Outcome {
        error: Some(reason),
        ..Outcome::default()
    }
}

/// Seconds as a clock: `0:31.5`.
fn clock(seconds: f64) -> String {
    let whole = seconds.max(0.0);
    let minutes = (whole / 60.0).floor();
    let rest = whole - minutes * 60.0;
    if (rest - rest.round()).abs() < 0.05 {
        format!("{minutes:.0}:{:02.0}", rest.round())
    } else {
        format!("{minutes:.0}:{rest:04.1}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seconds_read_as_a_clock() {
        assert_eq!(clock(31.0), "0:31");
        assert_eq!(clock(31.52), "0:31.5");
        assert_eq!(clock(65.0), "1:05");
        assert_eq!(clock(5.5), "0:05.5");
    }
}
