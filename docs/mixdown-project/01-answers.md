# Mixdown — design answers

Asked in chat on 2026-11-10.

## The request (user's words)

> user goes to file menu in the top of the screen or window -> "mixdown" -> a new tab called mixdown is opened with a few configurations to start
>
> configs: name (defaults to project name), path (where we export), file type (maybe we support just wav and mp3 to start), then a button at the bottom that says confirm

## Answers

| # | Question | Answer |
|---|----------|--------|
| 1 | Range | Bar 1 to the end of the last clip, plus a short tail so ringing hits aren't cut off. Loop region later. |
| 2 | Mix state | Exactly what you hear: track volume/pan/mute/solo and the master fader. Never the metronome. |
| 3 | File already exists | Auto-number (`song 2.wav`), never overwrite. Matches bounce.rs. |
| 4 | Default path | `<project folder>/mixdowns`; `~/Music` for a never-saved project. Browse button picks any folder. |
| 5 | WAV format | Bit depth choice: 16-bit (default), 24-bit, 32-bit float. Sample rate = engine rate. |
| 6 | MP3 | In v1, bitrate choice 128/192/256/320 kbps, default 320. |
| 7 | After Confirm | Tab stays open: progress bar + Cancel, then "Done: song.wav" with Show in Finder. |
| 8 | Render method | Offline only in v1, rendered from a snapshot so the user can keep working. Built so a "Live" method can be added later. User's words: "maybe we can have this as an option the user can select - render method fast, or live, that way down the road if the user wants to apply effects live as it is bounced, they can capture that." Then chose offline-only, live-ready, after discussing that effects render identically offline and live capture of knob moves is better served by automation or resampling. |
| 9 | Never-saved project | Name defaults to "Untitled"; exporting doesn't require saving. |

## Facts checked

- `mp3lame-encoder 0.3.0` builds on rustc 1.86 (vendored LAME via `cc`, no system install) and produced a valid 320 kbps / 48 kHz stereo MP3. LAME is LGPL — fine for now, matters if the app is ever distributed closed-source with static linking.
- MP3 (MPEG-1 Layer III) supports at most 48 kHz: an engine running at 88.2/96 kHz must be resampled before encoding.
