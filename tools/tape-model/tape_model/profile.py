# a session's measurements gathered into the tape effect's profile: what
# the dsp parts (wow and flutter, hiss, dropouts, crosstalk, eq) are set
# from. kept small for now (g5); the contract is in context/tape-project/
# 02-plan.md. the first folder is the session's 0 db pass

import json
from pathlib import Path

import numpy as np

SCHEMA = 1


def build(folders, machine):
  passes = []
  for f in folders:
    f = Path(f)
    passes.append((json.loads((f / "measure.json").read_text()), json.loads((f / "align.json").read_text())))
  ref, ref_align = passes[0]
  ch = ref["channels"][0]
  wf = [m["channels"][0]["wow_flutter"] for m, _ in passes]
  return {
    "schema": SCHEMA,
    "machine": machine,
    "speed_error": float(np.median([a["speed_error"] for _, a in passes])),
    "channel_delays_frames": [c["delay_frames"] for c in ref_align["channels"]],
    "response": ch["response"],
    "hiss": {k: ch["hiss"][k] for k in ("db_re_0vu", "third_octaves_hz", "third_octaves_db_re_0vu")},
    "wow_flutter": {
      "drift_rms": float(np.median([w["drift_rms"] for w in wf])),
      "wow_rms": float(np.median([w["wow_rms"] for w in wf])),
      "flutter_rms": float(np.median([w["flutter_rms"] for w in wf])),
      "peaks": wf[0]["peaks"],
    },
    "crosstalk": ref["crosstalk"],
    "dropouts": {k: v for k, v in ch["dropouts"].items() if k != "events"},
    "print_through": ch["print_through"],
    "headroom": [
      {"drive_db": m["channels"][0]["levels"]["drive_db"] or 0.0,
       "thd_3pct_db": m["channels"][0]["harmonics"][0]["thd_3pct_db"]}
      for m, _ in passes
    ],
    "passes": [str(f) for f in folders],
  }
