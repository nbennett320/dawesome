# tape-model

Test signals, measurement and (later) training for Dawesome's Tape effect.
The plan is in `context/tape-project/`. This folder may move to its own repo.

## Setup

```sh
python3 -m venv .venv
.venv/bin/pip install -e '.[dev]'
```

(`uv sync --extra dev` works too.)

## Use

```sh
# the test file played into the decks, and its map of sections
.venv/bin/tape-model testfile out/test.wav

# a simulated deck, to prove the pipeline before real recordings
.venv/bin/tape-model fake out/test.wav out/fake.wav --drive 6
.venv/bin/tape-model fake out/test.wav out/worn.wav --worn
.venv/bin/tape-model fake out/test.wav out/ancient.wav --tape ancient

# line a recording up with the test file, and measure it. the session's
# 0 dB pass is the reference for drive and 0 VU
.venv/bin/tape-model measure out/test.wav out/fake0.wav out/fake0
.venv/bin/tape-model measure out/test.wav out/fake.wav out/fake --reference out/fake0

.venv/bin/pytest -m "not slow"   # seconds
.venv/bin/pytest                 # with whole fake passes: several minutes
```

`align` writes `aligned.wav` (the recording on the test file's timeline,
32-bit float), `align.npz` (where every 24 frames came from, and how sure:
2 pilot-tracked, 1 approximate, 0 not aligned) and `align.json`. `measure`
adds `measure.json`.

`out/` and `recordings/` are gitignored. The decks' recordings never go in git.
