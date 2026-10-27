<p align="center">
  <a href="https://github.com/lacodda/kilna-plugin-cut/actions"><img src="https://img.shields.io/github/actions/workflow/status/lacodda/kilna-plugin-cut/ci.yml?style=flat-square" alt="CI"></a>
  <a href="https://github.com/lacodda/kilna-plugin-cut/releases"><img src="https://img.shields.io/github/v/release/lacodda/kilna-plugin-cut?style=flat-square" alt="Release"></a>
  <a href="https://github.com/lacodda/kilna-plugin-cut/blob/main/LICENSE"><img src="https://img.shields.io/github/license/lacodda/kilna-plugin-cut?style=flat-square" alt="License"></a>
</p>

> Cuts a short out of the stretches [kilna](https://github.com/lacodda/kilna) keeps for it - a vertical video, the picture over its own blur, its titles above and below, its cover at the end - into the short's own folder.

kilna marks where in a video a short is taken from and never opens the video itself. This plugin does the listening and the cutting: **Find the parts** on a short's Cuts tab marks where the parts of each video it is cut from begin and offers the parts worth a short; **Cut the short** lays a new `v{k}.mp4` in the short's folder, where kilna finds it as the short's finished file.

## Install

Windows (PowerShell):

```powershell
irm https://raw.githubusercontent.com/lacodda/kilna-plugin-cut/main/tools/install.ps1 | iex
```

macOS and Linux:

```sh
curl -fsSL https://raw.githubusercontent.com/lacodda/kilna-plugin-cut/main/tools/install.sh | sh
```

The installer puts the plugin in kilna's `plugins` folder and writes an example settings file beside it. It needs **ffmpeg** and **ffprobe** on the `PATH`; without them kilna lists the plugin with the reason instead of a button. `KILNA_PLUGIN_CUT_VERSION` pins a release, `KILNA_PLUGIN_CUT_DIR` another folder.

## What a cut looks like

A 1080x1920 frame:

- the stretch's picture, full width, over a darkened blur of itself;
- a top bar with the channel's name and the song's title, lettered to fit;
- a bottom bar with a call to watch the whole song, and a pointing hand;
- the short's cover for half a second at the end.

Every stretch of the short is cut in the order kilna splices them and joined in one pass, so the file runs exactly as long as its stretches and its cover. A cut never overwrites the one before it: `v2.mp4` lies down beside `v1.mp4`, and a cut stopped halfway leaves nothing kilna would take for the short's file.

The cover is the one kilna hands over, or else the newest picture in the short's `covers` folder.

## Finding the parts

**Find the parts** listens to each video the short is cut from - those with a finished file - and cuts nothing. The song is heard as half-second frames, each with its loudness and the twelve notes that sound in it. A part begins where the notes turn: the frames before a moment alike among themselves and unlike the frames after it. Up to five parts of 18 to 45 seconds, each running from one beginning to another, are offered with a reason:

| Reason | What was heard |
| --- | --- |
| `chorus` | Loud, and something like it sounds again elsewhere. |
| `loudest` | The loudest of the parts offered. |
| `repeats` | It comes back, though it is not loud. |
| `lift` | It comes in louder at once after a quieter stretch. |
| `part` | A part of its own. |

A part that fades out is offered last. kilna draws the beginnings on the video's track - a dragged edge lands on them - and the parts beside it, and the person takes one or none.

## Settings

`kilna-plugin-cut.json` beside the executable; every key is optional.

| Key | Default | Meaning |
| --- | --- | --- |
| `channel` | empty | The top bar's first line. Empty draws none. |
| `title_upper` | `true` | Letter the song's title in capitals. |
| `call` | `FULL VERSION` | The bottom bar's words. Empty draws none, and no hand. |
| `hand` | `true` | The pointing hand under the call. |
| `font`, `font_bold` | Arial, or DejaVu Sans on Linux | TrueType files for the channel, and for the title and the call. |
| `ink` | `white` | The colour of the words, as ffmpeg reads one. |
| `bar_shade` | `0.5` | How dark the bars over the blur are, 0 to 1. |
| `output` | `video` | Where the cut goes inside the short's folder. |
| `covers` | `covers` | Where the cover is looked for inside the short's folder. |
| `cover_seconds` | `0.5` | How long the cover stands at the end; `0` leaves it off. |
| `crf` | `20` | x264 quality; lower is better. |

`KILNA_PLUGIN_CUT_SETTINGS` points at another file.

## The protocol

A kilna plugin is an executable run once per call: `--manifest` prints what it offers, `run` reads one invocation on stdin and prints one outcome on stdout. This one offers two commands on the Cuts tab - a cut with a thirty-minute allowance and the listening with ten minutes - writes progress as JSON lines on stderr, which kilna draws as a bar with a Stop button, and answers with the file it made or with `suggestions`: for each donor, the file heard, where its parts begin and the parts offered (kilna v0.103 and later). See [Writing a plugin](https://lacodda.github.io/kilna/guides/writing-a-plugin/).

The pointing hand is rendered from [Noto Color Emoji](https://github.com/googlefonts/noto-emoji) (Apache License 2.0).

## License

MIT
