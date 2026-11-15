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
