// the tape's transport: how fast the tape moves past the heads, as a
// multiple of the right speed (about 1.0). one per tape, shared by every
// track on it: one tape, one capstan, so they all wobble together (D10)
//
// it's a physical model (T11), not a sum of sines:
//
//   motor and servo    the capstan's flywheel (inertia) driven by a motor
//                      whose servo (a PI loop) holds the speed. its
//                      reference wanders slowly (drift), and the load's
//                      friction varies (wow, through the loop)
//   reels              an off-centre reel tugs the tape once a turn. a
//                      reel turns faster the less tape it holds, so this
//                      wow's rate follows the tape's position
//   capstan, rollers   each turning part a little off-centre speeds and
//                      slows the tape once a turn (the sharp flutter
//                      peaks); they turn with the tape, so their rates
//                      follow its speed
//   tape stretch       friction at the heads shakes the springy span of
//                      tape between head and capstan (broad flutter)
//   scrape             the same at audio rate: fast, noisy, tiny
//   stick-slip         sticky old tape catching on the head, then
//                      springing free (off unless set)
//
// the slow parts run at a control rate of about 1 khz, interpolated to
// audio rate; scrape runs at audio rate. stop() and start() cut and
// restore the motor: the flywheel coasts down and spins up (part E's tape
// stop and start will use them)
//
// from_measured() sets its strengths from a deck's measured wow and
// flutter (tools/tape-model measure.py), so the bands measure.py reads
// come out as measured
//
// nothing allocates after new()

use std::f64::consts::PI;

// the most measured flutter peaks it takes (measure.py keeps 8)
pub const MAX_ROTORS: usize = 8;
// the slow parts' rate, roughly: a whole number of audio frames a step
pub const CONTROL_HZ: f64 = 1000.0;

// how measure.py reads wow and flutter: the speed on a 2 khz grid, over
// the 30 s flutter tone less half a second each end, its mean removed,
// then 2nd-order butterworth bands run forwards and back
const MEASURED_SECONDS: f64 = 29.0;
const GRID_HZ: f64 = 2000.0;
const DRIFT_HZ: f64 = 0.5;
const WOW_HZ: (f64, f64) = (0.5, 6.0);
const FLUTTER_HZ: (f64, f64) = (6.0, 200.0);

// with no reel peak among the measured ones (they're below 1 hz on a
// slow deck, where measure.py doesn't look), the reels get this share of
// the measured wow; the servo's friction the rest
const REEL_SHARE: f64 = 0.5;
// reset() runs the slow parts this long first, so the slow noise is
// already wandering (from rest, the drift would take seconds to start)
const WARM_UP_SECONDS: f64 = 5.0;
// the wow & flutter knob glides this fast (one-pole), so turning it
// doesn't step the speed
const AMOUNT_SECONDS: f64 = 0.05;
// a stuck tape springs free over this many times as long as it stuck
const RELEASE_PER_STICK: f64 = 1.5;

// one turning part: its rate at the right speed, and how much it speeds
// and slows the tape (a sine's peak, as a fraction of the speed)
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rotor {
  pub hz: f32,
  pub depth: f32,
}

// the transport's physical values. the noise strengths (drift, friction,
// stretch, scrape) are each the rms speed deviation that part makes on
// its own, at every frequency it reaches; measure.py's bands see less of
// them (see from_measured)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TransportSettings {
  // the mean speed's error (0.003: 0.3 % fast)
  pub speed_error: f32,

  // the tape: its speed past the heads (m/s; a 4-track runs at 9.53 cm/s),
  // how long a side plays, and the reels' radius empty and full (m)
  pub tape_speed: f32,
  pub side_seconds: f32,
  pub hub_radius: f32,
  pub full_radius: f32,
  // how much each reel's tug speeds and slows the tape (a sine's peak)
  pub reel_depth: f32,

  // the servo: its loop's natural frequency and damping
  pub servo_hz: f32,
  pub servo_damping: f32,
  // how long the flywheel takes to spin up, and to coast to a stop
  pub start_seconds: f32,
  pub stop_seconds: f32,
  // the servo's reference wandering: rms, and the band it wanders in (any
  // slower is a lasting speed error, the engine's varispeed, not wobble)
  pub drift: f32,
  pub drift_hz: (f32, f32),
  // the load's friction varying, through the loop: rms, and how fast it
  // varies
  pub friction: f32,
  pub friction_hz: f32,

  // the turning parts (unused ones have depth 0)
  pub rotors: [Rotor; MAX_ROTORS],

  // the tape span's shake: rms, its resonance, and the resonance's q
  pub stretch: f32,
  pub stretch_hz: f32,
  pub stretch_q: f32,
  // scrape flutter: rms, and where it's centred
  pub scrape: f32,
  pub scrape_hz: f32,

  // sticky tape: how often it catches, how much it slows the tape (a
  // fraction of the speed), and how long it holds
  pub stick_slips_per_minute: f32,
  pub stick_depth: (f32, f32),
  pub stick_seconds: (f32, f32),
}

// a fresh cassette on a 4-track: about the fake deck's values (wow
// 0.07 %, flutter 0.02 % with peaks at 7.3 and 21 hz, a little drift),
// no speed error (that's the engine's to add, not an insert's)
impl Default for TransportSettings {
  fn default() -> TransportSettings {
    TransportSettings::from_measured(0.0, 3.0e-4, 7.0e-4, 2.0e-4, &[(7.3, 3.0e-4), (21.0, 1.2e-4)])
  }
}

impl TransportSettings {
  // the physical parts with nothing wobbling: what from_measured starts
  // from. the values are a portastudio's at 3¾ ips on a c60, guessed
  pub fn still() -> TransportSettings {
    TransportSettings {
      speed_error: 0.0,
      tape_speed: 0.0953,
      side_seconds: 900.0,
      hub_radius: 0.011,
      full_radius: 0.0247,
      reel_depth: 0.0,
      servo_hz: 2.0,
      servo_damping: 0.7,
      start_seconds: 0.5,
      stop_seconds: 0.6,
      drift: 0.0,
      drift_hz: (0.02, 0.12),
      friction: 0.0,
      friction_hz: 3.0,
      rotors: [Rotor::default(); MAX_ROTORS],
      stretch: 0.0,
      stretch_hz: 35.0,
      stretch_q: 0.7,
      scrape: 4.0e-5,
      scrape_hz: 1500.0,
      stick_slips_per_minute: 0.0,
      stick_depth: (0.02, 0.08),
      stick_seconds: (0.01, 0.05),
    }
  }

  // fitted to a deck's measured wow and flutter (measure.py's
  // wow_flutter: the rms of the speed's deviation in each band, and its
  // peaks as (hz, depth), depth a sine's peak). the speed error is the
  // measured mean speed less 1
  //
  // each peak is a turning part at that rate, unless it's at a rate a reel
  // turns at, when it sets the reels' depth. what the parts and reels
  // leave of each band is made up by the three noises (drift, friction,
  // stretch), solved together since each reaches a little into the
  // others' bands
  pub fn from_measured(speed_error: f32, drift_rms: f32, wow_rms: f32, flutter_rms: f32, peaks: &[(f32, f32)]) -> TransportSettings {
    let mut s = TransportSettings::still();
    s.speed_error = speed_error;
    let speed = 1.0 + speed_error as f64;
    let mut left = [(drift_rms as f64).powi(2), (wow_rms as f64).powi(2), (flutter_rms as f64).powi(2)];

    // the reels' rates over a side, at the measured speed
    let (slowest, fastest) = (s.reel_hz(s.full_radius) * speed, s.reel_hz(s.hub_radius) * speed);
    let mut reel_peak: f64 = 0.0;
    let mut n = 0;
    for &(hz, depth) in peaks {
      let (hz, depth) = (hz as f64, depth as f64);
      if hz >= 0.9 * slowest && hz <= 1.1 * fastest {
        reel_peak = reel_peak.max(depth);
        continue;
      }
      if n == MAX_ROTORS || depth <= 0.0 {
        continue;
      }
      // its rate at the right speed: measured, it turned at the deck's
      s.rotors[n] = Rotor { hz: (hz / speed) as f32, depth: depth as f32 };
      n += 1;
      for (b, l) in left.iter_mut().enumerate() {
        *l -= depth * depth / 2.0 * measured(b, hz) * interpolated(hz, CONTROL_HZ);
      }
    }

    // the reels, as heard over a whole side: each band's share of a unit
    // reel tug, averaged over where the tape might be
    let mut reel_share = [0.0; 3];
    let steps = 64;
    for i in 0..steps {
      let p = (i as f64 + 0.5) / steps as f64;
      let (supply, take_up) = s.radii(p);
      for r in [supply, take_up] {
        let hz = s.tape_speed as f64 * speed / (2.0 * PI * r);
        for (b, share) in reel_share.iter_mut().enumerate() {
          *share += 0.5 * measured(b, hz) * interpolated(hz, CONTROL_HZ) / steps as f64;
        }
      }
    }
    let reel_power = if reel_peak > 0.0 { reel_peak * reel_peak } else { REEL_SHARE * left[1].max(0.0) / reel_share[1].max(1e-9) };
    s.reel_depth = reel_power.sqrt() as f32;
    for (l, share) in left.iter_mut().zip(reel_share) {
      *l = (*l - reel_power * share).max(0.0);
    }

    // the noises: each band's share of each at unit rms
    let shapes = Shapes::new(&s, CONTROL_HZ);
    let mut a = [[0.0; 3]; 3];
    for (c, source) in [Source::Drift, Source::Friction, Source::Stretch].into_iter().enumerate() {
      let var = shapes.variance(source);
      for (b, row) in a.iter_mut().enumerate() {
        row[c] = integrate(CONTROL_HZ / 2.0, |f| shapes.power(source, f) * 2.0 / CONTROL_HZ * measured(b, f) * interpolated(f, CONTROL_HZ)) / var;
      }
    }
    let x = nonnegative_solve(&a, &left);
    s.drift = x[0].sqrt() as f32;
    s.friction = x[1].sqrt() as f32;
    s.stretch = x[2].sqrt() as f32;
    s
  }

  // a reel's turns a second at radius r, at the right speed
  fn reel_hz(&self, r: f32) -> f64 {
    self.tape_speed as f64 / (2.0 * PI * r as f64)
  }

  // the supply and take-up reels' radii, `p` of the way through the side:
  // the tape's area moves from one to the other
  fn radii(&self, p: f64) -> (f64, f64) {
    let (hub, full) = ((self.hub_radius as f64).powi(2), (self.full_radius as f64).powi(2));
    let moved = (full - hub) * p.clamp(0.0, 1.0);
    ((full - moved).sqrt(), (hub + moved).sqrt())
  }
}

// ---- the transport

pub struct TapeTransport {
  settings: TransportSettings,
  rate: f64,
  seed: u64,
  // audio frames a control step
  period: usize,
  shapes: Shapes,
  // each noise's gain for unit rms
  gains: [f64; 3],
  scrape_gain: f64,
  scrape_coefs: Biquad,
  amount_target: f64,
  state: State,
}

// everything that moves, so reset() can put it all back
#[derive(Clone)]
struct State {
  rng: Rng,
  amount: f64,
  motor_on: bool,
  // the flywheel's speed, and the servo's integral
  omega: f64,
  integral: f64,
  // the reference wander's filters (high-pass in and out, two low-passes)
  hp_in: f64,
  hp_out: f64,
  drift: [f64; 2],
  friction: [f64; 2],
  stretch: [f64; 2],
  rotor_phase: [f64; MAX_ROTORS],
  // seconds into the side (at the right speed), and each reel's angle
  position: f64,
  reel_phase: [f64; 2],
  // stick-slip: seconds to the next, how long this one still holds, how
  // much it slows the tape, how far the tape at the head has fallen behind
  // the capstan (seconds of tape), and how fast it springs back
  next_stick: f64,
  sticking: f64,
  stick_depth: f64,
  lag: f64,
  release: f64,
  // the control points either side of now, and how far between them
  prev: f64,
  next: f64,
  step: usize,
  scrape: [f64; 2],
}

impl TapeTransport {
  pub fn new(settings: TransportSettings, rate: f32, seed: u64) -> TapeTransport {
    let rate = rate.max(1.0) as f64;
    let period = ((rate / CONTROL_HZ).round() as usize).max(1);
    let control = rate / period as f64;
    let mut t = TapeTransport {
      settings,
      rate,
      seed,
      period,
      shapes: Shapes::new(&settings, control),
      gains: [0.0; 3],
      scrape_gain: 0.0,
      scrape_coefs: Biquad::band_pass(settings.scrape_hz as f64, 0.7, rate),
      amount_target: 1.0,
      state: State::new(seed),
    };
    t.retune(true);
    t.reset();
    t
  }

  // new values, keeping where everything is. no allocation; working out
  // the noises' gains again (a few thousand complex sums) only when their
  // shapes change
  pub fn set_settings(&mut self, settings: TransportSettings) {
    let old = self.settings;
    let old_drag = self.shapes.drag;
    self.settings = settings;
    let reshaped = old.drift_hz != settings.drift_hz
      || old.friction_hz != settings.friction_hz
      || old.servo_hz != settings.servo_hz
      || old.servo_damping != settings.servo_damping
      || old.stretch_hz != settings.stretch_hz
      || old.stretch_q != settings.stretch_q
      || old.scrape_hz != settings.scrape_hz;
    if reshaped {
      let control = self.rate / self.period as f64;
      self.shapes = Shapes::new(&settings, control);
      self.scrape_coefs = Biquad::band_pass(settings.scrape_hz as f64, 0.7, self.rate);
    }
    self.retune(reshaped);
    let st = &mut self.state;
    st.position = st.position.min(settings.side_seconds.max(1.0) as f64);
    // the servo's integral holds the drag (and the load's slow part): move
    // it by as much as the drag changed, so the speed doesn't jump
    st.integral += self.shapes.drag - old_drag;
  }

  pub fn settings(&self) -> &TransportSettings {
    &self.settings
  }

  // the wow & flutter knob: 0 holds the speed steady, 1 is as measured,
  // 2 twice. it glides there
  pub fn set_amount(&mut self, amount: f32) {
    self.amount_target = amount.max(0.0) as f64;
  }

  // cut the motor: the flywheel coasts to a stop
  pub fn stop(&mut self) {
    self.state.motor_on = false;
  }

  // power the motor: the flywheel spins up and the servo locks
  pub fn start(&mut self) {
    self.state.motor_on = true;
  }

  pub fn running(&self) -> bool {
    self.state.motor_on
  }

  // the speed it's held at, on average (0 with the motor off): what
  // playback reads the tape against
  pub fn mean_speed(&self) -> f64 {
    if self.state.motor_on { 1.0 + self.settings.speed_error as f64 } else { 0.0 }
  }

  // seconds into the side, at the right speed
  pub fn tape_position(&self) -> f64 {
    self.state.position
  }

  // back to how new() left it (running, at speed): the same seed, so the
  // same wobble again. it runs WARM_UP_SECONDS of the slow parts, about
  // half a millisecond
  pub fn reset(&mut self) {
    self.state = State::new(self.seed);
    let st = &mut self.state;
    st.amount = self.amount_target;
    st.position = st.rng.uniform() * self.settings.side_seconds.max(1.0) as f64;
    for p in st.rotor_phase.iter_mut().chain(st.reel_phase.iter_mut()) {
      *p = st.rng.uniform() * 2.0 * PI;
    }
    st.omega = 1.0 + self.settings.speed_error as f64;
    st.integral = self.shapes.drag;
    st.next_stick = st.rng.wait(self.settings.stick_slips_per_minute);
    let warm = (WARM_UP_SECONDS * self.rate / self.period as f64) as usize;
    for _ in 0..warm {
      self.control_step();
    }
    self.state.prev = self.control_step();
    self.state.next = self.control_step();
    self.state.step = 0;
  }

  // the speed for each of the next out.len() audio frames
  pub fn fill_speed(&mut self, out: &mut [f32]) {
    let per = 1.0 / self.period as f64;
    let scrape_on = self.settings.scrape > 0.0;
    for o in out.iter_mut() {
      if self.state.step == self.period {
        self.state.prev = self.state.next;
        self.state.next = self.control_step();
        self.state.step = 0;
      }
      let st = &mut self.state;
      let k = st.step as f64 * per;
      let mut v = st.prev + (st.next - st.prev) * k;
      if scrape_on {
        let x = st.rng.bipolar() as f64 * 3f64.sqrt();
        v += self.scrape_coefs.run(&mut st.scrape, x) * self.scrape_gain * st.amount * st.omega;
      }
      st.step += 1;
      *o = v.max(0.0) as f32;
    }
  }

  // the gains that give each noise its rms (only worked out again when
  // the shapes change)
  fn retune(&mut self, reshaped: bool) {
    let s = &self.settings;
    if reshaped {
      self.shapes.unit = [
        1.0 / self.shapes.variance(Source::Drift).sqrt(),
        1.0 / self.shapes.variance(Source::Friction).sqrt(),
        1.0 / self.shapes.variance(Source::Stretch).sqrt(),
      ];
      let coefs = self.scrape_coefs;
      self.shapes.scrape_unit = 1.0 / integrate(self.rate / 2.0, |f| coefs.power(f, self.rate) * 2.0 / self.rate).sqrt();
    }
    self.shapes.drag = 1.0 / (s.stop_seconds.max(0.01) as f64);
    self.shapes.torque_max = self.shapes.drag + 1.0 / (s.start_seconds.max(0.01) as f64);
    let u = self.shapes.unit;
    self.gains = [s.drift as f64 * u[0], s.friction as f64 * u[1], s.stretch as f64 * u[2]];
    // uniform noise times √3 has unit variance
    self.scrape_gain = s.scrape as f64 * self.shapes.scrape_unit;
  }

  // one control step: the speed at its end
  fn control_step(&mut self) -> f64 {
    let s = self.settings;
    let sh = &self.shapes;
    let t = sh.dt;
    let gains = self.gains;
    let st = &mut self.state;

    if st.amount != self.amount_target {
      st.amount += (self.amount_target - st.amount) * sh.amount_k;
      if (st.amount - self.amount_target).abs() < 1e-6 {
        st.amount = self.amount_target;
      }
    }
    let amt = st.amount;
    let base = 1.0 + s.speed_error as f64;

    // the reference's wander: white, high-passed (slower is a lasting
    // error, not wobble), low-passed twice
    let w = st.rng.gauss();
    st.hp_out = sh.hp_a * (st.hp_out + w - st.hp_in);
    st.hp_in = w;
    st.drift[0] += sh.drift_a * (st.hp_out - st.drift[0]);
    st.drift[1] += sh.drift_a * (st.drift[0] - st.drift[1]);
    // the load's friction: white, low-passed twice
    st.friction[0] += sh.friction_a * (st.rng.gauss() - st.friction[0]);
    st.friction[1] += sh.friction_a * (st.friction[0] - st.friction[1]);
    let reference = base * (1.0 + amt * gains[0] * st.drift[1]);
    let load = amt * gains[1] * st.friction[1];

    // the motor: a pi servo on the flywheel's speed, its torque limited
    // (so it spins up at a steady rate), against a steady drag (so it
    // coasts down when cut). the integral only moves when the torque
    // isn't at a limit, so it doesn't wind up
    let mut torque = 0.0;
    if st.motor_on {
      let e = reference - st.omega;
      let integral = st.integral + sh.ki_t * e;
      let want = sh.kp * e + integral;
      if want > sh.torque_max {
        torque = sh.torque_max;
      } else if want < 0.0 {
        torque = 0.0;
      } else {
        torque = want;
        st.integral = integral;
      }
    } else {
      st.integral = sh.drag;
    }
    // (standing still, it stays put until the motor beats the drag)
    let net = torque - load;
    if st.omega > 0.0 || net > sh.drag {
      st.omega = (st.omega + t * (net - sh.drag)).max(0.0);
    }
    let omega = st.omega;

    // the tape span's shake
    let stretch = sh.stretch.run(&mut st.stretch, st.rng.gauss()) * gains[2];

    // the turning parts, each turning with the tape
    let mut wobble = stretch;
    for (r, phase) in s.rotors.iter().zip(st.rotor_phase.iter_mut()) {
      if r.depth != 0.0 {
        *phase += 2.0 * PI * r.hz as f64 * omega * t;
        if *phase > 2.0 * PI {
          *phase -= 2.0 * PI;
        }
        wobble += r.depth as f64 * phase.sin();
      }
    }

    // the reels: each turns at the tape's speed over its radius
    let side = s.side_seconds.max(1.0) as f64;
    st.position += omega * t;
    if st.position >= side {
      // the side's run out: turn the cassette over. the full reel is now
      // the supply, so the reels swap places
      st.position -= side;
      st.reel_phase.swap(0, 1);
    }
    let radii = s.radii(st.position / side);
    for (phase, r) in st.reel_phase.iter_mut().zip([radii.0, radii.1]) {
      *phase += omega * s.tape_speed as f64 / r * t;
      if *phase > 2.0 * PI {
        *phase -= 2.0 * PI;
      }
      wobble += s.reel_depth as f64 * phase.sin();
    }

    // stick-slip: the tape catches on the head and falls behind the
    // capstan, then springs free and catches up (the capstan pins it, so
    // what it lost it makes back)
    if omega > 0.0 {
      st.next_stick -= t;
      if st.next_stick <= 0.0 {
        let (lo, hi) = (s.stick_seconds.0 as f64, s.stick_seconds.1 as f64);
        st.sticking = st.rng.range(lo, hi.max(lo));
        st.release = (st.sticking * RELEASE_PER_STICK).max(t);
        st.stick_depth = st.rng.range(s.stick_depth.0 as f64, (s.stick_depth.1 as f64).max(s.stick_depth.0 as f64));
        st.next_stick = st.rng.wait(s.stick_slips_per_minute);
      }
    }
    let behind = if st.sticking > 0.0 {
      st.sticking -= t;
      st.stick_depth
    } else if st.lag > 1e-12 {
      -st.lag / st.release
    } else {
      st.lag = 0.0;
      0.0
    };
    st.lag += behind * t;
    wobble -= behind;

    omega * (1.0 + amt * wobble)
  }
}

impl State {
  fn new(seed: u64) -> State {
    State {
      rng: Rng::new(seed),
      amount: 1.0,
      motor_on: true,
      omega: 1.0,
      integral: 0.0,
      hp_in: 0.0,
      hp_out: 0.0,
      drift: [0.0; 2],
      friction: [0.0; 2],
      stretch: [0.0; 2],
      rotor_phase: [0.0; MAX_ROTORS],
      position: 0.0,
      reel_phase: [0.0; 2],
      next_stick: f64::INFINITY,
      sticking: 0.0,
      stick_depth: 0.0,
      lag: 0.0,
      release: 1.0,
      prev: 1.0,
      next: 1.0,
      step: 0,
      scrape: [0.0; 2],
    }
  }
}

// ---- the noises' shapes: their filters, and what they do to white noise

#[derive(Clone, Copy, Debug, PartialEq)]
enum Source {
  Drift,
  Friction,
  Stretch,
}

// the slow parts' coefficients at the control rate. the simulation runs
// exactly these, and power() is their exact response, so the gains (and
// from_measured's fit) are right at any rate
#[derive(Clone, Copy, Debug)]
struct Shapes {
  control: f64,
  dt: f64,
  hp_a: f64,
  drift_a: f64,
  friction_a: f64,
  // the servo (inertia 1: torque is acceleration)
  kp: f64,
  ki_t: f64,
  drag: f64,
  torque_max: f64,
  stretch: Biquad,
  amount_k: f64,
  unit: [f64; 3],
  scrape_unit: f64,
}

impl Shapes {
  fn new(s: &TransportSettings, control: f64) -> Shapes {
    let dt = 1.0 / control;
    let one_pole = |hz: f32| 1.0 - (-2.0 * PI * (hz.max(1e-4) as f64) * dt).exp();
    let wn = 2.0 * PI * s.servo_hz.max(0.01) as f64;
    Shapes {
      control,
      dt,
      hp_a: (-2.0 * PI * (s.drift_hz.0.max(1e-5) as f64) * dt).exp(),
      drift_a: one_pole(s.drift_hz.1),
      friction_a: one_pole(s.friction_hz),
      kp: 2.0 * s.servo_damping.max(0.05) as f64 * wn,
      ki_t: wn * wn * dt,
      drag: 1.0 / (s.stop_seconds.max(0.01) as f64),
      torque_max: 1.0,
      stretch: Biquad::band_pass(s.stretch_hz as f64, s.stretch_q as f64, control),
      amount_k: 1.0 - (-dt / AMOUNT_SECONDS).exp(),
      unit: [1.0; 3],
      scrape_unit: 1.0,
    }
  }

  // the power gain from unit white noise (at the control rate) to the
  // speed, at f hz
  fn power(&self, source: Source, f: f64) -> f64 {
    let z1 = C::expj(-2.0 * PI * f / self.control);
    let one = C::new(1.0, 0.0);
    let low = |a: f64| C::new(a, 0.0) / (one - z1 * (1.0 - a));
    // the servo: speed from its reference, and from a load torque
    let pi = C::new(self.kp, 0.0) + C::new(self.ki_t, 0.0) / (one - z1);
    let z = one / z1;
    let den = z - one + pi * self.dt;
    match source {
      Source::Drift => {
        let hp = (one - z1) * self.hp_a / (one - z1 * self.hp_a);
        let lp = low(self.drift_a);
        (hp * lp * lp * pi * self.dt / den).norm()
      }
      Source::Friction => {
        let lp = low(self.friction_a);
        (lp * lp * self.dt / den).norm()
      }
      Source::Stretch => self.stretch.power(f, self.control),
    }
  }

  // the variance unit white noise comes out with
  fn variance(&self, source: Source) -> f64 {
    integrate(self.control / 2.0, |f| self.power(source, f) * 2.0 / self.control)
  }
}

// how much of a frequency's power measure.py puts in band b (0 drift,
// 1 wow, 2 flutter): its butterworth band, run forwards and back (so its
// power response squared), less what removing the 29 s mean takes
fn measured(band: usize, f: f64) -> f64 {
  let warp = |f: f64| 2.0 * GRID_HZ * (PI * f.min(GRID_HZ * 0.4999) / GRID_HZ).tan();
  let w = warp(f);
  let p = match band {
    0 => 1.0 / (1.0 + (w / warp(DRIFT_HZ)).powi(4)),
    _ => {
      let (lo, hi) = if band == 1 { WOW_HZ } else { FLUTTER_HZ };
      let (lo, hi) = (warp(lo), warp(hi));
      let x = (w * w - lo * hi) / (w * (hi - lo)).max(1e-30);
      1.0 / (1.0 + x.powi(4))
    }
  };
  let x = PI * f * MEASURED_SECONDS;
  let sinc = if x < 1e-9 { 1.0 } else { x.sin() / x };
  p * p * (1.0 - sinc * sinc)
}

// what the control rate's straight-line interpolation leaves of f's power
fn interpolated(f: f64, control: f64) -> f64 {
  let x = PI * f / control;
  if x < 1e-9 { 1.0 } else { (x.sin() / x).powi(4) }
}

// the integral of g from 0 to `top` hz, on a log grid (the shapes reach
// from hundredths of a hz to hundreds)
fn integrate(top: f64, g: impl Fn(f64) -> f64) -> f64 {
  const LOW: f64 = 1e-5;
  const PER_DECADE: f64 = 120.0;
  let n = ((top / LOW).log10() * PER_DECADE).ceil() as usize;
  let step = (top / LOW).ln() / n as f64;
  // below the grid, flat
  let mut sum = g(LOW) * LOW;
  let mut prev = g(LOW) * LOW;
  for i in 1..=n {
    let f = LOW * (step * i as f64).exp();
    let here = g(f) * f;
    sum += 0.5 * (prev + here) * step;
    prev = here;
  }
  sum
}

// the non-negative x that best solves a x = b (3 unknowns): every subset
// of them tried, the best that needs none negative kept
fn nonnegative_solve(a: &[[f64; 3]; 3], b: &[f64; 3]) -> [f64; 3] {
  let mut best = ([0.0; 3], b.iter().map(|v| v * v).sum::<f64>());
  for mask in 1u32..8 {
    let mut cols = [0usize; 3];
    let mut k = 0;
    for c in 0..3 {
      if mask & (1 << c) != 0 {
        cols[k] = c;
        k += 1;
      }
    }
    let cols = &cols[..k];
    // the normal equations, on the chosen columns
    let mut m = [[0.0; 4]; 3];
    for (i, &ci) in cols.iter().enumerate() {
      for (j, &cj) in cols.iter().enumerate() {
        m[i][j] = (0..3).map(|r| a[r][ci] * a[r][cj]).sum();
      }
      m[i][k] = (0..3).map(|r| a[r][ci] * b[r]).sum();
    }
    let Some(sol) = gauss_solve(&mut m, k) else { continue };
    if sol[..k].iter().any(|&v| v < 0.0) {
      continue;
    }
    let mut x = [0.0; 3];
    for (i, &c) in cols.iter().enumerate() {
      x[c] = sol[i];
    }
    let err: f64 = (0..3).map(|r| (a[r].iter().zip(&x).map(|(p, q)| p * q).sum::<f64>() - b[r]).powi(2)).sum();
    if err < best.1 {
      best = (x, err);
    }
  }
  best.0
}

fn gauss_solve(m: &mut [[f64; 4]; 3], k: usize) -> Option<[f64; 3]> {
  for col in 0..k {
    let pivot = (col..k).max_by(|&i, &j| m[i][col].abs().total_cmp(&m[j][col].abs()))?;
    if m[pivot][col].abs() < 1e-30 {
      return None;
    }
    m.swap(col, pivot);
    for row in 0..k {
      if row != col {
        let f = m[row][col] / m[col][col];
        for c in col..=k {
          m[row][c] -= f * m[col][c];
        }
      }
    }
  }
  let mut x = [0.0; 3];
  for i in 0..k {
    x[i] = m[i][k] / m[i][i];
  }
  Some(x)
}

// ---- small helpers playback shares

// a biquad's coefficients (normalized, a0 = 1), run in transposed direct
// form 2 on a state of two
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Biquad {
  b: [f64; 3],
  a: [f64; 2],
}

impl Biquad {
  // a band-pass with 0 db at its peak (the rbj cookbook's)
  pub(crate) fn band_pass(hz: f64, q: f64, rate: f64) -> Biquad {
    let w = 2.0 * PI * hz.clamp(1e-3, rate * 0.49) / rate;
    let alpha = w.sin() / (2.0 * q.max(0.05));
    let a0 = 1.0 + alpha;
    Biquad { b: [alpha / a0, 0.0, -alpha / a0], a: [-2.0 * w.cos() / a0, (1.0 - alpha) / a0] }
  }

  #[inline]
  pub(crate) fn run(&self, s: &mut [f64; 2], x: f64) -> f64 {
    let y = self.b[0] * x + s[0];
    s[0] = self.b[1] * x - self.a[0] * y + s[1];
    s[1] = self.b[2] * x - self.a[1] * y;
    y
  }

  pub(crate) fn power(&self, f: f64, rate: f64) -> f64 {
    let z1 = C::expj(-2.0 * PI * f / rate);
    let z2 = z1 * z1;
    let num = C::new(self.b[0], 0.0) + z1 * self.b[1] + z2 * self.b[2];
    let den = C::new(1.0, 0.0) + z1 * self.a[0] + z2 * self.a[1];
    (num / den).norm()
  }
}

// a complex number, for filters' responses
#[derive(Clone, Copy, Debug)]
pub(crate) struct C {
  pub(crate) re: f64,
  pub(crate) im: f64,
}

impl C {
  pub(crate) fn new(re: f64, im: f64) -> C {
    C { re, im }
  }

  pub(crate) fn expj(w: f64) -> C {
    C { re: w.cos(), im: w.sin() }
  }

  // |c|²
  pub(crate) fn norm(self) -> f64 {
    self.re * self.re + self.im * self.im
  }
}

impl std::ops::Add for C {
  type Output = C;
  fn add(self, o: C) -> C {
    C::new(self.re + o.re, self.im + o.im)
  }
}

impl std::ops::Sub for C {
  type Output = C;
  fn sub(self, o: C) -> C {
    C::new(self.re - o.re, self.im - o.im)
  }
}

impl std::ops::Mul for C {
  type Output = C;
  fn mul(self, o: C) -> C {
    C::new(self.re * o.re - self.im * o.im, self.re * o.im + self.im * o.re)
  }
}

impl std::ops::Mul<f64> for C {
  type Output = C;
  fn mul(self, k: f64) -> C {
    C::new(self.re * k, self.im * k)
  }
}

impl std::ops::Div for C {
  type Output = C;
  fn div(self, o: C) -> C {
    let d = o.norm();
    C::new((self.re * o.re + self.im * o.im) / d, (self.im * o.re - self.re * o.im) / d)
  }
}

impl std::ops::Div<f64> for C {
  type Output = C;
  fn div(self, k: f64) -> C {
    C::new(self.re / k, self.im / k)
  }
}

// a small seedable random number generator: xorshift64*, seeded through
// splitmix64 so neighbouring seeds give unrelated streams. no
// dependencies, and the same numbers on every machine
#[derive(Clone, Debug)]
pub(crate) struct Rng {
  state: u64,
  spare: Option<f64>,
}

impl Rng {
  pub(crate) fn new(seed: u64) -> Rng {
    let mut z = seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^= z >> 31;
    Rng { state: if z == 0 { 1 } else { z }, spare: None }
  }

  #[inline]
  pub(crate) fn next_u64(&mut self) -> u64 {
    let mut x = self.state;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    self.state = x;
    x.wrapping_mul(0x2545_f491_4f6c_dd1d)
  }

  // in [0, 1)
  pub(crate) fn uniform(&mut self) -> f64 {
    (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
  }

  // in [-1, 1), for audio-rate noise
  #[inline]
  pub(crate) fn bipolar(&mut self) -> f32 {
    (self.next_u64() >> 40) as f32 * (2.0 / (1u32 << 24) as f32) - 1.0
  }

  pub(crate) fn range(&mut self, lo: f64, hi: f64) -> f64 {
    lo + (hi - lo) * self.uniform()
  }

  // waiting times between events that come `mean` apart on average
  pub(crate) fn exponential(&mut self, mean: f64) -> f64 {
    -mean * (1.0 - self.uniform()).ln()
  }

  // seconds to the next of events that come `per_minute` a minute
  pub(crate) fn wait(&mut self, per_minute: f32) -> f64 {
    if per_minute <= 0.0 { f64::INFINITY } else { self.exponential(60.0 / per_minute as f64) }
  }

  // normal, mean 0, variance 1 (box-muller, two at a time)
  pub(crate) fn gauss(&mut self) -> f64 {
    if let Some(v) = self.spare.take() {
      return v;
    }
    let u = 1.0 - self.uniform();
    let v = self.uniform();
    let r = (-2.0 * u.ln()).sqrt();
    self.spare = Some(r * (2.0 * PI * v).sin());
    r * (2.0 * PI * v).cos()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn measured_peaks_become_turning_parts() {
    // (whether the bands come out as measured: tests/transport.rs)
    let s = TransportSettings::from_measured(0.003, 4.8e-4, 6.7e-4, 1.7e-4, &[(7.32, 3.0e-4), (21.0, 1.2e-4)]);
    assert!(s.drift > 0.0 && s.friction > 0.0 && s.stretch >= 0.0, "{s:?}");
    assert_eq!(s.rotors[0].depth, 3.0e-4);
    assert!((s.rotors[0].hz - 7.32 / 1.003).abs() < 1e-4);
    assert_eq!(s.rotors[2].depth, 0.0);
  }

  #[test]
  fn a_peak_where_a_reel_turns_goes_to_the_reels() {
    let s = TransportSettings::from_measured(0.0, 3.0e-4, 7.0e-4, 2.0e-4, &[(1.2, 4.0e-4), (7.3, 3.0e-4)]);
    assert_eq!(s.reel_depth, 4.0e-4);
    assert_eq!(s.rotors[0].hz, 7.3);
    assert_eq!(s.rotors[1].depth, 0.0);
  }

  #[test]
  fn the_same_seed_wobbles_the_same() {
    let s = TransportSettings::default();
    let (mut a, mut b) = (TapeTransport::new(s, 48_000.0, 3), TapeTransport::new(s, 48_000.0, 3));
    let (mut x, mut y) = (vec![0.0; 4800], vec![0.0; 4800]);
    a.fill_speed(&mut x);
    b.fill_speed(&mut y[..1000]);
    b.fill_speed(&mut y[1000..]);
    assert_eq!(x, y);
    a.reset();
    a.fill_speed(&mut y);
    assert_eq!(x, y);
    let mut c = TapeTransport::new(s, 48_000.0, 4);
    c.fill_speed(&mut y);
    assert_ne!(x, y);
  }

  #[test]
  fn at_amount_0_it_holds_its_speed() {
    let s = TransportSettings { speed_error: 0.003, stick_slips_per_minute: 60.0, ..TransportSettings::default() };
    let mut t = TapeTransport::new(s, 48_000.0, 9);
    t.set_amount(0.0);
    t.reset();
    let mut v = vec![0.0; 48_000];
    t.fill_speed(&mut v);
    assert!(v.iter().all(|&x| x == 1.003f32), "{:?}", v.iter().find(|&&x| x != 1.003f32));
  }

  #[test]
  fn new_settings_dont_jolt_the_speed() {
    // a longer coast changes the drag the servo holds: against a twin left
    // alone, the speed mustn't move
    let (mut t, mut twin) = (TapeTransport::new(TransportSettings::default(), 8_000.0, 2), TapeTransport::new(TransportSettings::default(), 8_000.0, 2));
    let (mut v, mut w) = (vec![0.0; 8_000], vec![0.0; 8_000]);
    t.fill_speed(&mut v);
    twin.fill_speed(&mut w);
    t.set_settings(TransportSettings { stop_seconds: 2.0, ..TransportSettings::default() });
    t.fill_speed(&mut v);
    twin.fill_speed(&mut w);
    let worst = v.iter().zip(&w).map(|(a, b)| (a - b).abs()).fold(0.0, f32::max);
    assert!(worst < 1e-6, "{worst}");
  }

  #[test]
  fn it_coasts_down_and_spins_up() {
    let mut t = TapeTransport::new(TransportSettings::default(), 8_000.0, 1);
    let mut v = vec![0.0; 8_000];
    t.stop();
    t.fill_speed(&mut v);
    // the settings' stop takes 0.6 s
    assert!(v[2_400] > 0.3 && v[2_400] < 0.7, "{}", v[2_400]);
    assert_eq!(v[7_999], 0.0);
    t.start();
    t.fill_speed(&mut v);
    assert!(v[2_000] > 0.3 && v[2_000] < 0.7, "{}", v[2_000]);
    assert!((v[7_999] - 1.0).abs() < 0.01, "{}", v[7_999]);
  }

  #[test]
  fn the_random_numbers_are_even_and_normal() {
    let mut r = Rng::new(1);
    let n = 200_000;
    let (mut m, mut v) = (0.0, 0.0);
    for _ in 0..n {
      let g = r.gauss();
      m += g;
      v += g * g;
    }
    assert!((m / n as f64).abs() < 0.01 && (v / n as f64 - 1.0).abs() < 0.02);
    let u: f64 = (0..n).map(|_| r.bipolar() as f64).map(|x| x * x).sum::<f64>() / n as f64;
    assert!((u - 1.0 / 3.0).abs() < 0.005, "{u}");
  }
}
