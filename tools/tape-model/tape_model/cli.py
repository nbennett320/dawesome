# tape-model testfile OUT.wav          the test file, and OUT.map.json
# tape-model fake IN.wav OUT.wav ...   a pass through the fake deck, and
#                                      OUT.truth.npz

import argparse
import hashlib
import json
from dataclasses import replace
from pathlib import Path

import numpy as np
import soundfile as sf

from . import faketape, testfile


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
    deck=json.dumps(truth["deck"]),
  )
  print(f"{args.out}: {len(out) / rate:.1f} s, drive {deck.drive_db:+.1f} dB, {tape} tape")
  print(f"{truth_path}: the true wobble and offset")


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

  args = parser.parse_args(argv)
  args.run(args)
