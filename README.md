# pi-calc

A fast pi calculator using the Chudnovsky algorithm. Written in Rust.

Computes pi to arbitrary precision with an explicit upper bound of 100,000,000 decimal digits.

## Usage

```
pi-calc [FLAGS] [DIGITS]
```

- `DIGITS` -- number of decimal digits (default: 1000, max: 100,000,000)
- `-q, --quiet` -- suppress timing info, output only the digits
- `-h, --help` -- print help

### Examples

```
pi-calc           # 1000 digits
pi-calc 100       # 100 digits
pi-calc -q 10000  # 10000 digits, quiet mode
```

## How it works

Uses the [Chudnovsky algorithm](https://en.wikipedia.org/wiki/Chudnovsky_algorithm), the same method used for world-record pi computations. Each iteration yields ~14.18 digits.

The series is summed by binary splitting over GMP integers. The product tree cancels common
factors before multiplication and runs independent work in parallel. Large multiplication,
Newton division, and decimal string conversion use FLINT's vectorized and multithreaded
arithmetic. The result is computed in binary fixed point and converted to decimal in blocks.
See [docs/algorithm.md](docs/algorithm.md).

## Performance

Release-build wall time on an Apple M4 Max with Rust 1.96.1. Both builds redirected stdout
away from the terminal. The 1 million and 10 million rows are medians of three alternating
runs. The 100 million row is one run per version.

| Decimal digits | Previous binary splitting (`9a68a2c`) | Optimized | Speedup |
| ---: | ---: | ---: | ---: |
| 1,000,000 | 0.139 s | 0.086 s | 1.6× |
| 10,000,000 | 2.43 s | 1.23 s | 2.0× |
| 100,000,000 | 35.28 s | 14.70 s | 2.4× |

At 100 million digits, peak resident memory is 2.65 GB, down from 3.20 GB in the previous
binary-splitting implementation. The 100 million measurement was made while the test machine
was heavily loaded, so it is a conservative wall-clock result rather than a best-case benchmark.

To measure the release build on your machine without including terminal output time:

```sh
cargo build --release
/usr/bin/time -p target/release/pi-calc --quiet 100000 > /dev/null
```

## Build

Requires Rust 1.85+, autotools, and GMP/MPFR dev libraries.

```bash
# Debian/Ubuntu
sudo apt install autoconf automake libtool libgmp-dev libmpfr-dev libmpc-dev

cargo build --release
```

## Test

The integration suite verifies output digits and CLI error handling:

```sh
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

## License

MIT
