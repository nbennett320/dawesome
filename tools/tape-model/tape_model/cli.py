# tape-model testfile OUT.wav             the test file, and OUT.map.json
# tape-model fake IN.wav OUT.wav ...      a pass through the fake deck, and
#                                         OUT.truth.npz
# tape-model align TEST.wav REC.wav DIR   a recording lined up with the test
#                                         file: DIR/aligned.wav, align.npz,
#                                         align.json
# tape-model measure TEST.wav REC.wav DIR [--reference DIR0]
#                                         aligned, then measured: and
#                                         DIR/measure.json
# tape-model train TEST.wav DIR... --out MODELDIR
#                                         the tape core trained on measured
#                                         passes: model.json, fixture.json,
#                                         train.json (needs the train extra)
# tape-model profile DIR... --machine NAME --out FILE
#                                         a session's measurements as the
#                                         effect's profile

import argparse
import hashlib
import json
from dataclasses import replace
from pathlib import Path

import numpy as np
import soundfile as sf

from . import align as aligner
from . import faketape, measure, testfile


def _sibling(path, suffix):
  path = Path(path)
  return path.with_name(path.stem + suffix)


def _write(path, audio, rate):
  Path(path).parent.mkdir(parents=True, exist_ok=True)
  if np.max(np.abs(audio)) >= 1.0:
    raise SystemExit(f"{path} would clip")
  sf.write(path, audio, rate, subtype="PCM_24")


def make_testfile(args):
  audio, test_map = testfile.build(seed=args.seed)
  _write(args.out, audio, test_map.rate)
  test_map.sha256 = hashlib.sha256(Path(args.out).read_bytes()).hexdigest()
  map_path = _sibling(args.out, ".map.json")
  map_path.write_text(json.dumps(test_map.to_json(), indent=2))
  print(f"{args.out}: {test_map.seconds:.1f} s, peak {20 * np.log10(np.max(np.abs(audio))):.1f} dBFS")
  print(f"{map_path}: {len(test_map.sections)} sections")


def make_fake(args):
  audio, rate = sf.read(args.input, dtype="float64", always_2d=True)
  tape = "worn" if args.worn else args.tape
  deck = replace(faketape.preset(tape), drive_db=args.drive, seed=args.seed)
  out, truth = faketape.run(audio, deck, rate)
  sf.write(args.out, out, rate, subtype="PCM_24")
  truth_path = _sibling(args.out, ".truth.npz")
  np.savez_compressed(
    truth_path,
    pos=truth["pos"],
    offset=truth["offset"],
    rate=truth["rate"],
    dropouts=truth["dropouts"],
    deck=json.dumps(truth["deck"]),
  )
  print(f"{args.out}: {len(out) / rate:.1f} s, drive {deck.drive_db:+.1f} dB, {tape} tape")
  print(f"{truth_path}: the true wobble and offset")


def _load_test(path):
  path = Path(path)
  map_path = _sibling(path, ".map.json")
  if not map_path.exists():
    raise SystemExit(f"no {map_path}: the test file's map goes next to it")
  test_map = testfile.Map.from_json(json.loads(map_path.read_text()))
  if test_map.sha256 and hashlib.sha256(path.read_bytes()).hexdigest() != test_map.sha256:
    raise SystemExit(f"{path} isn't the file its map describes (sha256 differs)")
  test, rate = sf.read(path, dtype="float64", always_2d=True)
  return test, test_map


def _json(value):
  if isinstance(value, (np.floating, np.integer)):
    return value.item()
  if isinstance(value, np.ndarray):
    return value.tolist()
  raise TypeError(type(value))


def _align(args):
  test, test_map = _load_test(args.test)
  rec, rate = sf.read(args.recording, dtype="float64", always_2d=True)
  try:
    result = aligner.align(rec, rate, test, test_map)
  except aligner.AlignError as error:
    raise SystemExit(f"{args.recording}: {error}")
  out = Path(args.out)
  out.mkdir(parents=True, exist_ok=True)
  sf.write(out / "aligned.wav", result.aligned.astype(np.float32), test_map.rate, subtype="FLOAT")
  np.savez_compressed(
    out / "align.npz",
    quality=result.quality,
    grid_every=aligner.GRID,
    **{f"grid_{c}": t.grid for c, t in enumerate(result.tracks)},
    **{f"pos_{c}": t.pos for c, t in enumerate(result.tracks)},
    **{f"track_quality_{c}": t.quality for c, t in enumerate(result.tracks)},
  )
  summary = result.summary()
  summary["recording"] = str(args.recording)
  summary["test_sha256"] = test_map.sha256
  summary["bridges"] = [t.bridges for t in result.tracks]
  (out / "align.json").write_text(json.dumps(summary, indent=2, default=_json))
  for c, ch in enumerate(summary["channels"]):
    tracked = f"pilot tracked {ch['pilot_tracked']:.0%}" if ch["pilot"] else f"no pilot (snr {ch['pilot_snr_db']:.0f} dB): content tracking only"
    usable = float((result.quality[:, c] >= 1).mean())
    print(f"  channel {c + 1}: {tracked}, {usable:.0%} usable, delay {ch['delay_frames']:+.3f} frames")
  print(f"{out}: speed {summary['speed']:.5f}, aligned.wav, align.npz, align.json")
  return result, test, test_map


def make_align(args):
  _align(args)


def make_measure(args):
  result, test, test_map = _align(args)
  reference = None
  if args.reference:
    reference = json.loads((Path(args.reference) / "measure.json").read_text())
  m = measure.measure(result, result.recording, test, test_map, reference)
  (Path(args.out) / "measure.json").write_text(json.dumps(m, indent=2, default=_json))
  for c, ch in enumerate(m["channels"]):
    lv, wf, h = ch["levels"], ch["wow_flutter"], ch["hiss"]
    drive = f"drive {lv['drive_db']:+.2f} dB, " if lv["drive_db"] is not None else ""
    print(
      f"  channel {c + 1}: {drive}hiss {h['db_re_0vu']:.1f} dB re 0 VU, "
      f"wow {wf['wow_rms'] * 100:.3f} %, flutter {wf['flutter_rms'] * 100:.3f} % rms, "
      f"3 % THD at {ch['harmonics'][0]['thd_3pct_db']} dB"
    )
  print(f"{args.out}/measure.json")


def _held_out(args, test_map):
  spans = []
  for item in args.hold_out or []:
    name, _, rest = item.partition(":")
    s = test_map.section(name)
    a, _, b = rest.partition("-")
    spans.append((s.start + float(a or 0.0), s.start + float(b) if b else s.end))
  if not args.hold_out:
    # by default the last 20 s of the music: heard by no window
    music = test_map.section("music")
    spans.append((music.end - 20.0, music.end))
  return spans


def make_train(args):
  from . import model as tape_model
  from . import train
  from .dataset import coverage, load_pass

  test, test_map = _load_test(args.test)
  passes = [load_pass(d, test, test_map) for d in args.passes]
  held_drives = set(args.hold_out_drive or [])
  training = [p for p in passes if round(p.drive_db) not in held_drives]
  spans = _held_out(args, test_map)
  report = {"passes": {}, "held_out_spans": spans, "held_out_drives": sorted(held_drives)}
  for p in passes:
    cov = coverage(p, test_map.rate)
    report["passes"][p.name] = {"drive_db": p.drive_db, "training": p in training, "usable_seconds": cov}
    print(f"  {p.name}: drive {p.drive_db:+.2f} dB, usable {sum(cov.values()):.0f} s (music {cov.get('music', 0):.0f} s){'' if p in training else ', held out'}")
  net, log = train.train(training, test_map.rate, spans, args.minutes, args.hidden, args.seed)
  report.update(log)
  drives = [p.drive_db for p in training]
  spec = tape_model.export(net, test_map.rate, (min(drives), max(drives)), [p.name for p in training], args.machine)
  out = Path(args.out)
  out.mkdir(parents=True, exist_ok=True)
  tape_model.save(spec, out / "model.json")
  # the fixture: half a second of the held-out music, at a trained drive
  # and one between trained drives
  music = test_map.section("music")
  a = int((music.end - 10.0) * test_map.rate)
  clip = test[a:a + test_map.rate // 2, 0]
  between = (sorted(drives)[0] + sorted(drives)[1]) / 2 if len(drives) > 1 else drives[0] + 1.0
  (out / "fixture.json").write_text(json.dumps(tape_model.fixture(net, spec, clip, [0.0, between])))
  (out / "train.json").write_text(json.dumps(report, indent=1, default=_json))
  print(f"{out}: model.json, fixture.json, train.json ({log['steps']} steps, {log['minutes']:.1f} min)")


def make_profile(args):
  from . import profile
  spec = profile.build(args.passes, args.machine)
  Path(args.out).parent.mkdir(parents=True, exist_ok=True)
  Path(args.out).write_text(json.dumps(spec, indent=1, default=_json))
  print(f"{args.out}: {args.machine}, from {len(args.passes)} passes")


def main(argv=None):
  parser = argparse.ArgumentParser(prog="tape-model")
  commands = parser.add_subparsers(required=True)

  p = commands.add_parser("testfile", help="write the test file and its map")
  p.add_argument("out")
  p.add_argument("--seed", type=int, default=1)
  p.set_defaults(run=make_testfile)

  p = commands.add_parser("fake", help="pass a file through the fake deck")
  p.add_argument("input")
  p.add_argument("out")
  p.add_argument("--drive", type=float, default=0.0, help="dB past 0 VU")
  p.add_argument("--tape", choices=sorted(faketape.PRESETS), default="fresh")
  p.add_argument("--worn", action="store_true", help="short for --tape worn")
  p.add_argument("--seed", type=int, default=7)
  p.set_defaults(run=make_fake)

  p = commands.add_parser("align", help="line a recording of the test file up with it")
  p.add_argument("test")
  p.add_argument("recording")
  p.add_argument("out")
  p.set_defaults(run=make_align)

  p = commands.add_parser("measure", help="align, then measure what the tape did")
  p.add_argument("test")
  p.add_argument("recording")
  p.add_argument("out")
  p.add_argument("--reference", help="the session's 0 dB pass's output folder, for drive and 0 VU")
  p.set_defaults(run=make_measure)

  p = commands.add_parser("train", help="train the tape core on measured passes")
  p.add_argument("test")
  p.add_argument("passes", nargs="+", help="output folders of `measure`")
  p.add_argument("--out", required=True)
  p.add_argument("--machine", default="fake")
  p.add_argument("--minutes", type=float, default=30.0)
  p.add_argument("--hidden", type=int, default=32)
  p.add_argument("--seed", type=int, default=1)
  p.add_argument("--hold-out-drive", type=float, action="append", help="leave a pass at this drive out of training")
  p.add_argument("--hold-out", action="append", help="SECTION:START-END seconds into it, kept from training (default: the music's last 20 s)")
  p.set_defaults(run=make_train)

  p = commands.add_parser("profile", help="gather a session's measurements into the effect's profile")
  p.add_argument("passes", nargs="+", help="output folders of `measure`; the 0 dB pass first")
  p.add_argument("--machine", required=True)
  p.add_argument("--out", required=True)
  p.set_defaults(run=make_profile)

  args = parser.parse_args(argv)
  args.run(args)
