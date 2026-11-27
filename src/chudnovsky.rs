use crate::factors::{self, Factor, Sieve};
use crate::flint;
use rug::Integer;
use std::time::Instant;
use std::{env, thread};

const A: u64 = 545_140_134;
const B: u64 = 13_591_409;
const C3_OVER_24: u64 = 10_939_058_860_032_000; // 640320^3 / 24
const Q_SHIFT: u32 = C3_OVER_24.trailing_zeros();

// P, Q, and T can be divided by the same factor without changing the
// range's product P/Q or partial sum T/Q. Q's powers of two are kept
// separately because they cannot cancel against the odd P.
struct Pqt {
    p: Integer,
    q_odd: Integer,
    q_shift: usize,
    t: Integer,
}

struct Factored {
    value: Pqt,
    p: Vec<Factor>,
    q: Vec<Factor>,
}

fn split_small(
    lo: usize,
    hi: usize,
    sieve: &Sieve,
    p_factors: &mut Vec<Factor>,
    q_factors: &mut Option<Vec<Factor>>,
    need_p: bool,
) -> Pqt {
    if hi - lo == 1 {
        if lo == 0 {
            return Pqt {
                p: Integer::from(1),
                q_odd: Integer::from(1),
                q_shift: 0,
                t: Integer::from(B),
            };
        }
        let k = lo as u64;
        let parts = [6 * lo - 5, 2 * lo - 1, 6 * lo - 1];
        if need_p {
            for factor in parts {
                sieve.factor(factor, 1, p_factors);
            }
        }
        let product = parts[0] as u128 * parts[1] as u128 * parts[2] as u128;
        let p = Integer::from(product);
        let linear = u128::from(A * k + B);
        let mut t = product
            .checked_mul(linear)
            .map_or_else(|| Integer::from(product) * (A * k + B), Integer::from);
        if lo % 2 == 1 {
            t = -t;
        }

        let twos = lo.trailing_zeros() as usize;
        let odd = lo >> twos;
        if let Some(q_factors) = q_factors {
            sieve.factor(odd, 3, q_factors);
        }
        let odd_cubed = (odd as u128).pow(3);
        let odd_constant = u128::from(C3_OVER_24 >> Q_SHIFT);
        let q_odd = odd_cubed.checked_mul(odd_constant).map_or_else(
            || Integer::from(odd_cubed) * (C3_OVER_24 >> Q_SHIFT),
            Integer::from,
        );
        return Pqt {
            p,
            q_odd,
            q_shift: Q_SHIFT as usize + 3 * twos,
            t,
        };
    }

    let mid = lo + (hi - lo) / 2;
    let left = split_small(lo, mid, sieve, p_factors, q_factors, true);
    let right = split_small(mid, hi, sieve, p_factors, q_factors, need_p);
    combine(left, right, need_p, 1)
}

fn split(
    lo: usize,
    hi: usize,
    level: u32,
    parallel_depth: u32,
    workers: usize,
    sieve: &Sieve,
    need_p: bool,
) -> Factored {
    if hi - lo <= 64 {
        let mut p = Vec::with_capacity(4 * (hi - lo));
        // The leftmost ranges are never used as a right operand.
        let mut q = (lo != 0).then(|| Vec::with_capacity(7 * (hi - lo)));
        let value = split_small(lo, hi, sieve, &mut p, &mut q, need_p);
        factors::sort_and_collect(&mut p);
        let mut q = q.unwrap_or_default();
        if lo != 0 {
            factors::add_q_constant(&mut q, hi - lo);
            factors::sort_and_collect(&mut q);
        }
        return Factored { value, p, q };
    }

    let mid = lo + (hi - lo) / 2;
    let parallel = level < parallel_depth && hi - lo >= 2_048;
    let next = level + 1;
    let (left, right) = if parallel {
        thread::scope(|scope| {
            let left = scope.spawn(|| {
                let value = split(lo, mid, next, parallel_depth, workers, sieve, true);
                flint::end_thread();
                value
            });
            let right = split(mid, hi, next, parallel_depth, workers, sieve, need_p);
            (left.join().expect("split thread panicked"), right)
        })
    } else {
        (
            split(lo, mid, next, parallel_depth, workers, sieve, true),
            split(mid, hi, next, parallel_depth, workers, sieve, need_p),
        )
    };
    let (mut left, mut right) = (left, right);
    // Near the root the exact divisions and factor bookkeeping cost more
    // time than the slightly smaller multiplication operands save.
    if level >= 4 {
        let divisor = factors::cancel(&mut left.p, &mut right.q);
        if divisor != 1 {
            left.value.p.div_exact_mut(&divisor);
            right.value.q_odd.div_exact_mut(&divisor);
        }
    }

    Factored {
        value: combine(left.value, right.value, need_p, (workers >> level).max(1)),
        // Factor lists have no consumer above level four, so do not merge
        // them through the top of the tree only to drop them there.
        p: if need_p && level >= 5 {
            factors::merge(&left.p, &right.p)
        } else {
            Vec::new()
        },
        q: if lo > 0 && level >= 5 {
            factors::merge(&left.q, &right.q)
        } else {
            Vec::new()
        },
    }
}

fn combine(left: Pqt, right: Pqt, need_p: bool, workers: usize) -> Pqt {
    let mut t = flint::multiply_with_workers(&left.t, &right.q_odd, workers);
    t <<= right.q_shift;
    t += flint::multiply_with_workers(&left.p, &right.t, workers);
    let p = if need_p {
        flint::multiply_with_workers(&left.p, &right.p, workers)
    } else {
        Integer::new()
    };
    let q_odd = flint::multiply_with_workers(&left.q_odd, &right.q_odd, workers);
    Pqt {
        p,
        q_odd,
        q_shift: left.q_shift + right.q_shift,
        t,
    }
}

fn split_depth(workers: usize) -> u32 {
    if workers <= 1 {
        0
    } else {
        workers.next_power_of_two().trailing_zeros() + 1
    }
}

pub fn pi_fixed(scale: usize, bits: usize, workers: usize) -> (Integer, usize, usize) {
    // The extra terms put the series tail below one binary ulp.
    let limit = (scale as f64 / 14.181).ceil() as usize + 2;
    let split = |workers: usize| {
        let sieve = Sieve::new(6 * (limit - 1) - 1);
        split(0, limit, 0, split_depth(workers), workers, &sieve, false).value
    };
    let sqrt = || {
        let start = Instant::now();
        let root = flint::sqrt_10005(bits, workers);
        if env::var_os("PI_CALC_PROFILE").is_some() {
            eprintln!("profile: square root {:?}", start.elapsed());
        }
        root
    };
    let split_start = Instant::now();
    let (series, root) = if bits >= 330_000 && workers > 1 {
        thread::scope(|scope| {
            let root = scope.spawn(|| {
                let root = sqrt();
                flint::end_thread();
                root
            });
            (split(workers - 1), root.join().unwrap())
        })
    } else {
        (split(workers), sqrt())
    };
    if env::var_os("PI_CALC_PROFILE").is_some() {
        eprintln!("profile: binary splitting {:?}", split_start.elapsed());
    }

    let finalize_start = Instant::now();
    let mut q = series.q_odd;
    q <<= series.q_shift;
    let mut t = series.t;
    let length = t.as_limbs().len();
    let keep = bits / 64 + 3;
    // Three limbs beyond the output precision are enough: truncating both
    // operands by the same power of two changes the quotient by far less
    // than the fixed-point errors already bounded by the guard digits.
    if length > keep {
        let cut = length - keep;
        q = flint::drop_low_limbs(&q, cut);
        t = flint::drop_low_limbs(&t, cut);
    }
    let mut pi = root * 426880u32;
    pi = flint::multiply(&pi, &q);
    pi = flint::divide(&pi, &t);
    if env::var_os("PI_CALC_PROFILE").is_some() {
        eprintln!(
            "profile: final multiplication and division {:?}",
            finalize_start.elapsed()
        );
    }
    (pi, bits, limit)
}

#[cfg(test)]
mod tests {
    use super::split_depth;

    #[test]
    fn split_depth_supports_one_worker() {
        assert_eq!(split_depth(1), 0);
        assert_eq!(split_depth(2), 2);
        assert_eq!(split_depth(3), 3);
    }
}
