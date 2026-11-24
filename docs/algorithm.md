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

## Binary splitting over exact integers

The series is summed with binary splitting, the same technique used by
record-setting Chudnovsky programs. For a term range \([a,b)\) the recursion
returns a triple \((P, Q, T)\) of `rug::Integer`s with

\[
P = \prod_{k=a}^{b-1} p(k), \qquad
Q = \prod_{k=a}^{b-1} q(k), \qquad
\frac{T}{Q} = \sum_{k=a}^{b-1} (-1)^k (Ak + B)\prod_{j=a}^{k}\frac{p(j)}{q(j)},
\]

where \(A = 545140134\), \(B = 13591409\), and

\[
p(k) = (6k-5)(2k-1)(6k-1), \qquad
q(k) = k^3\,\frac{640320^3}{24}.
\]

The ratio \(p(k)/q(k)\) is exactly the ratio of consecutive series terms
\(T_k/T_{k-1}\) after cancelling common factors. Ranges merge with

\[
P = P_1P_2, \qquad Q = Q_1Q_2, \qquad T = T_1Q_2 + P_1T_2,
\]

so every multiplication is balanced (operands of similar size), which is what
GMP's sub-quadratic multiplication is good at. The whole sum is therefore one
product tree costing roughly \(O(M(n)\log n)\) instead of one full-precision
division per term. The tree's two halves are independent, so the top few
levels are evaluated on bounded scoped threads; the depth and the range size
both cap the parallelism.

## Precision and output

All arithmetic is exact integer arithmetic. The final value is computed as

\[
\left\lfloor \pi\cdot 10^{d+g} \right\rfloor
=
\left\lfloor
\frac{426880\,\left\lfloor\sqrt{10005\cdot 10^{2(d+g)}}\right\rfloor\, Q}{T}
\right\rfloor ,
\]

where \(d\) is the requested digit count and \(g = 16\) guard digits absorb the
integer-square-root truncation and the omitted series tail (each term gives
~14.181 digits, so `ceil(digits / 14.181) + 2` terms leave a tail far below
the guard). The program then prints the integer part and exactly `digits`
fractional digits, truncating rather than rounding — the same contract as
before.

`--quiet` sends only the pi value to standard output. In normal mode the pi
value is still the only standard-output payload; timing and iteration details
are diagnostics on standard error. That split lets scripts pipe stdout
directly while still allowing humans to see performance information.

## Complexity and bounds

The maximum supported request is 100,000,000 decimal digits. This explicit
bound prevents accidental requests that would need infeasible memory and time.
Large requests scale by the number of Chudnovsky terms and by arbitrary
precision arithmetic cost, not by a simple linear relationship.

The implementation is a single binary. It uses [GMP](https://gmplib.org/) for
exact integers through the [`rug`](https://docs.rs/rug/) crate; no
floating-point arithmetic is involved anywhere in the computation.
