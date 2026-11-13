---
title: Troubleshooting
---

# 14 Troubleshooting

Most problems show a message in the app. This chapter lists those messages,
what they mean, and what to do.

## 14-1 No sound

1. Choose **Preferences ▸ Audio Settings…**. The **Audio** tab opens.
2. Look at **Playback Status**. It shows the device in use, its sample rate,
   its buffer and its latency.
   - If it reads **Nothing: the output couldn't be opened**, Dawesome has no
     output. Choose another device under **Output**, or **System default**.
3. Check that **Output** names the device you are listening on. A saved
   device that is unplugged is shown as **(not connected)**.
4. Click **Refresh devices** after plugging a device in, to list it.
5. Check the mixer: a muted track, a soloed other track, or a fader at the
   bottom gives silence. See [Mixer](./mixer.md).

Messages you may see at the top of the **Audio** tab:

| Message | Meaning |
| --- | --- |
| The saved audio output couldn't be used (…), so the system default is playing instead | The device saved last time is gone, or won't take the saved buffer size. Pick a device again. |
| The audio output couldn't be opened: there's no audio output device | The computer reports no output at all. Connect or enable one. |
| …there's no audio output named '…' | The chosen device isn't connected. |
| The audio settings are in use, but couldn't be saved for next time: … | Playback works, but the settings file couldn't be written. See [14-5](#_14-5-where-settings-and-messages-are). |

::: tip NOTE
Changing the output or buffer size pauses playback and stops any recording in
progress.
:::

## 14-2 Crackling or dropouts

Crackles mean the computer can't fill the audio buffer in time.

1. Open **Preferences ▸ Audio Settings…**.
2. Under **Buffer size**, choose a bigger buffer (for example 512 or 1024
   frames). Each choice shows its length in milliseconds.

A bigger buffer adds delay between pressing play and hearing sound. A smaller
one responds faster but needs more of the computer.

If the tab says **Set to N, but *device* won't go lower than M**, the device
is using a bigger buffer than you asked for. If it says **'*device*' can't use
a buffer of N frames**, that size isn't one the device accepts; choose another,
or **Device default**.

Plugins also add work. Bypassing or removing heavy effects reduces crackling.
<!-- VERIFY: not tested; plugin load vs crackle is a reasonable inference only -->

## 14-3 A plugin isn't listed, or won't load

<!-- PROVISIONAL -->

Dawesome hosts VST3 plugins only. It looks for them in the standard VST3
folders (see [Specifications](./specs.md)).

**The plugin isn't in the Effects browser**

1. Open the **Effects** page of the sidebar.
2. If it reads **No VST3 effects found** (or instruments), the list under
   **Looked in:** shows the folders that were searched. Install the plugin's
   `.vst3` into one of them.
3. Click the rescan button beside the search field to look again.

Each plugin is scanned in a separate process, so a plugin that crashes can't
take Dawesome down. Plugins that fail are listed at the bottom under
**Couldn't be read (N)**, with the reason:

| Reason | Meaning |
| --- | --- |
| crashed while being scanned | The plugin crashed when opened. |
| didn't finish scanning in 15 seconds | The plugin hung. |
| the scanner failed (exit code N) | The scanner reported an error. |

Rescanning tries these again.

**The plugin is listed but won't load**

The message names the plugin and the reason, for example:

- **… can't run at N Hz** — the plugin doesn't support your output's sample
  rate. Choose an output running at another rate.
- **… can't process 32-bit audio**, **… has no audio output** — the plugin
  can't be used by Dawesome.

**Plugin windows on Windows and Linux**

Opening a plugin's own interface shows **Plugin windows aren't supported on
this system yet**. Plugin editors open on macOS only in this version.

**A project's plugins are missing**

When a project uses a plugin that isn't installed, a banner says **These
plugins aren't installed, so their effects are passing sound straight through
(their settings are kept)**. Install the plugin and open the project again.

## 14-4 A project won't open

When a project can't be opened, nothing in the open project changes, and a
banner reads **Couldn't open the project: …** followed by the reason:

| Reason | Meaning |
| --- | --- |
| could not open '…' | The file is missing, or you don't have permission to read it. |
| not a valid project file | The file isn't a Dawesome project, or it is damaged. |
| this project was saved by a newer version of dawesome (format N, this version reads up to M) | Update Dawesome. |
| the project's tempo (…) is out of range | The tempo saved in the file is outside 1–1000 BPM. |
| the project's time signature (…) is invalid | The file's time signature has a zero in it. |

**Clips are missing after opening**

If some samples can't be found, the project still opens and a banner lists
them: **N clips were left out because their samples couldn't be loaded**.
Each line gives the file's path and the reason. Samples inside the project's
folder are saved relative to it, so move the whole project folder together.
Samples outside it are saved by their full path.

**Clips from an older version were left out**

When a project from an earlier version of Dawesome is opened, pattern clips
that played nothing (past their pattern's end) are left out, and a banner
says so:

| Banner | Meaning |
| --- | --- |
| 1 empty pattern clip from an older version was left out: it played nothing | One such clip was dropped. |
| N empty pattern clips from an older version were left out: they played nothing | N such clips were dropped. |

Nothing you could hear is lost. See
[7-8 Patterns from older versions](./patterns.md#_7-8-patterns-from-older-versions).

**I need my project as it was before the update**

When an older project is first saved in the new format, Dawesome keeps a
copy of the original beside it as `<name>.v3.dawesome` (`v1` or `v2` for the
oldest formats). Open that copy in an older version, or rename it to get the
old file back. See [4-5-3](./projects.md#_4-5-3-projects-from-older-versions).

Close a banner with **×**.

## 14-5 Where settings and messages are

Dawesome keeps its settings in a `dawesome` folder in your system's settings
folder:

| System | Folder |
| --- | --- |
| macOS | `~/Library/Application Support/dawesome/` |
| Windows | `%APPDATA%\dawesome\` <!-- VERIFY: dirs::config_dir() is Roaming AppData --> |
| Linux | `~/.config/dawesome/` <!-- VERIFY: or $XDG_CONFIG_HOME --> |

| File | Holds |
| --- | --- |
| `preferences.json` | Appearance, themes, where plugins open |
| `audio-settings.json` | Output and input devices, buffer size, recording offset |
| `plugins.json` | The list of plugins found, and those that failed |
| `recent-projects.json` | **File ▸ Open Recent** |

A missing or unreadable file is treated as the defaults, so deleting one
resets that part of Dawesome. Quit Dawesome first.

::: tip NOTE
Deleting `plugins.json` makes Dawesome scan every plugin again at next launch.
:::

**Messages**

Dawesome doesn't write a log file. It prints messages (the audio device it
opened, plugin scan problems, themes that failed) to standard output. To see
them, start Dawesome from a terminal:

```sh
/Applications/Dawesome.app/Contents/MacOS/Dawesome
```

<!-- VERIFY: install path and binary name (mainBinaryName "Dawesome") on macOS; Windows/Linux equivalents -->
