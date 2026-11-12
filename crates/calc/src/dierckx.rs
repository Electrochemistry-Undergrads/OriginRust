//! Smoothing cubic spline with automatically chosen knots, after P. Dierckx (the method of the
//! FITPACK routine `curfit`, also behind NAG `e02be`): the least-squares cubic B-spline with the
//! fewest knots whose sum of squared residuals is close to a given value `s`, then the smoothness
//! penalty is tuned so that the residual sum equals `s` (to a relative 0.001).
//!
//! The method is written down in Dierckx, "Curve and Surface Fitting with Splines" (1993), chapter
//! 4. This file follows that description. Arrays are 1-based inside (index 0 unused) so that the
//! formulas can be read against the book.

// The 1-based index loops are kept on purpose: they read like the formulas of the book.
#![allow(clippy::needless_range_loop, clippy::manual_memcpy, clippy::explicit_counter_loop)]

const K: usize = 3;
const K1: usize = K + 1;
const K2: usize = K + 2;
const NMIN: usize = 2 * K1;
const TOL: f64 = 0.001;
const MAXIT: usize = 20;

/// A cubic spline in B-spline form.
#[derive(Debug, Clone, PartialEq)]
pub struct CubicBSpline {
    /// Knots, 0-based, with the end knots repeated four times.
    pub knots: Vec<f64>,
    /// B-spline coefficients, one per knot minus four.
    pub coefs: Vec<f64>,
}

/// Plane rotation that annihilates `piv` against the diagonal value `ww`; returns (cos, sin) and
/// replaces `ww` by the new diagonal value.
fn givens(piv: f64, ww: &mut f64) -> (f64, f64) {
    let store = piv.abs();
    let dd = if store >= *ww {
        store * (1.0 + (*ww / piv).powi(2)).sqrt()
    } else {
        *ww * (1.0 + (piv / *ww).powi(2)).sqrt()
    };
    let cs = *ww / dd;
    let sn = piv / dd;
    *ww = dd;
    (cs, sn)
}

fn rotate(cs: f64, sn: f64, a: &mut f64, b: &mut f64) {
    let (s1, s2) = (*a, *b);
    *b = cs * s2 + sn * s1;
    *a = cs * s1 - sn * s2;
}

/// Values of the four cubic B-splines that are not zero at `x`, where `t[l] <= x < t[l+1]`
/// (1-based knots). `h[1..=4]`.
fn basis(t: &[f64], l: usize, x: f64) -> [f64; K1 + 1] {
    let mut h = [0.0; K1 + 1];
    let mut hh = [0.0; K1 + 1];
    h[1] = 1.0;
    for j in 1..=K {
        hh[1..=j].copy_from_slice(&h[1..=j]);
        h[1] = 0.0;
        for i in 1..=j {
            let li = l + i;
            let lj = li - j;
            let f = hh[i] / (t[li] - t[lj]);
            h[i] += f * (t[li] - x);
            h[i + 1] = f * (x - t[lj]);
        }
    }
    h
}

/// Back substitution for an upper triangular band matrix `a` (bandwidth `k`, row i holds the
/// diagonal in column 1) and right side `z`; returns the solution (1-based).
fn back(a: &[[f64; K2 + 1]], z: &[f64], n: usize, k: usize) -> Vec<f64> {
    let mut c = vec![0.0; n + 1];
    let k1 = k - 1;
    c[n] = z[n] / a[n][1];
    let mut i = n - 1;
    for j in 2..=n {
        let mut store = z[i];
        let i1 = k1.min(j - 1);
        for l in 1..=i1 {
            store -= c[i + l] * a[i][l + 1];
        }
        c[i] = store / a[i][1];
        i -= 1;
    }
    c
}

/// Jumps of the third derivative of the B-splines at the interior knots (matrix `b`, one row per
/// interior knot, `K2` columns), the smoothness penalty of the fit.
fn discontinuities(t: &[f64], n: usize) -> Vec<[f64; K2 + 1]> {
    let nk1 = n - K1;
    let nrint = nk1 - K;
    let fac = nrint as f64 / (t[nk1 + 1] - t[K1]);
    let mut b = vec![[0.0; K2 + 1]; n - NMIN + 1];
    let mut h = [0.0; 2 * K1 + 1];
    for l in K2..=nk1 {
        let lmk = l - K1;
        for j in 1..=K1 {
            let ik = j + K1;
            let lj = l + j;
            let lk = lj - K2;
            h[j] = t[l] - t[lk];
            h[ik] = t[l] - t[lj];
        }
        let mut lp = lmk;
        for j in 1..=K2 {
            let mut jk = j;
            let mut prod = h[j];
            for _ in 1..=K {
                jk += 1;
                prod = prod * h[jk] * fac;
            }
            let lk = lp + K1;
            b[lmk][j] = (t[lk] - t[lp]) / prod;
            lp += 1;
        }
    }
    b
}

/// Root-finding step of the smoothing parameter (rational interpolation); updates the bracket.
fn rational_step(p1: &mut f64, f1: &mut f64, p2: f64, f2: f64, p3: &mut f64, f3: &mut f64) -> f64 {
    let p = if *p3 > 0.0 {
        let h1 = *f1 * (f2 - *f3);
        let h2 = f2 * (*f3 - *f1);
        let h3 = *f3 * (*f1 - f2);
        -(*p1 * p2 * h3 + p2 * *p3 * h1 + *p3 * *p1 * h2) / (*p1 * h1 + p2 * h2 + *p3 * h3)
    } else {
        (*p1 * (*f1 - *f3) * f2 - p2 * (f2 - *f3) * *f1) / ((*f1 - f2) * *f3)
    };
    if f2 < 0.0 {
        *p3 = p2;
        *f3 = f2;
    } else {
        *p1 = p2;
        *f1 = f2;
    }
    p
}

/// Adds one knot in the interval with the largest residual sum that still has data points inside.
fn add_knot(x: &[f64], t: &mut [f64], n: &mut usize, fpint: &mut [f64], nrdata: &mut [usize], nrint: &mut usize) {
    let mut fpmax = 0.0;
    let mut jbegin = 1;
    let (mut number, mut maxpt, mut maxbeg) = (0, 0, 0);
    for j in 1..=*nrint {
        let jpoint = nrdata[j];
        if !(fpmax >= fpint[j] || jpoint == 0) {
            fpmax = fpint[j];
            number = j;
            maxpt = jpoint;
            maxbeg = jbegin;
        }
        jbegin += jpoint + 1;
    }
    let ihalf = maxpt / 2 + 1;
    let nrx = maxbeg + ihalf;
    let next = number + 1;
    if next <= *nrint {
        for j in next..=*nrint {
            let jj = next + *nrint - j;
            fpint[jj + 1] = fpint[jj];
            nrdata[jj + 1] = nrdata[jj];
            let jk = jj + K;
            t[jk + 1] = t[jk];
        }
    }
    nrdata[number] = ihalf - 1;
    nrdata[next] = maxpt - ihalf;
    let am = maxpt as f64;
    fpint[number] = fpmax * nrdata[number] as f64 / am;
    fpint[next] = fpmax * nrdata[next] as f64 / am;
    t[next + K] = x[nrx];
    *n += 1;
    *nrint += 1;
}

/// Interval index `l` (1-based, `t[l] <= x < t[l+1]`, at most `nk1`) of each data point.
fn intervals(x: &[f64], t: &[f64], m: usize, nk1: usize) -> Vec<usize> {
    let mut l = K1;
    let mut out = vec![0; m + 1];
    for it in 1..=m {
        while x[it] >= t[l + 1] && l != nk1 {
            l += 1;
        }
        out[it] = l;
    }
    out
}

/// Fits a cubic smoothing spline with the residual sum of squares close to `s` (`s > 0`) to the
/// points `(xs, ys)`, which must be strictly increasing in x and at least four. Unit weights.
/// Returns the spline and the residual sum of squares reached.
pub fn smoothing_spline(xs: &[f64], ys: &[f64], s: f64) -> Option<(CubicBSpline, f64)> {
    let m = xs.len();
    if m < K1 || ys.len() != m || s.is_nan() || s <= 0.0 || xs.windows(2).any(|w| w[1].is_nan() || w[1] <= w[0]) {
        return None;
    }
    let nmax = m + K1;
    let mut x = vec![0.0; m + 1];
    let mut y = vec![0.0; m + 1];
    x[1..].copy_from_slice(xs);
    y[1..].copy_from_slice(ys);
    let acc = TOL * s;
    let nest = nmax;
    let mut t = vec![0.0; nest + 1];
    let mut n = NMIN;
    for i in 1..=K1 {
        t[i] = x[1];
        t[n - i + 1] = x[m];
    }
    let mut nrint = 1;
    let mut nrdata = vec![0usize; nest + 1];
    let mut fpint = vec![0.0; nest + 1];
    nrdata[1] = m.saturating_sub(2);
    let mut fp0 = 0.0;
    let mut fpold = 0.0;
    let mut nplus = 0usize;
    let mut first = true;

    let mut c: Vec<f64>;
    let mut fp;
    // Observation matrix kept from the last least-squares fit (used for the starting smoothing
    // parameter and as the base of the penalised system).
    let mut a: Vec<[f64; K2 + 1]>;
    let mut z: Vec<f64>;
    let mut iter = 0;
    loop {
        iter += 1;
        if iter > m + 1 {
            return None;
        }
        for i in 1..=K1 {
            t[i] = x[1];
            t[n + 1 - i] = x[m];
        }
        let nk1 = n - K1;
        // least-squares spline for the present knots
        a = vec![[0.0; K2 + 1]; nk1 + 1];
        z = vec![0.0; nk1 + 1];
        fp = 0.0;
        let ls = intervals(&x, &t, m, nk1);
        for it in 1..=m {
            let l = ls[it];
            let mut h = basis(&t, l, x[it]);
            let mut yi = y[it];
            let mut j = l - K1;
            for i in 1..=K1 {
                j += 1;
                let piv = h[i];
                if piv == 0.0 {
                    continue;
                }
                let (cs, sn) = givens(piv, &mut a[j][1]);
                rotate(cs, sn, &mut yi, &mut z[j]);
                if i == K1 {
                    break;
                }
                let mut i2 = 1;
                for i1 in (i + 1)..=K1 {
                    i2 += 1;
                    rotate(cs, sn, &mut h[i1], &mut a[j][i2]);
                }
            }
            fp += yi * yi;
        }
        c = back(&a, &z, nk1, K1);
        if first {
            fp0 = fp;
        }
        let fpms = fp - s;
        let ier_polynomial = first;
        first = false;
        if fpms.abs() < acc {
            break;
        }
        if fpms < 0.0 {
            if ier_polynomial {
                // the least-squares polynomial is already close enough
                let mut full = vec![0.0; nk1];
                full.copy_from_slice(&c[1..=nk1]);
                return Some((CubicBSpline { knots: t[1..=n].to_vec(), coefs: full }, fp));
            }
            // smoothing parameter search, below
            match tune_penalty(&x, &y, &t, n, m, &a, &z, s, acc, fp0, fpms) {
                Some((cc, f)) => {
                    c = cc;
                    fp = f;
                    break;
                }
                None => return None,
            }
        }
        if n == nmax {
            // interpolating spline reached
            break;
        }
        // how many knots to add
        if iter == 1 {
            nplus = 1;
        } else {
            let mut npl1 = nplus * 2;
            if fpold - fp > acc {
                npl1 = (nplus as f64 * fpms / (fpold - fp)) as usize;
            }
            nplus = (nplus * 2).min(npl1.max(nplus / 2).max(1));
        }
        fpold = fp;
        // residual sum of squares per knot interval
        let mut fpart = 0.0;
        let mut i = 1;
        let mut lf = K1;
        for it in 1..=m {
            let mut new = false;
            if x[it] >= t[lf + 1] && lf != nk1 {
                new = true;
                lf += 1;
            }
            let h = basis(&t, lf, x[it]);
            let mut term = 0.0;
            for j in 1..=K1 {
                term += c[lf - K1 + j] * h[j];
            }
            let term = (term - y[it]).powi(2);
            fpart += term;
            if new {
                let store = term * 0.5;
                fpint[i] = fpart - store;
                i += 1;
                fpart = store;
            }
        }
        fpint[nrint] = fpart;
        for _ in 0..nplus {
            add_knot(&x, &mut t, &mut n, &mut fpint, &mut nrdata, &mut nrint);
            if n == nmax {
                break;
            }
        }
        if n == nmax {
            // knots of the interpolating spline: not-a-knot
            for j in 1..=(m - K1) {
                t[j + K1] = x[j + 2];
            }
        }
    }
    let nk1 = n - K1;
    Some((CubicBSpline { knots: t[1..=n].to_vec(), coefs: c[1..=nk1].to_vec() }, fp))
}

/// Finds the smoothing parameter p for which the residual sum of squares of the penalised fit is
/// `s`, with the knots `t` fixed. Returns the coefficients (1-based) and the residual sum.
#[allow(clippy::too_many_arguments)]
fn tune_penalty(
    x: &[f64],
    y: &[f64],
    t: &[f64],
    n: usize,
    m: usize,
    a: &[[f64; K2 + 1]],
    z: &[f64],
    s: f64,
    acc: f64,
    fp0: f64,
    fpms0: f64,
) -> Option<(Vec<f64>, f64)> {
    let nk1 = n - K1;
    let n8 = n - NMIN;
    let b = discontinuities(t, n);
    let (mut p1, mut f1) = (0.0, fp0 - s);
    let (mut p3, mut f3) = (-1.0, fpms0);
    let mut p: f64 = (1..=nk1).map(|i| a[i][1]).sum();
    p = nk1 as f64 / p;
    let (mut ich1, mut ich3) = (false, false);
    let ls = intervals(x, t, m, nk1);
    for iter in 1..=MAXIT {
        let pinv = 1.0 / p;
        let mut c = z.to_vec();
        let mut g: Vec<[f64; K2 + 1]> = a.to_vec();
        for row in g.iter_mut() {
            row[K2] = 0.0;
        }
        for jj in 1..=n8 {
            let mut h = [0.0; K2 + 1];
            for i in 1..=K2 {
                h[i] = b[jj][i] * pinv;
            }
            let mut yi = 0.0;
            for j in jj..=nk1 {
                let piv = h[1];
                let (cs, sn) = givens(piv, &mut g[j][1]);
                rotate(cs, sn, &mut yi, &mut c[j]);
                if j == nk1 {
                    break;
                }
                let i2 = if j > n8 { nk1 - j } else { K1 };
                for i in 1..=i2 {
                    let i1 = i + 1;
                    let mut hv = h[i1];
                    rotate(cs, sn, &mut hv, &mut g[j][i1]);
                    h[i1] = hv;
                    h[i] = h[i1];
                }
                h[i2 + 1] = 0.0;
            }
        }
        let c = back(&g, &c, nk1, K2);
        let mut fp = 0.0;
        for it in 1..=m {
            let l = ls[it];
            let h = basis(t, l, x[it]);
            let mut term = 0.0;
            for j in 1..=K1 {
                term += c[l - K1 + j] * h[j];
            }
            fp += (term - y[it]).powi(2);
        }
        let fpms = fp - s;
        if fpms.abs() < acc {
            return Some((c, fp));
        }
        if iter == MAXIT {
            // the search did not converge; FITPACK still returns the last spline
            return Some((c, fp));
        }
        let (p2, f2) = (p, fpms);
        if !ich3 {
            if f2 - f3 <= acc {
                p3 = p2;
                f3 = f2;
                p *= 0.04;
                if p <= p1 {
                    p = p1 * 0.9 + p2 * 0.1;
                }
                continue;
            }
            if f2 < 0.0 {
                ich3 = true;
            }
        }
        if !ich1 {
            if f1 - f2 <= acc {
                p1 = p2;
                f1 = f2;
                p /= 0.04;
                if p3 < 0.0 {
                    continue;
                }
                if p >= p3 {
                    p = p2 * 0.1 + p3 * 0.9;
                }
                continue;
            }
            if f2 >= 0.0 {
                ich1 = true;
            }
        }
        if f2 >= f1 || f2 <= f3 {
            // FITPACK reports "theoretically impossible result" (too small a factor) and still
            // returns the spline it has
            return Some((c, fp));
        }
        p = rational_step(&mut p1, &mut f1, p2, f2, &mut p3, &mut f3);
    }
    None
}

impl CubicBSpline {
    /// Value at `x`, which is clamped to the knot range.
    pub fn eval(&self, x: f64) -> f64 {
        let n = self.knots.len();
        let nk1 = n - K1;
        let mut t = Vec::with_capacity(n + 1);
        t.push(0.0);
        t.extend_from_slice(&self.knots);
        let x = x.clamp(t[K1], t[nk1 + 1]);
        let mut l = K1;
        while x >= t[l + 1] && l != nk1 {
            l += 1;
        }
        let h = basis(&t, l, x);
        (1..=K1).map(|j| self.coefs[l - K1 + j - 1] * h[j]).sum()
    }
}
