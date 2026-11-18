"""Derive the Daubechies scaling filters DB2..DB10 (2N taps) by spectral factorisation and print them as Rust.

Clean derivation: the squared magnitude of the low-pass filter is cos^(2N)(w/2) P(sin^2(w/2)) with
P(y) = sum_{k<N} C(N-1+k, k) y^k. The roots of P (as a Laurent polynomial in z) inside the unit circle,
times (1+z)^N, give the minimum-phase factor; a few Newton steps on the defining equations (orthonormality
to the even shifts and N vanishing moments) polish the numbers to full double precision.
Run: python gen_db.py  (needs numpy). PyWavelets is used only to compare, if installed.
"""
import numpy as np
from math import comb, sqrt


def factor(n):
    # polynomial in y = (2 - z - 1/z)/4, degree n-1  ->  Laurent in z, multiply by z^(n-1)
    y = np.poly1d([-0.25, 0.5, -0.25])  # z * y(z) = -(z^2 - 2z + 1)/4
    tot = np.poly1d([0.0])
    for k in range(n):
        tot = tot + comb(n - 1 + k, k) * (y ** k) * np.poly1d([1.0, 0.0]) ** (n - 1 - k)
    roots = [r for r in tot.roots if abs(r) < 1]
    h = np.poly1d([1.0])
    for _ in range(n):
        h = h * np.poly1d([1.0, 1.0])
    for r in roots:
        h = h * np.poly1d([1.0, -r])
    c = np.real(h.coeffs)
    return c * sqrt(2) / c.sum()


from decimal import Decimal, getcontext

getcontext().prec = 60


def system(h, n, D):
    """Residual and Jacobian of the defining equations, in Decimal."""
    L = 2 * n
    rows, jac = [], []
    for k in range(n):
        rows.append(sum(h[i] * h[i + 2 * k] for i in range(L - 2 * k)) - D(1 if k == 0 else 0))
        j = [D(0)] * L
        for i in range(L - 2 * k):
            j[i] += h[i + 2 * k]
            j[i + 2 * k] += h[i]
        jac.append(j)
    for m in range(0, n):
        rows.append(sum(D((-1) ** i * i ** m) * h[i] for i in range(L)))
        jac.append([D((-1) ** i * i ** m) for i in range(L)])
    return rows, jac


def solve(a, b):
    n = len(b)
    a = [r[:] + [b[i]] for i, r in enumerate(a)]
    for c in range(n):
        p = max(range(c, n), key=lambda r: abs(a[r][c]))
        a[c], a[p] = a[p], a[c]
        for r in range(c + 1, n):
            f = a[r][c] / a[c][c]
            a[r] = [x - f * y for x, y in zip(a[r], a[c])]
    x = [Decimal(0)] * n
    for i in reversed(range(n)):
        x[i] = (a[i][n] - sum(a[i][j] * x[j] for j in range(i + 1, n))) / a[i][i]
    return x


def polish(h0, n):
    D = Decimal
    h = [D(float(v)) for v in h0]
    for _ in range(12):
        r, j = system(h, n, D)
        d = solve(j, r)
        h = [x - y for x, y in zip(h, d)]
    r, _ = system(h, n, D)
    return [float(x) for x in h], float(max(abs(x) for x in r))


if __name__ == "__main__":
    try:
        import pywt
    except ImportError:
        pywt = None
    for n in range(2, 11):
        h, res = polish(factor(n), n)
        if pywt:
            ref = np.array(pywt.Wavelet(f"db{n}").rec_lo)
            if np.abs(h - ref).max() > 1e-6:
                h = h[::-1]
            print(f"// db{n} max diff to pywt {np.abs(h - ref).max():.2e}  resid {res:.1e}")
        print(f"    // DB{n}\n    &[" + ", ".join(repr(float(v)) for v in h) + "],")
