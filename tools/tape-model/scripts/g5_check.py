# g5's criteria (context/tape-project/02-plan.md): a model trained on fake
# passes, judged against the fake deck's noise-free core on held-out music,
# per drive and per band, next to the linear baseline.
#
#   python scripts/g5_check.py out/test.wav MODELDIR out/g5/d0 out/g5/d-12 ...
#
# each pass folder's recording (DIR.wav) has its truth next to it
# (DIR.truth.npz), as `tape-model fake` writes them

import json
import sys
from dataclasses import replace
from pathlib import Path

import numpy as np
from scipy import signal

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from tape_model import faketape, measure, testfile  # noqa: E402
from tape_model import model as tape_model  # noqa: E402
from tape_model.align import GRID  # noqa: E402
from tape_model.cli import _load_test  # noqa: E402
from tape_model.dataset import load_pass  # noqa: E402
from tape_model.dsp import from_db, sinc_read, to_db  # noqa: E402
from tape_model.train import Linear  # noqa: E402

BANDS = (("low <200", None, 200.0), ("mid 200-2k", 200.0, 2000.0), ("high >2k", 2000.0, None))


def esr(err, want):
  return float(10 * np.log10(np.sum(err ** 2) / np.sum(want ** 2)))


def band(x, lo, hi, rate):
  if lo is None:
    sos = signal.butter(4, hi, fs=rate, output="sos")
  elif hi is None:
    sos = signal.butter(4, lo, btype="highpass", fs=rate, output="sos")
  else:
    sos = signal.butter(4, [lo, hi], btype="bandpass", fs=rate, output="sos")
  return signal.sosfiltfilt(sos, x)


class Straight:
  def __init__(self, x, rate):
    self.rec, self.rate = x, rate

  def __call__(self, a, b):
    return self.rec[int(a * self.rate):int(b * self.rate)]

  def frames(self, a, b):
    return int(a * self.rate), int(b * self.rate)

  def speed(self, a, b):
    return 1.0


def main():
  test_path, model_dir, *folders = sys.argv[1:]
  test, test_map = _load_test(test_path)
  rate = test_map.rate
  spec = tape_model.load(Path(model_dir) / "model.json")
  net = tape_model.from_spec(spec)
  report = json.loads((Path(model_dir) / "train.json").read_text())
  spans = report["held_out_spans"]
  passes = [load_pass(f, test, test_map) for f in folders]
  training = [p for p in passes if report["passes"][p.name]["training"]]
  linear = Linear(training, rate, spans)
  a, b = spans[0]
  warm = 2.0
  lo, hi = int((a - warm) * rate), int(b * rate)
  keep = slice(int(warm * rate), None)

  print(f"model: {spec['hidden']} hidden, {report['steps']} steps, {report['minutes']:.1f} min")
  print(f"held out: {spans}; held-out drives {report['held_out_drives']}")
  rows = []
  for folder, p in zip(folders, passes):
    folder = Path(folder)
    truth = np.load(folder.with_suffix(".truth.npz"))
    deck = replace(faketape.Deck(), **{k: v for k, v in json.loads(str(truth["deck"])).items() if k != "flutter"})
    core = faketape.core(test, deck) * from_db(deck.out_gain_db)
    z = np.load(folder / "align.npz")
    m = json.loads((folder / "measure.json").read_text())
    row = {"pass": p.name, "drive": p.drive_db, "training": report["passes"][p.name]["training"]}
    for c in range(p.inputs.shape[1]):
      # the truth on the aligned timeline: core, shifted by this pass's
      # alignment constant, normalized like the training target
      g, pos, q = z[f"grid_{c}"], z[f"pos_{c}"], z[f"track_quality_{c}"]
      k = g - int(truth["offset"])
      ok = (k >= 0) & (k < len(truth["pos"])) & (q == 2)
      const = float(np.median(pos[ok] - truth["pos"][k[ok]] + (deck.channel_delay if c == 1 else 0.0)))
      lv = m["channels"][c]["levels"]
      gain = lv["marker_gain_db"] - (lv["drive_db"] or 0.0)
      want = sinc_read(core[:, c], np.arange(lo, hi) - const)[keep] / from_db(gain)
      x = p.inputs[lo:hi, c]
      got = tape_model.run_torch(net, x, p.drive_db)[keep]
      lin = linear(x, p.drive_db)[keep]
      row[f"esr_{c}"] = esr(got - want, want)
      row[f"linear_{c}"] = esr(lin - want, want)
      if c == 0:
        for name, flo, fhi in BANDS:
          row[name] = esr(band(got - want, flo, fhi, rate), band(want, flo, fhi, rate))
        # harmonics of the 1 khz steps: the model against the core
        s = test_map.section("steps_1000")
        sa, sb = int((s.start - warm) * rate), int(s.end * rate)
        y = np.zeros(len(test))
        y[sa:sb] = tape_model.run_torch(net, test[sa:sb, 0], p.drive_db)
        truth_steps = np.zeros(len(test))
        truth_steps[sa:sb] = sinc_read(core[:, 0], np.arange(sa, sb) - const) / from_db(gain)
        hm = measure.harmonics(Straight(y, rate), test, test_map, rate, 0, 0.0, "steps_1000", 0.0)
        hc = measure.harmonics(Straight(truth_steps, rate), test, test_map, rate, 0, 0.0, "steps_1000", 0.0)
        worst = 0.0
        for sm, sc in zip(hm["steps"], hc["steps"]):
          for i in (0, 1):
            if sc["harmonics_db"][i] is not None and sc["harmonics_db"][i] > -50 and sm["harmonics_db"][i] is not None:
              worst = max(worst, abs(sm["harmonics_db"][i] - sc["harmonics_db"][i]))
            elif sc["harmonics_db"][i] is not None and sc["harmonics_db"][i] > -50:
              worst = np.inf
        row["harmonics_worst_db"] = worst
    rows.append(row)

  rows.sort(key=lambda r: r["drive"])
  print(f"\n{'pass':8s} {'drive':>6s} {'train':>5s} {'esr L':>7s} {'esr R':>7s} {'linear':>7s} {'low':>7s} {'mid':>7s} {'high':>7s} {'h2/h3':>6s}")
  for r in rows:
    print(f"{r['pass']:8s} {r['drive']:+6.1f} {'yes' if r['training'] else 'HELD':>5s} {r['esr_0']:7.1f} {r['esr_1']:7.1f} {r['linear_0']:7.1f} "
          f"{r['low <200']:7.1f} {r['mid 200-2k']:7.1f} {r['high >2k']:7.1f} {r['harmonics_worst_db']:6.2f}")
  print("\nusable training seconds per pass (music):")
  for name, info in sorted(report["passes"].items(), key=lambda kv: kv[1]["drive_db"]):
    cov = info["usable_seconds"]
    print(f"  {name:8s} {info['drive_db']:+6.1f}: total {sum(cov.values()):6.1f} s, music {cov.get('music', 0):5.1f} s, steps {cov.get('steps_1000', 0) + cov.get('steps_100', 0):5.1f} s, sweeps {sum(v for k, v in cov.items() if k.startswith('sweep')):5.1f} s")
  fx = json.loads((Path(model_dir) / "fixture.json").read_text())
  worst = max(float(np.max(np.abs(tape_model.run_numpy(spec, case["input"], case["drive_db"]) - np.array(case["output"], dtype=np.float32)))) for case in fx["cases"])
  print(f"\nT1 parity (numpy from model.json vs torch fixture, drives {[c['drive_db'] for c in fx['cases']]}): max diff {worst:.2e}")
  (Path(model_dir) / "g5_check.json").write_text(json.dumps({"rows": rows, "parity": worst}, indent=1))


if __name__ == "__main__":
  main()
