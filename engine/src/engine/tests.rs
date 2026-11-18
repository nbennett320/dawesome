use super::*;
use rodio::buffer::SamplesBuffer;

const RATE: u32 = 1_000;

fn master() -> Arc<mixer::ChannelControls> {
  mixer::Mixer::default().master()
}

// a stereo click: `frames` frames of 1.0 at the test rate
fn click(frames: usize) -> SamplesBuffer<f32> {
  SamplesBuffer::new(2, RATE, vec![1.; frames * 2])
}

// render `frames` frames, returning the left channel
fn render(renderer: &mut Renderer, frames: usize) -> Vec<f32> {
  let mut out = vec![0.; frames * 2];
  renderer.render(&mut out, Instant::now(), Duration::ZERO);
  out.chunks(2).map(|f| f[0]).collect()
}

fn first_sound(samples: &[f32]) -> Option<usize> {
  samples.iter().position(|&s| s != 0.)
}

#[test]
fn test_voices_start_on_their_frame() {
  let (engine, mut renderer) = Engine::new(RATE);
  // the run starts after some frames have played
  render(&mut renderer, 37);

  engine.start_run(1, master(), vec![engine.voice(click(3), Some(1), 10, Route::Direct)]).unwrap();
  // a voice sent later lands on its frame too, even mid-buffer
  engine.play(vec![engine.voice(click(3), Some(1), 45, Route::Direct)]).unwrap();
  let out = render(&mut renderer, 64);

  assert_eq!(engine.anchor(1).unwrap().frame, 37);
  assert_eq!(&out[8..14], &[0., 0., 1., 1., 1., 0.]);
  assert_eq!(first_sound(&out[14..]), Some(45 - 14));
  assert_eq!(engine.frames(), 37 + 64);
}

#[test]
fn test_voices_can_start_in_a_later_buffer() {
  let (engine, mut renderer) = Engine::new(RATE);
  engine.start_run(1, master(), vec![engine.voice(click(1), Some(1), 100, Route::Direct)]).unwrap();

  let mut out = render(&mut renderer, 64);
  out.extend(render(&mut renderer, 64));

  assert_eq!(first_sound(&out), Some(100));
}

#[test]
fn test_a_late_voice_plays_from_where_it_would_be() {
  let (engine, mut renderer) = Engine::new(RATE);
  engine.start_run(1, master(), vec![]).unwrap();
  render(&mut renderer, 10);

  // should have started at frame 5: its first 5 frames are skipped
  let ramp = SamplesBuffer::new(2, RATE, (0..40).map(|i| (i / 2 + 1) as f32).collect::<Vec<_>>());
  engine.play(vec![engine.voice(ramp, Some(1), 5, Route::Direct)]).unwrap();
  let out = render(&mut renderer, 4);

  assert_eq!(out, vec![6., 7., 8., 9.]);
}

#[test]
fn test_stopping_fades_out_and_drops_what_has_not_started() {
  let (engine, mut renderer) = Engine::new(RATE);
  engine.start_run(1, master(), vec![
    engine.voice(click(500), Some(1), 0, Route::Direct),
    engine.voice(click(500), Some(1), 200, Route::Direct),
  ]).unwrap();
  render(&mut renderer, 50);

  engine.stop_run(1).unwrap();
  let out = render(&mut renderer, 300);

  // a 10 frame fade at 1kHz, then nothing, and the later voice never plays
  assert!(out[0] > 0.8 && out[0] <= 1.);
  assert!(out[5] < out[0]);
  assert!(out[10..].iter().all(|&s| s == 0.));
  assert!(renderer.voices.is_empty());
}

#[test]
fn test_a_new_run_replaces_the_old_one_and_stale_voices_are_dropped() {
  let (engine, mut renderer) = Engine::new(RATE);
  engine.start_run(1, master(), vec![]).unwrap();
  render(&mut renderer, 10);

  engine.start_run(2, master(), vec![engine.voice(click(1), Some(2), 5, Route::Direct)]).unwrap();
  // the old run's scheduler hasn't noticed yet
  engine.play(vec![engine.voice(click(1), Some(1), 12, Route::Direct)]).unwrap();
  // stopping the old run again does nothing to the new one
  engine.stop_run(1).unwrap();
  let out = render(&mut renderer, 20);

  assert_eq!(engine.anchor(2).unwrap().frame, 10);
  assert!(engine.anchor(1).is_none());
  assert_eq!(out.iter().filter(|&&s| s != 0.).count(), 1);
  assert_eq!(first_sound(&out), Some(5));
}

#[test]
fn test_previews_play_now_and_ignore_runs() {
  let (engine, mut renderer) = Engine::new(RATE);
  render(&mut renderer, 10);
  engine.preview(click(3)).unwrap();
  engine.stop_run(0).unwrap();

  let out = render(&mut renderer, 8);
  assert_eq!(&out[..4], &[1., 1., 1., 0.]);
}

#[test]
fn test_stopping_previews_fades_them_out_and_leaves_the_run_playing() {
  let (engine, mut renderer) = Engine::new(RATE);
  // the run at a quarter, under a preview at full
  engine.start_run(1, master(), vec![
    engine.voice(SamplesBuffer::new(2, RATE, vec![0.25f32; 1_000]), Some(1), 0, Route::Direct),
  ]).unwrap();
  engine.preview(click(500)).unwrap();
  let out = render(&mut renderer, 20);
  assert!(out.iter().all(|&s| s == 1.25));

  engine.stop_previews().unwrap();
  let out = render(&mut renderer, 50);
  // a 10 frame fade at 1kHz, then the run alone
  assert!(out[0] > 1. && out[0] <= 1.25);
  assert!(out[5] < out[0]);
  assert!(out[10..].iter().all(|&s| s == 0.25));
  assert_eq!(renderer.voices.len(), 1);
}

// another sample heard: the one before stops, and the new one doesn't
#[test]
fn test_a_preview_sent_after_a_stop_plays_on() {
  let (engine, mut renderer) = Engine::new(RATE);
  engine.preview(click(500)).unwrap();
  render(&mut renderer, 20);

  engine.stop_previews().unwrap();
  engine.preview(SamplesBuffer::new(2, RATE, vec![0.25f32; 100])).unwrap();
  let out = render(&mut renderer, 60);
  // the old one fades over 10 frames, under the new one, which plays whole
  assert!(out[0] > 1. && out[0] <= 1.25);
  assert!(out[5] < out[0]);
  assert!(out[10..50].iter().all(|&s| s == 0.25));
  assert!(out[50..].iter().all(|&s| s == 0.));
}

#[test]
fn test_the_anchor_is_when_the_run_is_heard() {
  let (engine, mut renderer) = Engine::new(RATE);
  engine.start_run(3, master(), vec![]).unwrap();
  let now = Instant::now();
  let mut out = vec![0.; 16];
  renderer.render(&mut out, now, Duration::from_millis(12));

  let heard = engine.anchor(3).unwrap().heard.duration_since(now);
  assert!(heard >= Duration::from_micros(11_999) && heard <= Duration::from_micros(12_001));
  assert_eq!(engine.latency(), Duration::from_millis(12));
}

#[test]
fn test_voices_are_resampled_to_the_engine_rate() {
  let (engine, mut renderer) = Engine::new(RATE);
  // 10 frames at twice the engine's rate last 5 of its frames
  let fast = SamplesBuffer::new(2, RATE * 2, vec![1f32; 20]);
  engine.preview(fast).unwrap();

  let out = render(&mut renderer, 10);
  let sounding = out.iter().filter(|&&s| s != 0.).count();
  assert!((4..=6).contains(&sounding), "{} frames sounded", sounding);
}

// render `frames` frames, returning (left, right)
fn render_stereo(renderer: &mut Renderer, frames: usize) -> Vec<(f32, f32)> {
  let mut out = vec![0.; frames * 2];
  renderer.render(&mut out, Instant::now(), Duration::ZERO);
  out.chunks(2).map(|f| (f[0], f[1])).collect()
}

// `frames` frames of `level` on both sides
fn steady(frames: usize, level: f32) -> SamplesBuffer<f32> {
  SamplesBuffer::new(2, RATE, vec![level; frames * 2])
}

fn close(a: f32, b: f32) -> bool {
  (a - b).abs() < 1e-5
}

// multiplies by a gain
struct Gain(f32);

impl Processor for Gain {
  fn process(&mut self, input: [&[f32]; 2], output: [&mut [f32]; 2], _: &[processor::Event], _: &Transport) {
    for (output, input) in output.into_iter().zip(input) {
      for (o, i) in output.iter_mut().zip(input) { *o = i * self.0; }
    }
  }
}

// plays its input back `frames` frames later (and only that)
struct Delay {
  line: [Vec<f32>; 2],
  at: usize,
}

impl Delay {
  fn new(frames: usize) -> Self {
    Delay { line: [vec![0.; frames], vec![0.; frames]], at: 0 }
  }
}

impl Processor for Delay {
  fn process(&mut self, input: [&[f32]; 2], output: [&mut [f32]; 2], _: &[processor::Event], _: &Transport) {
    let len = self.line[0].len();
    for (side, (output, input)) in output.into_iter().zip(input).enumerate() {
      for (i, (o, s)) in output.iter_mut().zip(input).enumerate() {
        let at = (self.at + i) % len;
        *o = self.line[side][at];
        self.line[side][at] = *s;
      }
    }
    self.at = (self.at + input[0].len()) % len;
  }
}

// passes the signal through, noting the transport and block length it was given
struct Watch(Arc<Mutex<Vec<(Transport, usize)>>>);

impl Processor for Watch {
  fn process(&mut self, input: [&[f32]; 2], output: [&mut [f32]; 2], _: &[processor::Event], transport: &Transport) {
    self.0.lock().unwrap().push((*transport, input[0].len()));
    for (output, input) in output.into_iter().zip(input) { output.copy_from_slice(input); }
  }
}

fn track(engine: &Engine, mixer: &mixer::Mixer, number: u32, source: SamplesBuffer<f32>, at: u64) -> Box<Voice> {
  engine.voice(source, Some(1), at, Route::Track(mixer.track_route(number)))
}

#[test]
fn test_track_buses_mix_as_gain_per_voice() {
  let mixer = mixer::Mixer::default();
  mixer.set_volume_db(0, -6.);
  mixer.set_pan(0, -0.5);
  mixer.set_volume_db(1, -12.);
  mixer.set_pan(1, 0.75);

  // a different shape on each side, so pan shows
  let ramp = |frames: usize, level: f32| -> Vec<f32> {
    (0..frames).flat_map(|i| {
      let s = level * (i + 1) as f32 / frames as f32;
      [s, -0.5 * s]
    }).collect()
  };
  let voices = [(0, 0, ramp(40, 0.3)), (1, 10, ramp(50, 0.25)), (0, 30, ramp(20, 0.2))];

  let (engine, mut renderer) = Engine::new(RATE);
  engine.start_run(1, mixer.master(), voices.iter()
    .map(|(number, at, samples)| track(&engine, &mixer, *number, SamplesBuffer::new(2, RATE, samples.clone()), *at))
    .collect()).unwrap();
  // in uneven buffers, so voices cross them
  let mut out = render_stereo(&mut renderer, 37);
  out.extend(render_stereo(&mut renderer, 63));

  let mut expected = vec![(0f32, 0f32); 100];
  for (number, at, samples) in &voices {
    let controls = mixer.track(*number);
    let volume = mixer::db_to_gain(controls.volume_db());
    let pan = controls.pan();
    let gain = (volume * (1. - pan).min(1.), volume * (1. + pan).min(1.));
    for (i, frame) in samples.chunks(2).enumerate() {
      expected[*at as usize + i].0 += frame[0] * gain.0;
      expected[*at as usize + i].1 += frame[1] * gain.1;
    }
  }
  for (i, (got, want)) in out.iter().zip(&expected).enumerate() {
    assert!(close(got.0, want.0) && close(got.1, want.1), "frame {}: {:?} != {:?}", i, got, want);
  }
}

#[test]
fn test_a_track_chain_changes_only_its_track_and_the_master_chain_all_but_direct() {
  let mixer = mixer::Mixer::default();
  let play = |engine: &Engine, renderer: &mut Renderer| -> Vec<f32> {
    engine.start_run(1, mixer.master(), vec![
      track(engine, &mixer, 0, steady(10, 0.2), 0),
      track(engine, &mixer, 1, steady(10, 0.4), 20),
      engine.voice(steady(10, 0.3), Some(1), 40, Route::Direct),
    ]).unwrap();
    let out = render(renderer, 60);
    vec![out[5], out[25], out[45]]
  };

  let (engine, mut renderer) = Engine::new(RATE);
  engine.set_track_chain(&mixer.track_route(0), vec![Slot::new(1, Box::new(Gain(0.5)))]).unwrap();
  let out = play(&engine, &mut renderer);
  assert!(close(out[0], 0.1) && close(out[1], 0.4) && close(out[2], 0.3), "{:?}", out);

  let (engine, mut renderer) = Engine::new(RATE);
  engine.set_master_chain(vec![Slot::new(1, Box::new(Gain(0.5)))]).unwrap();
  let out = play(&engine, &mut renderer);
  assert!(close(out[0], 0.1) && close(out[1], 0.2) && close(out[2], 0.3), "{:?}", out);
}

#[test]
fn test_an_effect_tail_rings_on_after_the_voices_end_and_after_a_stop() {
  let mixer = mixer::Mixer::default();
  let (engine, mut renderer) = Engine::new(RATE);
  engine.set_track_chain(&mixer.track_route(0), vec![Slot::new(1, Box::new(Delay::new(50)))]).unwrap();

  // the voice is over by frame 5, its echo comes at 50
  engine.start_run(1, mixer.master(), vec![track(&engine, &mixer, 0, steady(5, 0.5), 0)]).unwrap();
  let out = render(&mut renderer, 20);
  assert!(out.iter().all(|&s| s == 0.));
  assert!(renderer.voices.is_empty());

  engine.stop_run(1).unwrap();
  let out = render(&mut renderer, 60);
  assert_eq!(first_sound(&out), Some(50 - 20));
  assert!(out[30..35].iter().all(|&s| close(s, 0.5)));
  assert!(out[35..].iter().all(|&s| s == 0.));
}

#[test]
fn test_a_bypassed_slot_passes_the_signal_through() {
  let mixer = mixer::Mixer::default();
  let (engine, mut renderer) = Engine::new(RATE);
  let silence = Slot::new(7, Box::new(Gain(0.)));
  silence.set_bypassed(true);
  assert_eq!(silence.id(), 7);
  engine.set_track_chain(&mixer.track_route(0), vec![silence.clone()]).unwrap();

  engine.start_run(1, mixer.master(), vec![track(&engine, &mixer, 0, steady(100, 0.5), 0)]).unwrap();
  let out = render(&mut renderer, 10);
  assert!(out.iter().all(|&s| s == 0.5));

  silence.set_bypassed(false);
  let out = render(&mut renderer, 10);
  assert!(out.iter().all(|&s| s == 0.));
}

#[test]
fn test_a_replaced_chain_comes_back_to_be_dropped() {
  let mixer = mixer::Mixer::default();
  let (engine, mut renderer) = Engine::new(RATE);
  let first = Slot::new(1, Box::new(Gain(1.)));
  let master = Slot::new(2, Box::new(Gain(1.)));
  engine.set_track_chain(&mixer.track_route(3), vec![first.clone()]).unwrap();
  engine.set_master_chain(vec![master.clone()]).unwrap();
  // nothing has rendered, so nothing has taken them
  assert!(!engine.wait_for_chains(Duration::from_millis(5)));

  render(&mut renderer, 8);
  assert!(engine.wait_for_chains(Duration::from_secs(1)));
  assert_eq!(Arc::strong_count(&first), 2);
  assert_eq!(Arc::strong_count(&master), 2);

  engine.set_track_chain(&mixer.track_route(3), vec![Slot::new(3, Box::new(Gain(1.)))]).unwrap();
  engine.set_master_chain(vec![]).unwrap();
  render(&mut renderer, 8);
  assert!(engine.wait_for_chains(Duration::from_secs(1)));
  assert_eq!(Arc::strong_count(&first), 1);
  assert_eq!(Arc::strong_count(&master), 1);
}

#[test]
fn test_a_track_voice_lands_on_its_frame() {
  let mixer = mixer::Mixer::default();
  let (engine, mut renderer) = Engine::new(RATE);
  render(&mut renderer, 37);

  engine.start_run(1, mixer.master(), vec![track(&engine, &mixer, 2, steady(3, 0.5), 10)]).unwrap();
  // one past a whole block, in the same buffer
  let late = MAX_CHUNK as u64 + 5;
  engine.play(vec![track(&engine, &mixer, 2, steady(1, 0.5), late)]).unwrap();
  let out = render(&mut renderer, MAX_CHUNK + 64);

  assert_eq!(&out[8..14], &[0., 0., 0.5, 0.5, 0.5, 0.]);
  assert_eq!(first_sound(&out[14..]).map(|i| i + 14), Some(late as usize));
}

#[test]
fn test_processors_see_the_transport() {
  let mixer = mixer::Mixer::default();
  let (engine, mut renderer) = Engine::new(RATE);
  let seen = Arc::new(Mutex::new(Vec::new()));
  let watch = Slot::new(1, Box::new(Watch(seen.clone())));
  // (kept running through the silence: at this rate one block is longer
  // than the two seconds a silent chain rests after)
  watch.set_showing(true);
  engine.set_master_chain(vec![watch]).unwrap();
  render(&mut renderer, 16);

  let start = RunStart { tempo: 93.5, beats_per_bar: 3, beat_unit: 8, origin: 1_000 };
  engine.start_run_at(1, mixer.master(), start, vec![]).unwrap();
  render(&mut renderer, 64);
  // longer than a block: two calls
  render(&mut renderer, MAX_CHUNK + 10);
  engine.stop_run(1).unwrap();
  render(&mut renderer, 16);

  let seen = seen.lock().unwrap();
  let blocks: Vec<(bool, i64, usize)> = seen.iter().map(|(t, frames)| (t.playing, t.position, *frames)).collect();
  assert_eq!(blocks, vec![
    (false, 0, 16),
    (true, 1_000, 64),
    (true, 1_064, MAX_CHUNK),
    (true, 1_064 + MAX_CHUNK as i64, 10),
    // stopped where the run got to
    (false, 1_074 + MAX_CHUNK as i64, 16),
  ]);
  let (t, _) = seen[1];
  assert_eq!((t.tempo, t.beats_per_bar, t.beat_unit, t.sample_rate), (93.5, 3, 8, RATE));
}

#[test]
fn test_mute_solo_and_meters_work_per_track() {
  let mixer = mixer::Mixer::default();
  let (engine, mut renderer) = Engine::new(RATE);
  let play = |renderer: &mut Renderer| -> Vec<f32> {
    // a moment of nothing first, so the buses start afresh, as new clips would
    render(renderer, 1);
    engine.start_run(1, mixer.master(), vec![
      track(&engine, &mixer, 0, steady(100, 0.5), 0),
      track(&engine, &mixer, 1, steady(100, 0.5), 100),
      track(&engine, &mixer, 2, steady(100, 0.5), 200),
    ]).unwrap();
    let out = render(renderer, 300);
    vec![out[50], out[150], out[250]]
  };

  mixer.set_volume_db(0, -6.);
  mixer.set_mute(1, true);
  let quieter = 0.5 * mixer::db_to_gain(-6.);
  let out = play(&mut renderer);
  assert!(close(out[0], quieter) && out[1] == 0. && close(out[2], 0.5), "{:?}", out);
  // each meter shows its bus, after the fader
  let levels = mixer.take_levels();
  assert_eq!(levels.tracks.iter().map(|&(n, _)| n).collect::<Vec<_>>(), vec![0, 1, 2]);
  assert!(close(levels.tracks[0].1, quieter));
  assert_eq!(levels.tracks[1].1, 0.);
  assert!(close(levels.tracks[2].1, 0.5));
  assert!(close(levels.master, 0.5));

  mixer.set_mute(1, false);
  mixer.set_solo(1, true);
  let out = play(&mut renderer);
  assert!(out[0] == 0. && close(out[1], 0.5) && out[2] == 0., "{:?}", out);
}

// an instrument: 0.25 while any pitch is held (each event at its offset),
// ignoring what comes in
struct Tone {
  held: [bool; 128],
}

impl Tone {
  fn slot(id: u64) -> Arc<Slot> {
    Slot::new_instrument(id, Box::new(Tone { held: [false; 128] }))
  }
}

impl Processor for Tone {
  fn process(&mut self, _: [&[f32]; 2], output: [&mut [f32]; 2], events: &[processor::Event], _: &Transport) {
    let [left, right] = output;
    let mut next = events.iter().peekable();
    for i in 0..left.len() {
      while let Some(event) = next.next_if(|e| e.offset() as usize <= i) {
        match *event {
          processor::Event::NoteOn { pitch, .. } => self.held[pitch as usize] = true,
          processor::Event::NoteOff { pitch, .. } => self.held[pitch as usize] = false,
        }
      }
      let level = if self.held.iter().any(|&h| h) { 0.25 } else { 0. };
      left[i] = level;
      right[i] = level;
    }
  }
}

// an engine with a Tone at the head of track 0's chain, and the mixer
fn with_tone() -> (Engine, Renderer, mixer::Mixer) {
  let mixer = mixer::Mixer::default();
  let (engine, renderer) = Engine::new(RATE);
  engine.set_track_chain(&mixer.track_route(0), vec![Tone::slot(1)]).unwrap();
  (engine, renderer, mixer)
}

// what the instrument at `at` in a bus's chain is playing
fn playing(bus: &Bus, at: usize) -> &Playing {
  // safe: the test is the only thread rendering
  unsafe { bus.chain[at].playing().unwrap() }
}

// the frames that sound, as runs of (from, to)
fn sounding(samples: &[f32]) -> Vec<(usize, usize)> {
  let mut runs: Vec<(usize, usize)> = Vec::new();
  for (i, &s) in samples.iter().enumerate() {
    if s == 0. { continue; }
    match runs.last_mut() {
      Some(run) if run.1 == i => run.1 = i + 1,
      _ => runs.push((i, i + 1)),
    }
  }
  runs
}

#[test]
fn test_notes_start_and_end_on_their_frames() {
  let (engine, mut renderer, mixer) = with_tone();
  render(&mut renderer, 37);

  let route = mixer.track_route(0);
  engine.start_run(1, mixer.master(), vec![]).unwrap();
  // mid-block, and one past a whole block, in the same buffer
  let late = MAX_CHUNK as u64 + 5;
  engine.play_notes(&route, 1, Some(1), vec![
    TimedNote::on(10, 60, 0.8),
    TimedNote::off(15, 60),
    TimedNote::on(late, 64, 0.8),
    TimedNote::off(late + 3, 64),
  ]).unwrap();
  let out = render(&mut renderer, MAX_CHUNK + 64);

  assert_eq!(sounding(&out), vec![(10, 15), (late as usize, late as usize + 3)]);
  // nothing held is left
  assert!(playing(&renderer.buses[0], 0).held_by_run.iter().all(|&n| n == 0));
}

#[test]
fn test_a_note_can_end_where_the_same_pitch_starts_again() {
  let (engine, mut renderer, mixer) = with_tone();
  engine.start_run(1, mixer.master(), vec![]).unwrap();
  // sent in the order a run sends them: the second note before the first ends
  engine.play_notes(&mixer.track_route(0), 1, Some(1), vec![
    TimedNote::on(5, 60, 1.),
    TimedNote::on(10, 60, 1.),
    TimedNote::off(10, 60),
    TimedNote::off(20, 60),
  ]).unwrap();
  let out = render(&mut renderer, 30);

  assert_eq!(sounding(&out), vec![(5, 20)]);
}

#[test]
fn test_a_note_over_before_it_arrives_is_left_out_not_hung() {
  let (engine, mut renderer, mixer) = with_tone();
  let route = mixer.track_route(0);
  engine.start_run(1, mixer.master(), vec![]).unwrap();
  // the notes come a buffer late (as a run's first slice can, if the
  // callback runs between the run's start and its notes)
  render(&mut renderer, 20);
  engine.play_notes(&route, 1, Some(1), vec![TimedNote::on(5, 60, 1.), TimedNote::off(10, 60)]).unwrap();
  let out = render(&mut renderer, 30);

  assert!(out.iter().all(|&s| s == 0.));
  let bus = &renderer.buses[0];
  assert!(bus.notes.is_empty());
  assert!(playing(bus, 0).held_by_run.iter().all(|&n| n == 0));
  // and a stop leaves nothing sounding
  engine.stop_run(1).unwrap();
  assert!(render(&mut renderer, 30).iter().all(|&s| s == 0.));
}

#[test]
fn test_a_late_note_that_hasnt_ended_plays_from_now_to_its_end() {
  let (engine, mut renderer, mixer) = with_tone();
  let route = mixer.track_route(0);
  engine.start_run(1, mixer.master(), vec![]).unwrap();
  render(&mut renderer, 20);
  engine.play_notes(&route, 1, Some(1), vec![TimedNote::on(5, 60, 1.), TimedNote::off(40, 60)]).unwrap();
  let out = render(&mut renderer, 30);

  assert_eq!(sounding(&out), vec![(0, 20)]);
  assert!(playing(&renderer.buses[0], 0).held_by_run.iter().all(|&n| n == 0));
}

#[test]
fn test_a_late_note_ending_where_the_same_pitch_starts_again_still_plays_on() {
  let (engine, mut renderer, mixer) = with_tone();
  let route = mixer.track_route(0);
  engine.start_run(1, mixer.master(), vec![]).unwrap();
  engine.play_notes(&route, 1, Some(1), vec![TimedNote::on(5, 60, 1.)]).unwrap();
  let mut out = render(&mut renderer, 20);
  // late: the first note's end and the next one's start (at a clip
  // boundary), and a short note that's over already
  engine.play_notes(&route, 1, Some(1), vec![
    TimedNote::on(12, 60, 1.),
    TimedNote::off(12, 60),
    TimedNote::on(14, 60, 1.),
    TimedNote::off(16, 60),
    TimedNote::off(30, 60),
  ]).unwrap();
  out.extend(render(&mut renderer, 20));

  assert_eq!(sounding(&out), vec![(5, 30)]);
  assert!(playing(&renderer.buses[0], 0).held_by_run.iter().all(|&n| n == 0));
}

#[test]
fn test_a_stop_ends_the_notes_a_run_holds() {
  let (engine, mut renderer, mixer) = with_tone();
  let route = mixer.track_route(0);
  engine.start_run(1, mixer.master(), vec![]).unwrap();
  // one held past the stop, one that hasn't started by then
  engine.play_notes(&route, 1, Some(1), vec![
    TimedNote::on(2, 60, 1.),
    TimedNote::off(500, 60),
    TimedNote::on(100, 62, 1.),
  ]).unwrap();
  let out = render(&mut renderer, 20);
  assert_eq!(sounding(&out), vec![(2, 20)]);

  engine.stop_run(1).unwrap();
  let out = render(&mut renderer, 200);
  assert!(out.iter().all(|&s| s == 0.));
  assert!(renderer.buses[0].notes.is_empty());
}

#[test]
fn test_a_new_run_ends_the_old_ones_notes_and_drops_its_late_ones() {
  let (engine, mut renderer, mixer) = with_tone();
  let route = mixer.track_route(0);
  engine.start_run(1, mixer.master(), vec![]).unwrap();
  engine.play_notes(&route, 1, Some(1), vec![TimedNote::on(0, 60, 1.)]).unwrap();
  render(&mut renderer, 10);

  engine.start_run(2, mixer.master(), vec![]).unwrap();
  // the old run's scheduler hasn't noticed yet
  engine.play_notes(&route, 1, Some(1), vec![TimedNote::on(12, 61, 1.), TimedNote::off(14, 61)]).unwrap();
  engine.play_notes(&route, 1, Some(2), vec![TimedNote::on(30, 62, 1.), TimedNote::off(35, 62)]).unwrap();
  let out = render(&mut renderer, 50);

  assert_eq!(sounding(&out), vec![(30, 35)]);
}

#[test]
fn test_an_audition_plays_now_and_outlives_a_stop() {
  let (engine, mut renderer, mixer) = with_tone();
  let route = mixer.track_route(0);
  render(&mut renderer, 10);
  engine.start_run(1, mixer.master(), vec![]).unwrap();
  engine.play_notes(&route, 1, None, vec![TimedNote::on(0, 60, 1.)]).unwrap();
  // the run holds the same pitch, and another
  engine.play_notes(&route, 1, Some(1), vec![TimedNote::on(0, 60, 1.), TimedNote::on(0, 70, 1.)]).unwrap();
  let out = render(&mut renderer, 10);
  assert_eq!(sounding(&out), vec![(0, 10)]);

  // the run's notes end, but not the audition's
  engine.stop_run(1).unwrap();
  let out = render(&mut renderer, 10);
  assert_eq!(sounding(&out), vec![(0, 10)]);
  assert_eq!(playing(&renderer.buses[0], 0).held_by_audition[60], 1);

  engine.play_notes(&route, 1, None, vec![TimedNote::off(0, 60)]).unwrap();
  let out = render(&mut renderer, 10);
  assert!(out.iter().all(|&s| s == 0.));
}

#[test]
fn test_an_instrument_is_added_to_the_clips_and_both_go_through_the_effects() {
  let (engine, mut renderer, mixer) = with_tone();
  let route = mixer.track_route(0);
  let instrument = Tone::slot(1);
  engine.set_track_chain(&route, vec![instrument.clone(), Slot::new(2, Box::new(Gain(0.5)))]).unwrap();
  mixer.set_volume_db(0, -6.);
  let fader = mixer::db_to_gain(-6.);

  engine.start_run(1, mixer.master(), vec![track(&engine, &mixer, 0, steady(100, 0.2), 0)]).unwrap();
  engine.play_notes(&route, 1, Some(1), vec![TimedNote::on(20, 60, 1.), TimedNote::off(40, 60)]).unwrap();
  let out = render(&mut renderer, 60);
  assert!(close(out[10], 0.2 * 0.5 * fader), "{}", out[10]);
  assert!(close(out[30], 0.45 * 0.5 * fader), "{}", out[30]);
  assert!(close(out[50], 0.2 * 0.5 * fader), "{}", out[50]);

  // bypassed, the clip passes through, and the notes aren't heard
  instrument.set_bypassed(true);
  engine.play_notes(&route, 1, Some(1), vec![TimedNote::on(65, 60, 1.)]).unwrap();
  let out = render(&mut renderer, 20);
  assert!(out.iter().all(|&s| close(s, 0.2 * 0.5 * fader)));
}

#[test]
fn test_too_many_notes_waiting_are_dropped_not_made_room_for() {
  let (engine, mut renderer, mixer) = with_tone();
  engine.start_run(1, mixer.master(), vec![]).unwrap();
  let notes: Vec<TimedNote> = (0..MAX_NOTES as u64 + 10).map(|n| TimedNote::on(1_000 + n, 60, 1.)).collect();
  engine.play_notes(&mixer.track_route(0), 1, Some(1), notes).unwrap();
  render(&mut renderer, 1);
  let bus = &renderer.buses[0];
  assert_eq!(bus.notes.len(), MAX_NOTES);
  assert_eq!(bus.notes.capacity(), MAX_NOTES);
}

#[test]
fn test_two_instruments_on_a_bus_each_hear_only_their_own_notes() {
  let mixer = mixer::Mixer::default();
  let (engine, mut renderer) = Engine::new(RATE);
  let route = mixer.track_route(0);
  let (a, b) = (Tone::slot(1), Tone::slot(2));
  // set before it first plays, so it starts there rather than ramping
  b.set_gain(2.);
  engine.set_track_chain(&route, vec![a, b, Slot::new(3, Box::new(Gain(0.5)))]).unwrap();

  // the same pitch on both: a's note-off doesn't end b's note
  engine.start_run(1, mixer.master(), vec![]).unwrap();
  engine.play_notes(&route, 1, Some(1), vec![TimedNote::on(10, 60, 1.), TimedNote::off(20, 60)]).unwrap();
  engine.play_notes(&route, 2, Some(1), vec![TimedNote::on(15, 60, 1.), TimedNote::off(30, 60)]).unwrap();
  let out = render(&mut renderer, 40);

  assert!(out[..10].iter().all(|&s| s == 0.));
  assert!(out[10..15].iter().all(|&s| close(s, 0.25 * 0.5)), "{:?}", &out[10..15]);
  assert!(out[15..20].iter().all(|&s| close(s, (0.25 + 0.5) * 0.5)), "{:?}", &out[15..20]);
  assert!(out[20..30].iter().all(|&s| close(s, 0.5 * 0.5)), "{:?}", &out[20..30]);
  assert!(out[30..].iter().all(|&s| s == 0.));
  for at in 0..2 {
    assert!(playing(&renderer.buses[0], at).held_by_run.iter().all(|&n| n == 0));
  }
}

#[test]
fn test_an_instrument_on_the_master_plays_through_the_master_chain() {
  let mixer = mixer::Mixer::default();
  let (engine, mut renderer) = Engine::new(RATE);
  engine.set_master_chain(vec![Tone::slot(5), Slot::new(6, Box::new(Gain(0.5)))]).unwrap();

  // with a track's clip and a hit straight on the master
  engine.start_run(1, mixer.master(), vec![
    track(&engine, &mixer, 0, steady(100, 0.2), 0),
    engine.voice(steady(100, 0.1), Some(1), 0, Route::Master),
  ]).unwrap();
  engine.play_master_notes(5, Some(1), vec![TimedNote::on(20, 60, 1.), TimedNote::off(40, 60)]).unwrap();
  let out = render(&mut renderer, 60);
  assert!(close(out[10], 0.3 * 0.5), "{}", out[10]);
  assert!(close(out[30], 0.55 * 0.5), "{}", out[30]);
  assert!(close(out[50], 0.3 * 0.5), "{}", out[50]);

  // a key auditioned on it, with nothing else playing
  engine.stop_run(1).unwrap();
  render(&mut renderer, 20);
  engine.play_master_notes(5, None, vec![TimedNote::on(0, 64, 1.)]).unwrap();
  let out = render(&mut renderer, 10);
  assert!(out.iter().all(|&s| close(s, 0.25 * 0.5)), "{:?}", out);
  engine.play_master_notes(5, None, vec![TimedNote::off(0, 64)]).unwrap();
  assert!(render(&mut renderer, 10).iter().all(|&s| s == 0.));
}

#[test]
fn test_an_instruments_gain_ramps_to_a_new_value_across_a_block() {
  let (engine, mut renderer, mixer) = with_tone();
  let route = mixer.track_route(0);
  let instrument = Tone::slot(1);
  engine.set_track_chain(&route, vec![instrument.clone()]).unwrap();
  engine.play_notes(&route, 1, None, vec![TimedNote::on(0, 60, 1.)]).unwrap();
  let out = render(&mut renderer, 10);
  assert!(out.iter().all(|&s| s == 0.25));

  instrument.set_gain(0.5);
  let out = render(&mut renderer, 10);
  // falling a step a frame, rather than all at once
  assert!(out[0] < 0.25 && out[0] > 0.125, "{:?}", out);
  assert!(out.windows(2).all(|w| w[1] < w[0]), "{:?}", out);
  assert!(close(out[9], 0.125), "{:?}", out);
  let out = render(&mut renderer, 10);
  assert!(out.iter().all(|&s| close(s, 0.125)), "{:?}", out);

  // bypassed, it adds nothing
  instrument.set_bypassed(true);
  assert!(render(&mut renderer, 10).iter().all(|&s| s == 0.));
}

#[test]
fn test_a_stop_ends_what_each_instrument_holds_for_the_run() {
  let mixer = mixer::Mixer::default();
  let (engine, mut renderer) = Engine::new(RATE);
  let route = mixer.track_route(0);
  let (a, b) = (Tone::slot(1), Tone::slot(2));
  b.set_gain(2.);
  engine.set_track_chain(&route, vec![a, b]).unwrap();

  // a holds a note of the run's, b one of the run's and a key auditioned
  // (of the pitch a holds, which an audition on a doesn't keep)
  engine.start_run(1, mixer.master(), vec![]).unwrap();
  engine.play_notes(&route, 1, Some(1), vec![TimedNote::on(0, 60, 1.), TimedNote::off(500, 60)]).unwrap();
  engine.play_notes(&route, 2, Some(1), vec![TimedNote::on(0, 62, 1.), TimedNote::off(500, 62)]).unwrap();
  engine.play_notes(&route, 2, None, vec![TimedNote::on(0, 60, 1.)]).unwrap();
  let out = render(&mut renderer, 10);
  assert!(out.iter().all(|&s| close(s, 0.75)), "{:?}", out);

  engine.stop_run(1).unwrap();
  let out = render(&mut renderer, 10);
  assert!(out.iter().all(|&s| close(s, 0.5)), "{:?}", out);
  let bus = &renderer.buses[0];
  assert!(playing(bus, 0).held_by_run.iter().all(|&n| n == 0));
  assert!(playing(bus, 1).held_by_run.iter().all(|&n| n == 0));
  assert_eq!(playing(bus, 1).held_by_audition[60], 1);
  assert!(bus.notes.is_empty());

  engine.play_notes(&route, 2, None, vec![TimedNote::off(0, 60)]).unwrap();
  assert!(render(&mut renderer, 10).iter().all(|&s| s == 0.));
}

#[test]
fn test_notes_for_an_instrument_not_there_are_dropped_or_wait_for_its_chain() {
  let (engine, mut renderer, mixer) = with_tone();
  let route = mixer.track_route(0);

  // played now, for an id the chain hasn't, and on a master with none
  engine.play_notes(&route, 9, None, vec![TimedNote::on(0, 60, 1.)]).unwrap();
  engine.play_master_notes(9, None, vec![TimedNote::on(0, 60, 1.)]).unwrap();
  assert!(render(&mut renderer, 10).iter().all(|&s| s == 0.));
  assert!(renderer.buses[0].notes.is_empty());
  assert!(renderer.master.notes.is_empty());
  assert!(playing(&renderer.buses[0], 0).held_by_audition.iter().all(|&n| n == 0));

  // still to come, for an instrument whose chain comes before they do
  engine.start_run(1, mixer.master(), vec![]).unwrap();
  engine.play_notes(&route, 3, Some(1), vec![TimedNote::on(30, 60, 1.), TimedNote::off(40, 60)]).unwrap();
  let mut out = render(&mut renderer, 20);
  engine.set_track_chain(&route, vec![Tone::slot(1), Tone::slot(3)]).unwrap();
  out.extend(render(&mut renderer, 30));
  assert_eq!(sounding(&out), vec![(30, 40)]);
}

// ---- resting: a chain that's silent, in and out, is left out

// an effect that passes its input through and counts the frames it's
// run for. it can say how long it rings on, and that a parameter is
// waiting (which a block takes)
struct Counted {
  frames: Arc<AtomicUsize>,
  tail: Option<u64>,
  pending: Arc<AtomicBool>,
}

impl Counted {
  fn new(tail: Option<u64>) -> (Counted, Arc<AtomicUsize>, Arc<AtomicBool>) {
    let frames = Arc::new(AtomicUsize::new(0));
    let pending = Arc::new(AtomicBool::new(false));
    (Counted { frames: frames.clone(), tail, pending: pending.clone() }, frames, pending)
  }
}

impl Processor for Counted {
  fn process(&mut self, input: [&[f32]; 2], output: [&mut [f32]; 2], _: &[processor::Event], _: &Transport) {
    self.frames.fetch_add(input[0].len(), Ordering::Relaxed);
    self.pending.store(false, Ordering::Relaxed);
    for (output, input) in output.into_iter().zip(input) { output.copy_from_slice(input); }
  }

  fn tail_frames(&self) -> Option<u64> { self.tail }

  fn has_pending(&self) -> bool { self.pending.load(Ordering::Relaxed) }
}

// an effect that hums whatever comes in (a tape machine's hiss), counting
// its frames
struct Hum(Arc<AtomicUsize>);

impl Processor for Hum {
  fn process(&mut self, input: [&[f32]; 2], output: [&mut [f32]; 2], _: &[processor::Event], _: &Transport) {
    self.0.fetch_add(input[0].len(), Ordering::Relaxed);
    for output in output { output.fill(0.01); }
  }
}

// render `frames`, a hundred at a time (a rest is counted in blocks)
fn render_blocks(renderer: &mut Renderer, frames: usize) -> Vec<f32> {
  (0..frames / 100).flat_map(|_| render(renderer, 100)).collect()
}

// two seconds, at the tests' rate
const REST: usize = 2 * RATE as usize;

#[test]
fn test_a_silent_chain_rests_after_two_seconds_and_sound_wakes_it_at_once() {
  let mixer = mixer::Mixer::default();
  let (engine, mut renderer) = Engine::new(RATE);
  let (counted, frames, _) = Counted::new(Some(0));
  engine.set_track_chain(&mixer.track_route(0), vec![Slot::new(1, Box::new(counted))]).unwrap();

  // a voice five frames long: the chain runs for its block, and for two
  // seconds of silence after it
  engine.start_run(1, mixer.master(), vec![track(&engine, &mixer, 0, steady(5, 0.5), 0)]).unwrap();
  let out = render_blocks(&mut renderer, 5_000);
  assert_eq!(sounding(&out), vec![(0, 5)]);
  assert_eq!(frames.load(Ordering::Relaxed), 100 + REST);

  // and not again, however long the silence
  assert_eq!(engine.resting_chains(), 1);
  render_blocks(&mut renderer, 3_000);
  assert_eq!(frames.load(Ordering::Relaxed), 100 + REST);

  // a voice in the middle of a block wakes it for that block, and is
  // heard from its own frame
  engine.play(vec![track(&engine, &mixer, 0, steady(5, 0.5), 8_000 + 37)]).unwrap();
  let out = render_blocks(&mut renderer, 300);
  assert_eq!(engine.resting_chains(), 0);
  assert_eq!(sounding(&out), vec![(37, 42)]);
  assert!(out[37..42].iter().all(|&s| close(s, 0.5)));
  assert_eq!(frames.load(Ordering::Relaxed), 100 + REST + 300);
}

#[test]
fn test_an_echo_within_two_seconds_is_heard_and_a_longer_one_needs_its_tail_said() {
  for (delay, tail, heard) in [(1_500, Some(0), true), (2_500, Some(0), false), (2_500, Some(2_500), true), (2_500, None, true)] {
    let mixer = mixer::Mixer::default();
    let (engine, mut renderer) = Engine::new(RATE);
    let (counted, _, _) = Counted::new(tail);
    engine.set_track_chain(&mixer.track_route(0), vec![
      Slot::new(1, Box::new(Delay::new(delay))),
      // (says how long the chain rings on)
      Slot::new(2, Box::new(counted)),
    ]).unwrap();
    engine.start_run(1, mixer.master(), vec![track(&engine, &mixer, 0, steady(5, 0.5), 0)]).unwrap();
    let out = render_blocks(&mut renderer, 6_000);
    let echo = if heard { vec![(delay, delay + 5)] } else { vec![] };
    assert_eq!(sounding(&out), echo, "a delay of {} with a tail of {:?}", delay, tail);
  }
}

#[test]
fn test_a_plugin_that_never_stops_or_that_sounds_by_itself_never_rests() {
  let mixer = mixer::Mixer::default();
  let (engine, mut renderer) = Engine::new(RATE);
  let (endless, endless_frames, _) = Counted::new(None);
  let hum_frames = Arc::new(AtomicUsize::new(0));
  engine.set_track_chain(&mixer.track_route(0), vec![Slot::new(1, Box::new(endless))]).unwrap();
  engine.set_track_chain(&mixer.track_route(1), vec![Slot::new(2, Box::new(Hum(hum_frames.clone())))]).unwrap();
  // (the hum's track is muted: its chain is still sounding, before the
  // fader, so it's still run)
  mixer.set_mute(1, true);

  let out = render_blocks(&mut renderer, 6_000);
  assert!(out.iter().all(|&s| s == 0.));
  assert_eq!(endless_frames.load(Ordering::Relaxed), 6_000);
  assert_eq!(hum_frames.load(Ordering::Relaxed), 6_000);

  // unmuted, it's there at once
  mixer.set_mute(1, false);
  let out = render_blocks(&mut renderer, 1_000);
  assert!(close(*out.last().unwrap(), 0.01), "{:?}", out.last());
}

#[test]
fn test_a_note_wakes_a_resting_instrument_on_its_frame_and_a_held_one_keeps_it_awake() {
  let mixer = mixer::Mixer::default();
  let (engine, mut renderer) = Engine::new(RATE);
  // (an instrument that's silent even while it holds a note: a pad with
  // a slow attack)
  let (counted, frames, _) = Counted::new(Some(0));
  engine.set_track_chain(&mixer.track_route(0), vec![
    Slot::new_instrument(1, Box::new(counted)),
    Tone::slot(2),
  ]).unwrap();
  let route = mixer.track_route(0);
  render_blocks(&mut renderer, 5_000);
  assert_eq!(frames.load(Ordering::Relaxed), REST);

  // a note for the tone, 37 frames into a run: heard from that frame
  engine.start_run(1, mixer.master(), vec![]).unwrap();
  engine.play_notes(&route, 2, Some(1), vec![TimedNote::on(37, 60, 1.), TimedNote::off(60, 60)]).unwrap();
  let out = render_blocks(&mut renderer, 100);
  assert_eq!(sounding(&out), vec![(37, 60)]);

  // a note held on the silent one keeps the chain running for as long as
  // it's held, then for two seconds from the block that ends it
  engine.play_notes(&route, 1, None, vec![TimedNote::on(0, 64, 1.)]).unwrap();
  let before = frames.load(Ordering::Relaxed);
  render_blocks(&mut renderer, 6_000);
  assert_eq!(frames.load(Ordering::Relaxed), before + 6_000);
  engine.play_notes(&route, 1, None, vec![TimedNote::off(0, 64)]).unwrap();
  render_blocks(&mut renderer, 6_000);
  assert_eq!(frames.load(Ordering::Relaxed), before + 6_000 + REST);
}

#[test]
fn test_a_showing_interface_a_turned_knob_a_switch_and_a_new_chain_run_a_resting_chain() {
  let mixer = mixer::Mixer::default();
  let (engine, mut renderer) = Engine::new(RATE);
  let (counted, frames, pending) = Counted::new(Some(0));
  let slot = Slot::new(1, Box::new(counted));
  engine.set_track_chain(&mixer.track_route(0), vec![slot.clone()]).unwrap();
  let ran = |renderer: &mut Renderer, frames_in: usize| {
    let before = frames.load(Ordering::Relaxed);
    render_blocks(renderer, frames_in);
    frames.load(Ordering::Relaxed) - before
  };
  assert_eq!(ran(&mut renderer, 5_000), REST);

  // its interface on screen: it runs for as long as it's there, and for
  // two seconds from the last block it was
  slot.set_showing(true);
  assert_eq!(ran(&mut renderer, 5_000), 5_000);
  slot.set_showing(false);
  assert_eq!(ran(&mut renderer, 5_000), REST - 100);

  // a knob turned: two seconds from the block that takes it
  pending.store(true, Ordering::Relaxed);
  assert_eq!(ran(&mut renderer, 5_000), REST);
  assert!(!pending.load(Ordering::Relaxed));

  // switched off, it isn't run at all; switched back on, it is again
  slot.set_bypassed(true);
  assert_eq!(ran(&mut renderer, 5_000), 0);
  slot.set_bypassed(false);
  assert_eq!(ran(&mut renderer, 5_000), REST);

  // and a chain that's changed
  let (other, other_frames, _) = Counted::new(Some(0));
  engine.set_track_chain(&mixer.track_route(0), vec![slot.clone(), Slot::new(2, Box::new(other))]).unwrap();
  assert_eq!(ran(&mut renderer, 5_000), REST);
  assert_eq!(other_frames.load(Ordering::Relaxed), REST);
}

#[test]
fn test_the_master_chain_rests_too_and_what_goes_round_it_is_still_heard() {
  let mixer = mixer::Mixer::default();
  let (engine, mut renderer) = Engine::new(RATE);
  let (counted, frames, _) = Counted::new(Some(0));
  engine.set_master_chain(vec![Slot::new(1, Box::new(counted))]).unwrap();
  let out = render_blocks(&mut renderer, 5_000);
  assert!(out.iter().all(|&s| s == 0.));
  assert_eq!(frames.load(Ordering::Relaxed), REST);

  // a preview goes straight to the output, and doesn't wake it
  engine.preview(click(3)).unwrap();
  let out = render_blocks(&mut renderer, 100);
  assert_eq!(&out[..4], &[1., 1., 1., 0.]);
  assert_eq!(frames.load(Ordering::Relaxed), REST);

  // a track's voice does, and comes out at its level from its frame
  engine.start_run(1, mixer.master(), vec![track(&engine, &mixer, 0, steady(5, 0.5), 20)]).unwrap();
  let out = render_blocks(&mut renderer, 100);
  assert_eq!(sounding(&out), vec![(20, 25)]);
  assert!(out[20..25].iter().all(|&s| close(s, mixer::soft_limit(0.5))), "{:?}", &out[20..25]);
  assert_eq!(frames.load(Ordering::Relaxed), REST + 100);
}

#[test]
fn test_a_bus_with_no_chain_is_as_it_was() {
  // nothing to rest: a voice two seconds into silence plays as ever
  let mixer = mixer::Mixer::default();
  let (engine, mut renderer) = Engine::new(RATE);
  engine.start_run(1, mixer.master(), vec![track(&engine, &mixer, 0, steady(5, 0.5), 4_010)]).unwrap();
  let out = render_blocks(&mut renderer, 5_000);
  assert_eq!(sounding(&out), vec![(4_010, 4_015)]);
}

// plays through the real output: run with `cargo test -- --ignored`
#[test]
#[ignore = "needs an audio output device"]
fn test_the_device_output_runs() {

  let engine = engine().expect("could not open the output");
  let start = engine.frames();
  engine.start_run(99, master(), vec![
    engine.voice(sound_buffer::SampleBuffer::load("assets/assets_66-hh-01-or.wav").unwrap().convert_samples(), Some(99), 0, Route::Direct),
  ]).unwrap();

  let anchor = engine.wait_for_anchor(99, Duration::from_secs(2)).unwrap();
  assert!(anchor.frame >= start);
  assert!(engine.wait_for_frame(anchor.frame + engine.frames_in(Duration::from_millis(300)), || true));
  engine.stop_run(99).unwrap();

  println!(
    "rate {} Hz, buffer {} frames, output latency {:?}, anchored at frame {}",
    engine.sample_rate(), engine.buffer_frames(), engine.latency(), anchor.frame,
  );
  assert!(engine.buffer_frames() > 0);
}

// opens real outputs: run with `cargo test -- --ignored`. doesn't touch
// the saved settings
#[test]
#[ignore = "needs an audio output device"]
fn test_outputs_open_with_settings() {
  for frames in [128, 512] {
    let output = open_output(&AudioSettings { buffer_frames: Some(frames), ..Default::default() })
      .expect("could not open the output");
    println!(
      "{}: asked for {} frames, got {} ({:?} latency)",
      output.engine.device_name(), frames, output.engine.buffer_frames(), output.engine.latency(),
    );
    // some devices go no lower than they can (Bluetooth outputs stop at
    // 256, whatever range they claim), but never ask for less
    assert!(output.engine.buffer_frames() >= frames as u64);
  }

  let missing = open_output(&AudioSettings { output_device: Some("no such device".into()), ..Default::default() });
  assert!(missing.err().unwrap().contains("no such device"));
  let odd = open_output(&AudioSettings { buffer_frames: Some(100), ..Default::default() });
  assert!(odd.err().unwrap().contains("100 frames"));
}

// ---- the tape (always-live mode)

mod tape_tests {
  use super::*;
  use ::tape::{MachineSettings, PlaybackSettings, RecordSide, SimpleCore, SimpleCoreSettings, TapeMachine, TransportSettings};

  const TAPE_RATE: u32 = 48_000;

  // a tape with nothing random on it: no wobble, hiss, dropouts or dbx
  fn still() -> MachineSettings {
    let mut playback = PlaybackSettings::default();
    playback.dropouts_per_minute = 0.0;
    MachineSettings { transport: TransportSettings::still(), playback, wow_flutter: 0.0, hiss: 0.0, dbx: false, output_db: 0.0 }
  }

  fn live_tape(settings: MachineSettings, before_effects: bool, tracks: &[u32]) -> Box<tape::EngineTape> {
    let rate = TAPE_RATE as f32;
    let mut machine = TapeMachine::new(settings, rate, 1, processor::MAX_BLOCK);
    let mut sides = Vec::new();
    for &number in tracks {
      let mut side = || {
        let core = Box::new(SimpleCore::new(SimpleCoreSettings::default(), rate, processor::MAX_BLOCK));
        machine.add_track(Some(RecordSide::new(core, rate, processor::MAX_BLOCK)), 0.0)
      };
      sides.push((number, [side(), side()], true));
    }
    tape::EngineTape::new(machine, before_effects, TAPE_RATE, sides)
  }

  // a tape whose tracks wait for prints
  fn printed_tape(tracks: &[u32]) -> Box<tape::EngineTape> {
    let mut machine = TapeMachine::new(still(), TAPE_RATE as f32, 1, processor::MAX_BLOCK);
    let sides = tracks.iter().map(|&n| (n, [machine.add_track(None, 0.0), machine.add_track(None, 0.0)], false)).collect();
    tape::EngineTape::new(machine, true, TAPE_RATE, sides)
  }

  // a print: a slow sine, in two stretches with silence between
  fn print(frames: usize) -> Arc<tape::Print> {
    let wave = |from: usize, n: usize| -> Vec<f32> { (from..from + n).map(|i| (i as f32 * 0.003).sin() * 0.1).collect() };
    let half = frames / 2;
    Arc::new(tape::Print {
      rate: TAPE_RATE,
      regions: vec![
        tape::Region { start: 0, sides: [wave(0, half - 1000), wave(0, half - 1000)] },
        tape::Region { start: half as i64, sides: [wave(half, half), wave(half, half)] },
      ],
    })
  }

  // what a print should sound like on a still, quiet tape: itself, on time
  fn printed_at(p: &tape::Print, from: i64, frames: usize) -> Vec<f32> {
    let mut out = vec![0.0; frames];
    p.read(0, from, &mut out);
    out
  }

  // a sine on a track, at 48 khz, `level` of full scale
  fn tone(frames: usize, level: f32) -> SamplesBuffer<f32> {
    let samples: Vec<f32> = (0..frames).flat_map(|i| {
      let s = level * (i as f32 * 2.0 * std::f32::consts::PI * 440.0 / TAPE_RATE as f32).sin();
      [s, s]
    }).collect();
    SamplesBuffer::new(2, TAPE_RATE, samples)
  }

  fn play(tape: Option<Box<tape::EngineTape>>, chain: Vec<Arc<Slot>>, level: f32) -> Vec<f32> {
    let mixer = mixer::Mixer::default();
    let (engine, mut renderer) = Engine::new(TAPE_RATE);
    engine.set_tape(tape).unwrap();
    if !chain.is_empty() { engine.set_track_chain(&mixer.track_route(0), chain).unwrap(); }
    engine.start_run(1, mixer.master(), vec![engine.voice(tone(9600, level), Some(1), 0, Route::Track(mixer.track_route(0)))]).unwrap();
    let mut out = render(&mut renderer, 4800);
    out.extend(render(&mut renderer, 4800));
    out
  }

  #[test]
  fn test_tape_off_or_with_nothing_on_it_plays_as_before() {
    // T14: no tape, and a tape with only another track on it, are the same
    // to the last bit
    let without = play(None, vec![], 0.1);
    let other_track = play(Some(live_tape(still(), true, &[5])), vec![], 0.1);
    assert_eq!(without, other_track);
    assert!(without.iter().any(|&s| s != 0.));
  }

  #[test]
  fn test_a_track_on_tape_is_the_core_and_late() {
    let dry = play(None, vec![], 0.05);
    let taped = play(Some(live_tape(still(), true, &[0])), vec![], 0.05);
    // the playback head reads 8 ms back
    let delay = (0.008 * TAPE_RATE as f32).round() as usize;
    // (a sine starts at zero: its first sound is a frame in)
    let start = first_sound(&dry).unwrap();
    assert!(start <= 1);
    let late = first_sound(&taped).unwrap() - start;
    assert!(late >= delay - 20 && late <= delay + 20, "first sound {late} frames late");
    // quiet, so near unity: the same tone, delayed, give or take the eq
    let rms = |x: &[f32]| (x.iter().map(|s| s * s).sum::<f32>() / x.len() as f32).sqrt();
    let ratio = rms(&taped[delay + 2000..]) / rms(&dry[2000..9600 - delay]);
    assert!((ratio - 1.0).abs() < 0.1, "level {ratio}");
  }

  #[test]
  fn test_the_output_knob_reaches_a_running_tape() {
    let mixer = mixer::Mixer::default();
    let (engine, mut renderer) = Engine::new(TAPE_RATE);
    engine.set_tape(Some(live_tape(still(), true, &[0]))).unwrap();
    engine.start_run(1, mixer.master(), vec![engine.voice(tone(19200, 0.05), Some(1), 0, Route::Track(mixer.track_route(0)))]).unwrap();
    let before = render(&mut renderer, 9600);
    engine.set_tape_settings(MachineSettings { output_db: -6.0, ..still() }).unwrap();
    let after = render(&mut renderer, 9600);
    let peak = |x: &[f32]| x.iter().fold(0f32, |m, s| m.max(s.abs()));
    let ratio = peak(&after[2000..]) / peak(&before[2000..]);
    assert!((ratio - 0.5012).abs() < 0.02, "{ratio}");
  }

  #[test]
  fn test_before_or_after_the_track_s_effects() {
    // loud enough to saturate: tape then half the level isn't half the
    // level then tape
    let half = || vec![Slot::new(1, Box::new(Gain(0.5)))];
    let before = play(Some(live_tape(still(), true, &[0])), half(), 0.8);
    let after = play(Some(live_tape(still(), false, &[0])), half(), 0.8);
    let differ = before.iter().zip(&after).filter(|(a, b)| (*a - *b).abs() > 1e-3).count();
    assert!(differ > 1000, "{differ} frames differ");
  }

  // ---- printed tracks

  // a run from song frame `origin`. track 0 gets a silent clip: a track's
  // bus is made when it has something to play (a printed track always has
  // its clips), and the clip is what's direct, crossed out for the print
  fn run_from(engine: &Engine, mixer: &mixer::Mixer, run: u64, origin: i64, _voices: Vec<Box<Voice>>) {
    let start = RunStart { origin, ..RunStart::default() };
    let silent = engine.voice(tone(96_000, 0.0), Some(run), 0, Route::Track(mixer.track_route(0)));
    engine.start_run_at(run, mixer.master(), start, vec![silent]).unwrap();
  }

  #[test]
  fn test_a_print_reads_as_silence_between_its_stretches() {
    let p = print(20_000);
    let mut out = vec![1.0; 3000];
    p.read(0, 8_000, &mut out);
    // 8000..9000 is sound, 9000..10000 the gap, 10000.. sound again
    assert!(out[..900].iter().any(|&s| s != 0.0));
    assert!(out[1000..2000].iter().all(|&s| s == 0.0));
    assert!(out[2100..].iter().any(|&s| s != 0.0));
    // past the end, and before the start
    let mut out = vec![1.0; 100];
    p.read(1, 30_000, &mut out);
    assert!(out.iter().all(|&s| s == 0.0));
    p.read(1, -50, &mut out);
    assert!(out[..50].iter().all(|&s| s == 0.0) && out[51..].iter().any(|&s| s != 0.0));
  }

  #[test]
  fn test_a_printed_track_plays_its_print_on_time() {
    let mixer = mixer::Mixer::default();
    let (engine, mut renderer) = Engine::new(TAPE_RATE);
    engine.set_tape(Some(printed_tape(&[0]))).unwrap();
    let p = print(40_000);
    engine.set_tape_print(0, Some(p.clone())).unwrap();
    run_from(&engine, &mixer, 1, 0, vec![]);
    let out = render(&mut renderer, 12_000);
    // after the 50 ms crossfade from direct (silence here), the print
    let want = printed_at(&p, 0, 12_000);
    let worst = out[3000..].iter().zip(&want[3000..]).map(|(a, b)| (a - b).abs()).fold(0f32, f32::max);
    assert!(worst < 2e-3, "off by {worst}");
  }

  #[test]
  fn test_a_seek_starts_the_print_from_there_with_nothing_stale() {
    let mixer = mixer::Mixer::default();
    let (engine, mut renderer) = Engine::new(TAPE_RATE);
    engine.set_tape(Some(printed_tape(&[0]))).unwrap();
    let p = print(40_000);
    engine.set_tape_print(0, Some(p.clone())).unwrap();
    run_from(&engine, &mixer, 1, 0, vec![]);
    render(&mut renderer, 9_600);
    // a jump to frame 25000: what comes out is the print from there at once
    run_from(&engine, &mixer, 2, 25_000, vec![]);
    let out = render(&mut renderer, 4_800);
    let want = printed_at(&p, 25_000, 4_800);
    let worst = out.iter().zip(&want).map(|(a, b)| (a - b).abs()).fold(0f32, f32::max);
    assert!(worst < 2e-3, "off by {worst}");
  }

  #[test]
  fn test_without_a_print_or_stopped_a_track_is_direct() {
    // no print yet: exactly as with no tape (D64)
    let with_tape = {
      let mixer = mixer::Mixer::default();
      let (engine, mut renderer) = Engine::new(TAPE_RATE);
      engine.set_tape(Some(printed_tape(&[0]))).unwrap();
      engine.start_run(1, mixer.master(), vec![engine.voice(tone(9600, 0.1), Some(1), 0, Route::Track(mixer.track_route(0)))]).unwrap();
      render(&mut renderer, 9600)
    };
    assert_eq!(with_tape, play(None, vec![], 0.1));
    // stopped: the tape has nothing to do with a printed track
    let t = printed_tape(&[0]);
    assert_eq!(t.uses(0, false), None);
    assert_eq!(t.uses(0, true), None);
  }

  #[test]
  fn test_an_out_of_date_print_crosses_back_to_direct() {
    let mixer = mixer::Mixer::default();
    let (engine, mut renderer) = Engine::new(TAPE_RATE);
    engine.set_tape(Some(printed_tape(&[0]))).unwrap();
    engine.set_tape_print(0, Some(print(96_000))).unwrap();
    run_from(&engine, &mixer, 1, 0, vec![]);
    render(&mut renderer, 9_600);
    engine.set_tape_print(0, None).unwrap();
    let out = render(&mut renderer, 9_600);
    // still sounding as it crosses (50 ms), then silent: direct (nothing)
    assert!(out[..1000].iter().any(|&s| s != 0.0));
    assert!(out[3000..].iter().all(|&s| s == 0.0));
  }
}

// ---- varispeed (part E): the whole song at another speed

mod varispeed_tests {
  use super::*;

  const SONG_RATE: u32 = 48_000;

  // a built-in effect (crates/dsp), at its defaults
  fn native(id: &str) -> Box<dyn processor::Processor> {
    crate::daw::native::load(id, SONG_RATE).unwrap().take_processor().unwrap()
  }

  // a song: a tone on track 0 through a gain and the built-in EQ, a delay
  // and the built-in Delay on track 1's chain, and an instrument playing
  // notes on track 2, all through the master and its limiter
  fn song(engine: &Engine, mixer: &mixer::Mixer) {
    let tone = |frames: usize, hz: f32, level: f32| -> SamplesBuffer<f32> {
      let samples: Vec<f32> = (0..frames).flat_map(|i| {
        let s = level * (i as f32 * 2.0 * std::f32::consts::PI * hz / SONG_RATE as f32).sin();
        [s, s * 0.5]
      }).collect();
      SamplesBuffer::new(2, SONG_RATE, samples)
    };
    engine.set_track_chain(&mixer.track_route(0), vec![Slot::new(1, Box::new(Gain(0.7))), Slot::new(4, native("dawesome:eq"))]).unwrap();
    engine.set_track_chain(&mixer.track_route(1), vec![Slot::new(2, Box::new(Delay::new(1_234))), Slot::new(5, native("dawesome:delay"))]).unwrap();
    engine.set_track_chain(&mixer.track_route(2), vec![Tone::slot(3)]).unwrap();
    let voices = vec![
      engine.voice(tone(96_000, 440.0, 0.4), Some(1), 100, Route::Track(mixer.track_route(0))),
      engine.voice(tone(30_000, 1_250.0, 0.6), Some(1), 7_000, Route::Track(mixer.track_route(1))),
    ];
    engine.start_run(1, mixer.master(), voices).unwrap();
    let notes = (0..20).flat_map(|n| [TimedNote::on(n * 9_000 + 50, 60, 1.0), TimedNote::off(n * 9_000 + 4_000, 60)]).collect();
    engine.play_notes(&mixer.track_route(2), 3, Some(1), notes).unwrap();
  }

  // the song rendered in device buffers of `frames`, `buffers` of them;
  // `before` runs on the engine before the song starts
  fn render_song(before: impl Fn(&Engine), buffers: usize, frames: usize) -> (Vec<f32>, Engine, Renderer) {
    let mixer = mixer::Mixer::default();
    let (engine, mut renderer) = Engine::new(SONG_RATE);
    before(&engine);
    song(&engine, &mixer);
    let mut out = Vec::new();
    let mut buffer = vec![0.; frames * 2];
    for _ in 0..buffers {
      renderer.render(&mut buffer, Instant::now(), Duration::ZERO);
      out.extend_from_slice(&buffer);
    }
    (out, engine, renderer)
  }

  #[test]
  fn test_at_1x_the_varispeed_is_out_of_the_path() {
    // T14: an engine told to play at 1x (tape off, or the PITCH knob at 0)
    // is the same, to the last bit, as one never told anything
    let (never, _, renderer) = render_song(|_| {}, 40, 512);
    assert!(renderer.straight());
    let (told, _, renderer) = render_song(|engine| engine.set_speed(1.0).unwrap(), 40, 512);
    assert!(renderer.straight());
    assert_eq!(never, told);
    assert!(never.iter().filter(|&&s| s != 0.).count() > 10_000);
  }

  #[test]
  fn test_moved_off_1x_and_back_it_leaves_the_path_and_plays_the_song_as_it_is() {
    let (reference, _, _) = render_song(|_| {}, 200, 512);
    let mixer = mixer::Mixer::default();
    let (engine, mut renderer) = Engine::new(SONG_RATE);
    song(&engine, &mixer);
    let mut buffer = vec![0.; 512 * 2];
    let mut render = |renderer: &mut Renderer| renderer.render(&mut buffer, Instant::now(), Duration::ZERO);
    for _ in 0..4 { render(&mut renderer); }
    engine.set_speed(1.06).unwrap();
    for _ in 0..20 { render(&mut renderer); }
    assert!(!renderer.straight());
    engine.set_speed(1.0).unwrap();
    // the glide back and the ease onto a whole frame: out of the path
    // within 2 s
    let mut buffers = 0;
    while !renderer.straight() {
      render(&mut renderer);
      buffers += 1;
      assert!(buffers < 2 * SONG_RATE as usize / 512, "still resampling");
    }
    // from here on, the song as it is: what's heard is what's rendered,
    // song frame for song frame
    let at = engine.frames() as usize;
    let mut out = vec![0.; 512 * 2];
    for _ in 0..10 {
      renderer.render(&mut out, Instant::now(), Duration::ZERO);
    }
    let rendered = engine.frames() as usize;
    assert_eq!(&out[..], &reference[(rendered - 512) * 2..rendered * 2]);
    assert!(rendered > at);
  }

  #[test]
  fn test_at_another_speed_the_song_is_read_that_much_faster_and_the_clock_follows() {
    let mixer = mixer::Mixer::default();
    let (engine, mut renderer) = Engine::new(SONG_RATE);
    engine.set_speed(1.1).unwrap();
    song(&engine, &mixer);
    let mut buffer = vec![0.; 512 * 2];
    // past the glide
    for _ in 0..100 { renderer.render(&mut buffer, Instant::now(), Duration::ZERO); }
    let from = engine.frames();
    for _ in 0..100 { renderer.render(&mut buffer, Instant::now(), Duration::ZERO); }
    let read = (engine.frames() - from) as f64 / (100. * 512.);
    assert!((read - 1.1).abs() < 1e-3, "read {read}");
    assert_eq!(engine.speed(), 1.1);
    // the song's clock runs 10 % fast: a second from the last buffer's
    // heard time is 1.1 s of song
    let heard = engine.shared.epoch + Duration::from_nanos(engine.shared.clock_heard_ns.load(Ordering::Relaxed));
    let a = engine.song_instant(heard);
    let b = engine.song_instant(heard + Duration::from_secs(1));
    assert!(((b - a).as_secs_f64() - 1.1).abs() < 1e-9);
    // and back
    let real = engine.real_instant(b);
    assert!(signed_secs(real, heard + Duration::from_secs(1)).abs() < 1e-6);
  }

  #[test]
  fn test_at_1x_the_clock_is_the_clock() {
    let (_, engine, _) = render_song(|_| {}, 10, 512);
    let now = Instant::now();
    assert_eq!(engine.song_instant(now), now);
    assert_eq!(engine.real_instant(now), now);
  }

  #[test]
  fn test_a_coast_runs_the_song_down_to_a_standstill_then_stops_the_run() {
    let mixer = mixer::Mixer::default();
    let (engine, mut renderer) = Engine::new(SONG_RATE);
    song(&engine, &mixer);
    let mut buffer = vec![0.; 512 * 2];
    for _ in 0..10 { renderer.render(&mut buffer, Instant::now(), Duration::ZERO); }
    let from = engine.frames();
    let covers = engine.coast_reach().as_secs_f64();
    assert!((covers - 0.3).abs() < 1e-9);
    engine.coast_run(1).unwrap();
    let mut buffers = 0;
    while renderer.run.is_some() {
      renderer.render(&mut buffer, Instant::now(), Duration::ZERO);
      buffers += 1;
      assert!(buffers < 100, "never stopped");
    }
    // 0.6 s of coast, covering 0.3 s of song
    let took = buffers as f64 * 512. / SONG_RATE as f64;
    assert!((took - 0.6).abs() < 0.02, "{took}");
    let song = (engine.frames() - from) as f64 / SONG_RATE as f64;
    assert!((song - covers).abs() < 0.01, "{song}");
    // the motor's back on for whatever plays next, faded in
    renderer.render(&mut buffer, Instant::now(), Duration::ZERO);
    assert_eq!(engine.speed(), 1.);
  }

  #[test]
  fn test_a_spun_up_run_starts_from_a_standstill() {
    let mixer = mixer::Mixer::default();
    let (engine, mut renderer) = Engine::new(SONG_RATE);
    engine.start_run_with(1, mixer.master(), RunStart::default(), vec![], true).unwrap();
    let mut buffer = vec![0.; 480 * 2];
    renderer.render(&mut buffer, Instant::now(), Duration::ZERO);
    renderer.render(&mut buffer, Instant::now(), Duration::ZERO);
    // 10 ms in: barely moving, and the lag it'll have built up is said
    assert!(engine.speed() < 0.05, "{}", engine.speed());
    assert!((engine.lag_seconds() - 0.25).abs() < 0.03, "{}", engine.lag_seconds());
  }

  #[test]
  fn test_back_at_1x_a_new_run_is_anchored_on_the_clock_itself() {
    // after the knob has been turned and come back, the song's clock has
    // drifted from the clock: a new run starts it afresh, so pitch 0 is
    // today's timing again (T14)
    let mixer = mixer::Mixer::default();
    let (engine, mut renderer) = Engine::new(SONG_RATE);
    song(&engine, &mixer);
    let mut buffer = vec![0.; 512 * 2];
    engine.set_speed(1.08).unwrap();
    for _ in 0..40 { renderer.render(&mut buffer, Instant::now(), Duration::ZERO); }
    engine.set_speed(1.0).unwrap();
    while !renderer.straight() { renderer.render(&mut buffer, Instant::now(), Duration::ZERO); }
    renderer.render(&mut buffer, Instant::now(), Duration::ZERO);
    assert!(signed_secs(engine.song_instant(Instant::now()), Instant::now()).abs() > 1e-3, "it drifted");

    engine.start_run(2, mixer.master(), vec![]).unwrap();
    let now = Instant::now();
    let latency = Duration::from_millis(7);
    renderer.render(&mut buffer, now, latency);
    assert_eq!(engine.anchor(2).unwrap().heard, now + latency);
    assert_eq!(engine.song_instant(now), now);
    assert_eq!(engine.real_instant(now), now);
  }

  #[test]
  fn test_a_coasting_run_isn_t_cut_short_by_its_stop_and_play_picks_it_up_from_where_it_s_got_to() {
    let mixer = mixer::Mixer::default();
    let (engine, mut renderer) = Engine::new(SONG_RATE);
    song(&engine, &mixer);
    let mut buffer = vec![0.; 512 * 2];
    for _ in 0..10 { renderer.render(&mut buffer, Instant::now(), Duration::ZERO); }
    engine.coast_run(1).unwrap();
    // the run's thread stops it as it finishes: it coasts on
    engine.stop_run(1).unwrap();
    // 0.32 s of a 0.6 s coast
    for _ in 0..30 { renderer.render(&mut buffer, Instant::now(), Duration::ZERO); }
    assert!(renderer.run.is_some());
    let slowed = engine.speed();
    assert!(slowed > 0.4 && slowed < 0.55, "{slowed}");

    // Play again, spinning up: from the speed it's coasted down to
    engine.start_run_with(2, mixer.master(), RunStart::default(), vec![], true).unwrap();
    // (the speed's said as each buffer starts)
    renderer.render(&mut buffer, Instant::now(), Duration::ZERO);
    let from = engine.speed();
    assert!((from - slowed).abs() < 0.02, "{from} {slowed}");
    renderer.render(&mut buffer, Instant::now(), Duration::ZERO);
    assert!(engine.speed() > from, "{} {}", engine.speed(), from);
    assert_eq!(renderer.run.as_ref().map(|run| run.id), Some(2));
    while !renderer.straight() { renderer.render(&mut buffer, Instant::now(), Duration::ZERO); }
    assert_eq!(engine.speed(), 1.);
  }
}

// ---- scrubbing (part S): a scrub plays over the engine, from the proxy

mod scrub_tests {
  use super::*;
  use super::scrub::Proxy;

  const OUT: u32 = 48_000;

  // a proxy that's silent but for a burst of tone from 1 s to 1.2 s
  fn proxy() -> Arc<Proxy> {
    let samples: Vec<i16> = (0..48_000).map(|i| {
      if (24_000..28_800).contains(&i) { ((i as f32 * 0.3).sin() * 20_000.) as i16 } else { 0 }
    }).collect();
    Arc::new(Proxy { rate: 24_000, samples: samples.into_boxed_slice() })
  }

  fn song(engine: &Engine, mixer: &mixer::Mixer) {
    let samples: Vec<f32> = (0..96_000).flat_map(|i| { let s = (i as f32 * 0.01).sin() * 0.3; [s, s] }).collect();
    engine.start_run(1, mixer.master(), vec![
      engine.voice(SamplesBuffer::new(2, OUT, samples), Some(1), 0, Route::Track(mixer.track_route(0))),
    ]).unwrap();
  }

  fn buffers(renderer: &mut Renderer, n: usize) -> Vec<f32> {
    let mut all = Vec::new();
    let mut out = vec![0.; 480 * 2];
    for _ in 0..n {
      renderer.render(&mut out, Instant::now(), Duration::ZERO);
      all.extend_from_slice(&out);
    }
    all
  }

  #[test]
  fn test_a_scrub_is_heard_where_it_s_dragged_and_let_go_leaves_the_engine_as_it_was() {
    let mixer = mixer::Mixer::default();
    let (plain, mut plain_renderer) = Engine::new(OUT);
    song(&plain, &mixer);
    let unscrubbed = buffers(&mut plain_renderer, 60);

    let (engine, mut renderer) = Engine::new(OUT);
    song(&engine, &mixer);
    let mut out = buffers(&mut renderer, 10);
    assert_eq!(engine.scrub_at(), None);
    // dragged from 0.9 s across the burst to 1.3 s
    engine.scrub_start(proxy(), 0.9, None, None).unwrap();
    engine.scrub_drive(1.3);
    out.extend(buffers(&mut renderer, 20));
    assert!((engine.scrub_at().unwrap() - 1.3).abs() < 0.05, "{:?}", engine.scrub_at());
    engine.scrub_end().unwrap();
    out.extend(buffers(&mut renderer, 30));
    assert_eq!(engine.scrub_at(), None);
    assert!(renderer.scrub.is_none(), "let go of");

    // heard over the song while it was held, and nothing of it after
    let held = &out[10 * 960..30 * 960];
    let worst = held.iter().zip(&unscrubbed[10 * 960..30 * 960]).map(|(a, b)| (a - b).abs()).fold(0., f32::max);
    assert!(worst > 0.1, "the burst was heard: {worst}");
    assert_eq!(&out[32 * 960..], &unscrubbed[32 * 960..]);
  }

  #[test]
  fn test_a_scrub_begun_over_another_takes_over() {
    let (engine, mut renderer) = Engine::new(OUT);
    engine.scrub_start(proxy(), 0.5, None, None).unwrap();
    buffers(&mut renderer, 2);
    engine.scrub_start(proxy(), 1.0, Some(-2.0), Some((::tape::TransportSettings::default(), 1.0))).unwrap();
    buffers(&mut renderer, 2);
    // the second, winding back from 1 s
    assert!(engine.scrub_at().unwrap() < 1.0);
    engine.collect_garbage();
  }
}
