use rug::{Float, Integer};
use rug::ops::Pow;
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
10,000 digits in ~30ms, 1,000,000 in ~5 minutes on modern hardware.
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

fn main() {
    let (digits, quiet) = parse_args();

    // Precision in bits: log2(10) approx 3.322, plus guard bits
    let precision = (digits as f64 * (10.0_f64.log2())).ceil() as u32 + 64;

    let start = Instant::now();

    // Chudnovsky algorithm: pi = 426880*sqrt(10005) / K
    // K = sum (6k)! (13591409 + 545140134k) / ((3k)! (k)!^3 (-640320)^(3k))

    let c = Float::with_val(precision, 426880) * Float::with_val(precision, 10005).sqrt();

    let mut a = Integer::from(1);        // (6k)! / ((3k)! * (k!)^3)
    let mut b = Integer::from(13591409); // 13591409 + 545140134k
    let mut t = Integer::from(1);        // (-640320)^(3k)

    let mut sum = Float::with_val(precision, 13591409);

    // Each Chudnovsky term gives ~14.181 digits
    let limit = (digits as f64 / 14.181).ceil() as u32 + 2;

    for i in 1..=limit {
        let k: u64 = i.into();

        // Recurrence for a_k = (6k)! / ((3k)! * (k!)^3)
        let six_k = Integer::from(6 * k);
        let six_k_minus_1 = Integer::from(6 * k - 1);
        let six_k_minus_2 = Integer::from(6 * k - 2);
        let six_k_minus_3 = Integer::from(6 * k - 3);
        let six_k_minus_4 = Integer::from(6 * k - 4);
        let six_k_minus_5 = Integer::from(6 * k - 5);

        let three_k = Integer::from(3 * k);
        let three_k_minus_1 = Integer::from(3 * k - 1);
        let three_k_minus_2 = Integer::from(3 * k - 2);
        let k_cubed = Integer::from(k).pow(3);

        a *= six_k_minus_5 * six_k_minus_4 * six_k_minus_3 * six_k_minus_2 * six_k_minus_1 * six_k;
        a /= three_k_minus_2 * three_k_minus_1 * three_k * k_cubed;

        b += 545140134;
        t *= -262537412640768000i64;

        let term =
            Float::with_val(precision, Integer::from(&a * &b)) / Float::with_val(precision, &t);
        sum += term;
    }

    let pi = c / sum;
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
