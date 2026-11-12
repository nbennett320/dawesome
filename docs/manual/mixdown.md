---
title: Mixdown
---

# 12 Mixdown

Mixdown renders the whole song to a WAV or MP3 file, as you hear it in
the playlist.

## 12-1 Opening the Mixdown tab

Choose **File ▸ Mixdown…**. The **Mixdown** tab opens.

![The Mixdown tab](./img/mixdown.png)
<!-- SCREENSHOT: Mixdown tab with WAV chosen, numbered 1–5 -->

1. **Name** — The file's name. The extension (**.wav** or **.mp3**) is
   shown after the field and added for you.
2. **Path** — The folder the file goes in. Click **Browse…** to choose
   another.
3. **File type** — **WAV** or **MP3**.
4. **Bit depth** — **16-bit**, **24-bit** or **32-bit float**.
5. **Confirm** — Starts the mixdown.

When **File type** is **MP3**, **Bit depth** is replaced by **Bitrate**:
**128 kbps**, **192 kbps**, **256 kbps** or **320 kbps**.

### 12-1-1 Defaults

| Option | Default |
| --- | --- |
| **Name** | The project's name, or **Untitled** for a project never saved |
| **Path** | A `mixdowns` folder next to the project file, or your Music folder for a project never saved |
| **File type** | **WAV** |
| **Bit depth** | **16-bit** |
| **Bitrate** | **320 kbps** |

<!-- VERIFY: the "Music folder" default on Windows and Linux (the system's audio folder, else the home folder) -->

The name and folder you choose are kept for that project until you quit.
The format choices are kept until you quit. None of them are saved in the
project file.

## 12-2 Making a mixdown

1. Choose **File ▸ Mixdown…**.
2. Set **Name**, **Path** and the format.
3. Click **Confirm** (or press **Return (Enter)** in **Name**).

The tab shows **Mixing down…** with a progress bar. Click **Cancel** to
stop; nothing of the file is kept.

When it's done, the tab shows **Done** with the file's name and path.

- **Show in Finder** — Shows the file in the Finder (or your system's file
  manager).
- **New mixdown** — Returns to the options.

A mixdown carries on if you close the tab. Open it again to see how it's
going. Only one mixdown runs at a time.

If a file of that name is already there, nothing is written over: the new
file is numbered, for example `song 2.wav`.

## 12-3 What is rendered

- The whole playlist, from its start to the end of the last clip, then a
  tail of up to 10 seconds while sound (a reverb, a ringing hit) dies
  away. There is no option to render part of the song.
- Every track through its mixer strip as set when the mixdown starts
  (fader, pan, mute, solo), then the master strip and its limiter.
- The song is played once through. Loop, the metronome and the count-in
  are not rendered.
- **Sample rate:** the audio engine's rate (set by the output device; see
  [Installing and first launch](./install.md)). MP3 goes no higher than
  48 kHz; above that, the song is rendered at 44.1 or 48 kHz.
- **16-bit** WAV files are dithered.

The mixdown renders a copy of the song taken when it starts. You can keep
playing and editing meanwhile; changes don't reach the file.

::: tip NOTE
If the playlist is empty, the mixdown doesn't start and the tab says
**There's nothing in the playlist to mix down**.
:::

### 12-3-1 Effects and instruments

<!-- PROVISIONAL -->

Effects and instruments that are switched on are rendered too, with their
settings as they are when the mixdown starts. If one can't be loaded for
the mixdown, the mixdown fails rather than leaving it out.
