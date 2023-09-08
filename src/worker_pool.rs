use std::thread;
use std::sync::{Arc, Mutex, mpsc};

type Job = Box<dyn FnOnce() + Send + 'static>;
type JobReceiver = Arc<Mutex<mpsc::Receiver<Job>>>;
type Handle = thread::JoinHandle<()>;

/// Simple implementaion of the worker pool to run tasks in different threads.
/// 
/// It spawns `N` threads and [`mpsc::sync_channel`] with `N * 2` buffer size that all Workers will listen to. 
/// To run a task in pool use [`WorkerPool.execute()`]
pub struct WorkerPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::SyncSender<Job>>,
}

impl WorkerPool {
    /// Creates N workers where every worker spawns a thread and wait for a job.
    /// Panics if `size == 0`
    pub fn new(size: usize) -> WorkerPool {
        assert!(size > 0);

        let (tx, rx) = mpsc::sync_channel(size * 2);
        let mut workers = Vec::with_capacity(size);
        let rec = Arc::new(Mutex::new(rx));

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&rec)));
        }

        WorkerPool {
            workers,
            sender: Some(tx),
        }
    }

    /// Send a job to workers. 
    /// If all workers are busy and channel is full the function will block until one of the Workers are done.
    pub fn execute<J: FnOnce() + Send + 'static>(&self, f: J) {
        let job = Box::new(f);
        self.sender.as_ref().unwrap().send(job).unwrap();
    }
}

impl Drop for WorkerPool {
    fn drop(&mut self) {
        self.sender.take();

        for worker in &mut self.workers {
            if let Some(handle) = worker.handle.take() {
                handle.join().unwrap();
            }
        }
    }
}

#[allow(dead_code)]
struct Worker {
    id: usize,
    handle: Option<Handle>,
}

impl Worker {
    fn new(id: usize, rx: JobReceiver) -> Worker {
        let handle = thread::spawn(move || loop {
            let job = match rx.lock().expect("Can't acquire lock").recv() {
                Ok(data) => data,
                Err(_) => return,
            };

            job();
        });

        Worker {
            id,
            handle: Some(handle),
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn pool_execute() {
        let pool = WorkerPool::new(4);
        let (tx, rx) = mpsc::channel();
        let mut result = Vec::new();
        for i in 1..=5 {
            let c = tx.clone();
            pool.execute(move || c.send(i * 2).unwrap())
        }

        drop(tx);
        drop(pool);

        for d in rx {
            result.push(d);
        }

        result.sort();

        assert_eq!(result, vec![2, 4, 6, 8, 10])
    }

    #[test]
    #[should_panic]
    fn worker_panics() {
        let pool = WorkerPool::new(4);
        pool.execute(move || panic!());
    }

    #[test]
    #[should_panic]
    fn pool_with_zero_size() {
        WorkerPool::new(0);
    }
}