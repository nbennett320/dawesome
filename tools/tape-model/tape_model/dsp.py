# small dsp helpers shared by the test file and the fake tape. audio is
# float64, shaped (frames,) or (frames, channels)

import numpy as np
from scipy import signal

RATE = 48_000


def from_db(db):
  return 10.0 ** (np.asarray(db) / 20.0)


def to_db(x):
  return 20.0 * np.log10(np.maximum(np.abs(x), 1e-12))


def rms(x):
  return float(np.sqrt(np.mean(np.square(x))))


# an rbj cookbook biquad, as (b, a)
def biquad(kind, freq, rate=RATE, q=0.7071, gain_db=0.0):
  big_a = 10.0 ** (gain_db / 40.0)
  w = 2.0 * np.pi * freq / rate
  c, s = np.cos(w), np.sin(w)
  alpha = s / (2.0 * q)
  root = 2.0 * np.sqrt(big_a) * alpha
  if kind == "lowpass":
    b = [(1 - c) / 2, 1 - c, (1 - c) / 2]
    a = [1 + alpha, -2 * c, 1 - alpha]
  elif kind == "highpass":
    b = [(1 + c) / 2, -(1 + c), (1 + c) / 2]
    a = [1 + alpha, -2 * c, 1 - alpha]
  elif kind == "peak":
    b = [1 + alpha * big_a, -2 * c, 1 - alpha * big_a]
    a = [1 + alpha / big_a, -2 * c, 1 - alpha / big_a]
  elif kind == "highshelf":
    b = [
      big_a * ((big_a + 1) + (big_a - 1) * c + root),
      -2 * big_a * ((big_a - 1) + (big_a + 1) * c),
      big_a * ((big_a + 1) + (big_a - 1) * c - root),
    ]
    a = [
      (big_a + 1) - (big_a - 1) * c + root,
      2 * ((big_a - 1) - (big_a + 1) * c),
      (big_a + 1) - (big_a - 1) * c - root,
    ]
  elif kind == "lowshelf":
    b = [
      big_a * ((big_a + 1) - (big_a - 1) * c + root),
      2 * big_a * ((big_a - 1) - (big_a + 1) * c),
      big_a * ((big_a + 1) - (big_a - 1) * c - root),
    ]
    a = [
      (big_a + 1) + (big_a - 1) * c + root,
      -2 * ((big_a - 1) + (big_a + 1) * c),
      (big_a + 1) + (big_a - 1) * c - root,
    ]
  else:
    raise ValueError(f"no biquad called {kind}")
  b, a = np.array(b), np.array(a)
  return b / a[0], a / a[0]


def apply(ba, x):
  return signal.lfilter(ba[0], ba[1], x, axis=0)


# a steep butterworth, for keeping content clear of the pilot tone
def lowpass(x, freq, rate=RATE, order=10):
  sos = signal.butter(order, freq, fs=rate, output="sos")
  return signal.sosfilt(sos, x, axis=0)


# an elliptic low-pass, 100 db down within about a khz of `freq`: keeps
# content clear of the pilot tone (a butterworth is only ~10 db down there)
def brickwall(x, freq, rate=RATE):
  sos = signal.ellip(12, 0.1, 100.0, freq, fs=rate, output="sos")
  return signal.sosfilt(sos, x, axis=0)


def highpass(x, freq, rate=RATE, order=4):
  sos = signal.butter(order, freq, btype="highpass", fs=rate, output="sos")
  return signal.sosfilt(sos, x, axis=0)


def bandpass(x, low, high, rate=RATE, order=2):
  sos = signal.butter(order, [low, high], btype="bandpass", fs=rate, output="sos")
  return signal.sosfilt(sos, x, axis=0)


# pink noise with an rms of 1
def pink(rng, frames):
  spectrum = np.fft.rfft(rng.standard_normal(frames))
  f = np.arange(len(spectrum), dtype=np.float64)
  f[0] = 1.0
  spectrum /= np.sqrt(f)
  spectrum[0] = 0.0
  out = np.fft.irfft(spectrum, frames)
  return out / rms(out)


# raised-cosine fades in and out, in place
def fade(x, seconds, rate=RATE):
  n = min(int(seconds * rate), len(x) // 2)
  if n == 0:
    return x
  ramp = 0.5 - 0.5 * np.cos(np.linspace(0.0, np.pi, n))
  if x.ndim == 2:
    ramp = ramp[:, None]
  x[:n] *= ramp
  x[-n:] *= ramp[::-1]
  return x


def stereo(x):
  return x if x.ndim == 2 else np.stack([x, x], axis=1)


# x read at fractional frame positions with 4-point hermite interpolation,
# as dsp's interp.rs does. positions outside x read 0. done in chunks, so a
# minutes-long file doesn't need gigabytes
def read_at(x, pos, chunk=1 << 20):
  pad = np.concatenate([np.zeros((2,) + x.shape[1:]), x, np.zeros((3,) + x.shape[1:])])
  out = np.empty((len(pos),) + x.shape[1:])
  for start in range(0, len(pos), chunk):
    p = pos[start:start + chunk]
    i = np.floor(p).astype(np.int64)
    t = p - i
    valid = (i >= -1) & (i < len(x))
    k = np.clip(i + 2, 1, len(pad) - 3)
    if x.ndim == 2:
      t = t[:, None]
      valid = valid[:, None]
    y0, y1, y2, y3 = pad[k - 1], pad[k], pad[k + 1], pad[k + 2]
    c1 = 0.5 * (y2 - y0)
    c2 = y0 - 2.5 * y1 + 2.0 * y2 - 0.5 * y3
    c3 = 0.5 * (y3 - y0) + 1.5 * (y1 - y2)
    out[start:start + chunk] = np.where(valid, ((c3 * t + c2) * t + c1) * t + y1, 0.0)
  return out
