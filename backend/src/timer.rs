use std::sync::mpsc;
use std::sync;
use std::thread;
use std::time;

struct TimerBase {
  handle: Option<thread::JoinHandle<()>>
}

impl TimerBase {
  fn new() -> Self {
    TimerBase {
      handle: None
    }
  }

  fn timeout<F>(mut self, f: F, delay: time::Duration)
  where F: Fn() -> () + Send + Sync + 'static {
    let (tx, rx) = mpsc::channel();
    let f = sync::Arc::new(f);

    self.handle = Some(
      thread::spawn(move || {
        thread::sleep(delay);
        tx.send(f()).unwrap();
      })
    );

    rx.recv().unwrap();
  }

  // fn interval<F>(f: F, delay: time::Duration)
  // where F: Fn() -> () + Send + Sync + 'static {
  //   let (tx, rx) = mpsc::channel();
  //   let f = sync::Arc::new(f);

  //   thread::spawn(move || {
  //     thread::sleep(delay);
  //     tx.send(f()).unwrap();
  //   });

  //   rx.recv().unwrap();
  // }
}
