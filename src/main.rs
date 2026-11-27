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

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

const USAGE: &str = "\
pi-calc — compute pi to arbitrary precision

USAGE:
  pi-calc [FLAGS] [DIGITS]

FLAGS:
  -h, --help    Print this help message
  -q, --quiet   Suppress timing info, output only the digits

ARGS:
  <DIGITS>    Number of decimal digits to compute (default: 1000, limited only by memory, address space, and time)

The Chudnovsky algorithm is used. Each digit costs ~log2(10) approx 3.32 bits.
Runtime depends heavily on precision, hardware, and the GMP build.
";

fn parse_args() -> (usize, bool) {
    let args: Vec<String> = env::args().collect();
    let mut digits: Option<usize> = None;
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
                match a.parse::<usize>() {
                    Ok(n) if n > 0 => digits = Some(n),
                    Ok(_) => {
                        eprintln!("error: digits must be positive");
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

fn precision_inputs(digits: usize, guard: usize) -> Option<(usize, usize)> {
    Some((digits.checked_add(2 * guard)?, digits.checked_add(guard)?))
}

fn main() -> io::Result<()> {
    let (digits, quiet) = parse_args();
    let start = Instant::now();
    let workers = thread::available_parallelism().map_or(1, usize::from);
    flint::use_mimalloc();
    flint::set_workers(workers);
    let mut guard = 16usize;
    let (whole, text, elapsed, limit) = loop {
        let Some((scale, conversion_digits)) = precision_inputs(digits, guard) else {
            eprintln!("error: digit count exceeds this platform's addressable precision");
            process::exit(1);
        };
        let conversion = decimal::Conversion::new(conversion_digits, workers);
        let bits = conversion.fraction_bits();
        let (pi, bits, limit, powers) = thread::scope(|scope| {
            let powers = scope.spawn(|| {
                let powers = conversion.powers();
                flint::end_thread();
                powers
            });
            let (pi, bits, limit) = chudnovsky::pi_fixed(scale, bits, workers);
            (
                pi,
                bits,
                limit,
                powers.join().expect("decimal powers thread panicked"),
            )
        });
        let elapsed = start.elapsed();
        if env::var_os("PI_CALC_PROFILE").is_some() {
            eprintln!("profile: fixed-point total {elapsed:?}");
        }
        let conversion_start = Instant::now();
        let whole = Integer::from(&pi >> bits);
        let mut text = decimal::convert(&pi, conversion, &powers, workers);
        text.truncate(digits + guard);
        if env::var_os("PI_CALC_PROFILE").is_some() {
            eprintln!(
                "profile: decimal conversion {:?}",
                conversion_start.elapsed()
            );
        }
        let end = text.len() - guard;
        let suffix = &text.as_bytes()[end..];

        // The fixed-point error is below two ulps. If the guard digits
        // place the result near either truncation boundary, try more bits.
        if suffix.iter().all(|&d| d == b'0') || suffix.iter().all(|&d| d == b'9') {
            guard += 16;
            continue;
        }
        text.truncate(end);
        break (whole, text, elapsed, limit);
    };

    let mut out = io::stdout().lock();
    write!(out, "{whole}.")?;
    writeln!(out, "{text}")?;

    if !quiet {
        eprintln!(
            "computed {} digits in {:?} ({} iterations)",
            digits, elapsed, limit
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::precision_inputs;

    #[test]
    fn precision_inputs_reject_usize_overflow() {
        assert_eq!(precision_inputs(usize::MAX, 16), None);
        assert!(precision_inputs(100_000_001, 16).is_some());
    }
}
