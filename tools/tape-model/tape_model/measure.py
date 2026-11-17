# what a pass through tape did, measured from the test file's sections:
# drive, frequency response, harmonics and squash against level, hiss, wow
# and flutter, dropouts, crosstalk, print-through. the profile the tape
# effect is fitted to comes from these
#
# spectral measurements read the recording itself, each section found
# through the alignment: energy in a band doesn't care about a little
# wobble, and resampling would only add its own error. dropouts compare
# envelopes, so they read the aligned recording
#
# levels are against 0 vu: the reference pass's drive marker says where
# that is (the 0 db pass of a session). without one, this pass is taken
# to be it

import numpy as np
from scipy import signal

from . import testfile
from .align import GRID, tone_phase
from .dsp import from_db, to_db

# 1/6-octave points for the response
RESPONSE_HZ = tuple(float(f) for f in 1000.0 * 2.0 ** (np.arange(-30, 25) / 6.0) if 20.0 <= f <= 20_000.0)
HARMONICS = 5
HISS_BAND = (20.0, 20_000.0)
PILOT_NOTCH_HZ = 300.0
# wow and flutter bands (hz), on the speed's deviation from its mean
DRIFT_HZ = 0.5
WOW_HZ = (0.5, 6.0)
FLUTTER_HZ = (6.0, 200.0)
# dropouts: 10 ms windows, this far under the running ratio (over a second)
DROPOUT_WINDOW_S = 0.01
DROPOUT_DB = 3.0
DROPOUT_CONTENT_DB = -30.0
DROPOUT_TRANSIENT_WINDOWS = 9
DROPOUT_TRANSIENT_DB = 3.0


class _Cutter:
  # the recording for a span of the test file, through one channel's
  # alignment: test-file seconds in, recording frames out
  def __init__(self, rec, track, rate):
    self.rec, self.track, self.rate = rec, track, rate

  def frames(self, t0, t1):
    n = np.interp([t0 * self.rate, t1 * self.rate], self.track.pos, self.track.grid)
    return int(round(n[0])), int(round(n[1]))

  def __call__(self, t0, t1):
    a, b = self.frames(t0, t1)
    return self.rec[max(0, a):max(0, b)]

  def speed(self, t0, t1):
    a, b = self.frames(t0, t1)
    return (t1 - t0) * self.rate / max(1, b - a)


def _band_power(x, rate, low, high):
  spectrum = np.abs(np.fft.rfft(x * np.hanning(len(x)))) ** 2
  freqs = np.fft.rfftfreq(len(x), 1.0 / rate)
  return float(spectrum[(freqs >= low) & (freqs < high)].sum())


def _tone_db(x, hz, rate, width=50.0):
  # a tone's rms level in dbfs, from its band: hann-windowed energy
  if len(x) < 64:
    return None
  power = _band_power(x, rate, hz - width, hz + width)
  # a hann window's power gain is 3/8; an rfft's one-sided sum is n^2/4
  return float(10 * np.log10(power / (len(x) ** 2 * 3 / 8 / 2) + 1e-30))


def levels(cut, test, test_map, rate, channel):
  out = {}
  for name in ("drive_marker", "calibration"):
    s = test_map.section(name)
    t0, t1 = s.start + 0.5, s.end - 0.5
    hz = s.info["hz"]
    got = _tone_db(cut(t0, t1), hz * cut.speed(t0, t1), rate)
    want = _tone_db(test[int(t0 * rate):int(t1 * rate), channel], hz, rate)
    out[name] = None if got is None else got - want
  return {"marker_gain_db": out["drive_marker"], "calibration_gain_db": out["calibration"]}


def response(cut, test, test_map, rate, channel, name="sweep_-30"):
  # the energy in each 1/6 octave, recording against test file, from a
  # sweep (the tail just after it too, for what the tape smears)
  s = test_map.section(name)
  t0, t1 = s.start, s.end + 0.2
  got = cut(t0, t1)
  want = test[int(t0 * rate):int(t1 * rate), channel]
  speed = cut.speed(t0, t1)
  out = []
  for f in RESPONSE_HZ:
    lo, hi = f * 2 ** (-1 / 12), f * 2 ** (1 / 12)
    g = _band_power(got, rate, lo * speed, hi * speed) * speed * (len(want) / len(got)) ** 2
    w = _band_power(want, rate, lo, hi)
    out.append(10 * np.log10((g + 1e-30) / (w + 1e-30)))
  out = np.array(out)
  at_1k = out[np.argmin(np.abs(np.array(RESPONSE_HZ) - 1000.0))]
  return {"hz": list(RESPONSE_HZ), "db": list(out - at_1k), "gain_1k_db": float(at_1k), "sweep": name}


# a harmonic must be this far over the noise between harmonics to count
HARMONIC_OVER_NOISE_DB = 10.0


def harmonics(cut, test, test_map, rate, channel, gain_db, name, spread=1e-3):
  # each level step: the fundamental's gain against small-signal, and
  # harmonics 2-5 against the fundamental (None under the noise). each
  # band is as wide as the wobble (`spread`, the speed's deviation) moves it
  s = test_map.section(name)
  hz, step = s.info["hz"], s.info["step_seconds"]
  rows = []
  for i, level in enumerate(s.info["relative_db"]):
    t0, t1 = s.start + i * step + 0.1, s.start + (i + 1) * step - 0.1
    x = cut(t0, t1)
    speed = cut.speed(t0, t1)
    bin_hz = rate / max(1, len(x))

    def width(k):
      return 4.0 * spread * k * hz + 3.0 * bin_hz

    fund = _tone_db(x, hz * speed, rate, width(1))
    want = _tone_db(test[int(t0 * rate):int(t1 * rate), channel], hz, rate, width(1))
    rel = []
    for k in range(2, HARMONICS + 1):
      h = _tone_db(x, k * hz * speed, rate, width(k))
      # the noise just either side, scaled to the same width
      side = [_tone_db(x, (k + d) * hz * speed, rate, width(k)) for d in (-0.5, 0.5)]
      noise = 10 * np.log10(np.mean(from_db(np.array(side)) ** 2))
      # less the noise inside its band
      clean = 10 * np.log10(max(from_db(h) ** 2 - from_db(noise) ** 2, 1e-30))
      rel.append(clean - fund if h > noise + HARMONIC_OVER_NOISE_DB else None)
    heard = [r for r in rel if r is not None]
    thd = float(np.sqrt(np.sum(from_db(np.array(heard)) ** 2)) * 100) if heard else 0.0
    rows.append({"relative_db": level, "squash_db": fund - want - gain_db, "harmonics_db": rel, "thd_pct": thd})
  # the classic tape headroom: where the distortion reaches 3 %
  mol = None
  for a, b in zip(rows, rows[1:]):
    if 0.0 < a["thd_pct"] < 3.0 <= b["thd_pct"]:
      fa, fb = np.log10(a["thd_pct"]), np.log10(b["thd_pct"])
      mol = a["relative_db"] + (np.log10(3.0) - fa) / (fb - fa) * (b["relative_db"] - a["relative_db"])
      break
  return {"hz": hz, "steps": rows, "thd_3pct_db": mol}


def hiss(cut, test_map, rate, gain_db, pilot_hz):
  s = test_map.section("hiss")
  t0, t1 = s.start + 3.0, s.end - 2.0
  x = cut(t0, t1)
  speed = cut.speed(t0, t1)
  freqs, psd = signal.welch(x, rate, nperseg=1 << 14)
  df = freqs[1] - freqs[0]
  band = (freqs >= HISS_BAND[0]) & (freqs < HISS_BAND[1])
  notch = np.abs(freqs - pilot_hz * speed) <= PILOT_NOTCH_HZ
  total = float(np.sum(psd[band & ~notch]) * df)
  thirds = 1000.0 * 2.0 ** (np.arange(-17, 14) / 3.0)
  spectrum = []
  for f in thirds:
    sel = (freqs >= f * 2 ** (-1 / 6)) & (freqs < f * 2 ** (1 / 6)) & ~notch
    spectrum.append(float(10 * np.log10(np.sum(psd[sel]) * df + 1e-30)) - _ref_rms_db(gain_db))
  return {
    "db_re_0vu": float(10 * np.log10(total + 1e-30)) - _ref_rms_db(gain_db),
    "dbfs": float(10 * np.log10(total + 1e-30)),
    "third_octaves_hz": list(thirds),
    "third_octaves_db_re_0vu": spectrum,
  }


def _ref_rms_db(gain_db):
  # 0 vu in the recording: the reference sine's rms, through the tape
  return testfile.REF_DBFS - 3.0103 + gain_db


def speed_curve(cut, test_map, rate):
  # the tape's speed through the flutter tone, from the tone's own phase:
  # (seconds, speed as a multiple of the mean), at 2 khz
  s = test_map.section("flutter_tone")
  t0, t1 = s.start + 0.5, s.end - 0.5
  a, b = cut.frames(t0, t1)
  x = cut.rec[a:b]
  hz = s.info["hz"] * cut.speed(t0, t1)
  grid = np.arange(0, len(x), GRID)
  z = tone_phase(x, hz, rate, grid)
  phase = np.unwrap(np.angle(z)) + 2 * np.pi * hz * grid / rate
  pos = phase * rate / (2 * np.pi * s.info["hz"])
  speed = np.gradient(pos, grid)
  return grid / rate, speed / np.mean(speed), float(np.mean(speed))


def wow_flutter(cut, test_map, rate):
  t, v, mean = speed_curve(cut, test_map, rate)
  dev = v - 1.0
  grid_rate = rate / GRID
  edge = int(grid_rate)

  def band(lo, hi):
    if lo is None:
      sos = signal.butter(2, hi, fs=grid_rate, output="sos")
    else:
      sos = signal.butter(2, [lo, hi], btype="bandpass", fs=grid_rate, output="sos")
    return float(np.sqrt(np.mean(signal.sosfiltfilt(sos, dev)[edge:-edge] ** 2)))

  freqs, psd = signal.welch(dev - np.mean(dev), grid_rate, nperseg=min(len(dev), 1 << 14))
  df = freqs[1] - freqs[0]
  floor = signal.medfilt(psd, 31)
  peaks, _ = signal.find_peaks(psd, height=10 * floor)
  peaks = [p for p in peaks if 1.0 <= freqs[p] <= FLUTTER_HZ[1]]
  found = []
  for p in sorted(peaks, key=lambda p: -psd[p])[:8]:
    power = np.sum(psd[max(0, p - 3):p + 4]) * df
    found.append({"hz": float(freqs[p]), "depth": float(np.sqrt(2 * power))})
  return {
    "speed": mean,
    "drift_rms": band(None, DRIFT_HZ),
    "wow_rms": band(*WOW_HZ),
    "flutter_rms": band(*FLUTTER_HZ),
    "peaks": found,
  }


def dropouts(aligned, test, test_map, rate, channel, test_channel, quality=None):
  # where the recording's level dips against the test file's, in the
  # parts with content. needs the aligned recording
  # 10 ms windows every 5 ms, so a short dip on a window's edge still
  # lands whole in one
  w = int(DROPOUT_WINDOW_S * rate)
  hop = w // 2
  sos = signal.butter(2, (300.0, 3000.0), btype="bandpass", fs=rate, output="sos")
  got = signal.sosfiltfilt(sos, aligned[:, channel])
  want = signal.sosfiltfilt(sos, test[:, test_channel])
  length = min(len(got), len(want))

  def envelope(x):
    power = np.convolve(x[:length] ** 2, np.ones(w) / w, mode="valid")
    return np.sqrt(power[::hop])

  g, e = envelope(got), envelope(want)
  n = len(e)
  content = to_db(e) > testfile.REF_DBFS - 3.0103 + DROPOUT_CONTENT_DB
  # tape squashes a hit for a moment (the fake deck by up to 8 db, for one
  # window), which looks just like a dropout: on a transient, a dip only
  # counts if it lasts longer than that
  e_db = to_db(e)
  rising = e_db > signal.medfilt(e_db, DROPOUT_TRANSIENT_WINDOWS) + DROPOUT_TRANSIENT_DB
  # and an onset out of near-silence (the tape's phase smears a sliver of
  # it into the window either side)
  before = np.minimum.reduce([np.roll(e_db, k) for k in (2, 3, 4)])
  rising |= e_db > before + 2 * DROPOUT_TRANSIENT_DB
  rising = np.convolve(rising.astype(float), np.ones(5), mode="same") > 0
  if quality is not None:
    starts = np.arange(n) * hop
    q = np.minimum(quality[np.minimum(starts // GRID, len(quality) - 1)], quality[np.minimum((starts + w - 1) // GRID, len(quality) - 1)])
    content &= q >= 1
  ratio = np.where(content, to_db(g) - to_db(e), np.nan)
  # against its own running level (tape squash and level wander move slowly)
  idx = np.flatnonzero(content)
  if len(idx) < 10:
    return {"per_minute": None, "content_seconds": 0.0, "events": []}
  kernel = min(201, len(idx) - (1 - len(idx) % 2))
  med = np.interp(np.arange(n), idx, signal.medfilt(ratio[idx], kernel))
  low = content & (ratio < med - DROPOUT_DB)
  events = []
  for a, b in zip(*np.flatnonzero(np.diff(np.concatenate([[0], low.astype(int), [0]]))).reshape(-1, 2).T):
    if b - a < 3 and rising[a:b].any():
      continue
    events.append({"time": a * hop / rate, "seconds": ((b - a) * hop + w - hop) / rate, "depth_db": float(np.nanmax(med[a:b] - ratio[a:b]))})
  seconds = content.sum() * hop / rate
  depths = np.array([ev["depth_db"] for ev in events]) if events else np.array([0.0])
  lengths = np.array([ev["seconds"] for ev in events]) if events else np.array([0.0])
  return {
    "per_minute": len(events) / seconds * 60.0,
    "content_seconds": float(seconds),
    "depth_db_percentiles": list(np.percentile(depths, [10, 50, 90])),
    "seconds_percentiles": list(np.percentile(lengths, [10, 50, 90])),
    "events": events,
  }


def crosstalk(cuts, test_map, rate):
  if len(cuts) < 2:
    return None
  out = {}
  for name, src, dst in (("left_into_right", 0, 1), ("right_into_left", 1, 0)):
    s = test_map.section("left_only" if src == 0 else "right_only")
    t0, t1 = s.start + 0.5, s.end - 0.5
    hz = s.info["hz"] * cuts[src].speed(t0, t1)
    out[name] = _tone_db(cuts[dst](t0, t1), hz, rate) - _tone_db(cuts[src](t0, t1), hz, rate)
  out["mono"] = bool(out["left_into_right"] > -6.0 and out["right_into_left"] > -6.0)
  return out


# an echo must stand this far over the hiss after it
PRINT_THROUGH_MIN_DB = 2.0


def print_through(cut, test_map, rate, width=20.0, window_s=0.05, max_s=4.0):
  # the flutter tone's ghost: after it ends (post-echo, from the layer
  # outside) and before it starts (pre-echo, from the layer inside), a
  # reel's turn either side, against the hiss in the same band
  tone, quiet = test_map.section("flutter_tone"), test_map.section("hiss")
  hz = tone.info["hz"]
  speed = cut.speed(tone.start + 0.5, tone.end - 0.5)
  level = _tone_db(cut(tone.start + 0.5, tone.end - 0.5), hz * speed, rate, width)

  def band_db(t0, t1):
    return _tone_db(cut(t0, t1), hz * speed, rate, width)

  floor = np.median([band_db(t, t + window_s) for t in np.arange(quiet.start + 6.0, quiet.end - 2.0, window_s)])
  times = np.arange(tone.end + 0.05, tone.end + max_s, window_s)
  after = np.array([band_db(t, t + window_s) for t in times])
  # where it steps from echo down to hiss: the split that best separates
  # the two (the echo can be only a few db over the hiss)
  power = from_db(after) ** 2
  splits = np.arange(2, len(power) - 2)
  contrast = [10 * np.log10(np.mean(power[:k]) / np.mean(power[k:])) for k in splits]
  best = int(splits[int(np.argmax(contrast))])
  if max(contrast) < PRINT_THROUGH_MIN_DB:
    return {"found": False, "floor_db": float(floor - level)}
  period = float(times[best] - tone.end)
  post = float(10 * np.log10(np.mean(power[:best])) - level)
  # the pre-echo: the period before the tone starts, skipping anything
  # else in that stretch (an anchor burst, the 1 khz tones)
  pre_spans = [(tone.start - period + 0.05, tone.start - 0.05)]
  for s in test_map.sections:
    if s.name != "flutter_tone" and s.end > pre_spans[0][0] and s.start < tone.start and s.name.startswith("anchor"):
      a, b = pre_spans.pop()
      pre_spans += [(a, s.start - 0.05), (s.end + 0.05, b)]
  pre = [band_db(a, b) for a, b in pre_spans if b - a > 0.1]
  return {
    "found": True,
    "period_s": period,
    "post_db": post,
    "pre_db": float(np.max(pre) - level) if pre else None,
    "floor_db": float(floor - level),
  }


# everything, per channel. `reference` is another pass's measurement (the
# session's 0 db pass) for drive and 0 vu
def measure(alignment, rec, test, test_map, reference=None):
  rate = test_map.rate
  cuts = [_Cutter(rec[:, c], t, rate) for c, t in enumerate(alignment.tracks)]
  channels = []
  for c, cut in enumerate(cuts):
    tc = min(c, test.shape[1] - 1)
    lv = levels(cut, test, test_map, rate, tc)
    ref = reference["channels"][c]["levels"]["marker_gain_db"] if reference else lv["marker_gain_db"]
    drive = lv["marker_gain_db"] - ref if reference else None
    # 0 vu through the tape, in this pass's terms: the reference's gain,
    # plus this pass's drive (levels are against what went in)
    gain = ref + (drive or 0.0)
    wf = wow_flutter(cut, test_map, rate)
    spread = float(np.sqrt(wf["drift_rms"] ** 2 + wf["wow_rms"] ** 2 + wf["flutter_rms"] ** 2))
    channels.append({
      "levels": {**lv, "drive_db": drive},
      "response": response(cut, test, test_map, rate, tc),
      "response_by_level": [response(cut, test, test_map, rate, tc, n)["db"] for n in ("sweep_-18", "sweep_-10")],
      "harmonics": [harmonics(cut, test, test_map, rate, tc, gain, n, spread) for n in ("steps_1000", "steps_100")],
      "hiss": hiss(cut, test_map, rate, ref, test_map.pilot["hz"]),
      "wow_flutter": wf,
      "dropouts": dropouts(alignment.aligned, test, test_map, rate, c, tc, alignment.quality[:, c]),
      "print_through": print_through(cut, test_map, rate),
    })
  return {"channels": channels, "crosstalk": crosstalk(cuts, test_map, rate)}
