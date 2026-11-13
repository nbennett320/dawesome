---
title: Specifications
---

# 15 Specifications

Version 0.1.0.
<!-- VERIFY: version from engine/tauri.conf.json and Cargo.toml at c2295cb; update per release -->

## 15-1 Audio files Dawesome plays

| Format | Extensions |
| --- | --- |
| WAV (PCM integer and float) | `.wav` |
| FLAC | `.flac` |
| Ogg Vorbis | `.ogg` |
| MP3 | `.mp3` |

<!-- VERIFY: derived from the decoders compiled in (rodio 0.19 default features: hound, claxon, lewton, symphonia-mp3; symphonia 0.5 default features add riff/ogg/mkv formats and pcm/adpcm/vorbis/flac codecs). Not tested per format. ADPCM WAV and Matroska/WebM audio may also decode; AAC/M4A, AIFF and ALAC are not compiled in. -->

Files of any sample rate play at the output's rate; Dawesome converts them as
they play.

::: tip NOTE
Files are added from the sidebar's **Samples** browser by dragging.
Dropping files from your computer's file manager onto the window isn't
supported.
:::

<!-- VERIFY: the Samples browser lists the folder ./assets/ relative to the app's working directory, and lists every file in it whatever its type. How users get their own samples in is unclear; check before documenting the samples folder. -->

## 15-2 Files Dawesome writes

| What | Format |
| --- | --- |
| Recorded takes | WAV, 32-bit float, at the output's sample rate, one channel per input channel recorded |
| **Deep copy** and **Join** | WAV, 32-bit float, stereo, at the output's sample rate |
| Mixdown, WAV | 16-bit, 24-bit or 32-bit float; stereo; at the output's sample rate |
| Mixdown, MP3 | 128, 192, 256 or 320 kbps; stereo |

- A WAV mixdown can't be larger than 4 GB. For a longer song, choose a lower
  bit depth or MP3.
- An MP3 mixdown is made at no more than 48 kHz. Above that, it is made at
  44.1 kHz or 48 kHz, whichever the output's rate is a multiple of.
- A mixdown never overwrites a file. If the name is taken, a number is added:
  `song.wav`, `song 2.wav`, …

## 15-3 Sample rate and buffer size

| Item | Specification |
| --- | --- |
| Sample rate | The output device's own rate. There is no sample-rate setting in Dawesome; change it in your system's audio settings. <!-- VERIFY: macOS Audio MIDI Setup changes the default rate cpal reports --> |
| Buffer size | **Device default**, or 64, 128, 256, 512 or 1024 frames (only the sizes the device accepts are offered) |
| Output sample formats | 32-bit float, 16-bit and 32-bit integer, 16-bit unsigned |
| Output channels | Stereo. A mono device gets both sides mixed; on a device with more than two channels, channels 3 and up are silent. |
| Recording offset | −1000 to 1000 ms |

If the input can't run at the output's rate, takes are recorded at the input's
rate and converted as they play.

## 15-4 Plugins

<!-- PROVISIONAL -->

| Item | Specification |
| --- | --- |
| Formats | VST3 only |
| Kinds | Effects and instruments |
| Audio | 32-bit float processing |
| Scan timeout | 15 seconds per plugin |
| Plugin editor windows | macOS only |

Plugins are looked for in these folders, and the folders inside them:

| System | Folders |
| --- | --- |
| macOS | `/Library/Audio/Plug-Ins/VST3`, `~/Library/Audio/Plug-Ins/VST3` |
| Windows | `C:\Program Files\Common Files\VST3` (from `%COMMONPROGRAMFILES%`) |
| Linux | `~/.vst3`, `/usr/lib/vst3`, `/usr/local/lib/vst3` |

## 15-5 Projects

| Item | Specification |
| --- | --- |
| File | `.dawesome`, a JSON text file |
| Format version | 4 for a project with Pattern Sequencer rows, patterns or pattern folders. One without any is saved as version 3 if it has trimmed sample clips, or else as version 1, so older versions of Dawesome can open it. Projects of versions 1 to 3 open, and their patterns are converted (see [7-8](./patterns.md#_7-8-patterns-from-older-versions)). |
| Sample paths | Relative to the project's folder for samples inside it; full paths otherwise |
| Project folder | `recordings/` for takes, `clips/` for files made by **Deep copy** and **Join** |
| Plugins | Saved with their id, file location and settings. A missing plugin keeps its settings and is saved again unchanged. |
| Tempo | 1 to 1000 BPM |
| Playlist length | 128 bars of the project's time signature, or to the end of the last clip if that's later |
| Pattern length | 1 to 128 bars |

## 15-6 Systems

| System | Status |
| --- | --- |
| macOS | Supported. Plugin editor windows work here only. |
| Windows | Builds for it exist in the code; plugin editor windows aren't supported. <!-- VERIFY: not built or tested on Windows; CI runs on Ubuntu only --> |
| Linux | Builds for it exist in the code; plugin editor windows aren't supported. <!-- VERIFY: CI runs engine tests on Ubuntu; no release build checked --> |

Dawesome is built with Tauri 2 and needs Rust 1.86 or later to build from
source.
<!-- VERIFY: minimum macOS / Windows / Linux versions not stated anywhere in the code -->
