"""Prints crates/calc/tests/fixtures/scipy_filters.rs: reference numbers from scipy.signal for the IIR and FIR
filter design and application of `sheaf-calc` (`iir.rs`, `fir.rs`; T55).

Usage: python3 tools/iir-fir-ref.py > crates/calc/tests/fixtures/scipy_filters.rs
Needs numpy and scipy (the numbers in the repository were made with scipy 1.18.1). The input signal is made here
from a fixed seed. None of these numbers come from Origin: Origin's documentation names the filter families and
options but no formulas (see docs/spec/analysis-tools.md 17).

What is taken from scipy:
  IIR design   scipy.signal.iirfilter (zeros, poles and gain; numerator and denominator), sosfreqz (the magnitude of
               the sections), sosfilt and sosfiltfilt of the sections scipy pairs (Sheaf pairs the sections its own
               way, so the sections themselves are not compared, only what they compute).
  IIR order    scipy.signal.buttord, cheb1ord, cheb2ord, ellipord (order and natural frequencies).
  FIR design   scipy.signal.firwin (every window of scipy.signal.get_window), firls, kaiserord, kaiser_beta.
  FIR apply    scipy.signal.filtfilt with the FIR numerator, and np.convolve of the odd extension for one way.
"""
import numpy as np
import scipy
from scipy import signal

N = 100
rng = np.random.default_rng(55)
i = np.arange(N)
X = np.sin(2 * np.pi * 0.03 * i) + 0.5 * np.sin(2 * np.pi * 0.31 * i) + 0.2 * rng.standard_normal(N) + 3.0
MAG_W = [0.01, 0.05, 0.1, 0.2, 0.3, 0.45, 0.6, 0.8, 0.95]

METHODS = ['butter', 'cheby1', 'cheby2', 'ellip']
RESP = ['lowpass', 'highpass', 'bandpass', 'bandstop']


def f(v):
    return repr(float(v))


def arr(v):
    return '&[' + ', '.join(f(x) for x in v) + ']'


def carr(v):
    return '&[' + ', '.join('(%s, %s)' % (f(c.real), f(c.imag)) for c in v) + ']'


print('//! Numbers from scipy %s for the IIR and FIR filters (made by tools/iir-fir-ref.py). Do not edit by hand.' % scipy.__version__)
print('//! These numbers come from scipy, not from Origin.')
print('#![allow(clippy::excessive_precision, clippy::approx_constant, dead_code)]')
print('''
pub const SIGNAL: &[f64] = %s;
pub const MAG_W: &[f64] = %s;

/// `method` and `response` are indexes into butter, cheby1, cheby2, ellip and lowpass, highpass, bandpass, bandstop.
pub struct IirCase {
    pub name: &'static str,
    pub method: usize,
    pub response: usize,
    pub order: usize,
    pub wn: &'static [f64],
    pub rp: f64,
    pub rs: f64,
    pub zeros: &'static [(f64, f64)],
    pub poles: &'static [(f64, f64)],
    pub gain: f64,
    pub b: &'static [f64],
    pub a: &'static [f64],
    pub magnitude: &'static [f64],
    pub one_way: &'static [f64],
    pub zero_phase: &'static [f64],
}

pub const IIR: &[IirCase] = &[''' % (arr(X), arr(MAG_W)))

cases = []
for m, method in enumerate(METHODS):
    for r, resp in enumerate(RESP):
        band = r >= 2
        for order in ([1, 2, 3, 4, 7, 10] if r == 0 else [2, 3, 5, 8] if r == 1 else [2, 3, 4]):
            if m == 0 and order not in (1, 2, 4, 10, 3, 5, 8, 7):
                continue
            wn = [0.2, 0.55] if band and r == 2 else [0.3, 0.65] if band else [0.23 if r == 0 else 0.34]
            rp, rs = (1.0, 40.0) if m == 3 else (0.5, 30.0) if m == 1 else (1.0, 45.0)
            cases.append((method, m, resp, r, order, wn, rp, rs))

for method, m, resp, r, order, wn, rp, rs in cases:
    kw = dict(btype=resp, ftype=method)
    if method in ('cheby1', 'ellip'):
        kw['rp'] = rp
    if method in ('cheby2', 'ellip'):
        kw['rs'] = rs
    z, p, k = signal.iirfilter(order, wn if len(wn) > 1 else wn[0], output='zpk', **kw)
    b, a = signal.iirfilter(order, wn if len(wn) > 1 else wn[0], output='ba', **kw)
    sos = signal.iirfilter(order, wn if len(wn) > 1 else wn[0], output='sos', **kw)
    _, h = signal.sosfreqz(sos, worN=np.array(MAG_W) * np.pi)
    one = signal.sosfilt(sos, X)
    zero = signal.sosfiltfilt(sos, X)
    print('    IirCase { name: "%s %s %d", method: %d, response: %d, order: %d, wn: %s, rp: %s, rs: %s,' % (
        method, resp, order, m, r, order, arr(wn), f(rp), f(rs)))
    print('        zeros: %s, poles: %s, gain: %s,' % (carr(z), carr(p), f(k)))
    print('        b: %s, a: %s,' % (arr(b), arr(a)))
    print('        magnitude: %s, one_way: %s, zero_phase: %s },' % (arr(np.abs(h)), arr(one), arr(zero)))
print('];')

# minimum orders
print('''
/// `scipy.signal.buttord` and friends: the order and the natural frequencies, in the units of `wp`/`ws` (Nyquist = 1).
pub struct OrdCase {
    pub name: &'static str,
    pub method: usize,
    pub response: usize,
    pub wp: &'static [f64],
    pub ws: &'static [f64],
    pub gpass: f64,
    pub gstop: f64,
    pub order: usize,
    pub wn: &'static [f64],
}

pub const ORD: &[OrdCase] = &[''')
ordfun = {'butter': signal.buttord, 'cheby1': signal.cheb1ord, 'cheby2': signal.cheb2ord, 'ellip': signal.ellipord}
specs = [
    (0, [0.2], [0.3], 1.0, 40.0), (0, [0.1], [0.4], 3.0, 60.0), (0, [0.35], [0.5], 0.5, 30.0),
    (1, [0.3], [0.2], 1.0, 40.0), (1, [0.6], [0.45], 2.0, 50.0),
    (2, [0.3, 0.5], [0.2, 0.6], 1.0, 40.0), (2, [0.25, 0.4], [0.1, 0.7], 2.0, 60.0),
    (3, [0.2, 0.6], [0.3, 0.5], 1.0, 40.0), (3, [0.1, 0.8], [0.25, 0.5], 0.5, 50.0),
]
for method in METHODS:
    for r, wp, ws, gp, gs in specs:
        n, wn = ordfun[method](wp if len(wp) > 1 else wp[0], ws if len(ws) > 1 else ws[0], gp, gs)
        print('    OrdCase { name: "%s %s", method: %d, response: %d, wp: %s, ws: %s, gpass: %s, gstop: %s, order: %d, wn: %s },' % (
            method, RESP[r], METHODS.index(method), r, arr(wp), arr(ws), f(gp), f(gs), n, arr(np.atleast_1d(wn))))
print('];')

# windows
WIN = [('Bartlett', 'bartlett'), ('BartlettHanning', 'barthann'), ('Blackman', 'blackman'),
       ('BlackmanHarris', 'blackmanharris'), ('Bohman', 'bohman'), ('Chebyshev', ('chebwin', 100)),
       ('FlatTop', 'flattop'), ('Gaussian', None), ('Hamming', 'hamming'), ('Hanning', 'hann'),
       ('Kaiser', ('kaiser', 5.0)), ('Nuttall', 'nuttall'), ('Parzen', 'parzen'), ('Rectangle', 'boxcar'),
       ('Taylor', 'taylor'), ('Triangular', 'triang'), ('Tukey', ('tukey', 0.5))]


def getwin(spec, m):
    if spec is None:
        return signal.get_window(('gaussian', (m - 1) / (2 * 2.5)), m, fftbins=False)
    return signal.get_window(spec, m, fftbins=False)


print('''
/// A symmetric window of `m` points with the default parameter of `FirWindow` (index in `FirWindow::ALL`).
pub struct WindowCase {
    pub name: &'static str,
    pub window: usize,
    pub m: usize,
    pub values: &'static [f64],
}

pub const WINDOWS: &[WindowCase] = &[''')
for wi, (name, spec) in enumerate(WIN):
    for m in (21, 20, 64):
        print('    WindowCase { name: "%s", window: %d, m: %d, values: %s },' % (name, wi, m, arr(getwin(spec, m))))
print('];')

# firwin
print('''
/// `scipy.signal.firwin(numtaps, cutoff, window=..., pass_zero=..., scale=...)`; `response` as above.
pub struct FirwinCase {
    pub name: &'static str,
    pub window: usize,
    pub response: usize,
    pub numtaps: usize,
    pub cutoff: &'static [f64],
    pub scale: bool,
    pub h: &'static [f64],
}

pub const FIRWIN: &[FirwinCase] = &[''')
for wi, (name, spec) in enumerate(WIN):
    if spec is None:
        w = ('gaussian', (41 - 1) / (2 * 2.5))
    else:
        w = spec
    h = signal.firwin(41, 0.3, window=w, pass_zero=True, scale=True)
    print('    FirwinCase { name: "%s lowpass", window: %d, response: 0, numtaps: 41, cutoff: %s, scale: true, h: %s },' % (name, wi, arr([0.3]), arr(h)))
hamming = 8
for r, resp, cut, nt, pz in [(1, 'highpass', [0.4], 41, False), (2, 'bandpass', [0.2, 0.5], 41, False),
                             (3, 'bandstop', [0.3, 0.6], 41, True), (0, 'lowpass', [0.45], 20, True),
                             (2, 'bandpass', [0.15, 0.35], 64, False)]:
    for scale in (True, False):
        h = signal.firwin(nt, cut if len(cut) > 1 else cut[0], window='hamming', pass_zero=pz, scale=scale)
        print('    FirwinCase { name: "hamming %s", window: %d, response: %d, numtaps: %d, cutoff: %s, scale: %s, h: %s },' % (
            resp, hamming, r, nt, arr(cut), 'true' if scale else 'false', arr(h)))
print('];')

# firls
print('''
/// `scipy.signal.firls(numtaps, bands, desired, weight)` with constant desired gain per band (`bands` as pairs).
pub struct FirlsCase {
    pub name: &'static str,
    pub numtaps: usize,
    pub bands: &'static [f64],
    pub desired: &'static [f64],
    pub weight: &'static [f64],
    pub h: &'static [f64],
}

pub const FIRLS: &[FirlsCase] = &[''')
for name, nt, bands, des, wt in [
    ('lowpass', 41, [0, .2, .3, 1], [1, 0], [1, 10]),
    ('lowpass long', 101, [0, .1, .15, 1], [1, 0], [1, 1]),
    ('highpass', 41, [0, .3, .4, 1], [0, 1], [5, 1]),
    ('bandpass', 61, [0, .15, .2, .5, .55, 1], [0, 1, 0], [1, 1, 1]),
    ('bandstop', 61, [0, .2, .3, .5, .6, 1], [1, 0, 1], [1, 3, 1]),
]:
    desired = np.repeat(des, 2)
    weight = np.array(wt)
    h = signal.firls(nt, bands, desired, weight=weight, fs=2)
    print('    FirlsCase { name: "%s", numtaps: %d, bands: %s, desired: %s, weight: %s, h: %s },' % (
        name, nt, arr(bands), arr(des), arr(wt), arr(h)))
print('];')

# kaiserord
print('''
/// `scipy.signal.kaiserord(ripple, width)`.
pub const KAISER: &[(f64, f64, usize, f64)] = &[''')
for a, w in [(60, 0.1), (80, 0.05), (30, 0.2), (15, 0.3), (45.5, 0.07), (100, 0.03)]:
    nt, beta = signal.kaiserord(a, w)
    print('    (%s, %s, %d, %s),' % (f(a), f(w), nt, f(beta)))
print('];')

# application
print('''
/// The FIR application: `h`, one way (the odd extension of `numtaps - 1` points convolved, the delay of
/// `(numtaps - 1) / 2` points removed) and `scipy.signal.filtfilt(h, 1, x)`.
pub struct ApplyCase {
    pub name: &'static str,
    pub h: &'static [f64],
    pub one_way: &'static [f64],
    pub zero_phase: &'static [f64],
}

pub const APPLY: &[ApplyCase] = &[''')
for name, nt, cut in [('hamming 21', 21, 0.3), ('hamming 20', 20, 0.25), ('hamming 31', 31, 0.1)]:
    h = signal.firwin(nt, cut, window='hamming')
    pad = nt - 1
    left = 2 * X[0] - X[1:pad + 1][::-1]
    right = 2 * X[-1] - X[-2:-pad - 2:-1]
    e = np.concatenate([left, X, right])
    full = np.convolve(e, h)
    d = pad // 2
    one = full[pad + d: pad + d + N]
    zero = signal.filtfilt(h, [1.0], X)
    print('    ApplyCase { name: "%s", h: %s, one_way: %s, zero_phase: %s },' % (name, arr(h), arr(one), arr(zero)))
print('];')

# ---------------------------------------------------------------------------------------------
# Added after review: high orders, non-default window parameters, design-level Least Squares and
# minimum-order assemblies.

# high order IIR designs: the gain and what the sections compute (the zeros and poles are not listed)
print('''
/// High order designs (where the products of the zeros and poles pass 1e154 and a naive complex division
/// overflows): scipy's gain, its magnitude response (`sosfreqz`) and its one way output (`sosfilt`).
pub struct HighCase {
    pub name: &'static str,
    pub method: usize,
    pub response: usize,
    pub order: usize,
    pub wn: &'static [f64],
    pub rp: f64,
    pub rs: f64,
    pub gain: f64,
    pub magnitude: &'static [f64],
    pub one_way: &'static [f64],
}

pub const HIGH: &[HighCase] = &[''')
for method, resp, order, wn, rp, rs in [
    ('butter', 'bandpass', 100, [0.3, 0.6], 1.0, 40.0),
    ('butter', 'bandpass', 110, [0.3, 0.6], 1.0, 40.0),
    ('butter', 'bandstop', 100, [0.3, 0.6], 1.0, 40.0),
    ('butter', 'bandstop', 104, [0.3, 0.6], 1.0, 40.0),
    ('butter', 'bandpass', 142, [0.3, 0.6], 1.0, 40.0),
    ('cheby1', 'bandpass', 150, [0.3, 0.6], 1.0, 40.0),
    ('cheby2', 'bandpass', 94, [0.3, 0.6], 1.0, 80.0),
    ('cheby2', 'lowpass', 190, [0.3], 1.0, 80.0),
    ('ellip', 'bandstop', 24, [0.3, 0.6], 1.0, 80.0),
]:
    kw = dict(btype=resp, ftype=method)
    if method in ('cheby1', 'ellip'):
        kw['rp'] = rp
    if method in ('cheby2', 'ellip'):
        kw['rs'] = rs
    w = wn if len(wn) > 1 else wn[0]
    z, p, k = signal.iirfilter(order, w, output='zpk', **kw)
    sos = signal.iirfilter(order, w, output='sos', **kw)
    _, h = signal.sosfreqz(sos, worN=np.array(MAG_W) * np.pi)
    one = signal.sosfilt(sos, X)
    print('    HighCase { name: "%s %s %d", method: %d, response: %d, order: %d, wn: %s, rp: %s, rs: %s,' % (
        method, resp, order, METHODS.index(method), RESP.index(resp), order, arr(wn), f(rp), f(rs)))
    print('        gain: %s, magnitude: %s, one_way: %s },' % (f(k), arr(np.abs(h)), arr(one)))
print('];')

# windows with a parameter other than the default (`FirWindow::ALL` index, as above)
WINP = [('Chebyshev', 5, 80.0), ('Chebyshev', 5, 50.0), ('Gaussian', 7, 1.5), ('Gaussian', 7, 4.0),
        ('Kaiser', 10, 0.0), ('Kaiser', 10, 10.0), ('Taylor', 14, 20.0), ('Taylor', 14, 45.0),
        ('Tukey', 16, 0.0), ('Tukey', 16, 0.25), ('Tukey', 16, 1.0)]


def getwinp(name, m, p):
    if name == 'Chebyshev':
        return signal.get_window(('chebwin', p), m, fftbins=False)
    if name == 'Gaussian':
        return signal.get_window(('gaussian', (m - 1) / (2 * p)), m, fftbins=False)
    if name == 'Kaiser':
        return signal.get_window(('kaiser', p), m, fftbins=False)
    if name == 'Taylor':
        return signal.windows.taylor(m, nbar=4, sll=p, norm=True, sym=True)
    return signal.get_window(('tukey', p), m, fftbins=False)


print('''
/// A symmetric window of `m` points with the parameter `param`, and `firwin(41, 0.3, window)` with it.
pub struct WindowParamCase {
    pub name: &'static str,
    pub window: usize,
    pub m: usize,
    pub param: f64,
    pub values: &'static [f64],
    pub lowpass: &'static [f64],
}

pub const WINDOW_PARAMS: &[WindowParamCase] = &[''')
for name, wi, p in WINP:
    for m in (21, 20):
        # firwin takes no window array: the boxcar design times the window, scaled to unit gain at DC
        h41 = signal.firwin(41, 0.3, window='boxcar', pass_zero=True, scale=False) * getwinp(name, 41, p)
        h41 = h41 / h41.sum()
        print('    WindowParamCase { name: "%s", window: %d, m: %d, param: %s, values: %s, lowpass: %s },' % (
            name, wi, m, f(p), arr(getwinp(name, m, p)), arr(h41)))
print('];')

# design-level Least Squares: scipy firls with the band and weight mapping of the dialog, scaled or not
print('''
/// The Least Squares design of `FirSpec` for `response` (index as above) with the pass and stop edges
/// `wp` and `ws`, the weights, and the coefficients scipy's `firls` gives for the same bands (`scaled`: divided by
/// the gain at the scaling frequency, as `scale_to_unit`). Odd tap counts only (scipy refuses even ones).
pub struct LsDesignCase {
    pub name: &'static str,
    pub response: usize,
    pub numtaps: usize,
    pub wp: &'static [f64],
    pub ws: &'static [f64],
    pub pass_weight: f64,
    pub stop_weight: f64,
    pub plain: &'static [f64],
    pub scaled: &'static [f64],
}

pub const LS_DESIGN: &[LsDesignCase] = &[''')


def ls_bands(r, wp, ws):
    if r == 0:
        return [0, wp[0], ws[0], 1], [1, 0], 'pw sw'
    if r == 1:
        return [0, ws[0], wp[0], 1], [0, 1], 'sw pw'
    if r == 2:
        return [0, ws[0], wp[0], wp[1], ws[1], 1], [0, 1, 0], 'sw pw sw'
    return [0, wp[0], ws[0], ws[1], wp[1], 1], [1, 0, 1], 'pw sw pw'


def scale_freq(r, wp):
    return 1.0 if r == 1 else 0.5 * (wp[0] + wp[1]) if r == 2 else 0.0


for name, r, nt, wp, ws, pw, sw in [
    ('lowpass 41 weights 1 8', 0, 41, [0.2], [0.3], 1.0, 8.0),
    ('highpass 41 weights 1 8', 1, 41, [0.4], [0.3], 1.0, 8.0),
    ('highpass 41 weights 6 1', 1, 41, [0.4], [0.3], 6.0, 1.0),
    ('bandpass 51 weights 1 5', 2, 51, [0.3, 0.5], [0.2, 0.6], 1.0, 5.0),
    ('bandpass 51 weights 4 1', 2, 51, [0.3, 0.5], [0.2, 0.6], 4.0, 1.0),
    ('bandstop 51 weights 1 7', 3, 51, [0.2, 0.7], [0.3, 0.6], 1.0, 7.0),
    ('bandstop 51 weights 5 1', 3, 51, [0.2, 0.7], [0.3, 0.6], 5.0, 1.0),
]:
    bands, des, order = ls_bands(r, wp, ws)
    wts = [pw if t == 'pw' else sw for t in order.split()]
    h = signal.firls(nt, bands, np.repeat(des, 2), weight=np.array(wts), fs=2)
    alpha = 0.5 * (nt - 1)
    s = np.sum(h * np.cos(np.pi * scale_freq(r, wp) * (np.arange(nt) - alpha)))
    print('    LsDesignCase { name: "%s", response: %d, numtaps: %d, wp: %s, ws: %s, pass_weight: %s, stop_weight: %s,' % (
        name, r, nt, arr(wp), arr(ws), f(pw), f(sw)))
    print('        plain: %s, scaled: %s },' % (arr(h), arr(h / s)))
print('];')

# even tap counts: scipy refuses them, so the reference is the weighted least squares optimum of the type II
# amplitude A(w) = 2 sum_k h[M + k] cos(pi w (k + 1/2)) solved here by Gauss-Legendre quadrature of the
# normal equations (exact for these trigonometric integrands), independent of the Rust code.
print('''
/// Even tap counts (low pass and band pass only): the weighted least squares optimum by quadrature in numpy.
pub struct LsEvenCase {
    pub name: &'static str,
    pub response: usize,
    pub numtaps: usize,
    pub wp: &'static [f64],
    pub ws: &'static [f64],
    pub pass_weight: f64,
    pub stop_weight: f64,
    pub plain: &'static [f64],
}

pub const LS_EVEN: &[LsEvenCase] = &[''')
gx, gw = np.polynomial.legendre.leggauss(300)
for name, r, nt, wp, ws, pw, sw in [
    ('lowpass 40 weights 1 1', 0, 40, [0.2], [0.3], 1.0, 1.0),
    ('lowpass 40 weights 1 10', 0, 40, [0.2], [0.3], 1.0, 10.0),
    ('lowpass 30 weights 5 1', 0, 30, [0.25], [0.4], 5.0, 1.0),
    ('bandpass 36 weights 1 4', 2, 36, [0.3, 0.5], [0.2, 0.6], 1.0, 4.0),
]:
    bands, des, order = ls_bands(r, wp, ws)
    wts = [pw if t == 'pw' else sw for t in order.split()]
    M = nt // 2
    nu = np.arange(M) + 0.5
    Q = np.zeros((M, M))
    rhs = np.zeros(M)
    for bi in range(len(des)):
        lo, hi = bands[2 * bi], bands[2 * bi + 1]
        w = 0.5 * (hi - lo) * gx + 0.5 * (hi + lo)
        wt = 0.5 * (hi - lo) * gw * wts[bi]
        B = np.cos(np.pi * np.outer(w, nu))
        Q += B.T @ (wt[:, None] * B)
        rhs += B.T @ (wt * des[bi])
    b = np.linalg.solve(Q, rhs)
    h = np.concatenate([b[::-1] / 2, b / 2])
    print('    LsEvenCase { name: "%s", response: %d, numtaps: %d, wp: %s, ws: %s, pass_weight: %s, stop_weight: %s, plain: %s },' % (
        name, r, nt, arr(wp), arr(ws), f(pw), f(sw), arr(h)))
print('];')

# minimum order window designs: the assembly of FirSpec::design from scipy's kaiserord and firwin
print('''
/// A minimum-order Kaiser window design: `firwin(n, cut, window=("kaiser", beta))` with `n`, `beta` from
/// `kaiserord(-20 log10(min(dev_p, dev_s)), narrowest gap)`, the cutoffs at the midpoints of each pass and stop
/// edge, `n` made odd for a high pass and a band stop. `dev_p` and `dev_s` are the linear deviations.
pub struct MinOrderCase {
    pub name: &'static str,
    pub response: usize,
    pub wp: &'static [f64],
    pub ws: &'static [f64],
    pub ripple_db: f64,
    pub attenuation_db: f64,
    pub dev_p: f64,
    pub dev_s: f64,
    pub numtaps: usize,
    pub beta: f64,
    pub h: &'static [f64],
}

pub const MIN_ORDER: &[MinOrderCase] = &[''')
for name, r, wp, ws, rp_db, rs_db in [
    ('lowpass 100 dB', 0, [0.2], [0.3], 0.5, 100.0),
    ('lowpass 40 dB narrow ripple', 0, [0.3], [0.36], 0.1, 40.0),
    ('highpass 80 dB', 1, [0.4], [0.3], 0.5, 80.0),
    ('bandpass 70 dB', 2, [0.3, 0.5], [0.2, 0.62], 0.2, 70.0),
    ('bandstop 90 dB', 3, [0.15, 0.8], [0.25, 0.6], 0.3, 90.0),
]:
    g = 10 ** (rp_db / 20)
    dp, ds = (g - 1) / (g + 1), 10 ** (-rs_db / 20)
    a = -20 * np.log10(min(dp, ds))
    width = min(abs(p - s) for p, s in zip(wp, ws))
    cut = [0.5 * (p + s) for p, s in zip(wp, ws)]
    nt, beta = signal.kaiserord(a, width)
    if r in (1, 3) and nt % 2 == 0:
        nt += 1
    h = signal.firwin(nt, cut if len(cut) > 1 else cut[0], window=('kaiser', beta), pass_zero=r in (0, 3), scale=True)
    print('    MinOrderCase { name: "%s", response: %d, wp: %s, ws: %s, ripple_db: %s, attenuation_db: %s, dev_p: %s, dev_s: %s, numtaps: %d, beta: %s,' % (
        name, r, arr(wp), arr(ws), f(rp_db), f(rs_db), f(dp), f(ds), nt, f(beta)))
    print('        h: %s },' % arr(h))
print('];')
