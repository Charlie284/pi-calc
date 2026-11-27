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

The series is summed by binary splitting over GMP integers. Large multiplication, Newton
division, square roots, and radix conversion use FLINT's vectorized and multithreaded arithmetic,
with GMP and FLINT allocations routed through mimalloc. Decimal digits are extracted as a
parallel binary tree using precomputed powers of ten. See [docs/algorithm.md](docs/algorithm.md).

## Performance

Release-build wall time on an Apple M4 Max with Rust 1.96.1, with stdout redirected away
from the terminal. Current 1 million and 10 million rows are medians of five runs. The
100 million row is one validated run.

| Decimal digits | Previous binary splitting (`9a68a2c`) | Current | Speedup |
| ---: | ---: | ---: | ---: |
| 1,000,000 | 0.139 s | 0.023 s | 6.0× |
| 10,000,000 | 2.43 s | 0.217 s | 11.2× |
| 100,000,000 | 35.28 s | 2.92 s | 12.1× |

The immediately preceding FLINT integration computed 100 million digits in 7.49 s on the
same machine, so the current pipeline is 2.6× faster than that baseline. Peak resident memory
at 100 million digits is 2.57 GB, down from 3.20 GB in the original binary-splitting build.

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
