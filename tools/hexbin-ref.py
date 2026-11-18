"""Prints crates/calc/tests/fixtures/mpl_hexbin.rs: reference hexagon counts from matplotlib for the
hexagonal binning of `sheaf-calc` (`hexbin.rs`, the Hexbin plot).

Usage: python3 -I tools/hexbin-ref.py > crates/calc/tests/fixtures/mpl_hexbin.rs
Needs numpy and matplotlib (the numbers in the repository were made with matplotlib 3.11.2, numpy 2.5.4).
None of the numbers come from Origin, which documents its Hexbin controls but not its algorithm.

`matplotlib.axes.Axes.hexbin(gridsize=(nx, ny), extent=(xmin, xmax, ymin, ymax), mincnt=1)` lays two
interleaved rectangular lattices over the extent, assigns each point to the nearer lattice centre with the
distance `dx^2 + 3 dy^2` in cell units, and returns the centres and counts. Together the two lattices are the
pointy-top offset-row lattice of Sheaf with `nx + 1` centres in the even rows, `ny + 1` even rows plus `ny`
odd rows, a row spacing of `(ymax - ymin) / (2 ny)`. The script also recomputes the counts by brute force
over the lattice centres with numpy and asserts that both agree, so the fixture is two independent
implementations. The inputs come from fixed seeds.
"""
import numpy as np
import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt


def f(v):
    return repr(float(v))


def arr(v):
    return "&[" + ", ".join(f(x) for x in v) + "]"


def brute(x, y, nx, ny, ext):
    xmin, xmax, ymin, ymax = ext
    sx = (xmax - xmin) / nx
    dy = (ymax - ymin) / (2 * ny)
    centres = []
    for r in range(2 * ny + 1):
        cols = nx + 1 if r % 2 == 0 else nx
        off = 0.0 if r % 2 == 0 else 0.5
        for c in range(cols):
            centres.append((xmin + (c + off) * sx, ymin + r * dy))
    centres = np.array(centres)
    # distance on the page where the hexagons are regular: dy cells are sqrt(3)/2 sx wide... in cell units
    u = (x[:, None] - centres[None, :, 0]) / sx
    v = (y[:, None] - centres[None, :, 1]) / (2 * dy) * np.sqrt(3.0)
    d = u * u + v * v
    best = np.argmin(d, axis=1)
    counts = np.bincount(best, minlength=len(centres))
    keys = []
    for r in range(2 * ny + 1):
        keys += [(r, c) for c in range(nx + 1 if r % 2 == 0 else nx)]
    return {keys[i]: int(c) for i, c in enumerate(counts) if c > 0}


def case(name, x, y, nx, ny=None):
    ext = (x.min(), x.max(), y.min(), y.max())
    ny = ny or int(nx / np.sqrt(3))
    fig, ax = plt.subplots()
    hb = ax.hexbin(x, y, gridsize=(nx, ny), extent=ext, mincnt=1)
    plt.close(fig)
    # matplotlib widens the extent by 1e-9; take each centre to its (row, column) in the lattice.
    sx = (ext[1] - ext[0]) / nx
    dy = (ext[3] - ext[2]) / (2 * ny)
    got = {}
    for o, c in zip(hb.get_offsets(), hb.get_array()):
        r = int(round((o[1] - ext[2]) / dy))
        col = int(round((o[0] - ext[0]) / sx - (0.5 if r % 2 else 0.0)))
        got[(r, col)] = int(c)
    want = brute(x, y, nx, ny, ext)
    assert got == want, name
    assert sum(got.values()) == len(x)
    cells = sorted((r, c, n) for (r, c), n in got.items())
    out = f"    Case {{\n        name: \"{name}\",\n        x: {arr(x)},\n        y: {arr(y)},\n        nx: {nx},\n        ny: {ny},\n"
    out += "        cells: &[" + ", ".join(f"({r}, {c}, {n})" for r, c, n in cells) + "],\n    },\n"
    return out


rng = np.random.default_rng(45)
cases = []
x = np.round(rng.uniform(0, 10, 300), 6)
y = np.round(rng.uniform(0, 6, 300), 6)
cases.append(case("uniform 300, 10 bins", x, y, 10))
x = np.round(rng.normal(0, 1, 500), 6)
y = np.round(0.6 * x + rng.normal(0, 0.8, 500), 6)
cases.append(case("correlated normal 500, 20 bins", x, y, 20))
x = np.round(np.concatenate([rng.normal(2, 0.3, 120), rng.normal(7, 0.5, 80), [0.0, 10.0]]), 6)
y = np.round(np.concatenate([rng.normal(1, 0.2, 120), rng.normal(5, 0.4, 80), [0.0, 6.0]]), 6)
cases.append(case("two clusters with corner points, 7 bins", x, y, 7, 4))
x = np.round(rng.integers(0, 12, 200).astype(float), 6)
y = np.round(rng.integers(0, 8, 200).astype(float), 6)
cases.append(case("integer grid with ties, 12 bins", x + 0.0, y + 0.0, 12, 7))

print("// Made by tools/hexbin-ref.py with matplotlib Axes.hexbin (and a numpy brute force); not Origin numbers.")
print("pub struct Case {")
print("    pub name: &'static str,")
print("    pub x: &'static [f64],")
print("    pub y: &'static [f64],")
print("    /// Cells along X (centres in an even row: nx + 1) and the matplotlib `ny`.")
print("    pub nx: usize,")
print("    pub ny: usize,")
print("    /// Non-empty cells as (row, column, count): row r is at ymin + r (ymax - ymin) / (2 ny), column c of an even row at xmin + c sx, of an odd row at xmin + (c + 1/2) sx.")
print("    pub cells: &'static [(usize, usize, usize)],")
print("}")
print()
print("pub const CASES: &[Case] = &[")
print("".join(cases), end="")
print("];")
