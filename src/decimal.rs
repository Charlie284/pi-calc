use crate::flint;
use rug::{ops::Pow, Integer};
use std::thread;

// Multiplication by 10^n moves n digits before the binary point. Compute
// it as multiplication by 5^n and a shift, then convert the extracted
// integer while the remaining fraction produces the next block.
pub fn convert(value: Integer, bits: u32, digits: u32, workers: usize) -> Vec<String> {
    let parallel = workers > 1 && digits >= 200_000;
    let count = if parallel { digits / 2 } else { digits };
    let scaled = flint::multiply(&value, &Integer::from(5).pow(count));
    let remaining_bits = bits - count;
    let integer = Integer::from(&scaled >> remaining_bits);
    if !parallel {
        return vec![padded(integer, count)];
    }

    let fraction = scaled.keep_bits(remaining_bits);
    thread::scope(|scope| {
        let first = scope.spawn(move || padded(integer, count));
        let mut rest = convert(fraction, remaining_bits, digits - count, workers - 1);
        rest.insert(0, first.join().unwrap());
        rest
    })
}

fn padded(integer: Integer, digits: u32) -> String {
    let string = flint::decimal_string(&integer);
    let width = digits as usize;
    if string.len() == width {
        return string;
    }
    let mut padded = String::with_capacity(width);
    padded.extend(std::iter::repeat_n('0', width - string.len()));
    padded.push_str(&string);
    padded
}
