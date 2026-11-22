# Algorithm and implementation

`pi-calc` computes pi with the [Chudnovsky formula](https://en.wikipedia.org/wiki/Chudnovsky_algorithm), the same family of rapidly convergent series used for record-setting pi computations.

## Formula

The program evaluates

\[
\pi = \frac{426880\sqrt{10005}}{\displaystyle\sum_{k=0}^{\infty} T_k}
\]

where

\[
T_k = \frac{(6k)!\,(13591409 + 545140134k)}
{(3k)!\,(k!)^3\,(-640320)^{3k}}.
\]

Each term contributes about 14.181 decimal digits. `pi-calc` computes
`ceil(digits / 14.181) + 2` terms so the requested digit count has a small
series margin.

## Exact-integer recurrence

Factorials are never materialized directly. The code keeps the term
coefficient

\[
a_k = \frac{(6k)!}{(3k)!(k!)^3}
\]

as a `rug::Integer` and updates it from `a_{k-1}` with only the factors that
change between consecutive terms:

\[
a_k = a_{k-1}\cdot
\frac{(6k-5)(6k-4)(6k-3)(6k-2)(6k-1)(6k)}
{(3k-2)(3k-1)(3k)k^3}.
\]

The numerator is divisible by the denominator at every step, so integer
division is exact. The sign denominator \((-640320)^{3k}\) is also kept
exactly and multiplied by \(-640320^3 = -262537412640768000\) per term. Each
term is then formed as a high-precision floating-point ratio and added to the
series sum.

## Precision and output

The MPFR working precision is

\[
\lceil d\log_2(10)\rceil + 64\text{ bits},
\]

where \(d\) is the requested decimal digit count. The guard bits reduce the
effect of roundoff during summation and the final division. The decimal
conversion requests `digits + 2` fractional characters, then the program
prints the integer part and exactly `digits` fractional digits.

`--quiet` sends only the pi value to standard output. In normal mode the pi
value is still the only standard-output payload; timing and iteration details
are diagnostics on standard error. That split lets scripts pipe stdout
directly while still allowing humans to see performance information.

## Complexity and bounds

The maximum supported request is 100,000,000 decimal digits. This explicit
bound prevents accidental requests that would need infeasible memory and time.
Large requests scale by the number of Chudnovsky terms and by arbitrary
precision arithmetic cost, not by a simple linear relationship.

The implementation is intentionally single-binary and synchronous. It uses
[GMP](https://gmplib.org/) for exact integers and
[MPFR](https://www.mpfr.org/) for correctly controlled floating-point
arithmetic through the [`rug`](https://docs.rs/rug/) crate.
