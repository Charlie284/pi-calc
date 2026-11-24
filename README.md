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

The series is summed by binary splitting over exact GMP integers (through the `rug` crate), with bounded parallelism across the top levels of the product tree. See [docs/algorithm.md](docs/algorithm.md) for the formula, the splitting recurrence, the exact-integer finalization, and the stdout/stderr contract.

## Performance

Runtime depends on the requested precision, CPU, Rust toolchain, and GMP build. Illustrative
median warm runs on an Apple M4 Max with Rust 1.96.1: about 0.01 seconds for 100,000 digits,
0.13 seconds for 1,000,000, and 3.2 seconds for 10,000,000. These are observations, not
cross-machine guarantees.

To measure the release build on your machine without including terminal output time:

```sh
cargo build --release
/usr/bin/time -p target/release/pi-calc --quiet 100000 > /dev/null
```

## Build

Requires Rust 1.75+ and GMP/MPFR dev libraries.

```bash
# Debian/Ubuntu
sudo apt install libgmp-dev libmpfr-dev libmpc-dev

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
