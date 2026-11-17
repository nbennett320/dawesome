# tape-model

Test signals, measurement and (later) training for Dawesome's Tape effect.
The plan is in `context/tape-project/`. This folder may move to its own repo.

## Setup

```sh
python3 -m venv .venv
.venv/bin/pip install -e '.[dev]'          # test file, fake deck, align, measure
.venv/bin/pip install -e '.[dev,train]'    # and training (PyTorch)
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

Training, on a session's measured passes (the 0 dB pass is the reference):

```sh
.venv/bin/tape-model train out/test.wav out/g5/d0 out/g5/d-12 ... \
  --out out/g5/model --minutes 30 --hold-out-drive 6
.venv/bin/tape-model profile out/g5/d0 out/g5/d-12 ... --machine fake --out out/g5/profile.json

# against the fake deck's truth (G5's criteria)
.venv/bin/python scripts/g5_check.py out/test.wav out/g5/model out/g5/d0 out/g5/d-12 ...
```

`train` runs on the CPU (for a network this small it's about 4x faster than
the Apple GPU) and writes `model.json` (the network, as the Rust core
reads it), `fixture.json` (an input and the expected output, for the Rust
core's parity test) and `train.json` (what it trained on, and how long).

`align` writes `aligned.wav` (the recording on the test file's timeline,
32-bit float), `align.npz` (where every 24 frames came from, and how sure:
2 pilot-tracked, 1 approximate, 0 not aligned) and `align.json`. `measure`
adds `measure.json`.

`out/` and `recordings/` are gitignored. The decks' recordings never go in git.
