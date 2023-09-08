use clap::Parser;
use sha2::digest::generic_array::GenericArray;
use sha2::digest::typenum::U32;
use sha2::{Digest, Sha256};
use std::sync::mpsc;
use std::thread;

mod worker_pool;

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
    /// If num_threads is not specified, it will be set to thread::available_parallelism() instead.
    #[arg(short = 'T', long = "threads", default_value_t = 0)]
    num_threads: usize,

    /// Number of hashes that one worker will process on call
    #[arg(short = 'S', long = "step", default_value_t = 10)]
    step: usize,

    /// Value to find
    #[arg(short = 'V', long = "value", default_value_t = '0')]
    value: char,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
struct HashPair {
    n: usize,
    hash: String,
}

impl HashPair {
    fn new<T: Into<String>>(n: usize, hash: T) -> HashPair {
        HashPair {
            n,
            hash: hash.into(),
        }
    }
}

fn main() {
    let mut cli: Cli = Cli::parse();

    if cli.num_threads == 0 {
        cli.num_threads = get_num_threads();
    }

    let hashes = generate_hash_with_zeroes(cli.num_threads, cli.value, cli.num_hash, cli.num_zeroes, cli.step);

    hashes.iter().for_each(|h| println!("{}, {}", h.n, h.hash));
}

fn get_num_threads() -> usize {
    thread::available_parallelism().unwrap().get()
}

fn generate_hash_with_zeroes(
    num_threads: usize,
    value: char,
    hash_limit: usize,
    num_zeroes: u32,
    step: usize,
) -> Vec<HashPair> {
    let pool = worker_pool::WorkerPool::new(num_threads);
    let (tx, rx) = mpsc::channel::<HashPair>();

    let mut hashes = Vec::<HashPair>::with_capacity(hash_limit * 2);
    let mut counter = 1;
    while hashes.len() < hash_limit {
        let sender = tx.clone();
        pool.execute(move || {
            for i in counter..counter + step {
                let hash = format!("{:x}", generate_sha256(i.to_string()));

                if count_slice_end(hash.as_ref(), &(value as u8)) == num_zeroes {
                    let _ = sender.send(HashPair::new(i, hash));
                }
            }
        });

        counter += step;

        if let Ok(data) = rx.try_recv() {
            hashes.push(data);
        }
    }
    drop(pool);

    // When hashes processed too quiclky (for example -N = 0), some hashes could be lost.
    // To prevent that we just collect all generated hashes from workers, sort vec and truncate to needed limit.
    drop(tx);
    for data in rx {
        hashes.push(data);
    }

    hashes.sort_by(|a, b| a.n.cmp(&b.n));
    hashes.truncate(hash_limit);

    hashes
}

/// Count number of elements at the end of `&[T]` that equal `e`. Returns number of entries when element in slice != `e`
fn count_slice_end<T: Eq>(data: &[T], e: &T) -> u32 {
    let mut counter = 0;

    for element in data.iter().rev() {
        if element != e {
            break;
        }

        counter += 1;
    }

    counter
}

fn generate_sha256(data: impl AsRef<[u8]>) -> GenericArray<u8, U32> {
    let mut hasher = Sha256::default();
    hasher.update(data);
    hasher.finalize()
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn count_slice_end_is_correct() {
        assert_eq!(
            count_slice_end(&[], &5),
            0
        );

        assert_eq!(
            count_slice_end(&[1, 2, 3, 3, 3], &3),
            3
        );

        assert_eq!(
            count_slice_end(&[1], &1),
            1
        );

        assert_eq!(
            count_slice_end("012347510000aaabbbeeeeee".as_bytes(), &('e' as u8)),
            6
        )
    }

    #[test]
    fn generate_hash_with_zero_limit() {
        assert_eq!(generate_hash_with_zeroes(12, '0', 0, 0, 100), vec![]);
    }

    #[test]
    fn generate_hash_with_no_zeroes() {
        assert_eq!(
            generate_hash_with_zeroes(12, '0', 10, 0, 10),
            vec![
                HashPair::new(
                    1,
                    "6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b"
                ),
                HashPair::new(
                    2,
                    "d4735e3a265e16eee03f59718b9b5d03019c07d8b6c51f90da3a666eec13ab35"
                ),
                HashPair::new(
                    3,
                    "4e07408562bedb8b60ce05c1decfe3ad16b72230967de01f640b7e4729b49fce"
                ),
                HashPair::new(
                    4,
                    "4b227777d4dd1fc61c6f884f48641d02b4d121d3fd328cb08b5531fcacdabf8a"
                ),
                HashPair::new(
                    5,
                    "ef2d127de37b942baad06145e54b0c619a1f22327b2ebbcfbec78f5564afe39d"
                ),
                HashPair::new(
                    6,
                    "e7f6c011776e8db7cd330b54174fd76f7d0216b612387a5ffcfb81e6f0919683"
                ),
                HashPair::new(
                    7,
                    "7902699be42c8a8e46fbbb4501726517e86b22c56a189f7625a6da49081b2451"
                ),
                HashPair::new(
                    8,
                    "2c624232cdd221771294dfbb310aca000a0df6ac8b66b696d90ef06fdefb64a3"
                ),
                HashPair::new(
                    9,
                    "19581e27de7ced00ff1ce50b2047e7a567c76b1cbaebabe5ef03f7c3017bb5b7"
                ),
                HashPair::new(
                    10,
                    "4a44dc15364204a80fe80e9039455cc1608281820fe2b24f1e5233ade6af1dd5"
                ),
            ]
        );
    }

    #[test]
    fn generate_hash_is_correct() {
        assert_eq!(
            generate_hash_with_zeroes(12, '0', 6, 3, 100),
            vec![
                HashPair::new(
                    4163,
                    "95d4362bd3cd4315d0bbe38dfa5d7fb8f0aed5f1a31d98d510907279194e3000"
                ),
                HashPair::new(
                    11848,
                    "cb58074fd7620cd0ff471922fd9df8812f29f302904b15e389fc14570a66f000"
                ),
                HashPair::new(
                    12843,
                    "bb90ff93a3ee9e93c123ebfcd2ca1894e8994fef147ad81f7989eccf83f64000"
                ),
                HashPair::new(
                    13467,
                    "42254207576dd1cfb7d0e4ceb1afded40b5a46c501e738159d8ac10b36039000"
                ),
                HashPair::new(
                    20215,
                    "1f463eb31d6fa7f3a7b37a80f9808814fc05bf10f01a3f653bf369d7603c8000"
                ),
                HashPair::new(
                    28892,
                    "dab12874ecae90c0f05d7d87ed09921b051a586c7321850f6bb5e110bc6e2000"
                ),
            ]
        );

        assert_eq!(
            generate_hash_with_zeroes(12, '0', 3, 5, 100),
            vec![
                HashPair::new(
                    828028,
                    "d95f19b5269418c0d4479fa61b8e7696aa8df197082b431a65ff37595c100000"
                ),
                HashPair::new(
                    2513638,
                    "862d4525b0b60779d257be2b3920b90e3dbcd60825b86cfc6cffa49a63c00000"
                ),
                HashPair::new(
                    3063274,
                    "277430daee71c67b356dbb81bb0a39b6d53efd19d14177a173f2e05358a00000"
                ),
            ]
        );
    }
}
