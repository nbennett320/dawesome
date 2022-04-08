use std::future;
use std::pin;
use std::sync;
use std::task;
use std::thread;
use std::time;

struct TimerBaseState {
  timed_out: bool,
  already_polled: bool,
}

struct TimerBase {
  delay: time::Duration,
  _handle: Option<thread::JoinHandle<()>>,
  state: sync::Arc<sync::Mutex<TimerBaseState>>,
}

impl TimerBase {
  fn _new(delay: time::Duration) -> Self {
    TimerBase {
      delay,
      _handle: None,
      state: sync::Arc::new(sync::Mutex::new(TimerBaseState {
        timed_out: false,
        already_polled: false,
      })),
    }
  }

  fn spawn_timer_thread(
    &self,
    context: &mut task::Context<'_>,
  ) {
    let delay = self.delay;
    let state = sync::Arc::clone(&self.state);
    let waker_handle = context.waker().clone();

    thread::spawn(move || {
      thread::sleep(delay);
      let mut state_handle =
        state.lock().expect("Can't lock the state in timer thread");
      (*state_handle).timed_out = true;
      waker_handle.wake();
    });
  }
}

impl future::Future for TimerBase {
  type Output = ();

  fn poll(
    self: pin::Pin<&mut Self>,
    context: &mut task::Context<'_>,
  ) -> task::Poll<Self::Output> {
    let mut state = self.state.lock().expect("Can't lock timer state");

    if state.timed_out {
      task::Poll::Ready(())
    } else if state.already_polled {
      task::Poll::Pending
    } else {
      (*state).already_polled = true;
      self.spawn_timer_thread(context);
      task::Poll::Pending
    }
  }
}

fn _spawn_timeout(delay: time::Duration) -> impl future::Future {
  TimerBase::_new(delay)
}

pub struct _Timer {}

impl _Timer {
  pub fn _new() -> Self {
    _Timer {}
  }

  pub fn _timeout(
    &self,
    delay: time::Duration,
  ) -> impl future::Future {
    _spawn_timeout(delay)
  }
}
