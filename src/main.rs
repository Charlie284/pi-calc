use rug::ops::Pow;
use rug::Integer;
use std::env;
use std::process;
use std::time::Instant;

const USAGE: &str = "\
pi-calc — compute pi to arbitrary precision

USAGE:
  pi-calc [FLAGS] [DIGITS]

FLAGS:
  -h, --help    Print this help message
  -q, --quiet   Suppress timing info, output only the digits

ARGS:
  <DIGITS>    Number of decimal digits to compute (default: 1000, max: 100_000_000)

The Chudnovsky algorithm is used. Each digit costs ~log2(10) approx 3.32 bits.
Runtime depends heavily on precision, hardware, and the GMP build.
";

const A: u64 = 545_140_134;
const B: u64 = 13_591_409;
const C3_OVER_24: u64 = 10_939_058_860_032_000; // 640320^3 / 24

fn parse_args() -> (u32, bool) {
    let args: Vec<String> = env::args().collect();
    let mut digits: Option<u32> = None;
    let mut quiet = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                print!("{}", USAGE);
                process::exit(0);
            }
            "-q" | "--quiet" => quiet = true,
            arg if arg.starts_with('-') => {
                eprintln!("error: unknown flag '{}'", arg);
                eprintln!("usage: pi-calc [--quiet] [digits]");
                process::exit(1);
            }
            _ => {
                let a = &args[i];
                if digits.is_some() {
                    eprintln!("error: unexpected argument '{}'", a);
                    process::exit(1);
                }
                match a.parse::<u32>() {
                    Ok(n) if n > 0 && n <= 100_000_000 => digits = Some(n),
                    Ok(_) => {
                        eprintln!("error: digits must be between 1 and 100,000,000");
                        process::exit(1);
                    }
                    Err(_) => {
                        eprintln!("error: '{}' is not a valid number", a);
                        process::exit(1);
                    }
                }
            }
        }
        i += 1;
    }

    (digits.unwrap_or(1_000), quiet)
}

// Binary-splitting triple for a Chudnovsky range [lo, hi):
//   sum_{k in [lo,hi)} (-1)^k (Ak+B) prod_{j=lo..=k} p(j)/q(j) = t / q
struct Pqt {
    p: Integer, // product of p(j) over the range
    q: Integer, // product of q(j) over the range
    t: Integer, // scaled partial sum
}

fn split(lo: u32, hi: u32) -> Pqt {
    if hi - lo == 1 {
        let k = u64::from(lo);
        if k == 0 {
            return Pqt {
                p: Integer::from(1),
                q: Integer::from(1),
                t: Integer::from(B),
            };
        }
        // p(k) = (6k-5)(2k-1)(6k-1), q(k) = k^3 * C^3/24
        let mut p = Integer::from(6 * k - 5);
        p *= 2 * k - 1;
        p *= 6 * k - 1;
        let mut q = Integer::from(k * k);
        q *= k;
        q *= C3_OVER_24;
        let mut t: Integer = (&p * (A * k + B)).into();
        if k % 2 == 1 {
            t = -t;
        }
        return Pqt { p, q, t };
    }
    let mid = lo + (hi - lo) / 2;
    let l = split(lo, mid);
    let r = split(mid, hi);
    combine(l, r)
}

// Parallel wrapper: the two halves of the tree are independent, so the top
// levels run on scoped threads. Bounded twice: depth caps the task count and
// small ranges stay serial, so small requests never pay spawn overhead.
fn split_par(lo: u32, hi: u32, depth: u32) -> Pqt {
    if depth == 0 || hi - lo <= 256 {
        return split(lo, hi);
    }
    let mid = lo + (hi - lo) / 2;
    let (l, r) = std::thread::scope(|s| {
        let handle = s.spawn(|| split_par(lo, mid, depth - 1));
        let r = split_par(mid, hi, depth - 1);
        (handle.join().expect("split thread panicked"), r)
    });
    combine(l, r)
}

fn combine(l: Pqt, r: Pqt) -> Pqt {
    Pqt {
        // fused multiply-add: t = l.t*r.q + l.p*r.t in one pass
        t: &l.t * &r.q + Integer::from(&l.p * &r.t),
        p: (&l.p * &r.p).into(),
        q: (&l.q * &r.q).into(),
    }
}

fn main() {
    let (digits, quiet) = parse_args();

    let start = Instant::now();

    // Chudnovsky algorithm: pi = 426880*sqrt(10005) / S
    // S = sum (-1)^k (6k)! (13591409 + 545140134k) / ((3k)! (k!)^3 640320^(3k))
    // Each term gives ~14.181 digits.
    let limit = (digits as f64 / 14.181).ceil() as u32 + 2;

    // Binary splitting turns the series into a balanced product tree, so the
    // per-term cost is a balanced GMP multiply instead of a full-precision
    // division. The root's P product is never used, so the top merge is
    // inlined to skip it.
    let mid = limit / 2;
    let (l, r) = std::thread::scope(|s| {
        let handle = s.spawn(|| split_par(0, mid, 3));
        let r = split_par(mid, limit, 3);
        (handle.join().expect("split thread panicked"), r)
    });
    let q: Integer = (&l.q * &r.q).into();
    let t: Integer = &l.t * &r.q + Integer::from(&l.p * &r.t);

    // pi * 10^scale = 426880 * sqrt(10005 * 10^(2*scale)) * q / t
    // Everything stays exact until the final truncating division, with `guard`
    // spare digits below the ones we print.
    let guard: u32 = 16;
    let scale = digits + guard;
    let mut radicand = Integer::from(10).pow(2 * scale);
    radicand *= 10005u32;
    let mut pi_scaled = Integer::from(426880u32);
    pi_scaled *= radicand.sqrt();
    pi_scaled *= &q;
    pi_scaled /= &t;

    let elapsed = start.elapsed();

    let pi_str = pi_scaled.to_string();
    println!("{}.{}", &pi_str[..1], &pi_str[1..=digits as usize]);

    if !quiet {
        eprintln!(
            "computed {} digits in {:?} ({} iterations)",
            digits, elapsed, limit
        );
    }
}
