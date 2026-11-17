# the model file: the numpy reference (built from the file alone) matches
# torch, and the training windows keep out of what's held out

import json

import numpy as np
import pytest

torch = pytest.importorskip("torch")

from tape_model import model as tape_model
from tape_model.dataset import PassData, Windows


def test_numpy_from_the_file_matches_torch(tmp_path):
  net = tape_model.build(hidden=8, seed=3)
  spec = tape_model.export(net, 48_000, (-12.0, 18.0), ["a"])
  tape_model.save(spec, tmp_path / "model.json")
  spec = tape_model.load(tmp_path / "model.json")
  audio = (np.random.default_rng(0).standard_normal(2000) * 0.1).astype(np.float32)
  for drive in (0.0, 7.5):
    want = tape_model.run_torch(net, audio, drive)
    got = tape_model.run_numpy(spec, audio, drive)
    assert np.max(np.abs(got - want)) <= 1e-5


def test_the_file_says_what_the_rust_core_needs():
  spec = tape_model.export(tape_model.build(hidden=4), 48_000, (-12.0, 18.0), ["a"])
  assert spec["schema"] == tape_model.SCHEMA and spec["rate"] == 48_000
  shapes = {k: v["shape"] for k, v in spec["weights"].items()}
  assert shapes == {"w_ih": [12, 2], "w_hh": [12, 4], "b_ih": [12], "b_hh": [12], "dense_w": [1, 4], "dense_b": [1]}
  assert spec["inputs"][1]["range"] == [-12.0, 18.0]
  json.dumps(spec)


def test_running_in_chunks_keeps_the_state():
  net = tape_model.build(hidden=8, seed=1)
  audio = (np.random.default_rng(1).standard_normal(5000) * 0.1).astype(np.float32)
  assert np.allclose(tape_model.run_torch(net, audio, 3.0, chunk=777), tape_model.run_torch(net, audio, 3.0), atol=1e-6)


def test_windows_keep_out_of_held_out_spans():
  n = 48_000 * 4
  p = PassData("p", 0.0, np.zeros((n, 1)), np.zeros((n, 1)), np.ones((n, 1), dtype=bool))
  w = Windows([p], 1024, 1024, [(1.0, 2.0)], 48_000)
  for _, _, s in w.starts:
    assert s + 2048 <= 48_000 or s >= 2 * 48_000
