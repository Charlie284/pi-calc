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
    q_shift: u32,
    t: Integer,
}

struct Factored {
    value: Pqt,
    p: Vec<Factor>,
    q: Vec<Factor>,
}

fn split_small(
    lo: u32,
    hi: u32,
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
        let k = u64::from(lo);
        let parts = [6 * lo - 5, 2 * lo - 1, 6 * lo - 1];
        if need_p {
            for factor in parts {
                sieve.factor(factor, 1, p_factors);
            }
        }
        let product = u128::from(parts[0]) * u128::from(parts[1]) * u128::from(parts[2]);
        let p = Integer::from(product);
        let mut t: Integer = (&p * (A * k + B)).into();
        if lo % 2 == 1 {
            t = -t;
        }

        let twos = lo.trailing_zeros();
        let odd = u64::from(lo >> twos);
        if let Some(q_factors) = q_factors {
            sieve.factor(odd as u32, 3, q_factors);
        }
        let mut q_odd = Integer::from(u128::from(odd).pow(3));
        q_odd *= C3_OVER_24 >> Q_SHIFT;
        return Pqt {
            p,
            q_odd,
            q_shift: Q_SHIFT + 3 * twos,
            t,
        };
    }

    let mid = lo + (hi - lo) / 2;
    let left = split_small(lo, mid, sieve, p_factors, q_factors, true);
    let right = split_small(mid, hi, sieve, p_factors, q_factors, need_p);
    combine(left, right, need_p, 0)
}

fn split(lo: u32, hi: u32, depth: u32, sieve: &Sieve, need_p: bool) -> Factored {
    if hi - lo <= 16 {
        let mut p = Vec::with_capacity(4 * (hi - lo) as usize);
        // The leftmost ranges are never used as a right operand.
        let mut q = (lo != 0).then(|| Vec::with_capacity(7 * (hi - lo) as usize));
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
    let parallel = depth > 0 && hi - lo >= 2_048;
    let (left, right) = if parallel {
        thread::scope(|scope| {
            let left = scope.spawn(|| split(lo, mid, depth - 1, sieve, true));
            let right = split(mid, hi, depth - 1, sieve, need_p);
            (left.join().expect("split thread panicked"), right)
        })
    } else {
        (
            split(lo, mid, 0, sieve, true),
            split(mid, hi, 0, sieve, need_p),
        )
    };
    let (mut left, mut right) = (left, right);
    let divisor = factors::cancel(&mut left.p, &mut right.q);
    if divisor != 1 {
        left.value.p.div_exact_mut(&divisor);
        right.value.q_odd.div_exact_mut(&divisor);
    }

    Factored {
        value: combine(
            left.value,
            right.value,
            need_p,
            if hi - lo >= 100_000 { depth } else { 0 },
        ),
        p: if need_p {
            factors::merge(&left.p, &right.p)
        } else {
            Vec::new()
        },
        q: if lo > 0 {
            factors::merge(&left.q, &right.q)
        } else {
            Vec::new()
        },
    }
}

fn combine(left: Pqt, right: Pqt, need_p: bool, depth: u32) -> Pqt {
    let (t, p, q_odd) = if depth > 0 {
        // The children have joined, so these products can reuse their workers.
        thread::scope(|scope| {
            let first = scope.spawn(|| {
                let mut term = flint::multiply(&left.t, &right.q_odd);
                term <<= right.q_shift;
                term
            });
            let second = || flint::multiply(&left.p, &right.t);
            let products = || {
                let q = flint::multiply(&left.q_odd, &right.q_odd);
                let p = if need_p {
                    flint::multiply(&left.p, &right.p)
                } else {
                    Integer::new()
                };
                (p, q)
            };
            if depth > 1 {
                let second = scope.spawn(second);
                let (p, q) = products();
                (first.join().unwrap() + second.join().unwrap(), p, q)
            } else {
                let second = second();
                let (p, q) = products();
                (first.join().unwrap() + second, p, q)
            }
        })
    } else {
        let mut t = flint::multiply(&left.t, &right.q_odd);
        t <<= right.q_shift;
        t += flint::multiply(&left.p, &right.t);
        let p = if need_p {
            flint::multiply(&left.p, &right.p)
        } else {
            Integer::new()
        };
        (t, p, flint::multiply(&left.q_odd, &right.q_odd))
    };
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
        (workers - 1).ilog2() + 1
    }
}

pub fn pi_fixed(scale: u32, workers: usize) -> (Integer, u32, u32) {
    let bits = (f64::from(scale) * 10.0_f64.log2()).ceil() as u32;
    // The extra terms put the series tail below one binary ulp.
    let limit = (f64::from(scale) / 14.181).ceil() as u32 + 2;
    let split = |workers: usize| {
        let sieve = Sieve::new(6 * (limit - 1) - 1);
        split(0, limit, split_depth(workers), &sieve, false).value
    };
    let sqrt = || {
        let start = Instant::now();
        let root = (Integer::from(10005u32) << (2 * bits)).sqrt();
        if env::var_os("PI_CALC_PROFILE").is_some() {
            eprintln!("profile: square root {:?}", start.elapsed());
        }
        root
    };
    let split_start = Instant::now();
    let (series, root) = if bits >= 330_000 && workers > 1 {
        thread::scope(|scope| {
            let root = scope.spawn(sqrt);
            (split(workers - 1), root.join().unwrap())
        })
    } else {
        (split(workers), sqrt())
    };
    if env::var_os("PI_CALC_PROFILE").is_some() {
        eprintln!("profile: binary splitting {:?}", split_start.elapsed());
    }

    let finalize_start = Instant::now();
    let mut pi = root * 426880u32;
    pi = flint::multiply(&pi, &series.q_odd);
    pi <<= series.q_shift;
    pi = flint::divide(&pi, &series.t);
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
        assert_eq!(split_depth(2), 1);
        assert_eq!(split_depth(3), 2);
    }
}
