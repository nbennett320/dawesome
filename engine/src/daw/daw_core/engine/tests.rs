use super::*;
use rodio::buffer::SamplesBuffer;

const RATE: u32 = 1_000;

fn master() -> Arc<daw::ChannelControls> {
  daw::Mixer::default().master()
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
  fn process(&mut self, input: [&[f32]; 2], output: [&mut [f32]; 2], _: &[plugin::Event], _: &Transport) {
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
  fn process(&mut self, input: [&[f32]; 2], output: [&mut [f32]; 2], _: &[plugin::Event], _: &Transport) {
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
  fn process(&mut self, input: [&[f32]; 2], output: [&mut [f32]; 2], _: &[plugin::Event], transport: &Transport) {
    self.0.lock().unwrap().push((*transport, input[0].len()));
    for (output, input) in output.into_iter().zip(input) { output.copy_from_slice(input); }
  }
}

fn track(engine: &Engine, mixer: &daw::Mixer, number: u32, source: SamplesBuffer<f32>, at: u64) -> Box<Voice> {
  engine.voice(source, Some(1), at, Route::Track(mixer.track_route(number)))
}

#[test]
fn test_track_buses_mix_as_gain_per_voice() {
  let mixer = daw::Mixer::default();
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
    let volume = daw::db_to_gain(controls.volume_db());
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
  let mixer = daw::Mixer::default();
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
  let mixer = daw::Mixer::default();
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
  let mixer = daw::Mixer::default();
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
  let mixer = daw::Mixer::default();
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
  let mixer = daw::Mixer::default();
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
  let mixer = daw::Mixer::default();
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
  let mixer = daw::Mixer::default();
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
  let quieter = 0.5 * daw::db_to_gain(-6.);
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
  fn process(&mut self, _: [&[f32]; 2], output: [&mut [f32]; 2], events: &[plugin::Event], _: &Transport) {
    let [left, right] = output;
    let mut next = events.iter().peekable();
    for i in 0..left.len() {
      while let Some(event) = next.next_if(|e| e.offset() as usize <= i) {
        match *event {
          plugin::Event::NoteOn { pitch, .. } => self.held[pitch as usize] = true,
          plugin::Event::NoteOff { pitch, .. } => self.held[pitch as usize] = false,
        }
      }
      let level = if self.held.iter().any(|&h| h) { 0.25 } else { 0. };
      left[i] = level;
      right[i] = level;
    }
  }
}

// an engine with a Tone at the head of track 0's chain, and the mixer
fn with_tone() -> (Engine, Renderer, daw::Mixer) {
  let mixer = daw::Mixer::default();
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
  let fader = daw::db_to_gain(-6.);

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
  let mixer = daw::Mixer::default();
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
  let mixer = daw::Mixer::default();
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
  let mixer = daw::Mixer::default();
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
  fn process(&mut self, input: [&[f32]; 2], output: [&mut [f32]; 2], _: &[plugin::Event], _: &Transport) {
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
  fn process(&mut self, input: [&[f32]; 2], output: [&mut [f32]; 2], _: &[plugin::Event], _: &Transport) {
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
  let mixer = daw::Mixer::default();
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
    let mixer = daw::Mixer::default();
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
  let mixer = daw::Mixer::default();
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
  let mixer = daw::Mixer::default();
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
  let mixer = daw::Mixer::default();
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
  let mixer = daw::Mixer::default();
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
  assert!(out[20..25].iter().all(|&s| close(s, daw::soft_limit(0.5))), "{:?}", &out[20..25]);
  assert_eq!(frames.load(Ordering::Relaxed), REST + 100);
}

#[test]
fn test_a_bus_with_no_chain_is_as_it_was() {
  // nothing to rest: a voice two seconds into silence plays as ever
  let mixer = daw::Mixer::default();
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
    engine.voice(daw::METRONOME_TICK_SOURCE.convert_samples(), Some(99), 0, Route::Direct),
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
