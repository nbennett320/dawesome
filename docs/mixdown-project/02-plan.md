# Mixdown — plan

Answers: `01-answers.md`. One milestone at a time; the user reviews each.

## Approach

Render on the **real playback path**, not on `bounce.rs`. bounce renders one clip
without track strip, master, solo or limiter, and cuts pattern hits at the clip's
end — none of which is "what you hear". Playback's pieces can run without a device:

- `Engine::new(rate)` (`daw_core/engine.rs:123`) gives an `(Engine, Renderer)` pair
  with no device; `Renderer::render(out, now, ZERO)` fills a buffer on demand.
- `AudioGraph::run_sounds_at(at, rate, beat, metronome=false, resume)` gives a
  slice's voices, each already wrapped in its track's strip
  (`audiograph.rs:770`).
- The Renderer sums them, runs the master strip and `soft_limit` — the same as
  the speakers (`engine.rs:365-401`).
- The `bounce.rs` test helper (`bounce.rs:330-346`) already does exactly this.

### Snapshot (so the user keeps working)

Under the graph lock, then the sequencer lock (that order, `audiograph.rs:725`),
held only while copying:

- a new `AudioGraph::new(rate, tempo, max_beats)` with `nodes` and
  `pattern_nodes` cloned (cheap: the audio is `Arc`-shared)
- `sequencer`: `Arc::new(Mutex::new(live.clone()))`
- `mixer`: a fresh `Mixer::default()` with `.apply(&live.changed())`

The fresh mixer is required. The live strips re-read the faders every 64 frames,
so a render using them would pick up fader moves made mid-render. They also write
peaks, which would make the UI meters flicker. The renderer never touches the live
graph's `running` or `current_offset` fields.

### Render loop

```
end   = snapshot.end()                  // AudioGraph::end(), last clip's end
beat  = snapshot.beat_interval()
start_run(1, mixer.master(), slice(0))  // first slice with resume=true
for each beat boundary `at` < end:
    render frames up to `at`            // in chunks of ≤ 4096 frames
    engine.play(slice(at))              // one beat at a time: stays far under
                                        // MAX_VOICES (pending voices count) and
                                        // drains the garbage queue
    check cancel, update progress
tail: keep rendering after `end` until 250 ms in a row sit below -90 dBFS,
      capped at end + 10 s              // pattern hits ring past their clip
```

Times stay in `f64` or frames, using `run_frame` (see lessons.md on nanoseconds).

**Ready for a Live method:** the loop feeds a sink, `FnMut(&[f32])`, that collects
the samples, and the snapshot and the encoders don't know how the samples were
produced. A future Live method is a second driver: it plays through the device
and taps the master, feeding the same sink. Noted in `later.md`; not built now.

### Encoding

| Type | Format | Notes |
|------|--------|-------|
| WAV 16 | int PCM | TPDF dither when reducing from f32 (prevents low-level distortion on quiet fades) |
| WAV 24 | int PCM | no dither |
| WAV 32f | float | the samples as they are (no `written()` nudge; that is for clips read back at 16-bit) |
| MP3 | `mp3lame-encoder 0.3` | 128/192/256/320 CBR, quality Best. Engine rates > 48 kHz are resampled to 48 kHz first, because MP3 stops at 48k. No ID3 tags in v1. |

Writing reuses bounce's safe pattern: write `<name>.part`, then **hard-link** it
to the final name, which fails if the name is taken, then remove the `.part`
file. On that failure, try `name 2`, `name 3` and so on. The result is the
auto-numbering from answer 3, with no window for a race. The `mixdowns/` folder
is created on demand. Names are sanitised: `/` and `:` become `-`, whitespace is
trimmed, and an empty name is refused.

### Job and progress

Following the codebase's existing pattern (polling, like `collect_recording` and
the meters, rather than emit):

- `start_mixdown(opts) -> ()` spawns a thread and refuses if a job is running.
- `mixdown_status() -> Idle | Rendering{progress} | Encoding{progress} | Done{path} | Failed{message} | Cancelled`
- `cancel_mixdown()` sets an `AtomicBool`; the loop checks it each chunk, and any
  `.part` file is removed.
- `reveal_in_finder(path)`: `open -R` on macOS, `xdg-open <dir>` on Linux. This is
  an engine command, so no opener plugin or capability change is needed.

The job lives in the engine, so closing the tab mid-render doesn't stop it.
Reopening the tab shows the current status.

## Milestones

### M1: Offline renderer (`engine/src/mixdown.rs`)
- `snapshot(&InnerState) -> Snapshot`, `render(Snapshot, rate, sink, &Progress, &AtomicBool) -> Result<(), String>`
- Rate: `open_engine_rate().unwrap_or(44_100)`.
- Errors: an empty playlist gives "Nothing to mix down".
- **Tests** (run from `engine/`, with `timeout`):
  - a single clip matches playback's helper output, frame for frame
  - a muted track is silent and a soloed track plays alone
  - track/master volume and pan apply
  - the metronome is never in the render, even when it's turned on
  - the length is end + tail; a pattern hit ringing past its clip survives
  - fader moves during a render are not picked up, and the live meters see no peaks
  - cancel stops the render
  - awkward tempos (e.g. 87.3, 90 bpm) and a song longer than 1024 voices' worth of hits

### M2: Encoders and file naming
- `write_wav(path, rate, Bits16|Bits24|Float32, samples)`, `write_mp3(path, rate, kbps, samples)`, `unique_link(part, dir, name, ext)`
- Add `mp3lame-encoder = "0.3.0"` (verified to build on rustc 1.86).
- **Tests:**
  - read back with `hound` / `symphonia`: spec, length, values within the bit depth's tolerance
  - an existing file gives `name 2`; `.part` files never remain
  - odd names (`a/b`, `  `, a Kelvin sign)
  - a 96 kHz input produces a 48 kHz MP3

### M3: Commands and job state
- `MixdownJob` in managed state; the four commands are registered in `main.rs`.
- Default folder: `<project dir>/mixdowns` (via `project_files`, next to
  `clips_for` / `recordings_for`), or `~/Music` for a project that was never
  saved. Exposed as `mixdown_defaults() -> {name, dir}` so the frontend doesn't
  duplicate the rule.
- **Tests:** status moves Idle → Rendering → Encoding → Done; a second start
  while running is refused; cancel leaves the status Cancelled and no file behind.

### M4: Frontend tab
- **Menu:** `item("view-mixdown", "Mixdown…", None)` in File, after Save As, behind
  a separator (`app/window.rs:85`). It reaches the existing `view-` branch.
- **Register the view:**
  - `View.Mixdown` and the label `Mixdown` (`src/types/ui.ts`)
  - `mixdown: View.Mixdown` (`src/events/window.ts`)
  - a case in `src/views/matchViewComponent.tsx`
- **`src/views/mixdown/Mixdown.tsx`:** styled like `AudioSettings.tsx` (`ft-panel`,
  `ft-label`, `ft-readout`, `AnalogButton`, amber alert box).
  - **Form:** Name (text), Path (read-only text plus Browse…, using
    plugin-dialog `open({directory: true})`, already allowed), File type (WAV
    or MP3), then Bit depth (16/24/32-bit float) or Bitrate (128–320), depending
    on the type. Confirm is disabled while the name is blank.
  - **Running:** a progress bar showing "Rendering… 42%", then "Encoding…", and
    Cancel. The form is disabled. Status is polled every 100 ms while a job runs.
  - **Done:** "Done: song 2.wav" (the auto-numbered name actually used), with
    Show in Finder and New mixdown (back to the form, settings kept).
  - **Failed:** the message in the amber box, with the form usable again.
  - Settings are kept in a small `mixdownSlice` for the session, so closing and
    reopening the tab keeps them. They are not saved in the project.
- **Tests:** `openTab(View.Mixdown)` works, the slice's reducers, and the
  defaults when a project path is null or set.
- **Gate:** `pnpm exec tsc --noEmit --types jest` (baseline 3 errors), jest,
  `cargo test` from `engine/`.

### M5: Verification and acceptance
- An Opus verifier re-derives the render against playback at awkward tempos,
  pitches, fades and names (see lessons.md), with a scratch folder.
- Manual acceptance in the app:
  - export WAV and MP3 of a real project; listen and compare with playback
  - mute and solo a track
  - an unsaved project
  - export twice (gives `name 2`)
  - cancel mid-render
  - keep playing and editing during a render

## Decisions I made (say if you disagree)
1. TPDF dither on 16-bit WAV, none at 24/32.
2. The tail ends at 250 ms of silence below -90 dBFS, with a 10 s cap.
3. Progress by polling, not events, to match the codebase.
4. Show in Finder through a small engine command instead of adding `tauri-plugin-opener`.
5. MP3 above 48 kHz is resampled to 48 kHz; no ID3 tags in v1.
6. Mixdown settings last for the session; they are not saved in the project.
7. The rate is the open output's rate, or 44.1 kHz when no device is open.
