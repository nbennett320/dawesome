# Mixdown — decisions log

Decisions made while building, each with the alternative. The user's own answers are in `01-answers.md`.

| # | Decision | Alternative | Why |
|---|----------|-------------|-----|
| D1 | Render on the playback path (an `Engine::new(rate)` with no device, fed a beat of voices at a time) | Build on `bounce.rs` | bounce has no track strip, master, solo or limiter, and cuts pattern hits at the clip end: not "what you hear" |
| D2 | The snapshot gets a fresh `Mixer` set from `changed()` | Share the live mixer | Live strips re-read faders mid-render and write peaks to the UI meters |
| D3 | Tail: runs until 250 ms below -90 dBFS, capped at 10 s, faded over 10 ms only if capped | A fixed tail; render to a bar line | Ringing hits are kept, and a song that ends on silence ends at its last clip |
| D4 | TPDF dither on 16-bit WAV; none on 24/32 | No dither | Quiet fades become hiss rather than grainy steps |
| D5 | MP3 above 48 kHz: the song is **rendered** at 44.1k (a multiple of 44.1k) or 48k | Render at the engine rate and resample after (the plan's first idea) | No resampler is needed, and nothing is lost: it's the same render at a rate MP3 takes |
| D6 | Encoding streams as it renders, so there's no separate "Encoding" phase. The status is Idle, Rendering, Done, Failed or Cancelled | Render to memory, then encode | A long song at 96 kHz would hold hundreds of MB |
| D7 | The progress bar counts up to the last clip's end; the tail (≤ 10 s) isn't counted | Estimate the tail | The tail's length isn't known until it's rendered |
| D8 | Status by polling `mixdown_status` every 100 ms | Tauri events | Matches the record button and meters; nothing in the engine emits events yet |
| D9 | Show in Finder is an engine command (`open -R`) | `tauri-plugin-opener` | No new plugin, capability or JS package |
| D10 | Names: `/\:*?"<>|` and control characters become `-`; trimmed; leading dots dropped; the format's extension is dropped if typed; 200 characters at most; empty is refused | Refuse bad names | Typing "song.wav" or "a/b" should just work |
| D11 | Auto-numbering uses a hard link from a `.part` file (as bounce.rs does) | Check-then-rename | No race, and nothing is ever overwritten |
| D12 | The tab's choices are kept in a `mixdownSlice` for the session. A typed name or folder belongs to the project it was typed for | Local component state; saving with the project | Closing the tab keeps them; opening another project gets its own defaults |
| D13 | `watching`: after Confirm the tab shows the outcome (Done, or a Failed/Cancelled note) until New mixdown is pressed | Always show the last status | Reopening the tab days later shouldn't show an old "Done" unless it was never put away |
| D14 | The MP3 has no ID3 tags | A title tag from the name | Kept for later (`later.md`) |
| D15 | The rate is the open output's rate, or 44.1 kHz with none open | Ask the user | Answer 5: the engine rate |

## After the M5 review

| # | Decision | Alternative | Why |
|---|----------|-------------|-----|
| D16 | Where hard links aren't supported (exFAT USB sticks), a free name is taken by `rename` | Fail | The verifier confirmed on an exFAT image that every mixdown failed after the whole render |
| D17 | Names are capped at 200 bytes **and** 200 UTF-16 units, and the `.part` file has a short fixed name | 200 characters | HFS+ and exFAT count UTF-16 units: 127 emoji failed |
| D18 | Trimming spaces and leading dots repeats until nothing changes | One pass | ". . ." made a hidden file |
| D19 | A WAV refuses to pass 4 GB, with a message to choose a lower bit depth or MP3 | Let hound wrap | A u32 length wraps silently in release builds and leaves a corrupt file |
| D20 | A panic in the render thread is caught, so the job becomes Failed (debug builds; release aborts on panic) | Nothing | It left the job "Rendering" for good |
| D21 | The progress bar is reset when a mixdown starts; `start_mixdown` is async; a `mixdowns/` folder the mixdown made is removed if it fails or is cancelled | | Verifier nits 8, 9 and 10 |
