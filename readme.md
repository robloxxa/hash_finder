*This is a [test task](https://jl.pyshop.ru/tasks/rust-dev/) for a Internt Rust Dev in Pyshop company*
# Hash Finder
A cli tool to find **-F** number of hashes with **-N** zeroes on the end.

It's starts from number 1 and for every number it will generate sha256 hash. If generated hash ends with -N zeroes, the hash and number will be printed in console. It won't stop until -F hashes are found.

## Usage
```
Usage: hash_finder [OPTIONS] -N <NUM_ZEROES> -F <NUM_HASH>

Options:
  -N <NUM_ZEROES>
          Number of zeroes at the end of hash

  -F <NUM_HASH>
          Number of hashes to print

  -T, --threads <NUM_THREADS>
          Number of workers that will be working concurrently. If num_threads is not specified, it will be set to thread::available_parallelism() instead

          [default: 0]

  -S, --step <STEP>
          Number of hashes that one worker will process on call

          [default: 10]

  -V, --value <VALUE>
          Value to find

          [default: 0]

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

## Build
```shell
# Clone this repo
git clone link
# Change directory to cloned folder
cd hash_finder
# Build tool via cargo with -r (release) tag
cargo build -r

# OR use cargo run
cargo run -r

```