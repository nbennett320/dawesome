# Lessons — Playlist editing project

Read this at the start of every session on this project. One short entry per lesson: what, and why it mattered.

## Carried over from earlier Dawesome sessions

- **Do not use `git stash` to get a "before" baseline.** Compare against `git show HEAD:<file>` instead. Why: on 2026-11-08 a stash/pop round trip left a stale engine binary, and the app reported commands as "not found" until a clean rebuild.
- **If engine commands come back "not found" although they are registered in `engine/src/main.rs`,** run `cargo clean -p app && cargo build --no-default-features` in `engine/`, then restart `tauri dev`. Why: `tauri dev` builds with `--no-default-features`, which cargo caches separately from a plain `cargo build`.
- **Tauri is held at 2.11.x because the local compiler is rustc 1.86.** The JS and Rust Tauri minor versions must match. Why: Tauri 2.12 and later need a newer compiler, and the Tauri CLI refuses to build on a mismatch. Any new crate for this project has to build on 1.86 too.

## From this project

- **Check an exploration agent's surprising claims in the source before writing them down.** Worked on 2026-11-10: the "playback is 16-bit" and "pitch only relabels the sample rate" claims were confirmed in rodio's own source before they went into the codebase map. Why: design questions built on a wrong fact waste a whole question round.
- **Ask design questions as a questionnaire in the chat, not as a file to tick.** On 2026-11-10 the user asked for this after seeing the 165-question file. Record every answer in `docs/editing-project/` straight away, quoting the user's own words where they wrote any. Why: answering in chat is faster for them, and asking live lets later questions adapt to earlier answers.
- **Commits for this project: I commit at the end of each milestone, with one short lowercase line in the user's own style (for example "caching waveforms") and no trailer lines.** The user allowed commits "for just this one project" on 2026-11-10. Never stage `docs/todo.md`. Why: the user normally commits everything themselves.
- **When an answer changes what a later question means, rewrite the later question before asking it.** Worked on 2026-11-10: after "tape-style stretch" the library and render questions were dropped and the remaining stretch questions were reworded around the Pitch setting. Why: asking stale questions wastes the user's time and produces answers that contradict each other.
- **When the user hands over autonomy, keep a running decisions log.** On 2026-11-10 the user said to work overnight, assume my own recommendation for new questions, and report the decisions in the morning. Every such decision goes in `06-decisions-log.md` as it is made, with the alternative. Why: they want to be able to revisit each one, and they answered the last follow-ups (F22 onward) without reading them.
- **Run engine tests from `engine/`.** Why: the tests open samples by paths like `assets/...`, relative to the working directory.
