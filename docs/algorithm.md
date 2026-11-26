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

Each term contributes about 14.181 decimal digits. The number of terms is
`ceil(scale / 14.181) + 2`, where `scale` includes the requested digits and
the guard digits used to verify decimal truncation.

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

The factorial contribution to each term is obtained from the previous one
by multiplying by \(p(k)/q(k)\). The sign and linear factor \(Ak+B\) are
applied separately, with \(p(0)=q(0)=1\). Adjacent ranges merge by

\[
P = P_1P_2, \qquad Q = Q_1Q_2, \qquad T = T_1Q_2 + P_1T_2.
\]

If \(g\) divides both \(P_1\) and \(Q_2\), it divides all three merge
results. The implementation divides those two inputs by \(g\) before
multiplying, preserving \(P/Q\) and \(T/Q\) while keeping the operands
smaller. A sieve supplies odd prime factors, which are collected once per
small group of terms and merged in sorted order. Since \(P\) is always odd,
powers of two in \(Q\) are kept as a shift count. The two halves of large
ranges and the biggest independent merge products run on bounded scoped
threads. See [Bellard's description of factor cancellation](https://bellard.org/pi/pi2700e9/pipcrecord.pdf).

## Precision and output

For a binary scale of \(b\) bits, the calculator forms the fixed-point approximation

\[
x =
\left\lfloor
\frac{426880\,\left\lfloor\sqrt{10005\cdot 2^{2b}}\right\rfloor Q}{T}
\right\rfloor.
\]

The integer square root can run alongside the series. The series
alternates, with \(|T_1/T_0| < 2\cdot10^{-14}\) and
\(|T_k/T_{k-1}| < 10^{-14.181}\) for \(k\ge2\). The first omitted term
bounds the tail, so the two extra terms put the resulting error in pi
well below one binary unit. The root loses less than one unit before
multiplication by \(426880Q/T < 0.032\), and the final division loses
less than one unit. Together, these bounds put \(x\) within two units
of \(\pi\cdot 2^b\).

For decimal conversion, multiplying the remaining fraction by \(5^k\)
and moving its binary point by \(k\) extracts the next \(k\) decimal
digits. FLINT converts large extracted integers while the remaining fraction
is processed in parallel. Blocks are zero-padded to their fixed widths.

The first pass uses 32 decimal digits' worth of extra binary precision
and checks 16 extra decimal digits before truncating. A suffix of all
zeros or all nines triggers a retry at higher precision, since the
fixed-point error could cross a truncation boundary. The calculator
prints the integer part and exactly the requested number of fractional
digits.

`--quiet` sends only the pi value to standard output. In normal mode the pi
value goes to standard output, with timing and iteration details on standard
error. This lets scripts pipe the digits directly while keeping performance
information visible.

## Complexity and bounds

The maximum supported request is 100,000,000 decimal digits. This explicit
bound prevents accidental requests that would need infeasible memory and time.
Runtime grows with the number of Chudnovsky terms and the cost of multiplying,
dividing, and converting large integers.

The implementation uses [GMP](https://gmplib.org/) through the
[`rug`](https://docs.rs/rug/) crate for the series representation and small arithmetic.
Large multiplications call FLINT's small-prime FFT directly on GMP limbs; the final division
uses FLINT's multiplication-based Newton division, and large decimal strings use FLINT's
radix conversion. FLINT worker threads are sized from the available parallelism.
