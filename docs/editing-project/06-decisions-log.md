# 06 — Decisions made without you

You went to sleep on 2026-11-10 and told me to work through the night with full autonomy, to assume my own recommendation for any new question, and to report the decisions in the morning. This file is that report. Every entry is something you can reverse.

## Answers you gave without reading

You said you had not read the questions since F21. These answers were therefore my recommendations, not your considered choices. They are all in `01b-follow-ups.md`:

| # | Topic | What was recorded |
|---|---|---|
| F22 | Joined pattern length | Any whole number of bars |
| F23 | Joined pattern span | Rounded up to a whole bar, clip trimmed to the exact span |
| F24 | Pattern clips off the sixteenth grid | Join refused with a message |
| F25 | Joined pattern name | "first + second" |
| F26 | Remaining pattern-join rules | One track, steps merged, placed after the first source pattern |
| F27 | Clips page behaviour | Exactly like the Recordings page |
| F28 | Clips page rail icon | I draw it |
| F29 | Trim notch look | Triangle in the label band |
| F30 | Sample tab for a trimmed clip | Whole file, played part highlighted |
| F31 | "Split at playhead" while playing | Splits where the playhead is |
| F32 | Playhead snap for edges | Same rule as cuts |
| F33 | Multi-selection trim | Same edge, same amount, only the grabbed clip snaps |
| F34 | Multi-selection stretch | Same ratio, pattern clips skipped |
| F35 | "Halves equal the whole" exactness | Identical at any pitch and rate |
| F36 | Playlist tray layout | Tools, gap, snap controls |
| F37 | Where messages appear | The notice banner |
| F38 | Empty clipboard | Paste left out, no empty-space menu |
| F39 | Loop-end marker | Small marker while Loop is on |
| F40 | Loop while recording | Loop end frozen |
| G1 | Paste when paused off-grid | Snapped when Snap is on |
| G2 | Deep copy on a selection | One file per clip |
| H1 | Selection after Join | The joined clip |
| H2 | Paste of a clip whose pattern is gone | That clip is skipped |
| H3 | View after New or Open | Reset to the first 4 bars |

## Approval

- I treated "go ahead and work through the night" as approval of all 14 milestones in `03-plan.md`. The `Approved` boxes there are still unticked, because you did not tick them.
- M3 (undo): you did not say one commit or three. I chose three (playlist, then mixer and tempo, then sequencer), because that was the option I had offered for reviewability.

## Decisions made while building

Added below as they happen, newest last. Each says what I chose, why, and what the alternative was.

### M1 — 64-bar playlist and loop

- **The canvas keeps "zoom 1 = the whole playlist" and opens at zoom 16.** The plan said I would change the beat-to-pixel maths to a 16-beat view; keeping the existing coordinate system and changing only the opening zoom touches far less code. Alternative: redefine zoom 1 as 16 beats and allow zoom below 1.
- **Clip cards and drop outlines are now drawn in screen pixels across.** They used to be drawn inside the zoom transform, which stretches their left and right borders and corner radius by the zoom factor. At the old default (zoom 1) that was invisible; at the new opening zoom of 16 every clip would have had 16-pixel-wide side borders. The visible effect on existing behaviour: zoomed-in clips no longer get fat side borders. Alternative: none that keeps the opening view looking as it does today.
- **The loop-end marker** is a 1.5 px line with a small flag pointing back along the loop, in slate-600 (`theme.loopEnd`, the same colour as the ruler's bar numbers). Alternative: the sky accent, which I avoided because the playhead already uses it.
- **Ruler numbers are still beat numbers** (1, 5, 9 … on bars), as today, thinned to every bar, then every 2, 4, 8 … bars when zoomed out. Alternative: switch to bar numbers.
- **The playlist is 256 beats, not "64 bars of the project's time signature".** The canvas already assumes 4 beats to the bar for its grid, and the time signature cannot be changed in the UI. Alternative: 64 × the time signature's numerator.
- **The playhead is re-anchored each time the loop comes round.** The loop's length can now change during playback (move or trim the last clip), and the old "position modulo loop length since the run started" would then show the cursor in the wrong place. Now the engine counts from the last wrap, and the canvas and Transport tab ask the engine again whenever clips change.
- **If the loop gets shorter than where the playhead already is** (the last clip is deleted behind it), playback jumps to the start at the next beat. Alternative: play on to the old end first.
- **I could not look at the running app.** I launched it in dev mode; the engine side started without errors, but the screenshot came back black, so nothing on screen has been checked by me. Every `[M]` acceptance item is waiting for you.
- **Bar lines are never thinned; only ruler numbers are.** On a narrow canvas fully zoomed out, bar lines can be under 8 px apart. The spec says bar lines stay (F4), so they do.
- **The loop's end is frozen for every recording,** including one started with Loop off and switched on part-way (the verifier caught that my first version froze it only when Loop was on at the start).
- **The transport still reports one number for the loop** (`loopBeats`); its meaning changed from "always 16" to "where the loop ends". I did not add a field.
- **Left for later, noted in `later.md`:** the loop's end (and an overhanging clip's tail) can lie past bar 64, where the view cannot scroll; with Loop on, seeking past the loop's end and pressing Play starts from the beginning (the existing rule, more noticeable now); mouse-wheel zoom from the opening view out to all 64 bars takes about 80 notches because wheel zoom is capped at 3.5% per notch (trackpad pinch is not capped).
- **Verifier pass for M1:** no blockers; two should-fix items (the canvas and Transport tab going out of step with the engine after the loop's length changes during playback, and the marker not following pattern-length or tempo changes) and four minor ones, all fixed before the commit.

### M2 — Tempo keeps clips on their beats

- **Clip positions are still stored as times and are rescaled when the tempo changes.** The plan said I would store them in beats. Rescaling changes two functions instead of most of the audio graph and its tests. The cost: anything that remembers a time across a tempo change has to be rescaled too, so the undo history (M3) and clipboard (M10) remember beats, not times. Alternative: store beats everywhere, as planned.
- **The tempo field no longer applies each keystroke.** A typed tempo is taken up half a second after typing stops, on Enter, or when you leave the field. Before, typing "100" set the tempo to 1, then 10, then 100; now that a tempo change restarts playback, that would have lurched through each. This changes existing behaviour. Alternative: keep per-keystroke and accept the lurch.
- **Tempos below 1 bpm are refused,** in the field, the engine and project files (the limits were "above 0 and up to 1000"). A project file saved with a tempo under 1 no longer opens. Reason: at tiny tempos the beat length overflows and the engine panics. Alternative: none sensible.
- **A pattern playing on its own in the Sequencer starts over from its first step when the tempo changes.** The spec only covers the playlist. Alternative: leave it running at the old tempo until stopped, as before.
- **If the playhead's beat is past the end of the loop (or song) at the new tempo, playback starts from the beginning.** A sample clip covers fewer beats at a slower tempo, so the last clip's end, and with it the loop's end, can move in front of the playhead. This is the existing "Play from past the end starts at the top" rule. Alternative: stop instead.
- **Snapping is now exact.** Clip positions used to be snapped in whole microseconds, which put a clip snapped to beat 200 at 90 bpm about 0.14 ms early; they are snapped in nanoseconds now. I changed it because the error became visible once positions survive tempo changes. Existing behaviour otherwise unchanged.
- **A dropped sample's position is worked out after its file has been decoded,** so a tempo change during a slow decode cannot put it on the wrong beat.
- **The tempo is also locked for the short time after a recording stops while its takes are being placed,** not only while recording.
- **Verifier pass for M2:** no blockers; six should-fix items (typing a tempo while playing could throw playback to the start; the Sequencer's step light went stale; the tempo lock ended before takes were placed; an error could be returned after the tempo had already changed; the field could show a stale value; decisions were not logged) and seven minor ones. All fixed, except that the field's focus behaviour in the real window is unverified.

### M5 — Space, and M6 — Lasso over a clip

- **Space is handled once, on the window, before anything else sees the key,** and the key is swallowed on the way down and on the way up. A focused button would otherwise be "clicked" by Space (on key-up in some browser engines), which is how Space after clicking Stop would have pressed Stop again. Text fields and drop-down lists are left alone.
- **Shift+Space and Option+Space do nothing at all in the app,** outside text fields: they do not play, and they are also kept from buttons, tabs and scrolling panes (the verifier found that Shift+Space still clicked a focused button and paged a scrolling pane, while the controls that used to answer Space no longer did). Cmd+Space and Ctrl+Space are left to the system. The spec does not cover modified Space. Alternative: let Shift+Space play too.
- **A Shift-press is now decided on release or after 10 px of movement,** not at the moment of the press. So a Shift+click toggles its clip when the mouse comes up, not when it goes down.
- **I committed M5 and M6 before their verifier report came back.** The first verifier run died on a rate limit and the second was still running; the undo work (M3) shares two files with M5, and I did not want to hold it up. Whatever the verifier finds goes in as a follow-up commit, recorded below.
- **Verifier pass for M5 and M6 (follow-up commit "space and lasso fixes"):** no blockers. Fixed for M5: the missing test that Space and the Play key send the engine the same thing in Playlist, Sequencer and other views; a test with real key events on a real button, listened for on the window; Shift+Space as above. Fixed for M6: Delete and Shift+E are ignored while a Shift-press is held down (the release could otherwise select a clip that was just deleted); click-or-box is now decided in one tested function at release, so a quick drag let go before the next frame is a box, not a click; a press whose release goes to another app is dropped when the window loses focus; the press position is read from the mouse event itself; the tests now use a scrolled, zoomed view so canvas and playlist positions cannot be confused.
- **Not changed, for you to try in the app:** the tempo field keeps the keyboard after Enter, and a drop-down probably keeps it after a pick, so Space types or reopens the list until you click elsewhere. That follows the spec ("never while typing"), but may feel wrong right after changing Snap or the tempo. Alternative: give up the keyboard after Enter or a pick.

### M3 — Undo and redo

- **Three commits:** playlist clips, then mixer and tempo, then the Sequencer.
- **The engine keeps "before" and "after" pictures, not reverse commands.** Each step holds the changed clips (with their beats), and, when they changed, the whole mixer, the whole Sequencer and the tempo, as they were before and after. Undo puts the "before" back; redo the "after". Alternative (the plan's wording): each command records how to reverse itself; more code per command and easier to get subtly wrong.
- **One press of the mouse is one step.** The frontend numbers each press (pointer down to pointer up) and sends that number with every change made during it; the engine folds changes with the same number into one step. A press that ends where it began leaves no step. A change made from the keyboard (an arrow key on a fader) is a step each.
- **A visit to the tempo field is numbered the same way,** from the field getting the keyboard to losing it.
- **Undo of a step that is not about clips (a fader, a mute, the tempo) leaves the Playlist selection as it is.** The spec says the clips a step changed become the selection; a step that changed none would otherwise clear it. Alternative: clear it.
- **Undo and redo are ignored while the mouse is held down** (spec F8), and wait for any fader or clip-setting value still on its way to the engine.
- **Cmd+Z can arrive twice, from the Edit menu's shortcut and from the key press in the window.** Which route macOS uses is one of the things I could not check in the running app, so both are handled and a second arrival within 150 ms is dropped. If only one route turns out to be live, the other can be removed.
- **Cmd+Z works while a drop-down list has the keyboard.** Space treats a drop-down as "typing" (so Space opens it), but a drop-down has no text to undo, and it keeps the keyboard after you pick from it, so undo would have looked dead after changing Snap. Alternative: treat it as typing for undo too.
- **A recording's file can be sent to the Trash while the undo history still remembers a clip of it.** Delete a clip of a take, then delete the take in the Recordings page: that is allowed, because no clip uses the file any more. Undoing the clip's deletion then brings back a clip whose file is in the Trash; it still plays until the project is closed, and is left out the next time the project is opened. Alternative: refuse to delete a file the history still refers to, which would mean "this file is in use" for a file no visible clip uses. Noted in `later.md`.
- **The Edit menu's Undo and Redo are always enabled.** The engine reports whether there is anything to undo or redo, but nothing dims the items; the spec does not ask for it.
- **Verifier pass for M3 (follow-up commit "undo fixes"):** no blockers. Fixed: the history is cleared again at the end of opening a project, so a fader moved while the samples were loading cannot be "undone" into the new project; Cmd+Z with a drop-down focused (above); an undo that overlaps a sample still loading can no longer leave something to redo after the new clip arrives; a press whose release went missing (a context menu took it) no longer blocks undo until the next click; deleting with the same clip listed twice; tests added for files being left alone by undo and redo, for moving a pattern folder, for placing a pattern, and the group-move test now runs the real command.
- **Not fixed, logged:** a sample dropped and then Cmd+Z pressed before it has finished loading undoes the edit before it, because the drop is not in the history until the file has been read.

### M4 — Clip windows in the engine

- **Every sample is now decoded once, when its clip is made, and kept in memory as 16-bit audio.** Before, each clip kept the file's bytes and decoded them again on the audio thread every time it played. A clip that plays part of its file has to be able to start anywhere in it at once, which a streaming decoder cannot do. What it costs: memory. A WAV takes about what it took before; an MP3 takes about ten times its file size (five minutes of stereo is about 50 MB). The halves of a cut, copies, and the undo history all share one copy. Playback is still 16-bit, as before. Alternative: keep the bytes and seek the decoder, which is not sample-accurate for every format.
- **I wrote my own reader for clips in place of rodio's speed and sample-rate converters.** It gives, for a whole clip, exactly the samples the old path gave: a test compares the two bit for bit, across file rates of 22.05 to 96 kHz, both engine rates, mono and stereo, and pitches from −24 to +24. So whole clips sound as they did (acceptance C1, X11).
- **What "the two halves play exactly what the whole clip did" rests on.** Every clip of a file is read on one grid that starts where the file's first sample would be heard. The right half of a cut starts at a position worked out from that point, not from where the mouse was, so it can land up to a few nanoseconds from the snapped beat. That is far below one sample (about 20,000 ns) and cannot be heard or seen, but a clip's start is no longer always a round number in the project file. Tests play the whole clip and then the halves, from eight different playhead positions, at pitches −7, 0 and +5, with 44.1 and 48 kHz files on a 48 kHz engine, and compare every output frame: no differences. The same holds after cutting into four.
- **Where the guarantee ends.** The halves stop matching the whole, by a fraction of a sample at the seam, if you change the tempo after the cut (the two halves are then no longer back to back anyway), or change one half's pitch. A half with a fade is also not bit-identical through the fade, since the fade is the point.
- **A clip with a window is saved with its exact start beat** (no rounding to six places, which whole clips keep). Rounding would move a half by up to half a microsecond and break the seam after reopening the project.
- **Playing from the middle of a clip now reads the clip on the same grid as playing it from its start.** Before, starting playback inside a clip restarted the sample-rate conversion at the playhead, so the same clip came out a fraction of a sample differently depending on where you pressed Play. Not audible; mentioned because it is a change to existing behaviour.
- **MP3 and other block-coded files may sound very slightly different from before when the file's rate differs from the device's.** rodio's converter restarted at every decoded block for those formats (a known flaw); the new reader does not. WAV files, which are what the comparison tests use, are identical.
- **A pattern clip's window is kept in steps, and can fall between steps** (a cut with Snap off). A hit on the very step a window ends on belongs to the next clip, so a pattern cut on a step plays each hit once.
- **Trimming never moves the sound.** The engine command takes "which part of the file" and works out the clip's new start itself, so a trimmed start cannot drift from where the audio was.
- **When a trim would make a clip shorter than 10 ms (or one step), the engine moves the end later, not the start earlier.** The canvas already stops the edge being dragged; this is the engine's own backstop for bad input.
- **A clip's fades are shortened to fit when it is trimmed or cut, and stay shortened if it is opened out again** (spec Q61). Undo brings them back.

### M7 — Trim by dragging a clip's edge

- **An edge has to be dragged 10 pixels before it trims,** the same distance that turns a press on a clip into a move. Without that, a plain click on an edge with Snap on would have snapped the edge to the nearest grid line. Alternative: trim from the first pixel and accept that.
- **The edge keeps its distance from the mouse.** If you press 4 pixels inside the edge, the edge stays 4 pixels from the mouse while you drag, so it does not jump when the drag starts. Snapping applies to where the edge is, not where the mouse is.
- **While several clips are trimmed together, the bar is drawn on every clip's edge,** not only the grabbed one.
- **Trimmed-edge triangles are left out on a clip narrower than 28 pixels,** where they would cover the label.
- **A hit on a pattern clip's last, partly shown step is drawn cut off at the clip's edge.** A pattern clip trimmed between two steps shows the part of the step it covers.
- **After you let go, the clip stays as you dragged it until the engine's answer is drawn,** so it does not flick back for a frame. If the engine refuses the trim, the clip goes back.
- **Pressing an edge with Shift held still starts a lasso (or toggles the clip) for now.** Shift+edge becomes "stretch" in M8.
- **In the Sample tab, the beat ruler counts from where the clip starts,** not from the start of the file, since beat 1 of the clip is where it sits in the playlist. The ticks carry on across the dimmed parts at the same spacing. Alternative: count from the file's start.
- **The Sample tab piece (the dimmed whole-file waveform) was written by a Sonnet agent from a brief** and reviewed by me; the canvas work and the trim maths are mine.
- **Not verified:** everything here that is a gesture (F1, F2, F5, F6, F8, F9, F11–F17) needs the running app. The maths behind them is unit-tested in `edges.spec.ts`; the drawing is not testable without the app.

### M8 — Stretch

- **The engine is told the pitch and which edge stays put, not where the clip should start.** For a left-edge stretch it works out the new start itself from where the clip ended, so the end cannot drift. Alternative: send the start beat from the canvas.
- **A stretch by the left edge stops when the clip's start reaches the start of the playlist,** as well as at two octaves either way and at 10 ms.
- **Across a selection, "the same ratio" is taken from where the mouse is, not from where the grabbed clip stopped.** If the grabbed clip reaches its limit and you keep dragging, the other clips carry on to theirs. This matches how a multi-clip trim behaves. Alternative: everything stops when the grabbed clip does.
- **Shift+dragging a pattern clip's edge does nothing at all,** not even a lasso; Shift+clicking it without dragging toggles it, like any Shift+click.
- **Shift+pressing an edge of a clip that is not selected stretches just that clip and leaves the selection alone.** Shift is the selection key, so it never clears a selection.
- **The pitch a stretch writes is whatever makes the length exact, not rounded to a semitone,** so the Sample tab can show a value like −7.02. Snapping applies to where the edge lands on the timeline.
- **Not verified:** the gestures (G2 to G9) need the running app.
