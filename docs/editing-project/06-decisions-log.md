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
