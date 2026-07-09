# pi-calc

A very fast pi calculator using the Chudnovsky algorithm. Written in Rust.

Computes pi to arbitrary precision. 1,000 digits in ~124us, 10,000 in ~33ms, 100,000 in ~3s.

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

| Digits | Time      | Iterations |
|--------|-----------|------------|
| 100    | 37 us     | 10         |
| 1,000  | 124 us    | 73         |
| 10,000 | 33 ms     | 708        |
| 100,000| ~3 s      | 7,052      |

## Build

Requires Rust 1.75+ and GMP/MPFR dev libraries.

```bash
# Debian/Ubuntu
sudo apt install libgmp-dev libmpfr-dev libmpc-dev

cargo build --release
```

## License

MIT
