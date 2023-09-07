use clap::Parser;
use sha2::digest::generic_array::GenericArray;
use sha2::digest::typenum::U32;
use sha2::{Digest, Sha256};
use std::sync::mpsc::SendError;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Instant;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Number of zeroes at the end of SHA256 hash
    #[arg(short = 'N')]
    num_zeroes: u32,

    /// Number of hashes to output
    #[arg(short = 'F')]
    num_hash: usize,

    /// Number of threads
    #[arg(short = 'T', default_value_t = 0)]
    num_threads: usize,
}

type Job = Box<dyn FnOnce() + Send + Sync + 'static>;
type JobReceiver = Arc<Mutex<mpsc::Receiver<Job>>>;
type Handle = thread::JoinHandle<()>;

struct WorkerPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::SyncSender<Job>>,
}

impl WorkerPool {
    fn new(size: usize) -> WorkerPool {
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

    fn execute<J: FnOnce() + Send + Sync + 'static>(&self, f: J) -> Result<(), SendError<Job>> {
        let job = Box::new(f);
        self.sender.as_ref().unwrap().send(job)
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

        Worker { id, handle: Some(handle) }
    }
}

#[derive(PartialEq, PartialOrd, Eq, Ord)]
struct HashPair {
    n: usize,
    hash: String,
}

fn main() {
    let mut cli: Cli = Cli::parse();

    if cli.num_threads == 0 {
        cli.num_threads = thread::available_parallelism().unwrap().get() as usize;
    }

    let mut hashes = Vec::<HashPair>::new();

    let pool = WorkerPool::new(cli.num_threads);
    let (tx, rx) = mpsc::channel::<HashPair>();
    let mut counter = 1;
    let now = Instant::now();

    while hashes.len() < cli.num_hash {
        let sender = tx.clone();
        pool.execute(move || {
            let hash = format!("{:x}", generate_hash(counter.to_string()));

            if count_slice_end(hash.as_ref(), &('0' as u8)) == cli.num_zeroes {
                sender.send(HashPair {
                    n: counter,
                    hash: hash,
                }).unwrap();
            }
        }).unwrap();

        counter += 1;

        if let Ok(data) = rx.try_recv() {
            hashes.push(data);
        }
    }

    let end = now.elapsed();
    println!("{:?}", end);

    drop(rx);

    hashes.sort();

    hashes.iter().for_each(|h| println!("{}, {}", h.n, h.hash));

}

fn generate_hash(data: impl AsRef<[u8]>) -> GenericArray<u8, U32> {
    let mut hasher = Sha256::default();
    hasher.update(data);
    hasher.finalize()
}

fn count_slice_end<T: Eq + Sized>(data: &[T], e: &T) -> u32 {
    let mut counter = 0;

    for element in data.iter().rev() {
        if element != e {
            break;
        }

        counter += 1;
    }

    counter
}
