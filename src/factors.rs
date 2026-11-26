use rug::Integer;

#[derive(Clone, Copy)]
pub struct Factor {
    prime: u32,
    exponent: u32,
}

// P is odd, so only odd factors of Q can cancel. Store the least prime
// divisor of each odd composite. A zero entry represents a prime (or 1).
pub struct Sieve(Vec<u16>);

impl Sieve {
    pub fn new(max: u32) -> Self {
        let mut least = vec![0; (max / 2 + 1) as usize];
        let mut p = 3;
        while p * p <= max {
            if least[(p / 2) as usize] == 0 {
                for n in (p * p..=max).step_by((2 * p) as usize) {
                    let slot = &mut least[(n / 2) as usize];
                    if *slot == 0 {
                        *slot = p as u16;
                    }
                }
            }
            p += 2;
        }
        Self(least)
    }

    pub fn factor(&self, mut odd: u32, exponent: u32, factors: &mut Vec<Factor>) {
        while odd > 1 {
            let least = u32::from(self.0[(odd / 2) as usize]);
            if least == 0 {
                factors.push(Factor {
                    prime: odd,
                    exponent,
                });
                break;
            }
            factors.push(Factor {
                prime: least,
                exponent,
            });
            odd /= least;
        }
    }
}

pub fn sort_and_collect(factors: &mut Vec<Factor>) {
    factors.sort_unstable_by_key(|factor| factor.prime);
    factors.dedup_by(|next, first| {
        if next.prime == first.prime {
            first.exponent += next.exponent;
            true
        } else {
            false
        }
    });
}

pub fn merge(left: &[Factor], right: &[Factor]) -> Vec<Factor> {
    let mut merged = Vec::with_capacity(left.len().max(right.len()));
    let (mut i, mut j) = (0, 0);
    while let (Some(l), Some(r)) = (left.get(i), right.get(j)) {
        match l.prime.cmp(&r.prime) {
            std::cmp::Ordering::Less => {
                merged.push(*l);
                i += 1;
            }
            std::cmp::Ordering::Greater => {
                merged.push(*r);
                j += 1;
            }
            std::cmp::Ordering::Equal => {
                merged.push(Factor {
                    prime: l.prime,
                    exponent: l.exponent + r.exponent,
                });
                i += 1;
                j += 1;
            }
        }
    }
    merged.extend_from_slice(&left[i..]);
    merged.extend_from_slice(&right[j..]);
    merged
}

// Remove the intersection of the two factorizations and construct its value
// as a balanced product. Prime powers normally fit in a word at lower levels.
pub fn cancel(left: &mut Vec<Factor>, right: &mut Vec<Factor>) -> Integer {
    use rug::ops::Pow;

    let (mut i, mut j) = (0, 0);
    let mut parts = Vec::new();
    let mut word = 1u64;
    while let (Some(l), Some(r)) = (left.get_mut(i), right.get_mut(j)) {
        match l.prime.cmp(&r.prime) {
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
            std::cmp::Ordering::Equal => {
                let exponent = l.exponent.min(r.exponent);
                l.exponent -= exponent;
                r.exponent -= exponent;
                let prime = u64::from(l.prime);
                if let Some(power) = prime.checked_pow(exponent) {
                    if let Some(product) = word.checked_mul(power) {
                        word = product;
                    } else {
                        parts.push(Integer::from(word));
                        word = power;
                    }
                } else {
                    parts.push(Integer::from(prime).pow(exponent));
                }
                i += 1;
                j += 1;
            }
        }
    }
    left.retain(|factor| factor.exponent != 0);
    right.retain(|factor| factor.exponent != 0);
    if parts.is_empty() {
        Integer::from(word)
    } else {
        parts.push(Integer::from(word));
        product(&mut parts)
    }
}

fn product(parts: &mut [Integer]) -> Integer {
    if parts.len() == 1 {
        return std::mem::take(&mut parts[0]);
    }
    let (left, right) = parts.split_at_mut(parts.len() / 2);
    product(left) * product(right)
}

pub fn add_q_constant(factors: &mut Vec<Factor>, terms: u32) {
    // Odd part of (640320^3 / 24)^terms.
    for (prime, exponent) in [(3, 2), (5, 3), (23, 3), (29, 3)] {
        factors.push(Factor {
            prime,
            exponent: exponent * terms,
        });
    }
}
