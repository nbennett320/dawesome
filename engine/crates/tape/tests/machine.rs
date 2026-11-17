// what a tape machine may not do on the audio thread: allocate. adding a
// track allocates (it isn't done there); moving the tape, playing,
// recording live, the knobs and the drive don't

use tape::{MachineSettings, RecordSide, SimpleCore, SimpleCoreSettings, TapeMachine};

// ---- counting allocations (this test binary's allocator only; per
// thread, as tests run side by side)
mod counting {
  use std::alloc::{GlobalAlloc, Layout, System};
  use std::cell::Cell;

  thread_local! {
    static MADE: Cell<usize> = const { Cell::new(0) };
  }

  pub struct Counting;

  unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
      let _ = MADE.try_with(|made| made.set(made.get() + 1));
      System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
      System.dealloc(ptr, layout)
    }
  }

  #[global_allocator]
  static COUNTING: Counting = Counting;

  pub fn made() -> usize {
    MADE.with(|made| made.get())
  }
}

const RATE: f32 = 48_000.0;
const BLOCK: usize = 512;

#[test]
fn the_audio_thread_never_allocates() {
  let mut machine = TapeMachine::new(MachineSettings::default(), RATE, 5, BLOCK);
  let core = || Box::new(SimpleCore::new(SimpleCoreSettings::default(), RATE, BLOCK));
  let live = machine.add_track(Some(RecordSide::new(core(), RATE, BLOCK)), 0.0);
  let printed = machine.add_track(None, 0.7);
  let input: Vec<f32> = (0..BLOCK).map(|i| (i as f32 * 0.05).sin() * 0.2).collect();
  let mut out = vec![0.0; BLOCK];
  let mut quieter = machine.settings().clone();
  quieter.output_db = -3.0;
  quieter.hiss = 0.5;
  quieter.wow_flutter = 1.5;
  quieter.dbx = false;

  let before = counting::made();
  for i in 0..200 {
    machine.advance(BLOCK);
    machine.record_and_play(live, &input, &mut out);
    machine.play(printed, &input, &mut out);
    machine.set_drive(live, (i % 12) as f32);
    if i == 100 {
      // the knobs only: the profile's shapes stay as they are
      machine.set_settings(quieter.clone());
    }
  }
  machine.stop();
  machine.advance(BLOCK);
  machine.start();
  assert_eq!(counting::made() - before, 0);
}
