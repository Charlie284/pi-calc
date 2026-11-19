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

The implementation uses GMP (via the `rug` crate) for arbitrary-precision integer and floating-point arithmetic.

## Performance

The series is summed with binary splitting: the P/Q/T terms are combined as exact integers and
only one full-precision division is performed at the end, so runtime grows roughly as
O(n log^3 n) instead of quadratically.

Runtime depends on the requested precision, CPU, Rust toolchain, and GMP/MPFR build. An illustrative
single run on a 2-vCPU x86_64 Linux VM took about 0.03 seconds for 100,000 digits, 0.5 seconds for
1,000,000 digits, and 9 seconds for 10,000,000 digits (about 100 MB peak memory). These are
observations, not cross-machine guarantees.

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
