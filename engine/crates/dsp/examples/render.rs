// the modulation effects' target tones, rendered so they can be heard in
// a minute: each recipe (the settings the plan aims at: 80s clean guitar,
// a Juno-ish synth, a jet, a psychedelic phaser...) is run over a sound and
// written as a WAV
//
//   pnpm dsp:render                     a plucked chord, made here
//   pnpm dsp:render some.wav            your own (16/24-bit or float, mono
//                                       or stereo)
//   pnpm dsp:render some.wav --out dir  somewhere else than
//                                       engine/target/modulation-renders
//
// these aren't presets: they're the settings to reach for, written down
// (context/modulation-project/05-acceptance.md lists them too)

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use dsp::effects::{chorus, filter, flanger, phaser};
use dsp::{Entry, Runner, Store, Transport};

struct Recipe {
  name: &'static str,
  entry: &'static Entry,
  // parameter keys and plain values; the rest at their defaults
  set: &'static [(&'static str, f32)],
}

const RECIPES: &[Recipe] = &[
  Recipe { name: "chorus-80s-clean-guitar", entry: &chorus::ENTRY, set: &[
    ("voices", 0.), ("shape", 1.), ("rate", 0.8), ("depth", 50.), ("delay", 7.), ("spread", 100.), ("lowcut", 150.), ("tone", 5000.), ("mix", 50.),
  ] },
  Recipe { name: "chorus-juno-synth", entry: &chorus::ENTRY, set: &[
    // (a Juno-60's chorus I: a 0.5 Hz triangle swinging about ±1.6 ms, some 5 or 6 cents)
    ("voices", 1.), ("shape", 1.), ("rate", 0.5), ("depth", 100.), ("delay", 3.5), ("spread", 100.), ("lowcut", 40.), ("tone", 12_000.), ("mix", 50.),
  ] },
  Recipe { name: "chorus-lush-pads", entry: &chorus::ENTRY, set: &[
    ("voices", 3.), ("shape", 0.), ("rate", 0.3), ("depth", 60.), ("delay", 15.), ("spread", 90.), ("mix", 50.),
  ] },
  Recipe { name: "chorus-vocal-double", entry: &chorus::ENTRY, set: &[
    ("voices", 1.), ("rate", 0.25), ("depth", 15.), ("delay", 30.), ("spread", 80.), ("drift", 50.), ("mix", 40.),
  ] },
  Recipe { name: "chorus-bloom", entry: &chorus::ENTRY, set: &[
    ("voices", 0.), ("shape", 1.), ("rate", 0.8), ("depth", 50.), ("delay", 7.), ("spread", 100.), ("tone", 5000.), ("mix", 50.), ("bloom", 55.),
  ] },
  Recipe { name: "chorus-prism", entry: &chorus::ENTRY, set: &[
    ("voices", 3.), ("rate", 0.3), ("depth", 60.), ("delay", 8.), ("spread", 100.), ("mix", 50.), ("prism", 100.),
  ] },
  Recipe { name: "flanger-metallic", entry: &flanger::ENTRY, set: &[
    ("rate", 0.15), ("depth", 70.), ("manual", 1.), ("feedback", 80.), ("mix", 50.),
  ] },
  Recipe { name: "flanger-resonant-tuned-a", entry: &flanger::ENTRY, set: &[
    ("rate", 0.1), ("depth", 15.), ("manual", 2.), ("feedback", 90.), ("tune", 10.), ("mix", 50.),
  ] },
  Recipe { name: "flanger-jet-through-zero", entry: &flanger::ENTRY, set: &[
    ("rate", 0.1), ("depth", 100.), ("manual", 1.5), ("feedback", -40.), ("zero", 1.), ("mix", 50.),
  ] },
  Recipe { name: "flanger-barberpole-rising", entry: &flanger::ENTRY, set: &[
    ("rate", 0.4), ("manual", 3.), ("feedback", 80.), ("barberpole", 100.), ("mix", 50.),
  ] },
  Recipe { name: "flanger-envelope", entry: &flanger::ENTRY, set: &[
    ("depth", 0.), ("manual", 3.), ("feedback", 60.), ("envelope", 80.), ("mix", 50.),
  ] },
  Recipe { name: "phaser-psychedelic", entry: &phaser::ENTRY, set: &[
    ("rate", 0.3), ("depth", 100.), ("stages", 3.), ("feedback", 60.), ("spread", 50.), ("mix", 50.),
  ] },
  Recipe { name: "phaser-vibe", entry: &phaser::ENTRY, set: &[
    ("mode", 1.), ("rate", 4.), ("depth", 70.), ("feedback", 0.), ("mix", 50.),
  ] },
  Recipe { name: "phaser-funky-envelope", entry: &phaser::ENTRY, set: &[
    ("depth", 0.), ("stages", 1.), ("center", 250.), ("feedback", 65.), ("envelope", 100.), ("mix", 50.),
  ] },
  Recipe { name: "phaser-melt", entry: &phaser::ENTRY, set: &[
    ("stages", 5.), ("feedback", 50.), ("melt", 80.), ("mix", 50.),
  ] },
  Recipe { name: "phaser-barberpole-falling", entry: &phaser::ENTRY, set: &[
    ("stages", 3.), ("feedback", 70.), ("barberpole", -100.), ("rate", 0.3), ("mix", 50.),
  ] },
  // the Filter's modes (mode: 1 LFO, 2 Envelope, 3 Dynamic; shape 4
  // Square, 5 S&H; note 4 is 1/8, 7 is 1/4)
  Recipe { name: "filter-auto-wah", entry: &filter::ENTRY, set: &[
    ("type", 2.), ("slope", 0.), ("mode", 3.), ("floor", 300.), ("cutoff", 2500.), ("resonance", 55.), ("attack", 8.), ("decay", 80.), ("sensitivity", 45.),
  ] },
  Recipe { name: "filter-envelope-pluck", entry: &filter::ENTRY, set: &[
    ("mode", 2.), ("floor", 150.), ("cutoff", 5000.), ("resonance", 40.), ("attack", 3.), ("decay", 400.), ("sensitivity", 60.),
  ] },
  Recipe { name: "filter-synced-wobble", entry: &filter::ENTRY, set: &[
    ("mode", 1.), ("sync", 1.), ("note", 4.), ("floor", 150.), ("cutoff", 3000.), ("resonance", 45.), ("drive", 9.),
  ] },
  Recipe { name: "filter-sample-and-hold", entry: &filter::ENTRY, set: &[
    ("mode", 1.), ("sync", 1.), ("note", 4.), ("shape", 5.), ("floor", 300.), ("cutoff", 4000.), ("resonance", 60.),
  ] },
  Recipe { name: "filter-high-pass-sweep", entry: &filter::ENTRY, set: &[
    ("type", 1.), ("mode", 1.), ("sync", 0.), ("rate", 0.15), ("floor", 60.), ("cutoff", 2000.), ("resonance", 35.),
  ] },
  Recipe { name: "filter-talking-vowel", entry: &filter::ENTRY, set: &[
    ("mode", 1.), ("sync", 0.), ("rate", 0.5), ("floor", 200.), ("cutoff", 4000.), ("resonance", 30.), ("vowel", 100.),
  ] },
  Recipe { name: "filter-spin", entry: &filter::ENTRY, set: &[
    ("mode", 1.), ("sync", 0.), ("rate", 0.3), ("floor", 300.), ("cutoff", 5000.), ("resonance", 40.), ("spin", 100.),
  ] },
  Recipe { name: "filter-tuned-c-major", entry: &filter::ENTRY, set: &[
    ("mode", 1.), ("sync", 0.), ("rate", 0.25), ("shape", 1.), ("floor", 200.), ("cutoff", 3000.), ("resonance", 75.), ("tune", 2.),
  ] },
];

const RATE: u32 = 48_000;

fn main() {
  let args: Vec<String> = std::env::args().skip(1).collect();
  let mut input: Option<PathBuf> = None;
  let mut out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/modulation-renders");
  let mut i = 0;
  while i < args.len() {
    if args[i] == "--out" {
      out = PathBuf::from(args.get(i + 1).expect("--out takes a folder"));
      i += 2;
    } else {
      input = Some(PathBuf::from(&args[i]));
      i += 1;
    }
  }

  let (sound, rate) = match &input {
    Some(path) => read_wav(path).unwrap_or_else(|e| panic!("{} can't be read: {}", path.display(), e)),
    None => (plucked_chords(RATE), RATE),
  };
  fs::create_dir_all(&out).expect("the output folder can't be made");
  write_wav(&out.join("00-dry.wav"), &sound, rate).expect("the dry sound can't be written");
  for recipe in RECIPES {
    let wet = render(recipe, &sound, rate);
    let path = out.join(format!("{}.wav", recipe.name));
    write_wav(&path, &wet, rate).expect("a render can't be written");
    let peak = wet[0].iter().chain(&wet[1]).fold(0f32, |m, x| m.max(x.abs()));
    println!("{:<28} peak {:>6.1} dB", recipe.name, 20. * peak.max(1e-9).log10());
  }
  println!("\nwritten to {}", out.canonicalize().unwrap_or(out).display());
}

// the sound through the recipe's effect, then its tail
fn render(recipe: &Recipe, sound: &[Vec<f32>; 2], rate: u32) -> [Vec<f32>; 2] {
  let store = Store::new(recipe.entry);
  for (key, value) in recipe.set {
    let id = recipe.entry.id_of(key).unwrap_or_else(|| panic!("{} has no {}", recipe.entry.info.name, key));
    store.set_plain(id, *value);
  }
  let mut runner = Runner::new(store, rate as f32);
  let transport = Transport { sample_rate: rate, ..Transport::default() };
  let tail = (2 * rate) as usize;
  let frames = sound[0].len() + tail;
  let silence = vec![0f32; tail];
  let input: [Vec<f32>; 2] = std::array::from_fn(|s| [sound[s].as_slice(), &silence].concat());
  let mut out = [vec![0f32; frames], vec![0f32; frames]];
  let [l, r] = &mut out;
  for at in (0..frames).step_by(512) {
    let end = (at + 512).min(frames);
    runner.process([&input[0][at..end], &input[1][at..end]], [&mut l[at..end], &mut r[at..end]], &transport);
  }
  out
}

// ---- a sound to try them on: a clean, picked chord, strummed again and
// again (Karplus-Strong strings), the sort of thing a chorus is for

fn plucked_chords(rate: u32) -> [Vec<f32>; 2] {
  let seconds = 10.;
  let frames = (seconds * rate as f32) as usize;
  let mut out = vec![0f32; frames];
  // E major and A major, open-ish voicings
  let chords: [&[f32]; 2] = [&[82.41, 123.47, 164.81, 207.65, 246.94, 329.63], &[110., 164.81, 220., 277.18, 329.63]];
  let mut seed = 0x2545_f491_4f6c_dd1du64;
  let mut noise = move || {
    seed ^= seed << 13;
    seed ^= seed >> 7;
    seed ^= seed << 17;
    (seed >> 40) as f32 / (1u64 << 24) as f32 * 2. - 1.
  };
  for strum in 0..5 {
    let start = (strum as f32 * 2. * rate as f32) as usize;
    for (n, freq) in chords[strum % 2].iter().enumerate() {
      // each string a little after the last, as a pick crosses them
      let at = start + n * (rate as usize / 90);
      let period = (rate as f32 / freq).round() as usize;
      let mut string: Vec<f32> = (0..period).map(|_| noise() * 0.3).collect();
      let mut k = 0;
      for i in at..(at + 2 * rate as usize).min(frames) {
        let next = (k + 1) % period;
        let x = string[k];
        // a gentle low-pass in the loop: brighter at first, then mellow
        string[k] = 0.996 * 0.5 * (x + string[next]);
        out[i] += x;
        k = next;
      }
    }
  }
  let peak = out.iter().fold(0f32, |m, x| m.max(x.abs())).max(1e-6);
  let scaled: Vec<f32> = out.iter().map(|x| x * 0.5 / peak).collect();
  [scaled.clone(), scaled]
}

// ---- WAV files, by hand (the crate has no dependencies)

fn read_wav(path: &Path) -> Result<([Vec<f32>; 2], u32), String> {
  let bytes = fs::read(path).map_err(|e| e.to_string())?;
  if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
    return Err("it isn't a WAV file".into());
  }
  let u16_at = |i: usize| u16::from_le_bytes([bytes[i], bytes[i + 1]]);
  let u32_at = |i: usize| u32::from_le_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
  let (mut format, mut channels, mut rate, mut bits) = (0u16, 0u16, 0u32, 0u16);
  let mut at = 12;
  while at + 8 <= bytes.len() {
    let (id, size) = (&bytes[at..at + 4], u32_at(at + 4) as usize);
    let body = at + 8;
    if id == b"fmt " {
      format = u16_at(body);
      channels = u16_at(body + 2);
      rate = u32_at(body + 4);
      bits = u16_at(body + 14);
      if format == 0xFFFE { format = u16_at(body + 24); }
    } else if id == b"data" {
      let data = &bytes[body..(body + size).min(bytes.len())];
      let width = (bits / 8) as usize;
      let frames = data.len() / (width * channels.max(1) as usize);
      let sample = |i: usize| -> f32 {
        let s = &data[i * width..];
        match (format, bits) {
          (1, 16) => i16::from_le_bytes([s[0], s[1]]) as f32 / 32_768.,
          (1, 24) => (i32::from_le_bytes([0, s[0], s[1], s[2]]) >> 8) as f32 / 8_388_608.,
          (3, 32) => f32::from_le_bytes([s[0], s[1], s[2], s[3]]),
          _ => 0.,
        }
      };
      if !matches!((format, bits), (1, 16) | (1, 24) | (3, 32)) {
        return Err(format!("its samples ({} bits, format {}) aren't 16 or 24-bit, or float", bits, format));
      }
      let c = channels.max(1) as usize;
      let side = |s: usize| (0..frames).map(|f| sample(f * c + s.min(c - 1))).collect::<Vec<f32>>();
      return Ok(([side(0), side(1)], rate));
    }
    at = body + size + (size & 1);
  }
  Err("it has no sound in it".into())
}

// 32-bit float, stereo (an effect can go over full scale: nothing's
// clipped)
fn write_wav(path: &Path, sound: &[Vec<f32>; 2], rate: u32) -> std::io::Result<()> {
  let frames = sound[0].len();
  let data = (frames * 8) as u32;
  let mut f = fs::File::create(path)?;
  f.write_all(b"RIFF")?;
  f.write_all(&(36 + data).to_le_bytes())?;
  f.write_all(b"WAVEfmt ")?;
  f.write_all(&16u32.to_le_bytes())?;
  f.write_all(&3u16.to_le_bytes())?;
  f.write_all(&2u16.to_le_bytes())?;
  f.write_all(&rate.to_le_bytes())?;
  f.write_all(&(rate * 8).to_le_bytes())?;
  f.write_all(&8u16.to_le_bytes())?;
  f.write_all(&32u16.to_le_bytes())?;
  f.write_all(b"data")?;
  f.write_all(&data.to_le_bytes())?;
  let mut buf = Vec::with_capacity(frames * 8);
  for i in 0..frames {
    buf.extend_from_slice(&sound[0][i].to_le_bytes());
    buf.extend_from_slice(&sound[1][i].to_le_bytes());
  }
  f.write_all(&buf)
}
