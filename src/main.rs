mod chudnovsky;
mod decimal;
mod factors;
mod flint;

use rug::Integer;
use std::env;
use std::io::{self, Write};
use std::process;
use std::thread;
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

fn main() -> io::Result<()> {
    let (digits, quiet) = parse_args();
    let start = Instant::now();
    let workers = thread::available_parallelism().map_or(1, usize::from);
    flint::set_workers(workers);
    let mut guard = 16;
    let (whole, blocks, elapsed, limit) = loop {
        let scale = digits + 2 * guard;
        let (pi, bits, limit) = chudnovsky::pi_fixed(scale, workers);
        let elapsed = start.elapsed();
        if env::var_os("PI_CALC_PROFILE").is_some() {
            eprintln!("profile: fixed-point total {elapsed:?}");
        }
        let conversion_start = Instant::now();
        let whole = Integer::from(&pi >> bits);
        let mut blocks = decimal::convert(pi.keep_bits(bits), bits, digits + guard, workers);
        if env::var_os("PI_CALC_PROFILE").is_some() {
            eprintln!(
                "profile: decimal conversion {:?}",
                conversion_start.elapsed()
            );
        }
        let last = blocks.last_mut().unwrap();
        let end = last.len() - guard as usize;
        let suffix = &last.as_bytes()[end..];

        // The fixed-point error is below two ulps. If the guard digits
        // place the result near either truncation boundary, try more bits.
        if suffix.iter().all(|&d| d == b'0') || suffix.iter().all(|&d| d == b'9') {
            guard += 16;
            continue;
        }
        last.truncate(end);
        break (whole, blocks, elapsed, limit);
    };

    let mut out = io::stdout().lock();
    write!(out, "{whole}.")?;
    for block in blocks {
        out.write_all(block.as_bytes())?;
    }
    writeln!(out)?;

    if !quiet {
        eprintln!(
            "computed {} digits in {:?} ({} iterations)",
            digits, elapsed, limit
        );
    }
    Ok(())
}
