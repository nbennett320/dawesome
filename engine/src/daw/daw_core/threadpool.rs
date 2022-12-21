use std::sync::{
  Arc,
  Mutex
};
use std::sync::mpsc;
use std::sync::mpsc::{
  Sender,
  Receiver
};
use std::thread;
use std::vec::{Vec};

trait FnBox {
  fn call(self: Box<Self>);
}

impl<F: FnOnce()> FnBox for F {
  fn call(self: Box<F>) {
    (*self)()
  }
}

type Job = Box<dyn FnBox + Send + 'static>;

enum Message {
  NewJob(Job),
  Kill,
}

struct Worker {
  _id: usize,
  thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
  fn new(
    _id: usize,
    revc: Arc<Mutex<Receiver<Message>>>,
  ) -> Self {
    let thread = thread::spawn(move || loop {
      let msg = revc.lock().unwrap().recv().unwrap();
      match msg {
        Message::NewJob(job) => job.call(),
        Message::Kill => break,
      }
    });

    Worker {
      _id,
      thread: Some(thread),
    }
  }
}

pub struct ThreadPool {
  workers: Vec<Worker>,
  sender: Sender<Message>,
}

impl ThreadPool {
  pub fn new(size: usize) -> Self {
    assert!(size > 0);

    let (sender, rx): (Sender<Message>, Receiver<Message>) =
      mpsc::channel();
    let recv = Arc::new(Mutex::new(rx));
    let mut workers = Vec::with_capacity(size);

    for id in 0..size {
      workers.push(Worker::new(id, Arc::clone(&recv)));
    }

    ThreadPool { workers, sender }
  }

  pub fn exec<F>(
    &self,
    f: F,
  ) where
    F: FnOnce() + Send + 'static,
  {
    let job = Box::new(f);
    self.sender.send(Message::NewJob(job)).unwrap();
  }
}

impl Drop for ThreadPool {
  fn drop(&mut self) {
    for _ in &mut self.workers {
      self.sender.send(Message::Kill).unwrap();
    }

    for worker in &mut self.workers {
      if let Some(thread) = worker.thread.take() {
        thread.join().unwrap();
      }
    }
  }
}
