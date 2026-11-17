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

.venv/bin/pytest
```

`out/` and `recordings/` are gitignored. The decks' recordings never go in git.
