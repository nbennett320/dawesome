---
title: Projects
---

# 4 Projects

A project is one song: its playlist, patterns, mixer, effects and settings,
saved in a project file with the extension `.dawesome`. Recordings and other
audio Dawesome makes for the project are kept in folders beside it.

## 4-1 Starting a new project

Choose **File ▸ New Project** (**⌘N (Ctrl+N)**).

The playlist, Pattern Sequencer and mixer are cleared, and the window's title shows
**Untitled — Dawesome**.

If the current project has unsaved changes (see
[4-7](#_4-7-closing-dawesome)), Dawesome asks **Replace the current
project?** before it clears anything.

## 4-2 Opening a project

1. Choose **File ▸ Open…** (**⌘O (Ctrl+O)**).
2. If the current project has unsaved changes, Dawesome asks **Replace the
   current project?**.
   Confirm to carry on.
   <!-- VERIFY: the confirm dialog's button labels (system dialog, e.g. Yes/No or OK/Cancel). -->
3. In the **Open project** dialog, choose a `.dawesome` file.

When the project opens, the notice bar lists anything it couldn't find:

- **Samples** — Clips whose sample couldn't be loaded are left out of the
  playlist. Each missing file is listed with the reason.
<!-- PROVISIONAL -->
- **Plugins** — Effects whose plugin isn't installed pass sound straight
  through. Their settings are kept, and saved again with the project.

*Click* **×** to dismiss the notice.

::: tip NOTE
A project saved by a newer version of Dawesome, in a format this version
doesn't read, is refused. The notice bar says so.
:::

### 4-2-1 Open Recent

**File ▸ Open Recent** lists the eight projects you opened or saved most
recently, newest first. Each is listed by name; when two projects share a
name, the folder each is in is added.

- Choose a project to open it. As with **Open…**, Dawesome asks first if the
  current project has unsaved changes.
- A project that can't be opened (moved, deleted or damaged) is taken off the
  list.
- **Clear Recent** empties the list. With nothing in it, the submenu shows
  **No Recent Projects**.

The list is kept between launches (see [2-5](./install.md#_2-5-where-settings-are-kept)).

## 4-3 Saving a project

### 4-3-1 Save

Choose **File ▸ Save** (**⌘S (Ctrl+S)**).

- A project that has been saved before is saved to the same file.
- A project that has never been saved opens the **Save project** dialog, as
  **Save As…** does.

The project file is written in one go, so a save that fails part-way never
leaves a half-written file in its place.

### 4-3-2 Save As

1. Choose **File ▸ Save As…** (**⇧⌘S (Ctrl+Shift+S)**).
2. In the **Save project** dialog, type a name and choose where to save.
   The first time, the dialog starts in your Documents folder, named
   **Untitled**.
3. *Click* **Save**.

Dawesome makes a folder named after the project, and saves the project file
inside it ([4-5](#_4-5-the-project-folder)). If you choose a folder that
already has the project's name, it saves into that folder rather than
making another.

After **Save As…**, you are working in the new project. The project you saved
from keeps its file and its folders as they were: its recordings and clips
are copied into the new project's folder, not moved.

<!-- VERIFY: the dialog's confirm button label ("Save") is the system's own. -->

::: tip NOTE
If some recordings couldn't be brought into the project's folder, the notice
bar says **Saved, but some takes are still outside the project's folder**.
The project still uses them from where they are.
:::

## 4-4 Renaming a project from the cassette

You can name the project on the **Transport** tab's cassette.

1. *Double-click* the cassette (or press **Enter** on it).
2. Type a name, then press **Enter** or *click* away. **Esc** cancels.

- If the project has never been saved, the **Save project** dialog opens with
  the name filled in.
- If it has, Dawesome asks **Save as "*name*"?**, then saves a copy under the
  new name in a folder next to the old one, and carries on in the copy. The
  old project stays as it was. If a project of that name is already there,
  the question says it will be replaced.

## 4-5 The project folder

A saved project is a folder named after it:

```
my_song/
  my_song.dawesome
  recordings/
    my_song_11092026_0.wav
    my_song_11092026_1.wav
  clips/
    kick_copy_1.wav
```

- `my_song.dawesome` — The project file.
- `recordings/` — Takes you record. Each is named after the project, the
  date it was recorded (month, day, year) and its number that day. You can
  file takes into folders of your own from the **Recordings** browser.
- `clips/` — Audio files Dawesome writes itself, with **Deep copy** and
  **Join**. The folder is made the first time you use one of them.

These folders appear in the sidebar's **Recordings** and **Clips** browsers
(see [3-4](./parts.md#_3-4-sidebar)).

### 4-5-1 Recording before you save

You can record into a project that has never been saved. Its takes are kept
in a folder in Dawesome's application data folder (on macOS,
`~/Library/Application Support/dawesome/unsaved/`). When you first save the
project, they are moved into its `recordings/` folder and renamed after the
project.

::: tip NOTE
If you start a new project or open another one without saving, takes
recorded into the unsaved project stay in a dated folder inside
`dawesome/unsaved/`. No browser shows them; find them there in your file
manager.
:::

**Deep copy** and **Join** need a saved project, so their files have
somewhere to go. Save the project first.

### 4-5-2 Samples and moving a project

Samples you add from the samples browser are not copied into the project.
The project file stores where each one is:

- a sample inside the project's folder is stored relative to the folder, so
  the folder can be moved or shared;
- any other sample is stored by its full path.

::: warning CAUTION
If you move a project folder to another computer, samples from outside the
folder won't be found there. Their clips are left out when the project opens,
and listed in the notice bar.
:::

### 4-5-3 Projects from older versions

A project saved as a single `.dawesome` file, without a folder of its own,
still opens. **Save** writes it back where it is, with its recordings in a
`recordings` folder next to the file. **Save As…** gives it a folder.

A project with patterns from an earlier version of Dawesome opens with its
drum and note patterns turned into one kind of pattern, and its pattern
folders in one tree (see
[7-8 Patterns from older versions](./patterns.md#_7-8-patterns-from-older-versions)).
Opening it doesn't change the file. Saving it writes the new format.

::: warning CAUTION
A project with patterns, once saved by this version, can't be opened by
older versions of Dawesome: they say it was saved by a newer version. To
keep the original for an older version, copy the project's folder before
you save, or save the opened project under a new name with **Save As…**
(the original file is left as it was).

Moving or renaming a take the project uses, in the **Recordings** browser,
also updates the saved project file, and so saves it in the new format too.
:::

::: tip NOTE
The first time an older project is written over in the new format, whether
by **Save** or by moving a take, Dawesome keeps a copy of the original file
beside it, named for its format: `song.dawesome` becomes `song.v3.dawesome`
(`v1` or `v2` for the oldest formats). The copy is made once. If a copy of
that name is already there, it is left as it is. An older version of
Dawesome opens the copy. If the copy can't be made, the project isn't
saved and stays open; save it elsewhere with **Save As…**.
:::

## 4-6 What is saved

| Saved with the project | Not saved |
| --- | --- |
| Tempo, time signature, metronome, loop, snap and snap length, count-in | The undo history (it starts again with each project) |
| Playlist clips and pattern clips, with their trims and settings | The playlist's mouse tool (a project opens with the select tool) |
| Pattern Sequencer rows, patterns and pattern folders | Mixdown settings |
| Mixer settings of changed or named tracks | Audio settings and preferences (see [2-5](./install.md#_2-5-where-settings-are-kept)) |
| Armed tracks and their input channels | The Pattern Sequencer's **Used only** setting |
| Each channel's effects and their settings <!-- PROVISIONAL --> | |
<!-- VERIFY: whether the tab and pane layout, and plugin window positions (commit ba304a9), are saved with the project. -->

## 4-7 Closing Dawesome

Quitting Dawesome (**Dawesome ▸ Quit**, the Dock's **Quit**) or closing its
window quits straight away if the project has no unsaved changes. If it
has, Dawesome asks **Save changes to "*name*" before quitting?**:

- **Save** — Saves the project, then quits. A project that has never been
  saved opens the **Save project** dialog first; if you cancel it, or the
  save fails, Dawesome stays open.
- **Don't Save** — Quits, losing the changes.
- **Cancel** — Stays open.

While the project has unsaved changes, the window's title shows a **\*** after
its name (**my_song\* — Dawesome**). It goes away when you save, or undo
back to how the project was saved.

A change is anything that would be saved differently (see
[4-6](#_4-6-what-is-saved)), including effect settings and plugin window
positions. Undoing back to how the project was saved counts as no change. A
new project you haven't touched quits without asking.

<!-- VERIFY: a plugin whose saved settings differ every time they're read would always count as changed. -->
