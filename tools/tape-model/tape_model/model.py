# the tape core as a small recurrent network, and the file it's handed to
# dawesome in. the file is the contract (context/tape-project/02-plan.md):
# the rust core reads nothing else, so everything it needs is spelled out
# in it, and run_numpy below reads only the file, never the torch model.
# torch is only needed for training (the `train` extra)
#
# per sample, per channel, with drive in db:
#
#   x  = audio * 10^(drive / 20)          what reaches the tape (0 vu = the
#                                         test file's -18 dbfs peak sine)
#   in = [x, drive * drive_scale]
#   h  = gru(in, h)                       pytorch's gru: gates r, z, n;
#                                         h starts at zero
#   y  = x + dense(h)                     the output, normalized so 0 vu at
#                                         0 db drive comes back at unity

import json

import numpy as np

SCHEMA = 1
# drive enters the network as drive_db * this (+-18 db -> +-1)
DRIVE_SCALE = 1.0 / 18.0


def _torch():
  import torch
  return torch


def build(hidden=32, seed=1):
  torch = _torch()
  torch.manual_seed(seed)

  class TapeGRU(torch.nn.Module):
    def __init__(self):
      super().__init__()
      self.gru = torch.nn.GRU(2, hidden, batch_first=True)
      self.dense = torch.nn.Linear(hidden, 1)

    # audio and drive_db: (batch, frames). returns (batch, frames)
    def forward(self, audio, drive_db, state=None):
      x = audio * torch.pow(10.0, drive_db / 20.0)
      inputs = torch.stack([x, drive_db * DRIVE_SCALE], dim=-1)
      h, state = self.gru(inputs, state)
      return x + self.dense(h)[..., 0], state

  return TapeGRU()


def export(model, rate, drive_range, trained_on, machine="fake"):
  gru = model.gru

  def flat(t):
    return t.detach().cpu().numpy().astype(np.float32).ravel().tolist()

  hidden = gru.hidden_size
  return {
    "schema": SCHEMA,
    "machine": machine,
    "architecture": "gru",
    "rate": rate,
    "hidden": hidden,
    "inputs": [
      {"name": "audio", "scale": "audio * 10^(drive_db / 20), added back at the output"},
      {"name": "drive_db", "scale": DRIVE_SCALE, "range": list(drive_range)},
    ],
    "output": "audio * 10^(drive_db / 20) + dense(h); 0 VU at 0 dB drive comes back at unity",
    "state": "h starts at zero",
    "gates": "pytorch gru: rows r, z, n. r = sig(Wi_r in + bi_r + Wh_r h + bh_r); "
             "z = sig(Wi_z in + bi_z + Wh_z h + bh_z); n = tanh(Wi_n in + bi_n + r * (Wh_n h + bh_n)); "
             "h' = (1 - z) * n + z * h",
    "weights": {
      "w_ih": {"shape": [3 * hidden, 2], "values": flat(gru.weight_ih_l0)},
      "w_hh": {"shape": [3 * hidden, hidden], "values": flat(gru.weight_hh_l0)},
      "b_ih": {"shape": [3 * hidden], "values": flat(gru.bias_ih_l0)},
      "b_hh": {"shape": [3 * hidden], "values": flat(gru.bias_hh_l0)},
      "dense_w": {"shape": [1, hidden], "values": flat(model.dense.weight)},
      "dense_b": {"shape": [1], "values": flat(model.dense.bias)},
    },
    "trained_on": trained_on,
  }


def save(spec, path):
  with open(path, "w") as f:
    json.dump(spec, f)


def load(path):
  with open(path) as f:
    spec = json.load(f)
  if spec.get("schema") != SCHEMA:
    raise ValueError(f"{path}: schema {spec.get('schema')}, expected {SCHEMA}")
  return spec


def _weights(spec):
  return {k: np.array(v["values"], dtype=np.float32).reshape(v["shape"]) for k, v in spec["weights"].items()}


# the network from its file alone, one sample at a time, in float32: the
# reference the rust core has to match
def run_numpy(spec, audio, drive_db):
  w = _weights(spec)
  hidden = spec["hidden"]
  scale = np.float32(spec["inputs"][1]["scale"])
  audio = np.asarray(audio, dtype=np.float32)
  drive = np.broadcast_to(np.asarray(drive_db, dtype=np.float32), audio.shape)
  h = np.zeros(hidden, dtype=np.float32)
  out = np.empty_like(audio)
  w_ih, w_hh, b_ih, b_hh = w["w_ih"], w["w_hh"], w["b_ih"], w["b_hh"]
  dense_w, dense_b = w["dense_w"][0], w["dense_b"][0]

  def sigmoid(v):
    return np.float32(1.0) / (np.float32(1.0) + np.exp(-v))

  for t in range(len(audio)):
    x = audio[t] * np.float32(10.0) ** (drive[t] / np.float32(20.0))
    gi = w_ih[:, 0] * x + w_ih[:, 1] * (drive[t] * scale) + b_ih
    gh = w_hh @ h + b_hh
    r = sigmoid(gi[:hidden] + gh[:hidden])
    z = sigmoid(gi[hidden:2 * hidden] + gh[hidden:2 * hidden])
    n = np.tanh(gi[2 * hidden:] + r * gh[2 * hidden:])
    h = (np.float32(1.0) - z) * n + z * h
    out[t] = x + dense_w @ h + dense_b
  return out


# the torch model on a whole signal, on the cpu, in float32, in chunks
# (carrying the state), for evaluation and fixtures
def run_torch(model, audio, drive_db, chunk=1 << 16):
  torch = _torch()
  model = model.to("cpu").eval()
  audio = np.asarray(audio, dtype=np.float32)
  drive = np.broadcast_to(np.asarray(drive_db, dtype=np.float32), audio.shape)
  out = np.empty_like(audio)
  state = None
  with torch.no_grad():
    for a in range(0, len(audio), chunk):
      x = torch.from_numpy(audio[None, a:a + chunk].copy())
      d = torch.from_numpy(np.ascontiguousarray(drive[None, a:a + chunk]))
      y, state = model(x, d, state)
      out[a:a + chunk] = y[0].numpy()
  return out


# a reference input and the network's output for it, at two drives, for
# the rust core's parity test
def fixture(model, spec, audio, drives):
  cases = []
  for drive in drives:
    cases.append({
      "drive_db": float(drive),
      "input": np.asarray(audio, dtype=np.float32).tolist(),
      "output": run_torch(model, audio, drive).tolist(),
    })
  return {"schema": SCHEMA, "machine": spec["machine"], "rate": spec["rate"], "cases": cases}


# a torch model from its file (for evaluation: torch runs it fast)
def from_spec(spec):
  torch = _torch()
  net = build(spec["hidden"])
  w = _weights(spec)
  with torch.no_grad():
    net.gru.weight_ih_l0.copy_(torch.from_numpy(w["w_ih"]))
    net.gru.weight_hh_l0.copy_(torch.from_numpy(w["w_hh"]))
    net.gru.bias_ih_l0.copy_(torch.from_numpy(w["b_ih"]))
    net.gru.bias_hh_l0.copy_(torch.from_numpy(w["b_hh"]))
    net.dense.weight.copy_(torch.from_numpy(w["dense_w"]))
    net.dense.bias.copy_(torch.from_numpy(w["dense_b"]))
  return net
