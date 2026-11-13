// timings of the sequencer's hot paths, taken before and after the pattern
// merge (see context/pattern-merge-project/perf/measure.md). they print what
// they measure and check nothing, as timings change from machine to machine,
// so they only run when asked, in a release build, one at a time:
//
//   cargo test --release --no-default-features --bin app perf_ -- --ignored --show-output --test-threads=1
//
// the project they time is made in `big_project`, through the sequencer's
// own commands: the one place to change when the pattern model does
use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::time::{Duration, Instant};

use crate::daw;
use crate::history::{History, Step};

// the engine's own drum samples, for the rows
const SAMPLES: [&str; 8] = [
  "assets/assets_66-bd-01.wav",
  "assets/assets_66-sd-01.wav",
  "assets/assets_66-hh-01-or.wav",
  "assets/assets_66-hh-01-or-2.wav",
  "assets/clap.wav",
  "assets/kick1.wav",
  "assets/snare.wav",
  "assets/tink.wav",
];
const SAMPLE_ROWS: usize = 16;
const SYNTHS: u64 = 8;
// the long pattern. patterns can be 128 bars now, but this stays the
// baseline's 64 (the longest a pattern was then), so the numbers compare
const LONG_BARS: u32 = 64;
const RATE: u32 = 44_100;
const TEMPO: f32 = 120.;

// project (b) of make_big_project.py: 16 sample rows and 8 synth rows, and
// - drums: 4 bars, every step on every sample row
// - short: 4 bars, a note on every step for every synth row
// - long: LONG_BARS bars, a note on every step for every synth row
struct Big {
  samples: Vec<u64>,
  drums: u64,
  short: u64,
  long: u64,
}

fn big_project(seq: &mut daw::Sequencer) -> Big {
  let samples: Vec<u64> = (0..SAMPLE_ROWS).map(|n| seq.add_row(SAMPLES[n % SAMPLES.len()]).unwrap()).collect();
  let synths: Vec<u64> = (0..SYNTHS).map(|n| seq.add_synth_row(daw::Synth {
    id: 1_000 + n, plugin_id: "vst3:perf".into(), path: String::new(), name: format!("Synth {}", n + 1),
    vendor: String::new(), volume: 1., muted: false,
  })).collect();

  let drums = seq.add_pattern(4, 4).unwrap();
  let length = seq.pattern(drums).unwrap().length;
  for &row in &samples {
    for step in 0..length {
      seq.set_step(drums, row, step, true).unwrap();
    }
  }

  let mut notes_pattern = |bars: u32| {
    let id = seq.add_pattern(bars, 4).unwrap();
    let length = seq.pattern(id).unwrap().length;
    for (n, &row) in synths.iter().enumerate() {
      // a rising line, an octave per synth
      let notes = (0..length).map(|step| daw::PatternNote {
        pitch: (36 + n as u32 * 6 + step % 12) as u8, start: step as f64, length: 1., velocity: 0.8,
      }).collect();
      seq.set_pattern_notes(id, row, notes).unwrap();
    }
    id
  };
  let short = notes_pattern(4);
  let long = notes_pattern(LONG_BARS);
  Big { samples, drums, short, long }
}

// a graph holding project (b), with these clips: (pattern, bar, track)
fn big_graph(clips: impl Fn(&Big) -> Vec<(u64, u32, u32)>) -> (daw::AudioGraph<'static>, Big) {
  let mut graph = daw::AudioGraph::new(RATE, TEMPO, 1_024);
  let big = big_project(&mut graph.sequencer.lock().unwrap());
  let bar = graph.beat_interval() * 4;
  for (pattern, at, track) in clips(&big) {
    graph.add_pattern_node::<daw::timing::QuarterNote>(pattern, bar * at, track, None).unwrap();
  }
  graph.init();
  (graph, big)
}

// mean and 95th percentile, in microseconds
fn spread(times: &[Duration]) -> (f64, f64) {
  let mut us: Vec<f64> = times.iter().map(|t| t.as_secs_f64() * 1e6).collect();
  us.sort_by(|a, b| a.partial_cmp(b).unwrap());
  let mean = us.iter().sum::<f64>() / us.len() as f64;
  let p95 = us[((us.len() as f64 * 0.95) as usize).min(us.len() - 1)];
  (mean, p95)
}

fn report(name: &str, times: &[Duration], scheduled: usize) {
  let (mean, p95) = spread(times);
  let total: f64 = times.iter().map(|t| t.as_secs_f64() * 1e6).sum();
  let per = if scheduled > 0 { format!(", {:.3} µs per hit or note ({} scheduled)", total / scheduled as f64, scheduled) } else { String::new() };
  println!("perf {}: n={}, mean {:.1} µs, p95 {:.1} µs{}", name, times.len(), mean, p95, per);
}

// play `beats` beats from the top, `rounds` times, timing each beat's slice
// as run_playlist's slice_voices makes it (its sounds made voices, and its
// notes). returns the times and how many hits and notes they scheduled
fn time_slices(
  graph: &mut daw::AudioGraph<'static>,
  engine: &daw::Engine,
  beats: u32,
  rounds: usize,
  slice: impl Fn(&daw::AudioGraph<'static>, Duration, Duration, bool) -> (Vec<daw::SliceSound>, Vec<daw::SliceNote>),
) -> (Vec<Duration>, usize) {
  let beat = graph.beat_interval();
  let mut times = Vec::new();
  let mut scheduled = 0;
  for _ in 0..rounds {
    for n in 0..beats {
      let at = beat * n;
      graph.set_current_offset(Some(at));
      let start = Instant::now();
      let (sounds, notes) = slice(graph, at, beat, n == 0);
      let voices: Vec<_> = sounds.into_iter()
        .map(|(delay, source, route)| engine.boxed_voice(source, Some(1), engine.frames_in(at + delay), route))
        .collect();
      black_box((&voices, &notes));
      times.push(start.elapsed());
      scheduled += voices.len() + notes.len();
      // (dropped untimed: the engine drops them, off this thread)
      drop((voices, notes));
    }
  }
  (times, scheduled)
}

#[test]
#[ignore]
fn perf_slice() {
  let (engine, _renderer) = daw::Engine::new(RATE);
  let playlist = |graph: &daw::AudioGraph<'static>, at: Duration, beat: Duration, first: bool| (
    graph.run_sounds_at(at, RATE, beat, false, first),
    graph.run_notes(beat, None),
  );

  // the playlist: one clip of the drums and one of the long pattern
  let (mut graph, _) = big_graph(|big| vec![(big.drums, 0, 0), (big.long, 0, 1)]);
  let (times, scheduled) = time_slices(&mut graph, &engine, 16, 100, playlist);
  report("slice playlist, 2 clips, first 4 bars", &times, scheduled);

  // and with the playlist full of clips: the drums and the short pattern
  // end to end, under the long pattern
  let (mut graph, _) = big_graph(|big| {
    let mut clips = vec![(big.long, 0, 1)];
    for n in 0..LONG_BARS / 4 {
      clips.push((big.drums, n * 4, 0));
      clips.push((big.short, n * 4, 2));
    }
    clips
  });
  let (times, scheduled) = time_slices(&mut graph, &engine, LONG_BARS * 4, 10, playlist);
  report(&format!("slice playlist, {} clips, {} bars", graph.pattern_nodes.len(), LONG_BARS), &times, scheduled);

  // a pattern playing on its own
  let (mut graph, big) = big_graph(|_| vec![]);
  let alone = |id: u64| move |graph: &daw::AudioGraph<'static>, _: Duration, beat: Duration, first: bool| (
    graph.pattern_sounds(id, beat, false, first),
    graph.pattern_notes(id, beat),
  );
  let (times, scheduled) = time_slices(&mut graph, &engine, 16, 100, alone(big.drums));
  report("slice pattern alone, drums (4 bars)", &times, scheduled);
  let (times, scheduled) = time_slices(&mut graph, &engine, LONG_BARS * 4, 10, alone(big.long));
  report(&format!("slice pattern alone, long ({} bars)", LONG_BARS), &times, scheduled);
}

// the bytes held on the heap, for perf_history. (a test build's allocator:
// the app's is the system's, as it is)
static HELD: AtomicIsize = AtomicIsize::new(0);

struct Counting;

unsafe impl GlobalAlloc for Counting {
  unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
    HELD.fetch_add(layout.size() as isize, Ordering::Relaxed);
    System.alloc(layout)
  }

  unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
    HELD.fetch_add(layout.size() as isize, Ordering::Relaxed);
    System.alloc_zeroed(layout)
  }

  unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
    HELD.fetch_add(size as isize - layout.size() as isize, Ordering::Relaxed);
    System.realloc(ptr, layout, size)
  }

  unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
    HELD.fetch_sub(layout.size() as isize, Ordering::Relaxed);
    System.dealloc(ptr, layout)
  }
}

#[global_allocator]
static COUNTING: Counting = Counting;

fn held() -> isize {
  HELD.load(Ordering::Relaxed)
}

#[test]
#[ignore]
fn perf_history() {
  let mut seq = daw::Sequencer::default();
  let big = big_project(&mut seq);
  let rows = big.samples.clone();
  let drum_steps = seq.pattern(big.drums).unwrap().length;
  // the n-th step edit: a different step cleared each time
  let edit = |seq: &mut daw::Sequencer, n: u32| {
    let (row, step) = (rows[n as usize % rows.len()], (n / rows.len() as u32) % drum_steps);
    seq.set_step(big.drums, row, step, false).unwrap();
  };

  const N: u32 = 200;
  let time = |f: &mut dyn FnMut(u32)| -> Vec<Duration> {
    (0..N).map(|n| {
      let start = Instant::now();
      f(n);
      start.elapsed()
    }).collect()
  };

  let mut copies = Vec::new();
  report("history: copy the sequencer", &time(&mut |_| copies.push(seq.clone())), 0);
  drop(copies);
  let same = seq.clone();
  report("history: same_as, unchanged", &time(&mut |_| { black_box(seq.same_as(&same)); }), 0);
  let mut changed = seq.clone();
  edit(&mut changed, 0);
  report("history: same_as, one step changed", &time(&mut |_| { black_box(seq.same_as(&changed)); }), 0);

  // a step edit as the Sequencer tab's set_pattern_step records it (see
  // history::sequencer_edit): the sequencer copied before and after, and the
  // step kept, which compares the two
  let mut history = History::default();
  let step_edit = |seq: &mut daw::Sequencer, history: &mut History, n: u32| {
    let before = seq.clone();
    edit(seq, n);
    history.push(Step::new().with_sequencer(before, seq.clone()));
  };
  let mut work = seq.clone();
  report("history: one step edit's undo step", &time(&mut |n| step_edit(&mut work, &mut history, n)), 0);

  // what the history holds: 100 steps (its limit) of step edits. the lit
  // steps are notes now, so they count as notes
  let count = |id: u64| -> usize { seq.pattern(id).unwrap().notes.values().map(|notes| notes.len()).sum() };
  let notes = count(big.short) + count(big.long);
  let steps = count(big.drums);
  let payload = (notes + steps) * std::mem::size_of::<daw::PatternNote>();

  let before = held();
  let copy = seq.clone();
  let copy_bytes = held() - before;
  drop(copy);

  let before = held();
  let mut history = History::default();
  let mut work = seq.clone();
  for n in 0..crate::history::LIMIT as u32 {
    step_edit(&mut work, &mut history, n);
  }
  drop(work);
  let history_bytes = held() - before;
  black_box(&history);

  println!(
    "perf history: memory: {} synth notes and {} step notes ({} KiB as notes); one copy of the sequencer holds {} KiB; \
     100 step edits' history holds {} KiB, {:.0} times the notes",
    notes, steps, payload / 1024, copy_bytes / 1024, history_bytes / 1024, history_bytes as f64 / payload as f64,
  );
}
