use rug::{Float, Integer};
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
Runtime depends heavily on precision, hardware, and the GMP/MPFR build.
";

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

/// 640320^3 / 24
const C3_OVER_24: u64 = 10_939_058_860_032_000;

/// Returns (P, Q, T) for Chudnovsky terms in [a, b).
fn binary_split(a: u64, b: u64) -> (Integer, Integer, Integer) {
    if b - a == 1 {
        let (p, q) = if a == 0 {
            (Integer::from(1), Integer::from(1))
        } else {
            let p = Integer::from(6 * a - 5) * (2 * a - 1) * (6 * a - 1);
            let q = Integer::from(a) * a * a * C3_OVER_24;
            (p, q)
        };
        let mut t = Integer::from(&p * (Integer::from(545_140_134u64) * a + 13_591_409u64));
        if a % 2 == 1 {
            t = -t;
        }
        return (p, q, t);
    }
    let m = (a + b) / 2;
    let (pam, qam, tam) = binary_split(a, m);
    let (pmb, qmb, tmb) = binary_split(m, b);
    let t = Integer::from(&tam * &qmb) + Integer::from(&pam * &tmb);
    (pam * pmb, qam * qmb, t)
}

fn main() {
    let (digits, quiet) = parse_args();

    // Precision in bits: log2(10) approx 3.322, plus guard bits
    let precision = (digits as f64 * (10.0_f64.log2())).ceil() as u32 + 64;

    let start = Instant::now();

    // Chudnovsky algorithm: pi = 426880*sqrt(10005) / K
    // K = sum (6k)! (13591409 + 545140134k) / ((3k)! (k)!^3 (-640320)^(3k))

    // Each Chudnovsky term gives ~14.181 digits
    let limit = (digits as f64 / 14.181).ceil() as u32 + 2;

    // Binary splitting: sum all terms with exact integers, then divide once.
    let (_, q, t) = binary_split(0, u64::from(limit) + 1);

    let c = Float::with_val(precision, 426880) * Float::with_val(precision, 10005).sqrt();
    let pi = c * Float::with_val(precision, &q) / Float::with_val(precision, &t);
    let elapsed = start.elapsed();

    let pi_str = pi.to_string_radix(10, Some(digits as usize + 2));
    let (int_part, frac_part) = pi_str.split_once('.').unwrap_or((&pi_str, ""));
    let frac = &frac_part[..(digits as usize).min(frac_part.len())];

    println!("{}.{}", int_part, frac);

    if !quiet {
        eprintln!(
            "computed {} digits in {:?} ({} iterations)",
            digits, elapsed, limit
        );
    }
}
