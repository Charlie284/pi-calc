use crate::flint;
use rug::Integer;
use std::thread;

const FRACTION_GUARD_LIMBS: usize = 2;

#[derive(Clone, Copy)]
pub struct Conversion {
    chunk: usize,
    levels: u32,
    chunk_limbs: usize,
}

impl Conversion {
    pub fn new(digits: usize, workers: usize) -> Self {
        let mut levels = if workers <= 1 {
            0
        } else {
            workers.next_power_of_two().trailing_zeros() + 1
        };
        // Spawning a wide tree for tiny chunks costs more than formatting
        // them serially. Large conversions keep one oversubscribed level for
        // load balancing.
        while levels > 0 && digits.div_ceil(1 << levels) < 1_000 {
            levels -= 1;
        }
        let chunk = digits.div_ceil(1 << levels);
        let chunk_limbs = (chunk as f64 * std::f64::consts::LOG2_10 / 64.0) as usize + 1;
        Self {
            chunk,
            levels,
            chunk_limbs,
        }
    }

    pub fn limbs(self, level: u32) -> usize {
        self.chunk_limbs
            .checked_shl(level)
            .and_then(|limbs| limbs.checked_add(FRACTION_GUARD_LIMBS))
            .expect("digit count exceeds addressable precision")
    }

    pub fn fraction_bits(self) -> usize {
        64 * self.limbs(self.levels)
    }

    pub fn capacity(self) -> usize {
        self.chunk
            .checked_shl(self.levels)
            .expect("digit count exceeds addressable precision")
    }

    pub fn powers(self) -> Vec<Integer> {
        let mut powers = vec![pow10(self.chunk)];
        for _ in 1..self.levels {
            let last = &powers[powers.len() - 1];
            powers.push(flint::multiply_with_workers(last, last, 1));
        }
        powers
    }
}

fn pow5(exponent: usize) -> Integer {
    use rug::ops::Pow;
    if exponent < 65_536 {
        return Integer::from(5).pow(exponent as u32);
    }
    let half = pow5(exponent / 2);
    let square = flint::multiply_with_workers(&half, &half, 1);
    if exponent & 1 == 1 {
        square * 5usize
    } else {
        square
    }
}

fn pow10(exponent: usize) -> Integer {
    pow5(exponent) << exponent
}

fn worker<T>(work: impl FnOnce() -> T) -> T {
    let value = work();
    flint::end_thread();
    value
}

fn join<L: Send, R: Send>(
    parallel: bool,
    left: impl FnOnce() -> L + Send,
    right: impl FnOnce() -> R + Send,
) -> (L, R) {
    if !parallel {
        return (left(), right());
    }
    thread::scope(|scope| {
        let left = scope.spawn(|| worker(left));
        let right = right();
        (left.join().expect("conversion thread panicked"), right)
    })
}

// Writes `out` with the leading decimal digits of a binary fraction. Each
// level multiplies once by a precomputed power of ten to separate the lower
// half of the digits; the two halves then convert independently. The buffer
// is prefilled with zeroes, so short converted integers leave leading zeroes.
fn write_fraction(
    fraction: Integer,
    level: u32,
    conversion: Conversion,
    powers: &[Integer],
    out: &mut [u8],
    workers: usize,
) {
    let limbs = conversion.limbs(level);
    if level == 0 {
        let shifted = flint::multiply_with_workers(&fraction, &powers[0], workers);
        let digits = flint::decimal_string(&flint::drop_low_limbs(&shifted, limbs));
        let padding = out.len() - digits.len();
        out[padding..].copy_from_slice(digits.as_bytes());
        return;
    }

    let dropped = limbs - conversion.limbs(level - 1);
    let upper = flint::drop_low_limbs(&fraction, dropped);
    let shifted = flint::multiply_with_workers(&fraction, &powers[level as usize - 1], workers);
    drop(fraction);
    let shifted_limbs = shifted.as_limbs();
    let lower = flint::nonnegative_from_limbs(
        shifted_limbs
            .get(dropped..limbs.min(shifted_limbs.len()))
            .unwrap_or_default(),
    );
    drop(shifted);
    flint::end_thread();

    let (out_upper, out_lower) = out.split_at_mut(out.len() / 2);
    let workers = (workers / 2).max(1);
    join(
        true,
        || write_fraction(upper, level - 1, conversion, powers, out_upper, workers),
        || write_fraction(lower, level - 1, conversion, powers, out_lower, workers),
    );
}

pub fn convert(pi: &Integer, conversion: Conversion, powers: &[Integer], workers: usize) -> String {
    let limbs = conversion.limbs(conversion.levels);
    let fraction = flint::nonnegative_from_limbs(&pi.as_limbs()[..limbs.min(pi.as_limbs().len())]);
    let mut digits = vec![b'0'; conversion.capacity()];
    write_fraction(
        fraction,
        conversion.levels,
        conversion,
        powers,
        &mut digits,
        workers,
    );
    String::from_utf8(digits).expect("decimal digits are ASCII")
}

#[cfg(test)]
mod tests {
    use super::Conversion;

    #[test]
    fn precision_is_not_truncated_at_u32_boundary() {
        let bits = Conversion::new(1_300_000_000, 1).fraction_bits();
        assert!(bits > u32::MAX as usize);
    }
}
