---
title: Instruments and effects
---

# 9 Instruments and effects

<!-- PROVISIONAL -->

Dawesome hosts **VST3** plugins: **effects**, which change a channel's sound,
and **instruments**, which play a pattern's notes. It has no built-in effects
of its own.

Each mixer channel (every track, and the master) has a **chain**: the plugins
its sound goes through, in order. You build chains in the **Effects Rack**
tab, or by dropping plugins on the Mixer's strips.

## 9-1 Finding plugins

Dawesome looks for VST3 plugins in these folders:

| System | Folders |
| --- | --- |
| macOS | `/Library/Audio/Plug-Ins/VST3`, `~/Library/Audio/Plug-Ins/VST3` |
| Windows | `C:\Program Files\Common Files\VST3` |
| Linux | `~/.vst3`, `/usr/lib/vst3`, `/usr/local/lib/vst3` |

Subfolders are searched too (for vendor folders).

At launch, Dawesome scans plugins that are new or have changed since the last
scan. Each plugin is read on its own, so one that crashes or hangs (for more
than 15 seconds) doesn't stop the scan or the app. Plugins that fail are
skipped until they change, or until you rescan.

What was found is kept in `plugins.json` in Dawesome's settings folder (on
macOS, `~/Library/Application Support/dawesome/`).
<!-- VERIFY: settings folder path on macOS (dirs::config_dir) -->

## 9-2 The plugin browser

Click the effects key on the sidebar's rail (its tooltip says **Show
effects**) to show the plugin browser.
<!-- VERIFY: this sidebar page is being renamed/rebuilt as a plugin browser on branch vsts (working tree); labels here are from c2295cb -->

![The plugin browser](./img/plugin-browser.png)
<!-- SCREENSHOT: Effects page of the sidebar, a category open in each section, numbered 1–6 -->

1. **Search plugins** — Shows only plugins whose name, maker or category
   contain every word you type. Searching opens every matching folder.
2. Rescan key — Looks for plugins again, including those already known and
   those that couldn't be read. While it scans, **Scanning *n* of *n*…** and a
   progress bar are shown.
3. **Effects** — Effect plugins, in folders by category (Dynamics, EQ,
   Reverb, Delay, …; **Other** last). The number is how many are in the
   folder.
4. **Instruments** — Instrument plugins, by category.
5. Plugin — Its name and maker. Hover over it to see its version.
6. **Couldn’t be read (*n*)** — Plugins that crashed, hung or couldn't be
   read. Click to see each one and why.

If no plugins are found, the browser says **No VST3 effects found** (or
instruments) and lists the folders it looked in.

## 9-3 Adding a plugin to a channel

Do one of the following:

- *Drag* an effect from the browser onto a channel strip in the **Mixer**. It
  goes at the end of that channel's chain, and the Effects Rack opens on that
  channel with the new effect highlighted.
- *Drag* an effect into the **Effects Rack**. A line shows where it will go
  in the chain.

If a plugin can't be loaded, a message is shown and nothing changes.

### 9-3-1 Instruments

*Drag* an instrument onto a track's strip in the Mixer (or into the Effects
Rack while it shows a track).

- An instrument always goes first in its track's chain.
- A track has one instrument. Dropping another instrument on it replaces the
  old one. **Undo** brings the old one back as it was.
- The master can't hold an instrument.

## 9-4 The Effects Rack

Choose **Window ▸ Effects Rack**, or click a strip's **FX** jack in the
Mixer (see [10-5 FX jack](./mixer.md)).

![The Effects Rack tab](./img/effects-rack.png)
<!-- SCREENSHOT: Effects Rack on a track with an instrument and two effects, numbered 1–9 -->

1. Channel menu — The channel whose chain is shown: each track, then
   **Master**.
2. Count — e.g. **2 effects**, **Instrument, 2 effects**, **No effects**.
3. Signal flow — **In ↓** at the top (**Notes ↓** with an instrument), and
   **↓ Fader** (**↓ Master fader**) at the bottom.
4. Grip — *Drag* a row by any part except its keys to move it.
5. **On** — Switches the plugin on, or bypasses it. Its blue light is lit
   while it's on.
6. Name and maker. A plugin that isn't installed shows **Not installed**.
7. **Instrument** — Marks the track's instrument.
8. Open key — Opens the plugin's own interface, in a window or a tab (see
   [9-5](#_9-5-plugin-interfaces)).
9. Remove key — Takes the plugin off the channel.

### 9-4-1 Chain order

Sound goes through the chain from top to bottom:

1. The track's instrument, if it has one, plays the pattern notes.
2. **+ clips ↓** — The track's clips are mixed in with the instrument.
3. The effects, in order.
4. The channel's fader, pan, mute and solo (see [10 Mixer](./mixer.md)).

Effects come before the fader, so a muted track's effects keep running.
Effects keep running after you stop, so reverb and delay tails ring out.

### 9-4-2 Reordering and moving plugins

- *Drag* a row up or down to reorder it. An effect can't go above the
  instrument.
- *Drag* a row onto another channel's strip in the Mixer to move it to the
  end of that chain. Its settings are kept.
- *Drag* an instrument row onto another track's strip to move it there. This
  is refused if that track already has an instrument.

### 9-4-3 Missing plugins

When a project uses a plugin that isn't installed (or fails to load), its row
shows **Not installed**. Sound passes straight through it, and its saved
settings are kept, so the project still works on a machine that has it.

## 9-5 Plugin interfaces

Click a plugin's open key in the Effects Rack to show its own interface.
Where it opens is set in **Preferences** under **Plugins open**:

- **In a window of their own** (the default) — A floating window.
- **In a tab** — A tab named after the plugin, in the pane the Effects Rack
  is in.

A plugin's interface is in one place at a time: opening it in a window closes
its tab, and showing its tab closes its window.

### 9-5-1 Plugin tabs

![A plugin's tab](./img/plugin-tab.png)
<!-- SCREENSHOT: a docked plugin tab, numbered 1–3 -->

1. **On** — Same as in the Effects Rack.
2. Channel name — Click to show the plugin in the Effects Rack.
3. Pop-out key — Opens the interface in a window instead.

The tab is named after the plugin. When the project has more than one of the
same plugin, the channel is added (**Name - Track 2**), and its place in the
chain when a channel has more than one (**Name - Track 2 (3)**).

Several plugin tabs can be open at once. A resizable interface fills its
tab. A tab closes when its plugin is removed.

If the interface can't be shown in the tab, a message is shown with **Open in
a window**.

::: tip NOTE
Plugin windows and tabs are available on macOS only. On other systems,
opening one shows "Plugin windows aren't supported on this system yet".
:::
