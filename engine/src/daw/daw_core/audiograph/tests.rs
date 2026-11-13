use super::*;
use crate::daw::{self};
use std::time::Instant;

fn ms(n: u64) -> Duration { Duration::from_millis(n) }

#[test]
fn test_group_move_keeps_spacing() {
  let placed = [(1, ms(1_000), 0), (2, ms(1_250), 1), (3, ms(3_000), 1)];

  // clip 2 grabbed and moved 500ms later, one track down
  let moved = group_move(&placed, 2, ms(1_750), 2, ms(8_000), 4).unwrap();
  assert_eq!(moved, vec![(1, ms(1_500), 1), (2, ms(1_750), 2), (3, ms(3_500), 2)]);

  // and earlier, one track up
  let moved = group_move(&placed, 1, ms(400), 0, ms(8_000), 4).unwrap();
  assert_eq!(moved, vec![(1, ms(400), 0), (2, ms(650), 1), (3, ms(2_400), 1)]);
}

#[test]
fn test_group_move_stops_at_the_playlist_edges() {
  let placed = [(1, ms(1_000), 0), (2, ms(3_000), 1)];

  // dragged past the start: the first clip stops at 0
  let moved = group_move(&placed, 2, ms(500), 1, ms(8_000), 4).unwrap();
  assert_eq!(moved, vec![(1, ms(0), 0), (2, ms(2_000), 1)]);

  // dragged past the end: the last clip stops at the end
  let moved = group_move(&placed, 1, ms(7_500), 0, ms(8_000), 4).unwrap();
  assert_eq!(moved, vec![(1, ms(6_000), 0), (2, ms(8_000), 1)]);
}

#[test]
fn test_group_move_stops_at_the_first_track_and_the_new_track_area() {
  let placed = [(1, ms(0), 1), (2, ms(500), 3)];

  // up past track 0: the top clip stops on track 0
  let moved = group_move(&placed, 2, ms(500), 0, ms(8_000), 4).unwrap();
  assert_eq!(moved, vec![(1, ms(0), 0), (2, ms(500), 2)]);

  // down past the new-track area (track 4): the bottom clip stops there
  let moved = group_move(&placed, 1, ms(0), 6, ms(8_000), 4).unwrap();
  assert_eq!(moved, vec![(1, ms(0), 2), (2, ms(500), 4)]);
}

#[test]
fn test_group_move_needs_the_anchor() {
  assert!(group_move(&[(1, ms(0), 0)], 9, ms(0), 0, ms(8_000), 4).is_err());
}

#[test]
fn test_move_nodes_snaps_only_the_anchor() {
  // 120bpm: a beat is 500ms
  let mut graph = graph_with_ticks(&[1_000, 1_130]);
  let ids: Vec<u64> = graph.nodes.iter().map(|n| n.id).collect();
  let (a, b) = (ids[0], ids[1]);

  // a dropped at 1_620ms snaps to 1_500ms; b keeps its 130ms gap, off the grid
  let moved = graph.move_nodes(&ids, a, ms(1_620), 0, ms(8_000), Some(daw::timing::Note::QuarterNote)).unwrap();
  assert_eq!(moved, vec![(a, ms(1_500), 0), (b, ms(1_630), 1)]);
  assert_eq!(graph.get_node_by_id(b).unwrap().start_offset, ms(1_630));

  // without snap the anchor lands exactly where it was dropped
  graph.move_nodes(&ids, a, ms(1_620), 1, ms(8_000), None::<daw::timing::Note>).unwrap();
  assert_eq!(graph.get_node_by_id(a).unwrap().start_offset, ms(1_620));
  assert_eq!(graph.get_node_by_id(b).unwrap().track_number, 2);

  // an unknown id moves nothing
  assert!(graph.move_nodes(&[a, 9_999], a, ms(0), 0, ms(8_000), None::<daw::timing::Note>).is_err());
  assert_eq!(graph.get_node_by_id(a).unwrap().start_offset, ms(1_620));
}

#[test]
fn test_node_initialization() {
  let id = 1;
  let sample_path = daw::METRONOME_TICK_PATH.to_string();
  let start_offset = Duration::from_millis(0);
  let track_number = 0;
  let _sample_rate = 44_100;

  let node = AudioNode::new(id, sample_path, start_offset, track_number).unwrap();

  assert_eq!(node.start_offset, Duration::from_millis(0));
  assert_eq!(node.duration().as_millis(), daw::METRONOME_TICK_SOURCE.convert_samples().total_duration().unwrap().as_millis());
  assert_eq!(node.channels, daw::METRONOME_TICK_SOURCE.convert_samples().channels());
  assert_eq!(node.sample_path, daw::METRONOME_TICK_PATH);
  assert_eq!(node.sample_rate, 44_100);
  assert_eq!(node.track_number, track_number);
}

#[test]
fn test_audiograph_initialization() {
  let id = 1;
  let sample_path = daw::METRONOME_TICK_PATH.to_string();
  let start_offset = Duration::from_millis(400);
  let track_number = 0;
  let sample_rate = 44_100;

  let node = AudioNode::new(id, sample_path, start_offset, track_number).unwrap();
  let node_dur = node.duration().clone();

  let tempo = 120f32;

  let mut audiograph = AudioGraph::new(sample_rate, tempo);

  assert_eq!(audiograph.len(), 0);

  audiograph.add_node(node);
  
  assert_eq!(audiograph.len(), 1);
  assert_eq!(audiograph.nodes[0].id, 1);
  assert_eq!(audiograph.duration(), start_offset + node_dur);
}

// render a slice's sounds through an engine, as a run would play them,
// returning the left channel
fn render_run(sounds: Vec<SliceSound>, rate: u32, frames: usize) -> Vec<f32> {
  let (engine, mut renderer) = daw::Engine::new(rate);
  let voices = sounds
    .into_iter()
    .map(|(delay, source, route)| engine.boxed_voice(source, Some(1), engine.frames_in(delay), route))
    .collect();
  engine.start_run(1, daw::Mixer::default().master(), voices).unwrap();

  let mut out = vec![0f32; frames * 2];
  renderer.render(&mut out, Instant::now(), Duration::ZERO);
  out.chunks(2).map(|f| f[0]).collect()
}

fn first_sound(samples: &[f32]) -> Option<usize> {
  samples.iter().position(|&s| s != 0.)
}

#[test]
fn test_audiograph_run_plays_clips_on_their_frame() {
  let rate = 44_100;
  let mut audiograph = graph_with_ticks(&[650]);
  audiograph.init();
  let second = Duration::from_secs(1);

  // where the tick's sound starts, in the sample itself
  let tick = render_run(vec![metronome_tick()], rate, 2_000);
  let into_tick = first_sound(&tick).expect("the tick is silent");

  // the clip starts exactly 650ms (28,665 frames) into the run
  let clip = render_run(audiograph.run_sounds(second, false, false), rate, rate as usize);
  assert_eq!(first_sound(&clip), Some(28_665 + into_tick));

  // with the metronome on, its tick starts the slice
  let with_tick = render_run(audiograph.run_sounds(second, true, false), rate, rate as usize);
  assert_eq!(first_sound(&with_tick), Some(into_tick));

  // the metronome doesn't play without a run of the graph
  audiograph.pause();
  assert!(audiograph.run_sounds(second, true, false).is_empty());
}

#[test]
fn test_clips_keep_their_beat_when_the_tempo_changes() {
  let rate = 44_100;
  // a tick on beat 2 at 120bpm (1s in), and a pattern clip on beat 4
  let mut audiograph = graph_with_ticks(&[1_000]);
  audiograph.pattern_nodes.push(PatternNode::new(99, 0, ms(2_000), 1, 16));
  let played = audiograph.nodes[0].duration();

  // at 60bpm, beat 2 is 2s in, and beat 4 is 4s in
  audiograph.set_tempo(60.);
  assert_eq!(audiograph.nodes[0].start_offset, ms(2_000));
  assert_eq!(audiograph.pattern_nodes[0].start_offset, ms(4_000));
  // the clip plays for as long as it did
  assert_eq!(audiograph.nodes[0].duration(), played);

  // and it's heard there: its first frame is 2s (88,200 frames) into a run
  audiograph.init();
  let tick = render_run(vec![metronome_tick()], rate, 2_000);
  let into_tick = first_sound(&tick).expect("the tick is silent");
  let run = render_run(audiograph.run_sounds(Duration::from_secs(3), false, false), rate, 3 * rate as usize);
  assert_eq!(first_sound(&run), Some(88_200 + into_tick));
}

#[test]
fn test_a_clip_on_beat_8_is_heard_on_beat_8_at_another_tempo() {
  let rate = 44_100;
  // beat 8 at 120bpm is 4s in
  let mut audiograph = graph_with_ticks(&[4_000]);
  audiograph.init();
  let tick = render_run(vec![metronome_tick()], rate, 2_000);
  let into_tick = first_sound(&tick).expect("the tick is silent");

  // at 90bpm a beat is two thirds of a second, which isn't a whole number
  // of anything: beat 8 is 5.3333s in, frame 235,200
  audiograph.set_tempo(90.);
  let run = render_run(audiograph.run_sounds(Duration::from_secs(6), false, false), rate, 6 * rate as usize);
  assert_eq!(first_sound(&run), Some(235_200 + into_tick));

  // clips keep their order (the scheduler relies on it)
  let mut audiograph = graph_with_ticks(&[900, 0, 200, 4_000]);
  audiograph.set_tempo(77.);
  let starts: Vec<Duration> = audiograph.nodes.iter().map(|n| n.start_offset).collect();
  let mut sorted = starts.clone();
  sorted.sort();
  assert_eq!(starts, sorted);
}

#[test]
fn test_a_one_second_clip_is_a_second_long_at_any_tempo() {
  let path = one_second_at_48k();
  let mut audiograph = AudioGraph::new(44_100, 120.);
  let id = audiograph.construct_and_add_node(path, ms(1_000), 0).unwrap();
  assert_eq!(audiograph.get_node_by_id(id).unwrap().duration(), Duration::from_secs(1));

  audiograph.set_tempo(90.);
  assert_eq!(audiograph.get_node_by_id(id).unwrap().duration(), Duration::from_secs(1));
  audiograph.set_tempo(200.);
  assert_eq!(audiograph.get_node_by_id(id).unwrap().duration(), Duration::from_secs(1));
}

#[test]
fn test_snapping_is_exact_far_along_the_playlist() {
  // at 90bpm a beat is 666.666...ms. beat 200 is 133.33s in, to the
  // nanosecond (not 137 microseconds early, as snapping in whole
  // microseconds made it)
  let audiograph = AudioGraph::new(44_100, 90.);
  let beat = audiograph.beat_interval();
  let near = beat * 200 + Duration::from_millis(40);
  let snapped = audiograph.find_nearest_offset_subdivision(near, daw::timing::Note::QuarterNote);
  assert_eq!(snapped, beat * 200);
}

#[test]
fn test_audiograph_buffer_slice_with_silence() {
  let sample_rate = 44_100;
  let tempo = 120f32;

  let audiograph = AudioGraph::new(sample_rate, tempo);
  let runtime = Duration::from_secs(1);

  let (_controller, mixer) = audiograph.buffer_slice(runtime).unwrap();

  let mut zero_samples = Vec::<f32>::new();
  let mut nonzero_samples = Vec::<f32>::new();
  mixer.for_each(|x| {
    if x == 0f32 {
      zero_samples.push(x);
    } else {
      nonzero_samples.push(x);
    }
  });

  // a second of stereo silence. rodio rounds the time per sample to whole
  // nanoseconds, so it runs a few samples over
  let expected = 44_100 * 2;
  assert!(
    zero_samples.len() >= expected && zero_samples.len() < expected + 10,
    "expected about {} samples of silence, got {}", expected, zero_samples.len());
  assert_eq!(nonzero_samples.len(), 0);
}

#[test]
fn test_audiograph_buffer_slice_with_node() {
  let id = 1;
  let sample_path = daw::METRONOME_TICK_PATH.to_string();
  let start_offset = Duration::from_millis(650);
  let track_number = 0;
  let sample_rate = 44_100;

  let node = AudioNode::new(id, sample_path, start_offset, track_number).unwrap();

  let tempo = 120f32;

  let mut audiograph = AudioGraph::new(sample_rate, tempo);
  let buf = daw::SampleBuffer::load(node.sample_path()).unwrap();

  audiograph.add_node(node);
  
  let runtime = Duration::from_secs(1);

  let (_controller, mixer) = audiograph.buffer_slice(runtime).unwrap();

  let mut nonzero_samples = Vec::<f32>::new();
  mixer.for_each(|x| {
    if x != 0f32 {
      nonzero_samples.push(x);
    }
  });

  let mut node_samples = Vec::<f32>::new();
  buf.convert_samples().for_each(|x| {
    if x != 0f32 {
      node_samples.push(x);
    }
  });

  // (this was * 4 while the slice also held a metronome tick, which is the
  // same sample. the metronome now plays outside the mixer, see run_for)
  assert_eq!(node_samples.len() * 2, nonzero_samples.len());
}

#[test]
fn test_audiograph_beat_interval() {
  let mut audiograph = AudioGraph::new(44_100, 120f32);
  assert_eq!(audiograph.beat_interval().as_millis(), 500);

  audiograph.set_tempo(140f32);
  assert_eq!(audiograph.beat_interval().as_millis(), 428);

  audiograph.set_tempo(60f32);
  assert_eq!(audiograph.beat_interval().as_millis(), 1_000);

  assert_eq!(audiograph.beat_interval(), daw::timing::beat_interval_from_tempo(audiograph.tempo()));
}

#[test]
fn test_bad_sample_returns_error() {
  let mut audiograph = AudioGraph::new(44_100, 120f32);

  // missing file
  let res = audiograph.construct_and_add_node(
    "assets/does-not-exist.wav".to_string(), Duration::ZERO, 0);
  assert!(res.unwrap_err().contains("does-not-exist.wav"));

  // todo: also test a file that exists but is not audio. currently
  // rodio 0.15 falls back to minimp3 -> slice-deque, which hits UB
  // (aborts in debug) on unrecognised input, so it can't be tested yet

  // failed loads add nothing, don't use up ids,
  // and leave the graph usable
  assert_eq!(audiograph.len(), 0);
  let id = audiograph.construct_and_add_node(
    daw::METRONOME_TICK_PATH.to_string(), Duration::ZERO, 0).unwrap();
  assert_eq!(id, 0);
}

// a second of stereo noise at 48kHz, written to a temporary file
fn one_second_at_48k() -> String {
  let path = std::env::temp_dir()
    .join(format!("dawesome-48k-{}.wav", std::process::id()));
  let spec = hound::WavSpec {
    channels: 2,
    sample_rate: 48_000,
    bits_per_sample: 16,
    sample_format: hound::SampleFormat::Int,
  };
  let mut writer = hound::WavWriter::create(&path, spec).unwrap();
  for i in 0..48_000 * 2 {
    writer.write_sample(((i * 7919) % 2000 - 1000) as i16).unwrap();
  }
  writer.finalize().unwrap();
  path.to_string_lossy().into_owned()
}

#[test]
fn test_sample_lengths_use_the_file_rate() {
  let path = one_second_at_48k();
  let second = |d: Duration| (d.as_secs_f64() - 1.).abs() < 1e-3;

  // the graph runs at 44.1kHz, which used to make this 1.088s
  let mut audiograph = AudioGraph::new(44_100, 120f32);
  let id = audiograph.construct_and_add_node(path.clone(), Duration::ZERO, 0).unwrap();
  let node = audiograph.get_node_by_id(id).unwrap();
  assert!(second(node.sample_duration()), "clip: {:?}", node.sample_duration());
  assert_eq!(node.sample_rate, 48_000);

  let (dur, _) = preview_sample(&path).unwrap();
  assert!(second(dur), "preview: {:?}", dur);

  let mut seq = daw::Sequencer::default();
  let row = seq.add_row(&path).unwrap();
  let row_dur = seq.row(row).unwrap().sample_instrument().unwrap().played_len(daw::DEFAULT_PITCH);
  assert!(second(row_dur), "sequencer row: {:?}", row_dur);

  let _ = std::fs::remove_file(&path);
}

#[test]
fn test_preview_sample_matches_a_node() {
  let path = daw::METRONOME_TICK_PATH;
  let (dur, waveform) = preview_sample(path).unwrap();
  let node = AudioNode::new(0, path.to_string(), Duration::ZERO, 0).unwrap();

  assert_eq!(dur, node.sample_duration());
  assert_eq!(waveform.as_str(), &*node.waveform);
  assert!(preview_sample("does/not/exist.wav").is_err());
}

#[test]
fn test_track_count_reaches_the_highest_track() {
  let path = daw::METRONOME_TICK_PATH.to_string();
  let mut audiograph = AudioGraph::new(44_100, 120.);
  let default = daw::defaults::NUM_OF_TRACKS;

  assert_eq!(audiograph.track_count(), default);

  // two tracks used, but one is past the default number of tracks
  audiograph.construct_and_add_node(path.clone(), Duration::ZERO, 0).unwrap();
  audiograph.construct_and_add_node(path.clone(), Duration::ZERO, default + 1).unwrap();
  assert_eq!(audiograph.track_count(), default + 2);

  audiograph.clear();
  assert_eq!(audiograph.track_count(), default);
}

#[test]
fn test_node_settings_change_its_length() {
  let path = daw::METRONOME_TICK_PATH;
  let mut node = AudioNode::new(0, path.to_string(), Duration::ZERO, 0).unwrap();
  let sample_dur = node.sample_duration();
  assert_eq!(node.duration(), sample_dur);

  // an octave up plays twice as fast, so half as long
  let kept = node.set_settings(daw::ClipSettings {
    pitch: 12.,
    attack_ms: 1_000_000.,
    ..Default::default()
  });
  assert_eq!(node.duration(), sample_dur.div_f32(2.));
  assert_eq!(node.sample_duration(), sample_dur);
  // the fade was kept within the clip
  assert!((kept.attack_ms - node.duration().as_secs_f32() * 1000.).abs() < 0.01);

  // where the graph ends follows the clip's played length
  let mut audiograph = AudioGraph::new(44_100, 120.);
  audiograph.add_node(node);
  assert_eq!(audiograph.duration(), sample_dur.div_f32(2.));
}

#[test]
fn test_waveform_rejects_empty_and_tiny_samples() {
  assert!(daw::calc_waveform_from_samples(&[], 2, 44_100).is_none());
  assert!(daw::calc_waveform_from_samples(&[1, 2, 3], 0, 44_100).is_none());
  assert!(daw::calc_waveform_from_samples(&[1, 2, 3], 1, 0).is_none());
  // a few samples (< 0.25ms) used to divide by zero
  assert!(daw::calc_waveform_from_samples(&[100, -100, 50, -50], 1, 44_100).is_some());
}

#[test]
fn test_silence_has_a_flat_waveform() {
  // used to panic: silence smoothed and normalised by dividing by zero
  let levels = daw::calc_waveform_from_samples(&vec![0; 48_000], 1, 48_000).unwrap();
  assert!(!levels.is_empty());
  assert!(levels.iter().all(|&level| level == 0.));
}

#[test]
fn test_audiograph_node_ids_not_reused() {
  let mut audiograph = AudioGraph::new(44_100, 120f32);
  let path = || daw::METRONOME_TICK_PATH.to_string();

  let kick = audiograph.construct_and_add_node(path(), Duration::from_millis(0), 0).unwrap();
  let snare = audiograph.construct_and_add_node(path(), Duration::from_millis(500), 1).unwrap();
  let hat = audiograph.construct_and_add_node(path(), Duration::from_millis(250), 2).unwrap();
  assert_eq!((kick, snare, hat), (0, 1, 2));

  // removing a node must not free its id for reuse
  audiograph.remove_node(kick).unwrap();
  let clap = audiograph.construct_and_add_node(path(), Duration::from_millis(750), 3).unwrap();
  assert_eq!(clap, 3);

  // every remaining id resolves to exactly one node
  for id in [snare, hat, clap] {
    assert_eq!(audiograph.nodes.iter().filter(|n| n.id == id).count(), 1);
  }
  assert_eq!(audiograph.get_node_by_id(hat).unwrap().track_number, 2);
  assert_eq!(audiograph.get_node_by_id(clap).unwrap().track_number, 3);

  // externally assigned ids push the counter forward
  let node = AudioNode::new(10, path(), Duration::ZERO, 0).unwrap();
  audiograph.add_node(node);
  let next = audiograph.construct_and_add_node(path(), Duration::ZERO, 0).unwrap();
  assert_eq!(next, 11);
}

// build a graph with one metronome tick per offset (ms), each on its own track
fn graph_with_ticks(offsets_ms: &[u64]) -> AudioGraph<'static> {
  let mut audiograph = AudioGraph::new(44_100, 120f32);
  for (track, ms) in offsets_ms.iter().enumerate() {
    audiograph.construct_and_add_node(
      daw::METRONOME_TICK_PATH.to_string(),
      Duration::from_millis(*ms),
      track as u32).unwrap();
  }

  audiograph
}

#[test]
fn test_nodes_in_window() {
  let audiograph = graph_with_ticks(&[0, 0, 200, 500, 900]);
  let beat = audiograph.beat_interval(); // 500ms at 120bpm
  let starts = |from_ms: u64| -> Vec<u128> {
    audiograph
      .nodes_in_window(Duration::from_millis(from_ms), beat)
      .map(|n| n.start_offset.as_millis())
      .collect()
  };

  // all nodes in the window, including simultaneous ones
  assert_eq!(starts(0), vec![0, 0, 200]);
  // a node exactly at the window end belongs to the next window only
  assert_eq!(starts(500), vec![500, 900]);
  assert_eq!(starts(1_000), Vec::<u128>::new());
}

#[test]
fn test_end_is_where_the_last_clip_finishes() {
  assert_eq!(AudioGraph::new(44_100, 120f32).end(), Duration::ZERO);

  let audiograph = graph_with_ticks(&[900, 0, 200]);
  let tick = audiograph.nodes[0].duration();
  assert_eq!(audiograph.end(), Duration::from_millis(900) + tick);
}

#[test]
fn test_nodes_sounding_at() {
  let audiograph = graph_with_ticks(&[0, 200, 500]);
  let tick = audiograph.nodes[0].duration();
  let sounding = |at: Duration| -> Vec<u128> {
    audiograph
      .nodes_sounding_at(at)
      .map(|n| n.start_offset.as_millis())
      .collect()
  };

  // a node starting right at the playhead isn't resumed, it's scheduled
  // the usual way
  assert!(!sounding(Duration::from_millis(200)).contains(&200));
  // partway through the first tick
  assert_eq!(sounding(Duration::from_millis(1)), vec![0]);
  // after every tick has ended
  assert_eq!(sounding(Duration::from_millis(500) + tick), Vec::<u128>::new());
}

#[test]
fn test_resumed_slice_plays_the_rest_of_a_sounding_clip() {
  let energy = |at_ms: u64, resume: bool| -> f64 {
    let mut audiograph = graph_with_ticks(&[0]);
    audiograph.set_current_offset(Some(Duration::from_millis(at_ms)));
    let (_controller, mixer) = audiograph.slice_mix(audiograph.beat_interval(), resume);
    mixer.map(|x| x.abs() as f64).sum()
  };

  let tick = graph_with_ticks(&[0]).nodes[0].duration().as_millis() as u64;
  assert!(tick > 2, "the tick sample is too short to resume inside");
  let mid = tick / 2;

  // not resuming: the clip started before the slice, so it's silent
  assert_eq!(energy(mid, false), 0.);
  // resuming plays its second half, which is less than the whole tick
  let rest = energy(mid, true);
  assert!(rest > 0.);
  assert!(rest < energy(0, false));
}

#[test]
fn test_audiograph_buffer_slice_plays_every_node_in_window() {
  // total signal energy of the first beat, relative to a single tick
  let energy = |offsets_ms: &[u64]| -> f64 {
    let audiograph = graph_with_ticks(offsets_ms);
    let (_controller, mixer) = audiograph
      .buffer_slice(audiograph.beat_interval())
      .unwrap();
    mixer.map(|x| x.abs() as f64).sum()
  };
  let one = energy(&[0]);
  let ratio = |offsets_ms: &[u64]| (energy(offsets_ms) / one * 100.).round() / 100.;

  // previously each of these mixed only the first node (1.0)
  assert_eq!(ratio(&[0, 0, 0]), 3.);
  assert_eq!(ratio(&[0, 200, 400]), 3.);
  // node at 900ms is in the next beat
  assert_eq!(ratio(&[0, 200, 400, 900]), 3.);
}

#[test]
fn test_audiograph_buffer_set_functions() {
  let sample_rate = 44_100;
  let tempo = 120f32;

  let mut audiograph = AudioGraph::new(sample_rate, tempo);

  assert_eq!(audiograph.current_offset.unwrap().as_millis(), 0);
  audiograph.set_current_offset(Some(Duration::from_secs(2)));
  assert_eq!(audiograph.current_offset.unwrap().as_millis(), 2_000);

  assert_eq!(audiograph.tempo(), tempo);
  audiograph.set_tempo(75f32);
  assert_eq!(audiograph.tempo(), 75f32);
}

#[test]
fn test_audiograph_node_functions() {
  let id = 100;
  let sample_path = daw::METRONOME_TICK_PATH.to_string();
  let start_offset = Duration::from_millis(350);
  let sample_rate = 44_100;

  let node = AudioNode::new(
    id,
    sample_path,
    start_offset,
    0).unwrap();

  let tempo = 120f32;

  let mut audiograph = AudioGraph::new(sample_rate, tempo);

  audiograph.add_node(node);
  let id2 = audiograph.construct_and_add_node(
    "assets/assets_66-bd-01.wav".to_string(),
    Duration::from_millis(100),
    1).unwrap();
  let id3 = audiograph.construct_and_add_node_with_snap(
    "assets/assets_66-sd-01.wav".to_string(),
    Duration::from_millis(600),
    2,
    daw::timing::SixteenthNote::new()).unwrap();
  
  assert_eq!(audiograph.len(), 3);
  // clips on tracks 0-2 fit in the default number of tracks
  assert_eq!(audiograph.track_count(), daw::defaults::NUM_OF_TRACKS);
  assert_eq!(audiograph.get_node_by_id(id2).unwrap().start_offset.as_millis(), 100);

  // 600ms snapped to a 125ms grid (sixteenth notes at 120bpm) = 625ms
  assert_eq!(audiograph.get_node_by_id(id3).unwrap().start_offset.as_millis(), 625);
  
  let target = audiograph.get_node_by_id(id).unwrap();

  assert_eq!(target.id, id);
  assert_eq!(target.track_number, 0);
  assert_eq!(target.start_offset.as_millis(), start_offset.as_millis());

  audiograph.move_node(
    id, 
    Duration::from_millis(0),
    2).unwrap();
  let target = &audiograph.nodes[0];

  assert_eq!(target.id, id);
  assert_eq!(target.track_number, 2);
  assert_eq!(target.start_offset.as_millis(), 0);

  audiograph.move_node_with_snap(
    id, 
    Duration::from_millis(12),
    1,
  daw::timing::EighthNote::new()).unwrap();
  let target = &audiograph.nodes[0];

  assert_eq!(target.id, id);
  assert_eq!(target.track_number, 1);
  assert_eq!(target.start_offset.as_millis(), 0);

  // unknown ids return an error instead of panicking
  assert!(audiograph.move_node(9_999, Duration::ZERO, 0).is_err());
  assert!(audiograph.move_node_with_snap(
    9_999, Duration::ZERO, 0, daw::timing::EighthNote::new()).is_err());
  assert!(audiograph.remove_node(9_999).is_err());
  assert_eq!(audiograph.len(), 3);
}

const KICK: &str = "assets/assets_66-bd-01.wav";

// a graph with one pattern (one bar, kick on beats 1 and 3), and its ids
fn graph_with_pattern() -> (AudioGraph<'static>, u64, u64) {
  let graph = AudioGraph::new(44_100, 120.);
  let (row, pattern) = {
    let mut seq = graph.sequencer.lock().unwrap();
    let row = seq.add_row(KICK).unwrap();
    let pattern = seq.add_pattern(1, 4).unwrap();
    seq.set_pattern_step(pattern, row, 0, true, 4).unwrap();
    seq.set_pattern_step(pattern, row, 8, true, 4).unwrap();
    (row, pattern)
  };
  (graph, row, pattern)
}

#[test]
fn test_pattern_nodes_share_ids_and_snap() {
  let (mut graph, _, pattern) = graph_with_pattern();
  let clip = graph.construct_and_add_node(KICK.to_string(), ms(0), 0).unwrap();
  let placed = graph.add_pattern_node(pattern, ms(1_100), 2, Some(daw::timing::QuarterNote::new())).unwrap();

  assert_ne!(placed, clip);
  assert_eq!(graph.placement(placed), Some((ms(1_000), 2)));
  assert!(graph.has_clips());
  // a pattern that doesn't exist can't be placed
  assert!(graph.add_pattern_node::<daw::timing::QuarterNote>(99, ms(0), 0, None).is_err());
}

#[test]
fn test_pattern_nodes_move_and_remove_like_clips() {
  let (mut graph, _, pattern) = graph_with_pattern();
  let clip = graph.construct_and_add_node(KICK.to_string(), ms(0), 0).unwrap();
  let placed = graph.add_pattern_node::<daw::timing::QuarterNote>(pattern, ms(500), 1, None).unwrap();

  graph.move_node(placed, ms(2_000), 3).unwrap();
  assert_eq!(graph.placement(placed), Some((ms(2_000), 3)));

  // moved in a group with a sample clip
  let moved = graph.move_nodes::<daw::timing::QuarterNote>(
    &[clip, placed], clip, ms(500), 1, ms(8_000), None).unwrap();
  assert_eq!(moved, vec![(clip, ms(500), 1), (placed, ms(2_500), 4)]);

  graph.remove_node(placed).unwrap();
  assert!(graph.get_pattern_node_by_id(placed).is_none());
  assert!(graph.remove_node(placed).is_err());
}

#[test]
fn test_pattern_nodes_count_toward_tracks_and_the_end() {
  let (mut graph, _, pattern) = graph_with_pattern();
  assert_eq!(graph.end(), ms(0));
  graph.add_pattern_node::<daw::timing::QuarterNote>(pattern, ms(3_000), 6, None).unwrap();

  assert_eq!(graph.track_count(), 7);
  // a one bar pattern is 2s at 120bpm
  assert_eq!(graph.end(), ms(5_000));
  // a longer pattern leaves its clips as long as they were. a new clip of
  // it is as long as it is now
  graph.sequencer.lock().unwrap().set_pattern_bars(pattern, 2, 4).unwrap();
  assert_eq!(graph.end(), ms(5_000));
  graph.add_pattern_node::<daw::timing::QuarterNote>(pattern, ms(3_000), 6, None).unwrap();
  assert_eq!(graph.end(), ms(7_000));
}

// a graph with a synth row (its plugin in slot 7, after a sample row) and a
// one bar pattern of its notes
fn graph_with_notes(notes: Vec<daw::PatternNote>) -> (AudioGraph<'static>, u64) {
  let graph = AudioGraph::new(44_100, 120.);
  let pattern = {
    let mut seq = graph.sequencer.lock().unwrap();
    seq.add_row("assets/assets_66-bd-01.wav").unwrap();
    let row = seq.add_synth_row(daw::Synth {
      id: 7, plugin_id: "vst3:synth".into(), path: String::new(), name: "Synth".into(),
      vendor: String::new(), volume: 1., pan: 0., muted: false,
    });
    let pattern = seq.add_pattern(1, 4).unwrap();
    seq.set_pattern_notes(pattern, row, notes).unwrap();
    pattern
  };
  (graph, pattern)
}

#[test]
fn test_a_note_still_sounding_at_the_loops_end_ends_there() {
  // a step is 125ms at 120bpm: 60 from 1.5s to 2s, 62 from 1.875s
  let note = |pitch, start, length| daw::PatternNote { pitch, start, length, velocity: 1. };
  let (mut graph, pattern) = graph_with_notes(vec![note(60, 12., 4.), note(62, 15., 1.)]);
  graph.add_pattern_node::<daw::timing::QuarterNote>(pattern, ms(0), 0, None).unwrap();
  graph.init();
  graph.set_current_offset(Some(ms(1_500)));
  let heard = |notes: Vec<SliceNote>| -> Vec<(u8, Duration, Duration)> {
    notes.into_iter().map(|n| (n.pitch, n.on, n.off)).collect()
  };

  // not looping: as the pattern has them
  assert_eq!(heard(graph.run_notes(ms(500), None)), vec![(60, ms(0), ms(500)), (62, ms(375), ms(500))]);
  // looping at 1.75s (a loop held while recording, say): 60 ends at the
  // loop's end, where the next time round starts, and 62 is past it
  assert_eq!(heard(graph.run_notes(ms(500), Some(ms(1_750)))), vec![(60, ms(0), ms(250))]);
  assert!(graph.run_notes(ms(500), None).iter().all(|n| n.synth == 7 && n.route.as_ref().map(|r| r.number) == Some(0)));
}

#[test]
fn test_the_synths_each_track_plays() {
  let note = |pitch, start, length| daw::PatternNote { pitch, start, length, velocity: 1. };
  let (mut graph, pattern) = graph_with_notes(vec![note(60, 20., 4.)]);
  let empty = graph.sequencer.lock().unwrap().add_pattern(1, 4).unwrap();
  graph.add_pattern_node::<daw::timing::QuarterNote>(pattern, ms(0), 2, None).unwrap();
  graph.add_pattern_node::<daw::timing::QuarterNote>(pattern, ms(4_000), 5, None).unwrap();
  graph.add_pattern_node::<daw::timing::QuarterNote>(empty, ms(0), 3, None).unwrap();
  // (a note past the pattern's end still needs its copy: lengthening the
  // pattern brings it in)
  assert_eq!(graph.synth_tracks(), BTreeSet::from([(7, 2), (7, 5)]));

  // past the end, it isn't played: on its own, nor in the playlist
  graph.init();
  graph.set_current_offset(Some(ms(0)));
  assert!(graph.pattern_notes(pattern, ms(2_000)).is_empty());
  assert!(graph.run_notes(ms(8_000), None).is_empty());
  graph.sequencer.lock().unwrap().set_pattern_bars(pattern, 2, 4).unwrap();
  let solo = graph.pattern_notes(pattern, ms(4_000));
  assert_eq!(solo.len(), 1);
  assert!(solo[0].route.is_none() && solo[0].synth == 7);
}

#[test]
fn test_deleting_a_pattern_takes_its_clips() {
  let (mut graph, _, pattern) = graph_with_pattern();
  let other = graph.sequencer.lock().unwrap().add_pattern(1, 4).unwrap();
  let a = graph.add_pattern_node::<daw::timing::QuarterNote>(pattern, ms(0), 0, None).unwrap();
  let b = graph.add_pattern_node::<daw::timing::QuarterNote>(pattern, ms(2_000), 0, None).unwrap();
  let keep = graph.add_pattern_node::<daw::timing::QuarterNote>(other, ms(0), 1, None).unwrap();

  assert_eq!(graph.remove_pattern_nodes_of(pattern), vec![a, b]);
  assert_eq!(graph.pattern_nodes.len(), 1);
  assert_eq!(graph.pattern_nodes[0].id, keep);
}

#[test]
fn test_clear_takes_pattern_nodes() {
  let (mut graph, _, pattern) = graph_with_pattern();
  graph.add_pattern_node::<daw::timing::QuarterNote>(pattern, ms(0), 0, None).unwrap();
  graph.clear();
  assert!(!graph.has_clips());
}

#[test]
fn test_explode_tracks_on_an_empty_playlist() {
  // the first sample stays on the clip's track, the rest take the tracks
  // below it, past the tracks shown if need be
  assert_eq!(explode_tracks(&["kick", "snare", "hat"], 1, &[]), vec![1, 2, 3]);
  assert_eq!(explode_tracks(&["a", "b", "c", "d", "e"], 0, &[]), vec![0, 1, 2, 3, 4]);
}

#[test]
fn test_explode_tracks_go_below_the_clip() {
  // empty tracks above the clip are left alone: the samples take the empty
  // ones below it, then new ones
  let placed = [(0, None), (1, None), (4, None), (5, None), (6, None), (7, None), (9, None)];
  assert_eq!(explode_tracks(&["kick", "hat", "clap", "cymbal"], 7, &placed), vec![7, 8, 10, 11]);
}

#[test]
fn test_explode_tracks_reuses_a_samples_track() {
  // an earlier explode put kicks on 0 and snares on 5; another copy of the
  // pattern sits on 0
  let placed = [(0, Some("kick")), (5, Some("snare")), (0, None), (2, Some("bass"))];
  assert_eq!(explode_tracks(&["kick", "snare", "hat"], 0, &placed), vec![0, 5, 1]);
}

#[test]
fn test_explode_tracks_skips_shared_tracks() {
  // the clip's track holds another sample, so the first sample moves off it,
  // and a track with the sample and something else isn't its own
  let placed = [(0, Some("bass")), (1, Some("kick")), (1, Some("bass"))];
  assert_eq!(explode_tracks(&["kick"], 0, &placed), vec![2]);
}

#[test]
fn test_explode_tracks_tell_a_synth_row_from_a_sample() {
  // a clip of one synth row's notes is a sound of its own: the kick moves
  // off its track, and the synth goes onto it
  let placed = [(0, Some(daw::Sound::Synth(7)))];
  assert_eq!(explode_tracks(&[daw::Sound::Sample("kick"), daw::Sound::Synth(7)], 0, &placed), vec![1, 0]);
}

#[test]
fn test_explode_a_pattern_clip() {
  let (mut graph, kick, pattern) = graph_with_pattern();
  const SNARE: &str = "assets/assets_66-sd-01.wav";
  {
    let mut seq = graph.sequencer.lock().unwrap();
    let snare = seq.add_row(SNARE).unwrap();
    seq.set_pattern_step(pattern, snare, 4, true, 4).unwrap();
  }
  graph.mixer.set_name(0, "Drums");
  let clip = graph.add_pattern_node::<daw::timing::QuarterNote>(pattern, ms(2_000), 0, None).unwrap();

  let ids = graph.explode_pattern_node(clip).unwrap();
  assert!(graph.get_pattern_node_by_id(clip).is_none());
  // no sample clips: a pattern clip per sample
  assert!(graph.nodes.is_empty());
  assert_eq!(ids.len(), 2);

  let made: Vec<PatternNode> = ids.iter().map(|&id| graph.get_pattern_node_by_id(id).unwrap().clone()).collect();
  // where the clip was, the kick on its track, the snare on its own
  assert_eq!(made.iter().map(|n| (n.start_offset, n.track_number)).collect::<Vec<_>>(),
    vec![(ms(2_000), 0), (ms(2_000), 1)]);
  {
    let seq = graph.sequencer.lock().unwrap();
    assert_eq!(seq.sole_sound(made[0].pattern_id), Some(daw::Sound::Sample(KICK)));
    assert_eq!(seq.sole_sound(made[1].pattern_id), Some(daw::Sound::Sample(SNARE)));
    assert!(seq.pattern(made[0].pattern_id).unwrap().lit(kick, 8));
    // the source pattern is still there
    assert_eq!(seq.patterns().len(), 3);
  }

  // a named track keeps its name, an unnamed one takes its sample's
  assert_eq!(graph.mixer.name(0).as_deref(), Some("Drums"));
  assert_eq!(graph.mixer.name(1).as_deref(), Some("assets_66-sd-01"));

  // a second copy, exploded, reuses the patterns and the tracks
  let copy = graph.add_pattern_node::<daw::timing::QuarterNote>(pattern, ms(4_000), 3, None).unwrap();
  let again = graph.explode_pattern_node(copy).unwrap();
  let placed: Vec<(u64, u32)> = again.iter()
    .map(|&id| graph.get_pattern_node_by_id(id).unwrap())
    .map(|n| (n.pattern_id, n.track_number))
    .collect();
  assert_eq!(placed, vec![(made[0].pattern_id, 0), (made[1].pattern_id, 1)]);
  assert_eq!(graph.sequencer.lock().unwrap().patterns().len(), 3);

  // sample clips can't be exploded
  let sample = graph.construct_and_add_node(KICK.to_string(), ms(0), 0).unwrap();
  assert!(graph.explode_pattern_node(sample).is_err());
}

#[test]
fn test_explode_gives_a_synth_row_its_own_track_and_keeps_a_looping_clip_looping() {
  let (mut graph, kick, pattern) = graph_with_pattern();
  let keys = {
    let mut seq = graph.sequencer.lock().unwrap();
    let keys = seq.add_synth_row(daw::Synth {
      id: 7, plugin_id: "vst3:synth".into(), path: String::new(), name: "Keys".into(),
      vendor: String::new(), volume: 1., pan: 0., muted: false,
    });
    seq.set_pattern_notes(pattern, keys, vec![daw::PatternNote { pitch: 64, start: 2., length: 6., velocity: 1. }]).unwrap();
    keys
  };
  // round its one bar pattern two and a half times, from step 4
  let clip = graph.add_pattern_node::<daw::timing::QuarterNote>(pattern, ms(0), 2, None).unwrap();
  graph.set_pattern_window(clip, 4., Some(44.)).unwrap();

  let ids = graph.explode_pattern_node(clip).unwrap();
  let made: Vec<PatternNode> = ids.iter().map(|&id| graph.get_pattern_node_by_id(id).unwrap().clone()).collect();
  // the kick stays on the clip's track, the synth goes below it, named
  // after it, and each new clip loops as the clip did
  assert_eq!(made.iter().map(|n| n.track_number).collect::<Vec<_>>(), vec![2, 3]);
  assert_eq!(graph.mixer.name(3).as_deref(), Some("Keys"));
  assert!(made.iter().all(|n| (n.start, n.length) == (4., 40.)));
  {
    let seq = graph.sequencer.lock().unwrap();
    assert_eq!(seq.sole_sound(made[0].pattern_id), Some(daw::Sound::Sample(KICK)));
    assert_eq!(seq.sole_sound(made[1].pattern_id), Some(daw::Sound::Synth(keys)));
    assert!(seq.pattern(made[0].pattern_id).unwrap().lit(kick, 8));
  }
  // so the synth's notes are played on its track now
  assert_eq!(graph.synth_tracks(), BTreeSet::from([(7, 3)]));

  // another clip of the pattern, exploded, puts the synth on its track again
  let copy = graph.add_pattern_node::<daw::timing::QuarterNote>(pattern, ms(8_000), 0, None).unwrap();
  let again = graph.explode_pattern_node(copy).unwrap();
  let tracks: Vec<u32> = again.iter().map(|&id| graph.get_pattern_node_by_id(id).unwrap().track_number).collect();
  assert_eq!(tracks, vec![2, 3]);
}

#[test]
fn test_an_empty_pattern_clip_stays() {
  let mut graph = AudioGraph::new(44_100, 120.);
  let pattern = graph.sequencer.lock().unwrap().add_pattern(1, 4).unwrap();
  let clip = graph.add_pattern_node::<daw::timing::QuarterNote>(pattern, ms(0), 0, None).unwrap();
  assert!(graph.explode_pattern_node(clip).is_err());
  assert!(graph.get_pattern_node_by_id(clip).is_some());
}

// ---- clip windows: trimming and cutting ----

// noise at a file rate of its own, written to a temporary file
fn noise_wav(name: &str, rate: u32, channels: u16, frames: u32) -> String {
  let path = std::env::temp_dir().join(format!("dawesome-{}-{}.wav", name, std::process::id()));
  let spec = hound::WavSpec { channels, sample_rate: rate, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
  let mut writer = hound::WavWriter::create(&path, spec).unwrap();
  let mut seed = 99u32;
  for frame in 0..frames {
    for channel in 0..channels as u32 {
      seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
      let noise = ((seed >> 16) % 6_000) as f32 - 3_000.;
      let tone = (frame as f32 * (0.02 + channel as f32 * 0.01)).sin() * 9_000.;
      writer.write_sample((tone + noise) as i16).unwrap();
    }
  }
  writer.finalize().unwrap();
  path.to_string_lossy().into_owned()
}

// play the graph as the scheduler does (see daw::run_playlist): from
// `start`, a beat at a time, on an engine at `rate`. returns what's heard
// in `total`, left and right
fn play(graph: &mut AudioGraph<'static>, rate: u32, start: Duration, total: Duration) -> Vec<f32> {
  let (engine, mut renderer) = daw::Engine::new(rate);
  let beat = graph.beat_interval();
  graph.init();

  let mut voices = Vec::new();
  let mut at = Duration::ZERO;
  let mut first = true;
  while at < total {
    // from between beats, the part beat comes first
    let into = Duration::from_nanos((start.as_nanos() % beat.as_nanos()) as u64);
    let slice = if first && !into.is_zero() { beat - into } else { beat };
    graph.set_current_offset(Some(start + at));
    for (delay, source, route) in graph.run_sounds_at(at, rate, slice, false, first) {
      voices.push(engine.boxed_voice(source, Some(1), engine.frames_in(at + delay), route));
    }
    at += slice;
    first = false;
  }
  graph.pause();

  engine.start_run(1, graph.mixer.master(), voices).unwrap();
  let mut out = vec![0f32; engine.frames_in(total) as usize * 2];
  renderer.render(&mut out, Instant::now(), Duration::ZERO);
  out
}

// a graph at an awkward tempo with one clip of `path`, not on a beat or a
// frame, at a pitch
fn graph_with_clip(path: &str, pitch: f32) -> (AudioGraph<'static>, u64) {
  let mut graph = AudioGraph::new(44_100, 97.);
  let id = graph.construct_and_add_node(path.to_string(), Duration::from_nanos(333_333_333), 0).unwrap();
  let node = graph.get_mut_node_by_id(id).unwrap();
  node.set_settings(daw::ClipSettings { pitch, ..Default::default() });
  (graph, id)
}

fn differences(a: &[f32], b: &[f32]) -> usize {
  assert_eq!(a.len(), b.len());
  a.iter().zip(b).filter(|(a, b)| a != b).count()
}

#[test]
fn test_a_cut_clip_plays_exactly_what_the_whole_clip_did() {
  let rate = 48_000;
  let total = Duration::from_secs(3);
  for file_rate in [44_100, 48_000] {
    let path = noise_wav(&format!("cut-{}", file_rate), file_rate, 2, file_rate * 3 / 2);
    for pitch in [-7., 0., 5.] {
      let (mut graph, id) = graph_with_clip(&path, pitch);
      let clip_start = graph.get_node_by_id(id).unwrap().start_offset;
      let cut = clip_start + Duration::from_nanos(432_109_876);
      // played from the top, from a beat, from between beats before the
      // clip, from inside each half, and from the cut itself
      let starts = [
        Duration::ZERO, graph.beat_interval(), ms(100), clip_start + ms(200), cut + ms(100), cut,
        cut - Duration::from_nanos(1), clip_start,
      ];
      let whole: Vec<Vec<f32>> = starts.iter().map(|&start| play(&mut graph, rate, start, total)).collect();
      assert!(whole[0].iter().any(|&sample| sample != 0.));

      let right = graph.split_node(id, cut).unwrap().expect("a cut inside the clip");
      assert_eq!(graph.nodes.len(), 2);
      for (&start, whole) in starts.iter().zip(&whole) {
        let halves = play(&mut graph, rate, start, total);
        assert_eq!(differences(&halves, whole), 0,
          "{}Hz file, pitch {}, played from {:?}", file_rate, pitch, start);
      }

      // and cut again, into three
      let again = graph.get_node_by_id(right).unwrap().start_offset + ms(77);
      graph.split_node(right, again).unwrap().expect("a cut inside the second half");
      graph.split_node(id, clip_start + ms(50)).unwrap().expect("a cut inside the first half");
      assert_eq!(graph.nodes.len(), 4);
      for (&start, whole) in starts.iter().zip(&whole) {
        let pieces = play(&mut graph, rate, start, total);
        assert_eq!(differences(&pieces, whole), 0,
          "in four: {}Hz file, pitch {}, played from {:?}", file_rate, pitch, start);
      }
    }
  }
}

#[test]
fn test_a_cut_clip_keeps_its_fades_exactly() {
  let rate = 48_000;
  let total = Duration::from_secs(4);
  let path = noise_wav("cut-fades", 44_100, 2, 66_150);
  for pitch in [-7., 0., 5.] {
    let (mut graph, id) = graph_with_clip(&path, pitch);
    // fades that both fit in the half they stay with
    graph.get_mut_node_by_id(id).unwrap().set_settings(daw::ClipSettings {
      pitch, gain_db: -2., pan: -0.2, attack_ms: 60., decay_ms: 90.,
    });
    let node = graph.get_node_by_id(id).unwrap().clone();
    let whole = play(&mut graph, rate, Duration::ZERO, total);

    let right = graph.split_node(id, node.start_offset + node.duration() / 2).unwrap().unwrap();
    assert_eq!(graph.get_node_by_id(id).unwrap().settings().attack_ms, 60.);
    assert_eq!(graph.get_node_by_id(right).unwrap().settings().decay_ms, 90.);
    // not one sample differs, through either fade
    assert_eq!(differences(&play(&mut graph, rate, Duration::ZERO, total), &whole), 0, "pitch {}", pitch);
  }
}

#[test]
fn test_a_clip_whose_sample_starts_before_the_playlist_is_cut_exactly() {
  let rate = 48_000;
  let total = Duration::from_secs(3);
  let path = noise_wav("early", 44_100, 2, 66_150);
  let (mut graph, id) = graph_with_clip(&path, -7.);
  // its first 700ms hidden, then moved to 100ms in: its sample would
  // start well before the playlist does
  graph.set_window(id, ms(700), None).unwrap();
  graph.move_node(id, ms(100), 0).unwrap();
  assert!(graph.get_node_by_id(id).unwrap().origin() < 0);
  let starts = [Duration::ZERO, ms(350), ms(600)];
  let whole: Vec<Vec<f32>> = starts.iter().map(|&start| play(&mut graph, rate, start, total)).collect();
  assert!(whole[0].iter().any(|&sample| sample != 0.));

  graph.split_node(id, ms(550)).unwrap().expect("a cut inside the clip");
  for (&start, whole) in starts.iter().zip(&whole) {
    assert_eq!(differences(&play(&mut graph, rate, start, total), whole), 0, "played from {:?}", start);
  }
}

#[test]
fn test_clips_of_one_file_share_its_sound() {
  let path = noise_wav("shared", 44_100, 1, 4_410);
  let mut graph = AudioGraph::new(44_100, 120.);
  let a = graph.construct_and_add_node(path.clone(), Duration::ZERO, 0).unwrap();
  let b = graph.construct_and_add_node(path.clone(), ms(900), 1).unwrap();
  let pcm = |graph: &AudioGraph<'static>, id: u64| graph.get_node_by_id(id).unwrap().pcm.clone();
  // one decoded copy, however many clips, cut halves included
  assert!(Arc::ptr_eq(&pcm(&graph, a), &pcm(&graph, b)));
  let half = graph.split_node(a, ms(50)).unwrap().unwrap();
  assert!(Arc::ptr_eq(&pcm(&graph, a), &pcm(&graph, half)));
  assert_eq!(graph.get_node_by_id(a).unwrap().get_waveform(), graph.get_node_by_id(b).unwrap().get_waveform());

  // the file written again (longer): a clip made now plays the new sound,
  // and the clips there already keep what they had
  let longer = noise_wav("shared", 44_100, 1, 8_820);
  assert_eq!(longer, path);
  let c = graph.construct_and_add_node(path, ms(2_000), 2).unwrap();
  assert!(!Arc::ptr_eq(&pcm(&graph, a), &pcm(&graph, c)));
  assert_eq!(pcm(&graph, a).frames(), 4_410);
  assert_eq!(pcm(&graph, c).frames(), 8_820);
}

#[test]
fn test_the_halves_of_a_cut() {
  let path = noise_wav("halves", 44_100, 2, 66_150);
  let (mut graph, id) = graph_with_clip(&path, 0.);
  graph.get_mut_node_by_id(id).unwrap().track_number = 3;
  let kept = graph.get_mut_node_by_id(id).unwrap().set_settings(daw::ClipSettings {
    gain_db: -3., pan: 0.25, pitch: 2., attack_ms: 100., decay_ms: 1_200.,
  });
  let before = graph.get_node_by_id(id).unwrap().clone();
  let cut = before.start_offset + ms(400);

  let right = graph.split_node(id, cut).unwrap().unwrap();
  assert_ne!(right, id);
  let left = graph.get_node_by_id(id).unwrap().clone();
  let right = graph.get_node_by_id(right).unwrap().clone();

  // the first half is the clip that was there, where it was. the second
  // starts where the first ends, to the nanosecond
  assert_eq!(left.start_offset, before.start_offset);
  assert_eq!(left.start_offset + left.duration(), right.start_offset);
  assert!(right.start_offset.abs_diff(cut) < Duration::from_nanos(5));
  assert_eq!(left.origin(), right.origin());
  let ends = |clip: &AudioNode| clip.start_offset + clip.duration();
  assert!(ends(&right).abs_diff(ends(&before)) < Duration::from_micros(1));
  // both play the sample the clip did, the same way, on its track
  for half in [&left, &right] {
    assert_eq!(half.sample_path(), before.sample_path());
    assert_eq!(half.track_number, 3);
    assert_eq!((half.settings().gain_db, half.settings().pan, half.settings().pitch), (kept.gain_db, kept.pan, kept.pitch));
  }
  // the fade in stays with the first half, the fade out goes with the
  // second, shortened to it: the second half is under 1.2s long
  assert_eq!((left.settings().attack_ms, left.settings().decay_ms), (100., 0.));
  assert_eq!(right.settings().attack_ms, 0.);
  let right_ms = right.duration().as_secs_f32() * 1000.;
  assert!(right_ms < 1_200. && (right.settings().decay_ms - right_ms).abs() < 0.01);

  // too near an edge, or outside the clip, nothing is cut
  let count = graph.nodes.len();
  for at in [
    left.start_offset, left.start_offset + ms(9), right.start_offset - ms(9),
    right.start_offset + right.duration() - ms(5), ms(10), Duration::from_secs(30),
  ] {
    assert_eq!(graph.split_node(id, at).unwrap(), None, "cut at {:?}", at);
  }
  assert_eq!(graph.nodes.len(), count);
  assert_eq!(graph.get_node_by_id(id).unwrap().window(), left.window());
  assert!(graph.split_node(9_999, cut).is_err());
}

#[test]
fn test_a_trimmed_clip_plays_its_part_of_the_sample_where_it_was() {
  let rate = 48_000;
  let total = Duration::from_secs(3);
  let path = noise_wav("trim", 44_100, 2, 66_150);
  for pitch in [0., -7.] {
    let (mut graph, id) = graph_with_clip(&path, pitch);
    let before = graph.get_node_by_id(id).unwrap().clone();
    let whole = play(&mut graph, rate, Duration::ZERO, total);

    // from 0.2s to 0.9s of the sample
    graph.set_window(id, ms(200), Some(ms(900))).unwrap();
    let node = graph.get_node_by_id(id).unwrap().clone();
    assert_eq!(node.window(), Some(daw::Window { start: ms(200), end: Some(ms(900)) }));
    // the sound hasn't moved: the clip starts later by what was hidden
    assert_eq!(node.origin(), before.origin());
    let hidden = node.settings().played_exactly(ms(200));
    assert_eq!(node.start_offset, before.start_offset + hidden);
    let length = node.settings().played_exactly(ms(900)) - hidden;
    assert_eq!(node.duration(), length);

    let trimmed = play(&mut graph, rate, Duration::ZERO, total);
    let from = run_frame(node.start_offset.as_nanos() as i128, rate) as usize * 2;
    let to = run_frame((node.start_offset + node.duration()).as_nanos() as i128, rate) as usize * 2;
    assert!(to > from + 1_000);
    // silent outside the clip, and exactly what it was inside it
    assert!(trimmed[..from].iter().all(|&sample| sample == 0.));
    assert!(trimmed[to..].iter().all(|&sample| sample == 0.));
    assert_eq!(differences(&trimmed[from..to], &whole[from..to]), 0, "pitch {}", pitch);
    assert!(whole[..from].iter().any(|&sample| sample != 0.));

    // opened out again, it's the whole clip, back where it started
    graph.set_window(id, Duration::ZERO, None).unwrap();
    let node = graph.get_node_by_id(id).unwrap();
    assert_eq!(node.window(), None);
    assert_eq!(node.start_offset, before.start_offset);
    assert_eq!(differences(&play(&mut graph, rate, Duration::ZERO, total), &whole), 0);
  }
}

#[test]
fn test_a_trimmed_clip_is_as_long_at_any_tempo() {
  // a second of a two second sample, from half a second in, on beat 8
  let path = noise_wav("tempo", 44_100, 1, 88_200);
  let mut graph = AudioGraph::new(44_100, 120.);
  let id = graph.construct_and_add_node(path, ms(4_000), 0).unwrap();
  graph.set_window(id, ms(500), Some(ms(1_500))).unwrap();
  let length = |graph: &AudioGraph<'static>| graph.get_node_by_id(id).unwrap().duration();
  assert_eq!(length(&graph), ms(1_000));
  // (its start moved with the trim: half a second, a beat, later)
  assert_eq!(graph.get_node_by_id(id).unwrap().start_offset, ms(4_500));

  for tempo in [90., 61.3, 240., 120.] {
    graph.set_tempo(tempo);
    assert_eq!(length(&graph), ms(1_000), "at {}bpm", tempo);
    assert_eq!(graph.get_node_by_id(id).unwrap().window(), Some(daw::Window { start: ms(500), end: Some(ms(1_500)) }));
    // and it's still on beat 9
    let beat = graph.get_node_by_id(id).unwrap().start_offset.as_secs_f64() / graph.beat_interval().as_secs_f64();
    assert!((beat - 9.).abs() < 1e-6, "on beat {} at {}bpm", beat, tempo);
  }

  // heard for a second at the slower tempo: the frames past its end are silent
  graph.set_tempo(90.);
  let out = play(&mut graph, 44_100, Duration::ZERO, Duration::from_secs(9));
  let starts = run_frame(graph.get_node_by_id(id).unwrap().start_offset.as_nanos() as i128, 44_100) as usize;
  assert!(out[..starts * 2].iter().all(|&sample| sample == 0.));
  assert!(out[starts * 2..(starts + 44_100) * 2].iter().any(|&sample| sample != 0.));
  assert!(out[(starts + 44_100) * 2..].iter().all(|&sample| sample == 0.));
  assert!(out[(starts + 44_099) * 2] != 0. || out[(starts + 44_098) * 2] != 0.);
}

#[test]
fn test_trimming_to_a_bar_line_keeps_the_sound_either_side_of_it() {
  // four bars of sound from bar 1, at 120bpm: eight seconds
  let rate = 44_100;
  let path = noise_wav("bars", rate, 1, rate * 8);
  let mut graph = AudioGraph::new(rate, 120.);
  let id = graph.construct_and_add_node(path, Duration::ZERO, 0).unwrap();
  let total = Duration::from_secs(9);
  let whole = play(&mut graph, rate, Duration::ZERO, total);
  // bar 2 starts two seconds in
  let bar_2 = rate as usize * 2 * 2;

  // the right edge dragged back to bar 2: a one bar clip, whose last
  // frame is what played just before bar 2, and nothing after it
  graph.set_window(id, Duration::ZERO, Some(ms(2_000))).unwrap();
  assert_eq!(graph.get_node_by_id(id).unwrap().duration(), ms(2_000));
  let right = play(&mut graph, rate, Duration::ZERO, total);
  assert_eq!(differences(&right[..bar_2], &whole[..bar_2]), 0);
  assert!(right[bar_2 - 2] != 0. && right[bar_2 - 2] == whole[bar_2 - 2]);
  assert!(right[bar_2..].iter().all(|&sample| sample == 0.));

  // and back to the end of bar 2: a two bar clip, ending with what played
  // just before bar 3
  let bar_3 = bar_2 * 2;
  graph.set_window(id, Duration::ZERO, Some(ms(4_000))).unwrap();
  assert_eq!(graph.get_node_by_id(id).unwrap().duration(), ms(4_000));
  let two_bars = play(&mut graph, rate, Duration::ZERO, total);
  assert_eq!(differences(&two_bars[..bar_3], &whole[..bar_3]), 0);
  assert!(two_bars[bar_3 - 2] != 0. && two_bars[bar_3..].iter().all(|&sample| sample == 0.));

  // the left edge dragged from bar 1 to bar 2 instead: the clip starts on
  // bar 2, with what played at bar 2 before
  graph.set_window(id, ms(2_000), None).unwrap();
  assert_eq!(graph.get_node_by_id(id).unwrap().start_offset, ms(2_000));
  let left = play(&mut graph, rate, Duration::ZERO, total);
  assert!(left[..bar_2].iter().all(|&sample| sample == 0.));
  assert_eq!(differences(&left[bar_2..], &whole[bar_2..]), 0);
  assert!(left[bar_2] != 0. || left[bar_2 + 2] != 0.);

  // with a 100ms fade in, the clip fades in over the 100ms after its new
  // start: silent at it, half way up 50ms on, and all there by 100ms
  graph.get_mut_node_by_id(id).unwrap().set_settings(daw::ClipSettings { attack_ms: 100., ..Default::default() });
  let faded = play(&mut graph, rate, Duration::ZERO, total);
  let after = |ms: usize| bar_2 + rate as usize * ms / 1_000 * 2;
  assert_eq!(faded[bar_2], 0.);
  // (averaged over a few milliseconds of noise either side)
  let level = |samples: &[f32], at: usize| -> f32 {
    samples[at - 200..at + 200].iter().map(|sample| sample.abs()).sum()
  };
  let half = level(&faded, after(50)) / level(&whole, after(50));
  assert!((half - 0.5).abs() < 0.02, "half way up the fade the level is {}", half);
  assert_eq!(differences(&faded[after(101)..], &whole[after(101)..]), 0);
  assert!(differences(&faded[bar_2..after(99)], &whole[bar_2..after(99)]) > 1_000);
}

#[test]
fn test_a_clip_stretched_to_twice_its_length_plays_twice_as_many_frames() {
  // two bars of sound from bar 1, at 120bpm: four seconds
  let rate = 48_000;
  for file_rate in [44_100, 48_000] {
    let path = noise_wav(&format!("stretch-{}", file_rate), file_rate, 1, file_rate * 4);
    let mut graph = AudioGraph::new(44_100, 120.);
    let id = graph.construct_and_add_node(path, Duration::ZERO, 0).unwrap();
    // the frames it sounds for: up to its last frame that isn't silent
    let sounding = |graph: &mut AudioGraph<'static>| {
      let out = play(graph, rate, Duration::ZERO, Duration::from_secs(10));
      out.chunks(2).rposition(|frame| frame[0] != 0.).unwrap() + 1
    };
    let frames = sounding(&mut graph);
    assert!((frames as i64 - 4 * rate as i64).abs() <= 1, "{} frames", frames);

    // its right edge out to bar 5: twice its length, an octave down
    let kept = graph.stretch_node(id, -12., false).unwrap();
    assert_eq!(kept.pitch, -12.);
    assert_eq!(graph.get_node_by_id(id).unwrap().start_offset, Duration::ZERO);
    let stretched = sounding(&mut graph);
    assert!((stretched as i64 - 2 * frames as i64).abs() <= 1, "{} frames, from {}", stretched, frames);

    // and from its end instead: back to its length, ending where it ends
    // now, on bar 5
    graph.stretch_node(id, 0., true).unwrap();
    let node = graph.get_node_by_id(id).unwrap();
    assert_eq!(node.start_offset, ms(4_000));
    assert_eq!(node.start_offset + node.duration(), ms(8_000));
  }
}

#[test]
fn test_stretching_a_trimmed_clip_squeezes_only_what_it_plays() {
  // from 0.5s to 1.5s of a two second sample, with fades, on beat 8
  let path = noise_wav("stretch-trimmed", 44_100, 1, 88_200);
  let mut graph = AudioGraph::new(44_100, 120.);
  let id = graph.construct_and_add_node(path, ms(4_000), 0).unwrap();
  graph.set_window(id, ms(500), Some(ms(1_500))).unwrap();
  graph.get_mut_node_by_id(id).unwrap().set_settings(daw::ClipSettings { attack_ms: 400., decay_ms: 700., ..Default::default() });
  let before = graph.get_node_by_id(id).unwrap().clone();
  let window = before.window();
  let end = before.start_offset + before.duration();

  // to half its length, from its end: an octave up
  graph.stretch_node(id, 12., true).unwrap();
  let node = graph.get_node_by_id(id).unwrap().clone();
  // it plays the same part of the sample, in half the time, and ends
  // where it did
  assert_eq!(node.window(), window);
  assert_eq!(node.duration(), ms(500));
  assert_eq!(node.start_offset + node.duration(), end);
  assert_eq!(node.start_offset, before.start_offset + ms(500));
  // its fades are kept within its new length
  assert_eq!(node.settings().attack_ms, 400.);
  assert!((node.settings().decay_ms - 500.).abs() < 0.01);

  // and it's silent outside that half second: nothing hidden is heard
  let out = play(&mut graph, 44_100, Duration::ZERO, Duration::from_secs(7));
  let from = run_frame(node.start_offset.as_nanos() as i128, 44_100) as usize * 2;
  let to = run_frame(end.as_nanos() as i128, 44_100) as usize * 2;
  assert!(out[..from].iter().all(|&sample| sample == 0.));
  assert!(out[to..].iter().all(|&sample| sample == 0.));
  assert!(out[from..to].iter().any(|&sample| sample != 0.));
}

#[test]
fn test_trim_limits() {
  let path = noise_wav("limits", 44_100, 1, 44_100);
  let (mut graph, id) = graph_with_clip(&path, 0.);
  let sample = graph.get_node_by_id(id).unwrap().sample_duration();
  let window = |graph: &AudioGraph<'static>| graph.get_node_by_id(id).unwrap().window();

  // an end at or past the sample's is the sample's
  graph.set_window(id, ms(100), Some(Duration::from_secs(9))).unwrap();
  assert_eq!(window(&graph), Some(daw::Window { start: ms(100), end: None }));
  graph.set_window(id, Duration::ZERO, Some(sample)).unwrap();
  assert_eq!(window(&graph), None);

  // never shorter than 10ms: the edge that would cross the other stops
  graph.set_window(id, ms(500), Some(ms(503))).unwrap();
  assert_eq!(window(&graph), Some(daw::Window { start: ms(500), end: Some(ms(510)) }));
  graph.set_window(id, ms(995), None).unwrap();
  assert_eq!(window(&graph).unwrap().end, None);
  assert!(sample - window(&graph).unwrap().start >= MIN_CLIP);
  // at twice the speed, 10ms of sound is 20ms of the sample
  graph.set_window(id, Duration::ZERO, None).unwrap();
  graph.get_mut_node_by_id(id).unwrap().set_settings(daw::ClipSettings { pitch: 12., ..Default::default() });
  graph.set_window(id, ms(500), Some(ms(501))).unwrap();
  assert!((window(&graph).unwrap().end.unwrap().as_secs_f64() - 0.520).abs() < 1e-6);
  graph.get_mut_node_by_id(id).unwrap().set_settings(daw::ClipSettings::default());

  // fades are kept within what's left
  graph.set_window(id, Duration::ZERO, None).unwrap();
  graph.get_mut_node_by_id(id).unwrap().set_settings(daw::ClipSettings { attack_ms: 600., decay_ms: 50., ..Default::default() });
  graph.set_window(id, ms(100), Some(ms(400))).unwrap();
  let settings = graph.get_node_by_id(id).unwrap().settings();
  assert!((settings.attack_ms - 300.).abs() < 0.01);
  assert_eq!(settings.decay_ms, 50.);
  // and opening the clip out again doesn't bring them back
  graph.set_window(id, Duration::ZERO, None).unwrap();
  assert!((graph.get_node_by_id(id).unwrap().settings().attack_ms - 300.).abs() < 0.01);

  // a clip can't start before the playlist does: moved to the start with
  // its first half hidden, its left edge has nowhere to go
  graph.set_window(id, ms(500), None).unwrap();
  graph.move_node(id, Duration::ZERO, 0).unwrap();
  graph.set_window(id, ms(200), None).unwrap();
  let node = graph.get_node_by_id(id).unwrap();
  assert_eq!(node.start_offset, Duration::ZERO);
  assert!(node.window().unwrap().start.abs_diff(ms(500)) < Duration::from_nanos(3));

  assert!(graph.set_window(9_999, Duration::ZERO, None).is_err());
}

#[test]
fn test_changing_a_clip_changes_no_other_clip_of_the_sample() {
  let path = noise_wav("others", 44_100, 2, 44_100);
  let (mut graph, id) = graph_with_clip(&path, 0.);
  let other = graph.construct_and_add_node(path.clone(), ms(5_000), 1).unwrap();
  let heard = |graph: &AudioGraph<'static>| -> Vec<f32> {
    let node = graph.get_node_by_id(other).unwrap();
    node.voice(48_000, node.start_offset.as_nanos() as i128, 0).collect()
  };
  let before = graph.get_node_by_id(other).unwrap().clone();
  let sound = heard(&graph);
  assert!(sound.iter().any(|&sample| sample != 0.));

  graph.set_window(id, ms(100), Some(ms(700))).unwrap();
  let start = graph.get_node_by_id(id).unwrap().start_offset;
  let half = graph.split_node(id, start + ms(300)).unwrap().unwrap();
  graph.get_mut_node_by_id(id).unwrap().set_settings(daw::ClipSettings { pitch: 7., gain_db: -12., ..Default::default() });
  graph.set_window(half, ms(450), Some(ms(600))).unwrap();

  let after = graph.get_node_by_id(other).unwrap();
  assert!(after.same_clip(&before));
  assert_eq!(after.start_offset, before.start_offset);
  assert_eq!(after.window(), None);
  assert_eq!(differences(&heard(&graph), &sound), 0);
}

// a two bar pattern with the kick on steps 0, 8 and 20, placed at 2s
// (a step is 125ms at 120bpm): (graph, pattern, clip)
fn graph_with_pattern_clip() -> (AudioGraph<'static>, u64, u64) {
  let (mut graph, kick, pattern) = graph_with_pattern();
  {
    let mut seq = graph.sequencer.lock().unwrap();
    seq.set_pattern_bars(pattern, 2, 4).unwrap();
    seq.set_pattern_step(pattern, kick, 20, true, 4).unwrap();
  }
  let clip = graph.add_pattern_node::<daw::timing::QuarterNote>(pattern, ms(2_000), 1, None).unwrap();
  (graph, pattern, clip)
}

// the frames a sound starts on: where silence ends
fn onsets(samples: &[f32]) -> Vec<usize> {
  let left: Vec<f32> = samples.chunks(2).map(|frame| frame[0]).collect();
  // (the kick is a lot shorter than the gaps between its hits)
  let quiet = 2_000;
  (0..left.len())
    .filter(|&i| left[i] != 0. && (i < quiet || left[i - quiet..i].iter().all(|&s| s == 0.)))
    .collect()
}

#[test]
fn test_a_pattern_clip_plays_the_hits_in_its_window() {
  let rate = 44_100;
  let total = Duration::from_secs(8);
  let (mut graph, pattern, clip) = graph_with_pattern_clip();
  let whole = play(&mut graph, rate, Duration::ZERO, total);
  let hits = onsets(&whole);
  assert_eq!(hits.len(), 3);

  // from step 4 on: the hit on step 0 is gone, the others stay where
  // they were, and the clip starts half a second later
  graph.set_pattern_window(clip, 4., None).unwrap();
  assert_eq!(graph.get_pattern_node_by_id(clip).unwrap().start_offset, ms(2_500));
  assert_eq!(onsets(&play(&mut graph, rate, Duration::ZERO, total)), hits[1..]);
  assert_eq!(graph.end(), ms(6_000));

  // steps 4 up to 20: a hit on the step the window ends on isn't in it
  graph.set_pattern_window(clip, 4., Some(20.)).unwrap();
  assert_eq!(onsets(&play(&mut graph, rate, Duration::ZERO, total)), hits[1..2]);
  assert_eq!(graph.end(), ms(4_500));
  graph.set_pattern_window(clip, 4., Some(20.5)).unwrap();
  assert_eq!(onsets(&play(&mut graph, rate, Duration::ZERO, total)), hits[1..]);

  // the pattern made a bar long: the clip keeps its steps, and plays the
  // pattern again after its end: steps 4 to 16 (the hit on 8, at 3s), then
  // 0 to 4.5 (the hit on 0, at 4s)
  let span = |graph: &AudioGraph<'static>| {
    let node = graph.get_pattern_node_by_id(clip).unwrap();
    (node.start, node.end())
  };
  let at_4s = hits[2] - rate as usize / 2;
  graph.sequencer.lock().unwrap().set_pattern_bars(pattern, 1, 4).unwrap();
  assert_eq!(span(&graph), (4., 20.5));
  assert_eq!(graph.end(), ms(4_562) + Duration::from_micros(500));
  assert_eq!(onsets(&play(&mut graph, rate, Duration::ZERO, total)), vec![hits[1], at_4s]);
  graph.sequencer.lock().unwrap().set_pattern_bars(pattern, 2, 4).unwrap();
  assert_eq!(span(&graph), (4., 20.5));

  // an end that's no number at all is where it ends now
  graph.set_pattern_window(clip, 4., Some(f64::NAN)).unwrap();
  assert_eq!(span(&graph), (4., 20.5));

  // never under a step long. the end isn't kept to the pattern's, and nor
  // is the start: it carries on from there
  graph.set_pattern_window(clip, 10., Some(10.25)).unwrap();
  assert_eq!(span(&graph), (10., 11.));
  graph.set_pattern_window(clip, 31.5, None).unwrap();
  assert_eq!(span(&graph), (31.5, 32.5));
  graph.set_pattern_window(clip, 0., Some(99.)).unwrap();
  assert_eq!(span(&graph), (0., 99.));
  assert_eq!(graph.get_pattern_node_by_id(clip).unwrap().start_offset, ms(2_000));
  // nor longer than a clip can be
  graph.set_pattern_window(clip, 0., Some(1e300)).unwrap();
  assert_eq!(span(&graph), (0., MAX_PATTERN_CLIP));

  assert!(graph.set_pattern_window(9_999, 0., None).is_err());

  // exploded, each new clip plays the steps of its pattern the clip played
  graph.set_pattern_window(clip, 4., Some(20.5)).unwrap();
  let before = onsets(&play(&mut graph, rate, Duration::ZERO, total));
  let made = graph.explode_pattern_node(clip).unwrap();
  assert!(!made.is_empty());
  for id in &made {
    assert_eq!(span_of(&graph, *id), (4., 20.5));
  }
  assert_eq!(onsets(&play(&mut graph, rate, Duration::ZERO, total)), before);
}

// where a pattern clip starts and ends in its pattern, in steps
fn span_of(graph: &AudioGraph<'static>, id: u64) -> (f64, f64) {
  let node = graph.get_pattern_node_by_id(id).unwrap();
  (node.start, node.end())
}

#[test]
fn test_a_cut_pattern_clip_plays_exactly_what_the_whole_clip_did() {
  let rate = 48_000;
  let total = Duration::from_secs(8);
  let (mut graph, pattern, clip) = graph_with_pattern_clip();
  // off the grid, at a tempo whose steps aren't whole frames
  graph.set_tempo(97.);
  graph.pattern_nodes.iter_mut().find(|node| node.id == clip).unwrap().start_offset = Duration::from_nanos(1_234_567_891);
  let starts = [Duration::ZERO, ms(1_500), ms(3_000)];
  let whole: Vec<Vec<f32>> = starts.iter().map(|&start| play(&mut graph, rate, start, total)).collect();
  assert_eq!(onsets(&whole[0]).len(), 3);

  // between steps 8 and 9, then the second half again between 19 and 20
  let step = graph.beat_interval() / 4;
  let start = graph.get_pattern_node_by_id(clip).unwrap().start_offset;
  let right = graph.split_node(clip, start + step.mul_f64(8.5)).unwrap().expect("a cut inside the clip");
  let later = graph.split_node(right, start + step.mul_f64(19.75)).unwrap().expect("a cut inside the half");
  assert_eq!(graph.pattern_nodes.len(), 3);

  let node = |id: u64| graph.get_pattern_node_by_id(id).unwrap().clone();
  assert_eq!(span_of(&graph, clip), (0., 8.5));
  assert_eq!(span_of(&graph, right), (8.5, 19.75));
  assert_eq!(span_of(&graph, later), (19.75, 32.));
  assert!(node(clip).pattern_id == pattern && node(right).pattern_id == pattern && node(later).track_number == 1);
  assert_eq!(node(clip).origin(graph.beat_interval()), node(later).origin(graph.beat_interval()));

  for (&start, whole) in starts.iter().zip(&whole) {
    assert_eq!(differences(&play(&mut graph, rate, start, total), whole), 0, "played from {:?}", start);
  }

  // too near an edge, nothing is cut
  assert_eq!(graph.split_node(clip, start + step.mul_f64(0.5)).unwrap(), None);
  assert_eq!(graph.split_node(clip, start + step.mul_f64(8.)).unwrap(), None);
  assert_eq!(graph.pattern_nodes.len(), 3);
}

const LONG: &str = "assets/slide_guitar.wav";

fn pnote(pitch: u8, start: f64, length: f64, velocity: f32) -> daw::PatternNote {
  daw::PatternNote { pitch, start, length, velocity }
}

// a graph at an awkward tempo with a two bar pattern of the long sample
// (gated, unless `one_shot`) and the kick (one-shot), notes off the grid at
// other pitches and velocities, and a clip of it off the grid on track 1.
// (graph, the long sample's row, pattern, clip)
fn graph_with_pitched_notes(long: Vec<daw::PatternNote>, one_shot: bool) -> (AudioGraph<'static>, u64, u64, u64) {
  let mut graph = AudioGraph::new(44_100, 97.);
  let (row, pattern) = {
    let mut seq = graph.sequencer.lock().unwrap();
    let kick = seq.add_row(KICK).unwrap();
    let row = seq.add_row(LONG).unwrap();
    seq.set_row_gated(row, !one_shot).unwrap();
    let pattern = seq.add_pattern(2, 4).unwrap();
    seq.set_pattern_notes(pattern, row, long).unwrap();
    seq.set_pattern_notes(pattern, kick, vec![pnote(72, 7.9, 0.1, 0.6), pnote(48, 19.5, 1., 1.), pnote(60, 26., 1., 0.3)]).unwrap();
    (row, pattern)
  };
  let clip = graph.add_pattern_node::<daw::timing::QuarterNote>(pattern, Duration::ZERO, 1, None).unwrap();
  graph.pattern_nodes[0].start_offset = Duration::from_nanos(1_234_567_891);
  (graph, row, pattern, clip)
}

#[test]
fn test_a_gated_note_stops_on_the_frame_its_end_is_heard_and_a_one_shot_rings_on() {
  let rate = 48_000;
  let total = Duration::from_secs(4);
  // the long sample alone: from step 2.3 to 4 (a fifth up)
  let only = vec![pnote(67, 2.3, 1.7, 0.9)];
  let (mut graph, _, _, _) = graph_with_pitched_notes(only.clone(), false);
  let kick = graph.sequencer.lock().unwrap().rows()[0].id;
  graph.sequencer.lock().unwrap().set_row_muted(kick, true).unwrap();
  let node = graph.pattern_nodes[0].clone();
  let beat = graph.beat_interval();
  let frame = |at: i128| run_frame(at, rate) as usize;
  let (on, off) = (frame(node.heard_at(2.3, beat)), frame(node.heard_at(4., beat)));

  let gated = play(&mut graph, rate, Duration::ZERO, total);
  let left: Vec<f32> = gated.chunks(2).map(|frame| frame[0]).collect();
  assert!(left[..on].iter().all(|&s| s == 0.));
  assert!(left[on..on + 100].iter().any(|&s| s != 0.));
  // sounding up to its last 5ms (240 frames), then down to nothing by its end
  assert!(left[off - 400..off - 240].iter().any(|&s| s.abs() > 0.01));
  assert!(left[off - 3..off].iter().all(|&s| s.abs() < 0.001));
  assert!(left[off..].iter().all(|&s| s == 0.));

  // one-shot: the same note rings on past its end, the same up to there
  let (mut graph, _, _, _) = graph_with_pitched_notes(only, true);
  graph.sequencer.lock().unwrap().set_row_muted(kick, true).unwrap();
  let rung = play(&mut graph, rate, Duration::ZERO, total);
  assert!(rung[off * 2..(off + 2_000) * 2].iter().any(|&s| s.abs() > 0.01));
  assert!(rung[..(off - 240) * 2] == gated[..(off - 240) * 2]);
}

#[test]
fn test_a_cut_clip_of_pitched_and_gated_notes_plays_exactly_what_the_whole_clip_did() {
  let rate = 48_000;
  let total = Duration::from_secs(5);
  // (none of the gated notes crosses a cut: one that does is cut there, as
  // a synth's note is)
  let long = vec![pnote(55, 1., 3., 0.9), pnote(70, 10., 2.5, 0.5), pnote(64, 21., 5., 1.), pnote(50, 30., 6., 0.7)];
  let (mut graph, _, _, clip) = graph_with_pitched_notes(long, false);
  // from the start, and from the middle of notes of both kinds
  let starts = [Duration::ZERO, ms(1_500), ms(2_900)];
  let whole: Vec<Vec<f32>> = starts.iter().map(|&start| play(&mut graph, rate, start, total)).collect();
  assert!(whole.iter().all(|heard| heard.iter().any(|&s| s.abs() > 0.01)));

  let step = graph.beat_interval() / 4;
  let start = graph.get_pattern_node_by_id(clip).unwrap().start_offset;
  let right = graph.split_node(clip, start + step.mul_f64(8.5)).unwrap().expect("a cut inside the clip");
  graph.split_node(right, start + step.mul_f64(19.75)).unwrap().expect("a cut inside the half");
  for (&start, whole) in starts.iter().zip(&whole) {
    assert_eq!(differences(&play(&mut graph, rate, start, total), whole), 0, "played from {:?}", start);
  }
}

#[test]
fn test_a_pattern_on_its_own_ends_a_gated_note_at_the_pattern_end() {
  let rate = 48_000;
  let (mut graph, _, pattern, _) = graph_with_pitched_notes(vec![pnote(60, 15., 4., 1.)], false);
  graph.set_tempo(120.);
  // the last beat of the second bar (a step is 125ms): the note from step
  // 31 runs past the pattern's end
  graph.sequencer.lock().unwrap().set_pattern_notes(pattern, 1, vec![pnote(60, 31., 4., 1.)]).unwrap();
  graph.set_current_offset(Some(ms(3_500)));
  let sounds = graph.pattern_sounds(pattern, ms(500), rate, ms(500), false, false);
  assert_eq!(sounds.len(), 1);
  let (delay, voice, route) = sounds.into_iter().next().unwrap();
  assert_eq!(delay, ms(375));
  assert!(matches!(route, daw::Route::Master));
  // to 4s, where the pattern ends: 125ms
  assert_eq!(voice.count(), 6_000 * 2);
}

// ---- looping: a clip longer than its pattern plays it over and over ----

#[test]
fn test_a_four_bar_clip_of_a_one_bar_pattern_plays_it_four_times() {
  let rate = 44_100;
  let (mut graph, _, pattern) = graph_with_pattern();
  let clip = graph.add_pattern_node::<daw::timing::QuarterNote>(pattern, ms(1_000), 0, None).unwrap();
  let once = onsets(&play(&mut graph, rate, Duration::ZERO, Duration::from_secs(10)));
  // the kick on steps 0 and 8: a second apart at 120bpm
  assert_eq!(once.len(), 2);
  assert_eq!(once[1] - once[0], rate as usize);

  graph.set_pattern_window(clip, 0., Some(64.)).unwrap();
  assert_eq!(graph.end(), ms(9_000));
  let four = onsets(&play(&mut graph, rate, Duration::ZERO, Duration::from_secs(10)));
  let every_second: Vec<usize> = (0..8).map(|n| once[0] + n * rate as usize).collect();
  assert_eq!(four, every_second);
}

#[test]
fn test_a_patterns_length_changes_no_clip() {
  let rate = 44_100;
  let (mut graph, _, pattern) = graph_with_pattern();
  let whole = graph.add_pattern_node::<daw::timing::QuarterNote>(pattern, ms(0), 0, None).unwrap();
  let looping = graph.add_pattern_node::<daw::timing::QuarterNote>(pattern, ms(4_000), 1, None).unwrap();
  graph.set_pattern_window(looping, 4., Some(40.)).unwrap();
  let clips = |graph: &AudioGraph<'static>| graph.pattern_nodes.clone();
  let before = clips(&graph);
  let (end, heard) = (graph.end(), onsets(&play(&mut graph, rate, Duration::ZERO, Duration::from_secs(10))));

  for bars in [2, 4, 1] {
    graph.sequencer.lock().unwrap().set_pattern_bars(pattern, bars, 4).unwrap();
    assert_eq!(clips(&graph), before);
    assert_eq!(graph.end(), end);
  }
  // back at its length, it sounds as it did
  assert_eq!(onsets(&play(&mut graph, rate, Duration::ZERO, Duration::from_secs(10))), heard);
  assert_eq!(span_of(&graph, whole), (0., 16.));
}

#[test]
fn test_a_clip_starting_past_its_patterns_end_plays_from_there_in_it() {
  let rate = 44_100;
  let (mut graph, _, pattern) = graph_with_pattern();
  // the kick on steps 0 and 8, heard 1s and 2s in from a whole clip at 1s
  let clip = graph.add_pattern_node::<daw::timing::QuarterNote>(pattern, ms(1_000), 0, None).unwrap();
  let whole = onsets(&play(&mut graph, rate, Duration::ZERO, Duration::from_secs(4)));
  assert_eq!(whole.len(), 2);

  // from step 20 (step 4 of its second time round) for a bar: the kick on
  // step 8 half a second in, then the one on step 0 a second later
  graph.pattern_nodes.iter_mut().find(|node| node.id == clip).unwrap().set_span(20., 16.);
  let half = rate as usize / 2;
  assert_eq!(onsets(&play(&mut graph, rate, Duration::ZERO, Duration::from_secs(4))), vec![whole[1] - half, whole[0] + 3 * half]);
}

#[test]
fn test_a_gated_note_past_the_patterns_end_is_cut_there_each_time_round() {
  let rate = 48_000;
  // the long sample alone, from step 30 to 34 of the two bar pattern: two
  // steps past its end
  let (mut graph, _, _, clip) = graph_with_pitched_notes(vec![pnote(60, 30., 4., 1.)], false);
  let kick = graph.sequencer.lock().unwrap().rows()[0].id;
  graph.sequencer.lock().unwrap().set_row_muted(kick, true).unwrap();
  // round the pattern three times
  graph.set_pattern_window(clip, 0., Some(96.)).unwrap();
  let node = graph.pattern_nodes[0].clone();
  let beat = graph.beat_interval();
  let frame = |steps: f64| run_frame(node.heard_at(steps, beat), rate) as usize;
  let total = node.start_offset + node.duration(beat) + ms(500);
  let left: Vec<f32> = play(&mut graph, rate, Duration::ZERO, total).chunks(2).map(|frame| frame[0]).collect();

  assert!(left[..frame(30.)].iter().all(|&s| s == 0.));
  for round in 0..3 {
    let (on, off) = (frame(32. * round as f64 + 30.), frame(32. * (round + 1) as f64));
    assert!(left[on..on + 100].iter().any(|&s| s != 0.), "round {}", round);
    // down to nothing at the end of the pattern, then quiet until it starts
    // again (or for good, after the last)
    assert!(left[off - 400..off - 240].iter().any(|&s| s.abs() > 0.1), "round {}", round);
    assert!(left[off - 3..off].iter().all(|&s| s.abs() < 0.01), "round {}", round);
    let next = frame(32. * (round + 1) as f64 + 30.).min(left.len());
    assert!(left[off..next].iter().all(|&s| s == 0.), "round {}", round);
  }
}

#[test]
fn test_a_looping_clip_cut_in_two_plays_exactly_what_it_did_at_any_pattern_length() {
  let rate = 48_000;
  let total = Duration::from_secs(15);
  // gated notes, one past the two bar pattern's end (cut at each time
  // round), none sounding where the cuts are
  let long = vec![pnote(55, 1., 3., 0.9), pnote(70, 10., 2.5, 0.5), pnote(64, 21., 5., 1.), pnote(50, 30., 6., 0.7)];
  let (mut graph, _, pattern, clip) = graph_with_pitched_notes(long, false);
  graph.set_pattern_window(clip, 0., Some(80.)).unwrap();
  let uncut = graph.pattern_nodes.clone();
  let starts = [Duration::ZERO, ms(4_100), ms(8_000)];
  let whole: Vec<Vec<f32>> = starts.iter().map(|&start| play(&mut graph, rate, start, total)).collect();
  assert!(whole.iter().all(|heard| heard.iter().any(|&s| s.abs() > 0.01)));

  // on its second time round (step 8.5 of it), then on its third
  let step = graph.beat_interval() / 4;
  let start = graph.get_pattern_node_by_id(clip).unwrap().start_offset;
  let right = graph.split_node(clip, start + step.mul_f64(40.5)).unwrap().expect("a cut inside the clip");
  let last = graph.split_node(right, start + step.mul_f64(70.25)).unwrap().expect("a cut inside the half");
  // each half goes on from where it was cut, past the pattern's end
  assert_eq!((span_of(&graph, right), span_of(&graph, last)), ((40.5, 70.25), (70.25, 80.)));
  for (&start, whole) in starts.iter().zip(&whole) {
    assert_eq!(differences(&play(&mut graph, rate, start, total), whole), 0, "played from {:?}", start);
  }

  // two bars longer, the halves still play what the whole clip would (the
  // cuts are on steps 40.5 and 6.25 of it now, where no gated note sounds)
  graph.sequencer.lock().unwrap().set_pattern_bars(pattern, 4, 4).unwrap();
  let cut: Vec<Vec<f32>> = starts.iter().map(|&start| play(&mut graph, rate, start, total)).collect();
  graph.pattern_nodes = uncut;
  for (&start, cut) in starts.iter().zip(&cut) {
    assert_eq!(differences(&play(&mut graph, rate, start, total), cut), 0, "played from {:?}", start);
  }
}

// every note of a long pattern that a clip of steps `from` to `to` of it
// plays, played a slice at a time (slices of an awkward length, at an
// awkward tempo), is played once, in the slice it's heard in: as it was
// when every slice looked at every note. returns how many were played
fn each_note_played_once(from: f64, to: f64) -> usize {
  let mut graph = AudioGraph::new(44_100, 97.);
  let (row, pattern) = {
    let mut seq = graph.sequencer.lock().unwrap();
    let row = seq.add_row(KICK).unwrap();
    let synth = seq.add_synth_row(daw::Synth {
      id: 7, plugin_id: "vst3:synth".into(), path: String::new(), name: "Synth".into(),
      vendor: String::new(), volume: 1., pan: 0., muted: false,
    });
    let pattern = seq.add_pattern(8, 4).unwrap();
    // notes off the grid, two on some steps, and past the end
    let notes: Vec<daw::PatternNote> = (0..300).map(|n| pnote(40 + (n % 30) as u8, n as f64 * 0.47, 0.3, 1.)).collect();
    seq.set_pattern_notes(pattern, row, notes.clone()).unwrap();
    seq.set_pattern_notes(pattern, synth, notes).unwrap();
    (row, pattern)
  };
  // off the grid
  let clip = graph.add_pattern_node::<daw::timing::QuarterNote>(pattern, ms(777), 0, None).unwrap();
  graph.set_pattern_window(clip, from, Some(to)).unwrap();
  graph.init();
  let node = graph.get_pattern_node_by_id(clip).unwrap().clone();
  let beat = graph.beat_interval();
  let notes = graph.sequencer.lock().unwrap().pattern(pattern).unwrap().notes_of(row).to_vec();
  let mut expected: Vec<i128> = Vec::new();
  for pass in node.passes(128) {
    expected.extend(pass.notes(&notes).iter().map(|note| node.heard_at(pass.plays(note).0, beat)));
  }

  let slice = Duration::from_nanos(123_456_789);
  let (mut hits, mut synth_notes) = (Vec::new(), Vec::new());
  let mut at = Duration::ZERO;
  while at < node.start_offset + node.duration(beat) {
    graph.set_current_offset(Some(at));
    let start = at.as_nanos() as i128;
    for (delay, _, _) in graph.run_sounds_at(at, 48_000, slice, false, false) {
      hits.push(start + delay.as_nanos() as i128);
    }
    for note in graph.run_notes(slice, None) {
      synth_notes.push(start + note.on.as_nanos() as i128);
    }
    at += slice;
  }
  // (the hits' delays are whole nanoseconds of the same times)
  assert_eq!(hits, expected);
  assert_eq!(synth_notes, expected);
  expected.len()
}

#[test]
fn test_each_note_is_played_once_in_the_slice_its_heard_in() {
  assert!(each_note_played_once(3.3, 101.2) > 150);
}

#[test]
fn test_each_note_of_a_looping_clip_is_played_once_each_time_round() {
  // the 128 step pattern from step 100.1, round twice and a bit: its notes
  // up to step 128 (the ones past it never play) each time round
  let once = each_note_played_once(0., 128.);
  assert_eq!(once, 273);
  assert!(each_note_played_once(100.1, 400.) > 2 * once);
}

#[test]
fn test_a_run_from_deep_in_a_looping_clip_picks_up_every_note_still_sounding() {
  let rate = 44_100;
  // a bar (2s at 120bpm) of the kick on step 0, five octaves down: it rings
  // for many bars. round and round from the playlist's start, for 32 bars
  let mut graph = AudioGraph::new(rate, 120.);
  let (row, pattern) = {
    let mut seq = graph.sequencer.lock().unwrap();
    let row = seq.add_row(KICK).unwrap();
    let pattern = seq.add_pattern(1, 4).unwrap();
    seq.set_pattern_notes(pattern, row, vec![pnote(0, 0., 1., 1.)]).unwrap();
    (row, pattern)
  };
  let ring = graph.sequencer.lock().unwrap().row(row).unwrap().sample_instrument().unwrap().played_len(0);
  assert!(ring > ms(4_000), "{:?}", ring);
  let looping = graph.add_pattern_node::<daw::timing::QuarterNote>(pattern, Duration::ZERO, 0, None).unwrap();
  graph.set_pattern_window(looping, 0., Some(32. * 16.)).unwrap();
  // and a clip of it that ended long ago, and one that ended a bar ago
  graph.add_pattern_node::<daw::timing::QuarterNote>(pattern, Duration::ZERO, 1, None).unwrap();
  graph.add_pattern_node::<daw::timing::QuarterNote>(pattern, ms(36_000), 2, None).unwrap();
  graph.init();

  // from bar 20 (40s): every time round whose note still rings then, and
  // the one starting then. the clip a bar back too; not the one long gone
  graph.set_current_offset(Some(ms(40_000)));
  let sounds = graph.run_sounds_at(Duration::ZERO, rate, ms(500), false, true);
  let ringing = (0..20u64).filter(|k| ms(2_000 * k) + ring > ms(40_000)).count();
  assert!(ringing >= 2, "{}", ringing);
  assert_eq!(sounds.len(), ringing + 1 + 1);
  let on_track = |track: u32| sounds.iter().filter(|(_, _, route)| matches!(route,
    daw::Route::Track(to) if to.number == track)).count();
  assert_eq!((on_track(0), on_track(1), on_track(2)), (ringing + 1, 0, 1));
}
