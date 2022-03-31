use std::iter;
use std::sync::mpsc;
use std::sync;
use std::cmp;
use std::thread;
use std::time;
use std::vec;
use std::collections;

struct Schedule<'a, T> {
  when: time::SystemTime,
  data: T,
  guard: sync::MutexGuard<'a, T>,
  repeat: Option<time::Duration>,
}

impl <T> Ord for Schedule<'_, T> {
  fn cmp(&self, other: &Self) -> cmp::Ordering {
    self.date.cmp(&other.date).reverse()
  }
}

impl <T> PartialOrd for Schedule<'_, T> {
  fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
    self.date.partial_cmp(&other.date).map(|ord| ord.reverse())
  }
}

impl <T> Eq for Schedule<'_, T> {

}

impl <T> PartialEq for Schedule<'_, T> {
  fn eq(&self, other: &Self) -> bool {
    self.date.eq(&other.date)
  }
}

enum Op<'a, T> {
  Schedule(Schedule<'a, T>),
  Stop,
}

struct WaiterChannel<'a, T> {
  messages: sync::Mutex<Vec<Op<'a, T>>>,
  condvar: sync::Condvar,
}

impl <'a, T> WaiterChannel<'a, T> {
  fn with_capacity(size: usize) -> Self {
    WaiterChannel {
      messages: sync::Mutex::new(vec::Vec::with_capacity(size)),
      condvar: sync::Condvar::new(),
    }
  }
}

trait Executor<T> {
  fn exec(&mut self, data: T);
  fn exec_clone(&mut self, data: T) -> T;
}

struct CallbackExecutor;

impl Executor<Box<dyn FnMut() + Send>> for CallbackExecutor {
  fn exec(&mut self, mut data: Box<dyn FnMut() + Send>) {
    data();
  }

  fn exec_clone(&mut self, mut data: Box<dyn FnMut() + Send>) -> Box<dyn FnMut() + Send> {
    data();
    data
  }
}

struct DeliveryExecutor<T>
where T: 'static + Send {
  tx: mpsc::Sender<T>
}

impl <T> Executor<T> for DeliveryExecutor<T>
where T: 'static + Send + Clone {
  fn exec(&mut self, data: T) {
    let _ = self.tx.send(data);
  }

  fn exec_clone(&mut self, data: T) -> {
    let _ = self.tx.send(data.clone());
    data
  }
}

struct Scheduler<'a, T, E>
where E: Executor<T> {
  waiter: sync::Arc<WaiterChannel<'a, T>>,
  heap: collections::BinaryHeap<Schedule<'a, T>>,
  executor: E
}

impl <'a, T, E> Scheduler<'a, T, E>
where E: Executor<T> {
  // implement with_capacity 
  fn with_capacity(
    waiter: sync::Arc<WaiterChannel<T>>,
    executor: E,
    capacity: usize
  ) -> Self {
    Scheduler {
      waiter: waiter,
      executor: executor,
      heap: collections::BinaryHeap::with_capacity(capacity)
    }
  }

  // run the schedulrer
  fn run(&mut self) {
    enum Sleep {
      NotAtAll,
      UntilAwakened,
      AtMost(time::Duration)
    }

    let ref waiter = *self.waiter;
    
    loop {
      let mut sleep = if let Some(scheduler) = self.heap.peek() {
        let now = time::SystemTime::now();
        if scheduler.date > now {
          Sleep::AtMost(scheduler.date.signed_duration_since(now))
        } else {
          let scheduler = self.heap.pop().unwrap();
          
          if scheduler.guard.should_execute() {
            if let Some(delta) = scheduler.repeat {
              let data = self.executor.exec_clone(scheduler.data);

              self.heap.push(
                Schedule {
                  when: scheduler.when + delta,
                  data: data,
                  guard: scheduler.guard,
                  repeat: Some(delta)
                }
              );
            } else {
              self.executor.exec(scheduler.data);
            }
          }

          Sleep::NotAtAll
        }
      } else {

        Sleep::UntilAwakened
      };

      let mut lock = waiter.messages.lock().unwrap();
      for msg in lock.drain(..) {
        match msg {
          Op::Stop => {
            // stop timer
            return;
          }
          Op::Schedule(item) => {
            self.heap.push(item);
            sleep = Sleep::NotAtAll;
          }
        }
      }

      match sleep {
        Sleep::UntilAwakened => {
          let _ = waiter.condvar.wait(lock);
        }
        Sleep::AtMost(delay) => {
          let sec = delay.as_secs(); 
          let ns = (delay - time::Duration::from_secs(sec)).as_nanos();
          let duration = time::Duration::new(sec as u64, ns as u32);
          let _ = waiter.condvar.wait_timeout(lock, duration);
        }
        Sleep::NotAtAll => {}
      }
    }
  }
}

struct TimerBase<'a, T>
where T: 'static + Send {
  tx: mpsc::Sender<Op<'a, T>>,
}

impl <'a, T> TimerBase<'a, T>
where T: 'static + Send {
  // define new timer
  fn new<E>(executor: E) -> Self
  where E: 'static + Executor<T> + Send {
    Self::with_capacity(executor, 32)
  }

  fn with_capacity<E>(
    executor: E, 
    size: usize
  ) -> Self
  where E: 'static + Executor<T> + Send {
    let waiter_send = sync::Arc::new(WaiterChannel::with_capacity(size));
    let waiter_recv = waiter_send.clone();

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
      let ref waiter = *waiter_send;
      
      for msg in rx.iter() {
        let mut vec = waiter.messages.lock.unwrap();

        match msg {
          Op::Schedule(item) => {
            vec.push(Op::Schedule(item));
            waiter.condvar.notify_one();
          }
          Op::Stop => {
            vec.clear();
            vec.push(Op::Stop);
            waiter.convdar.notify_one();
            return;
          }
        }
      }
    });

    thread::Builder::new().name("Timer thread".to_owned()).spawn(move || {
      let mut scheduler = Scheduler::with_capacity(waiter_recv, executor, size);
      scheduler.run();
    }).unwrap();

    TimerBase {
      tx: tx
    }
  }

  fn schedule(
    &self, 
    when: time::SystemTime, 
    repeat: Option<time::Duration>,
    data: T
  ) -> sync::MutexGuard<'a, T> {
    let guard = sync::MutexGuard<'a, T>::new();
    self.tx.send(
      Op::Schedule(
        Schedule {
          when: when,
          data: data,
          guard: guard.clone(),
          repeat: repeat
        }
      )).unwrap();

      guard
  }

  // fn exec_on_date<D>(&self, date: time::)

  fn timeout(
    &self, 
    data: T, 
    delay: time::Duration
  ) -> sync::MutexGuard<T> {
    self.schedule(time::SystemTime::now().add(delay), None, data)
  }


  fn interval(
    &self, 
    data: T, 
    delay: time::Duration
  ) -> sync::MutexGuard<T> {
    self.schedule(time::SystemTime::now().add(delay), Some(delay), data)
  }
}

#[derive(Debug)]
pub struct Timer {
  base: TimerBase<Box<dyn FnMut() + Send>>
}

impl Timer {
  fn new() -> Self {
    Timer {
      base: TimerBase::new(CallbackExecutor)
    }
  }

  fn with_capacity(size: usize) -> Self {
    Timer {
      base: TimerBase::with_capacity(CallbackExecutor, size)
    }
  }

  fn timeout(
    &self, 
    f: dyn FnMut() -> (), 
    delay: time::Duration
  ) {
    self.base.timeout(Box::new(f), delay)
  }

  fn interval(
    &self, 
    f: dyn FnMut() -> (), 
    delay: time::Duration
  ) {
    self.base.interval(Box::new(f), delay)
  }
}
