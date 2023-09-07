use clap::Parser;
use sha2::digest::generic_array::GenericArray;
use sha2::digest::typenum::U32;
use sha2::{Digest, Sha256};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Instant;


/// A cli tool to find **-F** number of hashes with **-N** zeroes on the end.
/// 
/// It's starts from number 1 and for every number it will generate sha256 hash. 
/// If generated hash ends with -N zeroes, the hash and number will be printed in console.
/// It won't stop until -F hashes are found.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about)]
struct Cli {
    /// Number of zeroes at the end of hash
    #[arg(short = 'N')]
    num_zeroes: u32,

    /// Number of hashes to print
    #[arg(short = 'F')]
    num_hash: usize,

    /// Number of workers that will be working concurrently.
    /// If num_threads is not specified, it will be set to thread::available_paralellism() instead.
    #[arg(short = 'T', long = "threads", default_value_t = 0)]
    num_threads: usize,

    /// Number of hashes that one worker will process on call 
    #[arg(short = 'S', long = "step", default_value_t = 10)]
    step: usize
}

type Job = Box<dyn FnOnce() + Send + 'static>;
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

    fn execute<J: FnOnce() + Send + 'static>(&self, f: J) {
        let job = Box::new(f);
        let _ = self.sender.as_ref().unwrap().send(job);
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

        Worker {
            id,
            handle: Some(handle),
        }
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

    let hashes = generate_hash_with_zeroes(cli.num_threads, cli.num_hash, cli.num_zeroes, cli.step);

    hashes.iter().for_each(|h| println!("{}, {}", h.n, h.hash));
}

fn generate_hash_with_zeroes(num_threads: usize, hash_limit: usize, num_zeroes: u32, step: usize) -> Vec<HashPair> {
    let mut hashes = Vec::<HashPair>::new();
    let mut counter = 1;

    let pool = WorkerPool::new(num_threads);
    let (tx, rx) = mpsc::channel::<HashPair>();
    let now = Instant::now();

    while hashes.len() < hash_limit {
        let sender = tx.clone();
        pool.execute(move || {
            for i in counter..counter + step {
                let hash = format!("{:x}", generate_sha256(i.to_string()));

                if count_slice_end(hash.as_ref(), &('0' as u8)) == num_zeroes {
                    let _ = sender
                        .send(HashPair {
                            n: i,
                            hash: hash,
                        });
                }
            }
        });

        counter += step;

        if let Ok(data) = rx.try_recv() {
            hashes.push(data);
        }
    }
    let end = now.elapsed();
    println!("step: {} elapsed: {:?}", step, end);

    hashes.sort();

    hashes
}

fn generate_sha256(data: impl AsRef<[u8]>) -> GenericArray<u8, U32> {
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

// #[cfg(test)]
// mod tests {
//     use super::*;

//     fn bench_different_steps(b: &mut Bencher)
// }