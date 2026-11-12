---
title: Glossary
---

# 16 Glossary

Dawesome's words, and what other DAWs call the same thing. The comparisons are
approximate: other DAWs' features don't always work the same way.

## 16-1 Arranging

| Dawesome | Meaning | Elsewhere |
| --- | --- | --- |
| **Playlist** | The tab where you lay clips out on tracks against time to build the song. | Arrangement view (Ableton Live), Playlist (FL Studio), Tracks area (Logic Pro), Edit window (Pro Tools) |
| **Track** | A horizontal lane in the playlist. Each track has its own mixer strip. | Track |
| **Clip** | One block on the playlist. A *sample clip* plays an audio file; a *pattern clip* plays a pattern. A clip can play part of its sample or pattern (trimmed), and has its own **Gain**, **Pan**, **Pitch**, **Attack** and **Decay**. | Clip (Ableton, FL Studio), region (Logic Pro), clip/region (Pro Tools), event (Cubase) |
| **Sample** | An audio file. | Sample, audio file |
| **Take** | One recording, saved as a WAV file in the project's `recordings/` folder. | Take |
| **Select tool** (**S**) | The playlist's normal mouse tool: select, move and trim clips. | Pointer tool |
| **Cut tool** (**C**) | Click a clip to split it in two at that point. Not the same as **Cut (⌘X)**, which cuts to the clipboard. | Scissors tool (Logic Pro), Slice tool (FL Studio), Split (Ableton Live) |
| **Split at playhead** | Splits the clip at the playhead, from the clip menu. | Split at playhead (Logic Pro), Split (Ableton Live) |
| **Repeat** (**⌘R**) | Places a copy of the selected clips straight after them, keeping their gaps. | Duplicate (Ableton Live), Repeat (Logic Pro), Duplicate (Pro Tools) |
| **Join** (**Shift+J**) | Makes one clip of several on a track. Pieces of one sample that are still where a cut left them are put back together; other sample clips are written into one new file. Pattern clips become one new pattern. | Consolidate (Ableton Live, Pro Tools), Join Regions (Logic Pro) |
| **Deep copy** | Writes a clip out as a new audio file, with its trim, gain, pan, pitch, attack and decay applied, and switches the clip to it. The track's mixer strip and effects are not applied. | ≈ Make unique as sample (FL Studio), Bounce in Place (Logic Pro), Consolidate (Ableton Live) |
| **Explode into tracks** (**Shift+E**) | Splits a pattern clip into one clip per sample it plays, each on a track of its own, so each sample gets its own mixer strip. | ≈ Split by channel (FL Studio) |
| **Snap** | Makes moved, trimmed and cut clips land on the grid. The grid size is set beside the snap button (**1 bar** to **1/4 beat**). Hold **Option (Alt)** while dragging an edge to ignore it. | Snap to grid, grid quantize |
| **Lasso** | A box drawn with **Shift-drag** to select clips or notes. | Marquee, rubber band |

## 16-2 Patterns and notes

| Dawesome | Meaning | Elsewhere |
| --- | --- | --- |
| **Pattern** | A short loop of steps and notes, made in the **Sequencer** or **Piano Roll**, placed in the playlist as pattern clips. | Pattern (FL Studio), MIDI clip (Ableton Live), MIDI region (Logic Pro) |
| **Sequencer** | The drum machine tab: rows of samples, and a pattern of 16th-note steps for each. | Channel rack / step sequencer (FL Studio), Step Sequencer (Logic Pro) |
| **Piano Roll** | A pattern's notes on a grid, against a piano keyboard. | Piano roll, MIDI editor |

## 16-3 Recording

| Dawesome | Meaning | Elsewhere |
| --- | --- | --- |
| **Arm** | The **R** button on a track's mixer strip. Armed tracks record when you press **Record**. | Record enable, record arm |
| **Count in** | Plays one bar before recording starts. | Count-in, pre-roll |
| **Recording offset** | How much earlier takes are placed than the devices' reported latency says, set on the **Audio** tab. | Recording delay (Logic Pro), latency compensation |
| **Transport** | The tab with a cassette deck and its play, pause, stop and wind keys. The top bar has the main transport buttons too. | Transport |

## 16-4 Mixing

| Dawesome | Meaning | Elsewhere |
| --- | --- | --- |
| **Mixer** | The tab with every track's strip and the master. | Mixer, console |
| **Channel** / **strip** | One track's column in the mixer: fader, pan, mute, solo, meter, arm. | Channel strip, mixer track, insert (FL Studio) |
| **Master** | The strip every track feeds. | Master bus, stereo out |
| **Effects Rack** | One channel's plugins, listed in the order the sound passes through them. An instrument, if any, is always first. | Device chain (Ableton Live), insert slots (Logic Pro, Pro Tools), mixer effect slots (FL Studio). Not the same as an Ableton "Rack". |
| **FX jack** | The jack at the foot of a mixer strip. Shows how many effects the channel has, and opens its Effects Rack. | Insert section |
| **Instrument** | A plugin that plays a track's notes. | Software instrument, VSTi |
| **Effect** | A plugin that changes sound passing through it. | Insert effect, plugin |
| **Bypass** | Switching an effect off without removing it. | Bypass |
| **Mixdown** | **File ▸ Mixdown…**: the whole song, as heard, to a WAV or MP3 file. | Export (Ableton Live, FL Studio), Bounce (Logic Pro, Pro Tools), Export Audio Mixdown (Cubase) |

<!-- PROVISIONAL: Effects Rack, FX jack, Instrument, Effect, Bypass (VST work on branch vsts) -->

## 16-5 The window

| Dawesome | Meaning | Elsewhere |
| --- | --- | --- |
| **Tab** / **pane** | Each view (Playlist, Mixer, Sequencer…) opens as a tab. Tabs sit in panes, which can be split side by side or stacked. | Windows, views |
| **Sidebar** | The browsers on the left: **Samples**, **Patterns**, **Recordings**, **Clips** and **Effects**. | Browser |
| **Sample** tab | Shows one clip and how it plays: **Gain**, **Pan**, **Pitch**, **Attack**, **Decay**. Double-click a clip to open it. | Clip view (Ableton Live), region inspector (Logic Pro) |
| **Project folder** | The folder a project is saved in, holding the `.dawesome` file, `recordings/` and `clips/`. | Project folder |
