<p align="center">
  <a href="https://github.com/lacodda/kilna-plugin-cut/actions"><img src="https://img.shields.io/github/actions/workflow/status/lacodda/kilna-plugin-cut/ci.yml?style=flat-square" alt="CI"></a>
  <a href="https://github.com/lacodda/kilna-plugin-cut/releases"><img src="https://img.shields.io/github/v/release/lacodda/kilna-plugin-cut?style=flat-square" alt="Release"></a>
  <a href="https://github.com/lacodda/kilna-plugin-cut/blob/main/LICENSE"><img src="https://img.shields.io/github/license/lacodda/kilna-plugin-cut?style=flat-square" alt="License"></a>
</p>

> Builds a short out of the stretches [kilna](https://github.com/lacodda/kilna) keeps for it - a vertical video under the look of its channel: the picture, the ground, the stamp, a sign that moves with the sound, the words lit as they are sung, its cover at the end - into the short's own folder.

kilna marks where in a video a short is taken from, keeps what a channel's shorts look like, and never opens the video itself. This plugin does the listening and the building: **Find the parts** on a short's Cuts tab marks where the parts of each video it is cut from begin and offers the parts worth a short; **Build the short** lays a new `v{k}.mp4` in the short's folder, where kilna finds it as the short's finished file.

## Install

Windows (PowerShell):

```powershell
irm https://raw.githubusercontent.com/lacodda/kilna-plugin-cut/main/tools/install.ps1 | iex
```

macOS and Linux:

```sh
curl -fsSL https://raw.githubusercontent.com/lacodda/kilna-plugin-cut/main/tools/install.sh | sh
```

The installer puts the plugin in kilna's `plugins` folder. It needs kilna v0.168 or later, and **ffmpeg** and **ffprobe** on the `PATH`; without them kilna lists the plugin with the reason instead of a button. `KILNA_PLUGIN_CUT_VERSION` pins a release, `KILNA_PLUGIN_CUT_DIR` another folder.

## What a short looks like

What kilna says it looks like. The look is the style of the space - Settings, **Shorts** - with the short's variant laid over it, and kilna draws everything that stands still itself: the ground and its blots, the stamp, the sign, the title and the call. The plugin is handed those layers and moves them on a 1080x1920 frame at 30 frames a second:

- the stretches, spliced in order, as a 16:9 picture that pushes in slowly, with grain, a vignette and a tear of colour where the sound hits;
- the ground, whose blots drift and are thrown sideways on a hit;
- a strip under the picture that fills as the short plays, and the sound drawn as a filled wave, two lines or mirrored bars;
- the words of the song, lit word by word as they are sung;
- the stamp, with a ghost of itself on a hit;
- the sign, which grows with the loudness, sways and blinks;
- the short's cover at the end, pulling back to rest whole.

Every block is the look's to turn off. A build never overwrites the one before it: `v2.mp4` lies down beside `v1.mp4`, and a build stopped halfway leaves nothing kilna would take for the short's file.

The cover is the one kilna hands over, or else the newest picture in the short's `covers` folder.

### The words

kilna matches the song's text against a word alignment laid beside the song (`*.alignment.json`) and hands over every line with the second each word is sung at. The plugin places the video inside the recording those seconds were timed on - a video often opens a little later than its track - and shows the lines sung in each stretch. A short with no alignment is built without words.

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

In kilna: **Settings → Plugins**. What is set there is handed over with every run and lies on top of `kilna-plugin-cut.json` beside the executable, which sets the defaults the form starts from. Both are optional, and every key of the file too.

| Key | Default | Meaning |
| --- | --- | --- |
| `output` | `video` | Where the short goes inside its folder. |
| `covers` | `covers` | Where the cover is looked for inside the short's folder. |
| `crf` | `23` | x264 quality; lower is better. |

`KILNA_PLUGIN_CUT_SETTINGS` points at another file. The words on the frame, its colours and its fonts are the look's, set in kilna.

## The protocol

A kilna plugin is an executable run once per call: `--manifest` prints what it offers, `run` reads one invocation on stdin and prints one outcome on stdout. This one offers two commands on the Cuts tab - a build with a thirty-minute allowance and the listening with ten minutes - writes progress as JSON lines on stderr, which kilna draws as a bar with a Stop button, and answers with the file it made or with `suggestions`: for each donor, the file heard, where its parts begin and the parts offered. A build reads `subject.short` - the look, the layers and the words - which kilna hands over from v0.168; an older kilna is told so. See [Writing a plugin](https://lacodda.github.io/kilna/guides/writing-a-plugin/).

## License

MIT
