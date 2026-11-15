//! Hexagonal binning of X-Y points (Origin's Hexbin plot, Plot > Contour, Heatmap: Hexbin; graphing
//! spec 14.24): the plane is covered by pointy-top hexagons laid out on a lattice, and every point is
//! counted in the hexagon whose centre is nearest.
//!
//! Origin documents the controls, not the algorithm: per axis a Min Center, Max Center and either
//! a Bin Spacing or a Number of Bins; "Prefer Regular Hexagons" (for a square layer frame) and
//! "Derive Y from X" (the default) or "Derive X from Y". Everything below that Origin does not give is
//! Sheaf's own and unmeasured, and is listed in the spec:
//!
//! - The lattice is pointy-top. Row `r` holds centres at `y0 + r dy`; even rows hold centres at
//!   `x0 + c sx`, odd rows at `x0 + (c + 1/2) sx`.
//! - A Number of Bins `n` along an axis puts `n` centres on it, from Min Center to Max Center:
//!   spacing `(max - min) / (n - 1)`.
//! - The derived axis' spacing makes the hexagons regular on the page when the plot's frame has the
//!   aspect ratio `a` (width / height) and the axes' ranges are the centre ranges: with X given,
//!   `dy = sx * a * (yrange / xrange) * sqrt(3) / 2`; with Y given, the same equation solved for `sx`.
//!   Without "Prefer Regular Hexagons" `a` is taken as 1.
//! - Rows (or columns of the derived axis) run from Min Center up to the first centre on or beyond
//!   Max Center, so the whole range is covered and, with the default range, every point is counted.
//! - A point belongs to the hexagon of the nearest centre of the lattice (distance measured on the page,
//!   i.e. in the scaled space where the hexagons are regular); a point exactly between two centres goes
//!   to the one in the lower row, then the lower column. A point that lies outside the hexagon of that
//!   centre (beyond a Min or Max Center narrower than the data) is not counted.
//! - An axis whose range has no width (all values equal, or one point) is given the width 1 centred on
//!   the value.
//! - Points with a NaN or infinite X or Y are skipped.

use std::collections::BTreeMap;

/// `sqrt(3) / 2`: the distance between rows of a regular pointy-top lattice, in units of the spacing.
pub const ROW_SPACING: f64 = 0.866_025_403_784_438_6;
/// The most centres along an axis; a smaller spacing is raised.
const MAX_CELLS: f64 = 10_000.0;
/// Default number of bins along the given axis.
pub const DEFAULT_BINS: usize = 20;
/// Distances closer than this (in lattice units) are a tie.
const TIE: f64 = 1e-9;

/// How the centres of one axis are spaced.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum HexStep {
    /// This many centres from Min Center to Max Center (at least 2).
    Bins(usize),
    /// A distance between neighbouring centres, in data units.
    Spacing(f64),
}

/// The settings of one axis. `None` is the minimum or maximum of the data.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct HexAxisSetup {
    pub min_center: Option<f64>,
    pub max_center: Option<f64>,
    pub step: HexStep,
}

impl Default for HexAxisSetup {
    fn default() -> HexAxisSetup {
        HexAxisSetup { min_center: None, max_center: None, step: HexStep::Bins(DEFAULT_BINS) }
    }
}

/// Everything the binning takes.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct HexSetup {
    pub x: HexAxisSetup,
    pub y: HexAxisSetup,
    /// True (Origin's default): the X axis is the given one and Y follows; false: Y is given.
    pub derive_y_from_x: bool,
    /// Make the hexagons regular on the page (see the module documentation).
    pub prefer_regular: bool,
    /// Width / height of the plot frame on the page.
    pub frame_aspect: f64,
}

impl Default for HexSetup {
    fn default() -> HexSetup {
        HexSetup {
            x: HexAxisSetup::default(),
            y: HexAxisSetup::default(),
            derive_y_from_x: true,
            prefer_regular: true,
            frame_aspect: 1.0,
        }
    }
}

/// The lattice of centres.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct HexLattice {
    /// The centre of row 0, column 0.
    pub x0: f64,
    pub y0: f64,
    /// Distance between centres of a row.
    pub sx: f64,
    /// Distance between rows.
    pub dy: f64,
    /// Centres of an even row (an odd row has one less).
    pub cols: usize,
    pub rows: usize,
}

impl HexLattice {
    /// The centre of `(row, col)`.
    pub fn centre(&self, row: usize, col: usize) -> (f64, f64) {
        let off = if row % 2 == 1 { 0.5 } else { 0.0 };
        (self.x0 + (col as f64 + off) * self.sx, self.y0 + row as f64 * self.dy)
    }

    /// Number of centres in `row`.
    pub fn cols_in(&self, row: usize) -> usize {
        if row % 2 == 1 { self.cols.saturating_sub(1).max(1) } else { self.cols }
    }

    /// The six corners of the hexagon around `(cx, cy)`, from the top one clockwise on the page
    /// (Y up): the hexagon is `sx` wide and `2 / sqrt(3) * dy` ... i.e. regular where the lattice
    /// is regular.
    pub fn corners(&self, cx: f64, cy: f64) -> [(f64, f64); 6] {
        let r = 1.0 / 3f64.sqrt();
        let k = self.dy / ROW_SPACING;
        let (hx, vy) = (0.5 * self.sx, k);
        [
            (cx, cy + r * vy),
            (cx + hx, cy + 0.5 * r * vy),
            (cx + hx, cy - 0.5 * r * vy),
            (cx, cy - r * vy),
            (cx - hx, cy - 0.5 * r * vy),
            (cx - hx, cy + 0.5 * r * vy),
        ]
    }
}

/// One non-empty hexagon.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct HexBin {
    pub row: usize,
    pub col: usize,
    pub x: f64,
    pub y: f64,
    pub count: usize,
}

/// The result of the binning.
#[derive(Clone, Debug, PartialEq)]
pub struct Hexbin {
    pub lattice: HexLattice,
    /// The non-empty bins, by row then column.
    pub bins: Vec<HexBin>,
    /// Points counted in a bin.
    pub counted: usize,
    /// Finite points outside every hexagon of the lattice.
    pub outside: usize,
    /// The largest count.
    pub max_count: usize,
}

fn range_of(v: impl Iterator<Item = f64>) -> Option<(f64, f64)> {
    let mut it = v.filter(|x| x.is_finite());
    let first = it.next()?;
    Some(it.fold((first, first), |(a, b), x| (a.min(x), b.max(x))))
}

/// The centre range of one axis; width 1 around the value where it has none.
fn resolve_range(setup: &HexAxisSetup, data: (f64, f64)) -> (f64, f64) {
    let lo = setup.min_center.filter(|v| v.is_finite()).unwrap_or(data.0);
    let hi = setup.max_center.filter(|v| v.is_finite()).unwrap_or(data.1);
    if hi > lo { (lo, hi) } else { (lo - 0.5, lo + 0.5) }
}

/// The spacing a given axis asks for.
fn given_spacing(step: HexStep, range: f64) -> f64 {
    let s = match step {
        HexStep::Bins(n) => range / (n.max(2) - 1) as f64,
        HexStep::Spacing(s) if s.is_finite() && s > 0.0 => s,
        HexStep::Spacing(_) => range / (DEFAULT_BINS - 1) as f64,
    };
    s.max(range / MAX_CELLS)
}

/// The Min Center to Max Center ranges `((xlo, xhi), (ylo, yhi))` the lattice for these points uses; `None`
/// without a finite point.
pub fn centre_ranges(x: &[f64], y: &[f64], setup: &HexSetup) -> Option<((f64, f64), (f64, f64))> {
    let pairs = || x.iter().zip(y).filter(|(a, b)| a.is_finite() && b.is_finite());
    let xd = range_of(pairs().map(|p| *p.0))?;
    let yd = range_of(pairs().map(|p| *p.1))?;
    Some((resolve_range(&setup.x, xd), resolve_range(&setup.y, yd)))
}

/// The lattice for the points `(x, y)` (finite pairs only); `None` without one.
pub fn lattice(x: &[f64], y: &[f64], setup: &HexSetup) -> Option<HexLattice> {
    let pairs = || x.iter().zip(y).filter(|(a, b)| a.is_finite() && b.is_finite());
    let xd = range_of(pairs().map(|p| *p.0))?;
    let yd = range_of(pairs().map(|p| *p.1))?;
    let (xlo, xhi) = resolve_range(&setup.x, xd);
    let (ylo, yhi) = resolve_range(&setup.y, yd);
    let (xr, yr) = (xhi - xlo, yhi - ylo);
    let a = if setup.prefer_regular && setup.frame_aspect.is_finite() && setup.frame_aspect > 0.0 {
        setup.frame_aspect
    } else {
        1.0
    };
    // Row spacing per unit of X spacing that makes the hexagons regular.
    let ratio = a * (yr / xr) * ROW_SPACING;
    let (sx, dy) = if setup.derive_y_from_x {
        let sx = given_spacing(setup.x.step, xr);
        (sx, sx * ratio)
    } else {
        let dy = given_spacing(setup.y.step, yr);
        (dy / ratio, dy)
    };
    let (sx, dy) = (sx.max(xr / MAX_CELLS), dy.max(yr / MAX_CELLS));
    let count = |range: f64, step: f64| (range / step - 1e-9).ceil().max(0.0) as usize + 1;
    Some(HexLattice { x0: xlo, y0: ylo, sx, dy, cols: count(xr, sx), rows: count(yr, dy) })
}

/// The hexagon of `(px, py)`: its row and column, or `None` outside every hexagon of the lattice.
pub fn assign(l: &HexLattice, px: f64, py: f64) -> Option<(usize, usize)> {
    let u = (px - l.x0) / l.sx;
    let v = (py - l.y0) / l.dy * ROW_SPACING;
    let row_f = v / ROW_SPACING;
    let mut best: Option<(f64, usize, usize, f64, f64)> = None;
    let top = l.rows as f64 - 1.0;
    for dr in -1..=2 {
        let r = (row_f.floor() + dr as f64).clamp(0.0, top) as usize;
        let off = if r % 2 == 1 { 0.5 } else { 0.0 };
        let last = (l.cols_in(r) - 1) as f64;
        let c0 = (u - off).floor();
        for c in [c0, c0 + 1.0] {
            let c = c.clamp(0.0, last) as usize;
            let du = u - (c as f64 + off);
            let dv = v - r as f64 * ROW_SPACING;
            let d = du * du + dv * dv;
            if best.is_none_or(|b| d < b.0 - TIE) {
                best = Some((d, r, c, du, dv));
            }
        }
    }
    let (_, r, c, du, dv) = best?;
    // Inside the closed hexagon: |du| <= 1/2 and |dv| <= (1 - |du|) / sqrt(3).
    let (au, av) = (du.abs(), dv.abs());
    (au <= 0.5 + TIE && av <= (1.0 - au) / 3f64.sqrt() + TIE).then_some((r, c))
}

/// Counts the points in the hexagons. `None` when no point has a finite X and Y.
pub fn hexbin(x: &[f64], y: &[f64], setup: &HexSetup) -> Option<Hexbin> {
    let l = lattice(x, y, setup)?;
    let mut cells: BTreeMap<(usize, usize), usize> = BTreeMap::new();
    let (mut counted, mut outside) = (0, 0);
    for (&px, &py) in x.iter().zip(y).filter(|(a, b)| a.is_finite() && b.is_finite()) {
        match assign(&l, px, py) {
            Some(cell) => {
                *cells.entry(cell).or_insert(0) += 1;
                counted += 1;
            }
            None => outside += 1,
        }
    }
    let bins: Vec<HexBin> = cells
        .into_iter()
        .map(|((row, col), count)| {
            let (x, y) = l.centre(row, col);
            HexBin { row, col, x, y, count }
        })
        .collect();
    let max_count = bins.iter().map(|b| b.count).max().unwrap_or(0);
    Some(Hexbin { lattice: l, bins, counted, outside, max_count })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup(nx: usize) -> HexSetup {
        HexSetup { x: HexAxisSetup { step: HexStep::Bins(nx), ..Default::default() }, ..Default::default() }
    }

    #[test]
    fn a_single_point_is_one_bin_of_count_one() {
        let h = hexbin(&[3.0], &[4.0], &setup(10)).unwrap();
        assert_eq!((h.counted, h.outside, h.max_count), (1, 0, 1));
        assert_eq!(h.bins.len(), 1);
        // Width-1 range around the value: the point is within half a spacing of its centre.
        let b = h.bins[0];
        assert!((b.x - 3.0).abs() <= 0.5 * h.lattice.sx + 1e-12 && (b.y - 4.0).abs() <= h.lattice.dy);
    }

    #[test]
    fn counts_add_up_to_the_finite_points_and_non_finite_are_skipped() {
        let x = [0.0, 1.0, f64::NAN, 2.0, 3.0, f64::INFINITY, 4.0];
        let y = [0.0, 1.0, 1.0, f64::NAN, 3.0, 2.0, 4.5];
        let h = hexbin(&x, &y, &setup(5)).unwrap();
        // Pairs (0,0), (1,1), (3,3), (4,4.5) are finite.
        assert_eq!(h.counted + h.outside, 4);
        assert_eq!(h.counted, 4);
        assert_eq!(h.bins.iter().map(|b| b.count).sum::<usize>(), 4);
        assert!(hexbin(&[f64::NAN], &[1.0], &setup(5)).is_none());
        assert!(hexbin(&[], &[], &setup(5)).is_none());
    }

    #[test]
    fn the_lattice_is_pointy_top_with_offset_rows() {
        let s = HexSetup { prefer_regular: true, frame_aspect: 1.0, ..setup(5) };
        let l = lattice(&[0.0, 4.0], &[0.0, 4.0], &s).unwrap();
        // Bins(5) over 0..4: spacing 1; square frame and equal ranges: rows 0.866 apart.
        assert!((l.sx - 1.0).abs() < 1e-12 && (l.dy - ROW_SPACING).abs() < 1e-12);
        assert_eq!(l.centre(0, 2), (2.0, 0.0));
        let (x1, y1) = l.centre(1, 2);
        assert!((x1 - 2.5).abs() < 1e-12 && (y1 - ROW_SPACING).abs() < 1e-12);
        // 4 / 0.866 = 4.62 rows: the first centre beyond is row 5 (centre y 4.33).
        assert_eq!((l.cols, l.rows), (5, 6));
        assert_eq!((l.cols_in(0), l.cols_in(1)), (5, 4));
    }

    #[test]
    fn corners_of_a_regular_lattice_are_regular_on_the_page() {
        let l = HexLattice { x0: 0.0, y0: 0.0, sx: 2.0, dy: 2.0 * ROW_SPACING, cols: 3, rows: 3 };
        let c = l.corners(1.0, 1.0);
        let d = |a: (f64, f64), b: (f64, f64)| ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt();
        let side = d(c[0], c[1]);
        for i in 0..6 {
            assert!((d(c[i], c[(i + 1) % 6]) - side).abs() < 1e-12, "side {i}");
            assert!((d(c[i], (1.0, 1.0)) - side).abs() < 1e-12, "radius {i}");
        }
        // The side is 2 / sqrt(3) of half the width; flat sides are vertical (pointy top).
        assert!((side - 2.0 / 3f64.sqrt()).abs() < 1e-12);
        assert_eq!(c[1].0, c[2].0);
        assert_eq!(c[0].0, c[3].0);
    }

    #[test]
    fn frame_aspect_scales_the_row_spacing() {
        let x = [0.0, 10.0];
        let y = [0.0, 5.0];
        let sq = lattice(&x, &y, &HexSetup { frame_aspect: 1.0, ..setup(11) }).unwrap();
        let wide = lattice(&x, &y, &HexSetup { frame_aspect: 2.0, ..setup(11) }).unwrap();
        let off = lattice(&x, &y, &HexSetup { frame_aspect: 2.0, prefer_regular: false, ..setup(11) }).unwrap();
        assert!((sq.dy - 1.0 * 0.5 * ROW_SPACING).abs() < 1e-12);
        assert!((wide.dy - 2.0 * sq.dy).abs() < 1e-12);
        assert_eq!(off.dy, sq.dy);
    }

    #[test]
    fn derive_x_from_y_gives_y_spacing_and_solves_x() {
        let x = [0.0, 10.0];
        let y = [0.0, 5.0];
        let s = HexSetup {
            y: HexAxisSetup { step: HexStep::Spacing(0.5), ..Default::default() },
            derive_y_from_x: false,
            ..HexSetup::default()
        };
        let l = lattice(&x, &y, &s).unwrap();
        assert_eq!(l.dy, 0.5);
        // dy = sx * (5 / 10) * 0.866  =>  sx = 0.5 / (0.5 * 0.866)
        assert!((l.sx - 0.5 / (0.5 * ROW_SPACING)).abs() < 1e-12);
        assert_eq!(l.rows, 11);
    }

    #[test]
    fn all_equal_x_or_y_still_bins() {
        let h = hexbin(&[2.0; 6], &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], &setup(8)).unwrap();
        assert_eq!((h.counted, h.outside), (6, 0));
        let h = hexbin(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], &[7.0; 6], &setup(8)).unwrap();
        assert_eq!((h.counted, h.outside), (6, 0));
        let h = hexbin(&[2.0; 4], &[3.0; 4], &setup(8)).unwrap();
        assert_eq!((h.counted, h.bins.len(), h.max_count), (4, 1, 4));
    }

    #[test]
    fn points_on_a_shared_edge_go_to_the_lower_row_then_column() {
        // Lattice spacing 1, regular, origin (0, 0).
        let l = HexLattice { x0: 0.0, y0: 0.0, sx: 1.0, dy: ROW_SPACING, cols: 4, rows: 4 };
        // Halfway between (0,0) and (0,1) of row 0: lower column.
        assert_eq!(assign(&l, 0.5, 0.0), Some((0, 0)));
        // Halfway between centre (0,0) and (1,0) (at (0.5, 0.866)): the lower row.
        assert_eq!(assign(&l, 0.25, ROW_SPACING / 2.0), Some((0, 0)));
        // A centre is its own bin.
        assert_eq!(assign(&l, 1.5, ROW_SPACING), Some((1, 1)));
        // Same answer on repeated calls.
        assert_eq!(assign(&l, 0.5, 0.0), assign(&l, 0.5, 0.0));
    }

    #[test]
    fn the_default_range_counts_every_point_including_the_extremes() {
        // Points on every side of the data range, for several bin counts.
        let x = [0.0, 10.0, 0.0, 10.0, 5.0, 5.0, 2.5, 7.5];
        let y = [0.0, 0.0, 6.0, 6.0, 0.0, 6.0, 3.0, 3.0];
        for n in [2, 3, 5, 8, 20, 33] {
            let h = hexbin(&x, &y, &setup(n)).unwrap();
            assert_eq!((h.counted, h.outside), (8, 0), "bins {n}");
        }
    }

    #[test]
    fn a_narrow_centre_range_leaves_the_points_beyond_uncounted() {
        let x: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let y: Vec<f64> = (0..100).map(|i| (i % 7) as f64).collect();
        let s = HexSetup {
            x: HexAxisSetup { min_center: Some(40.0), max_center: Some(60.0), ..Default::default() },
            ..setup(5)
        };
        let h = hexbin(&x, &y, &s).unwrap();
        assert!(h.counted > 0 && h.outside > 50 && h.counted + h.outside == 100);
        assert!(h.bins.iter().all(|b| b.x >= 40.0 && b.x <= 60.0 + h.lattice.sx));
    }

    #[test]
    fn bins_come_ordered_by_row_then_column() {
        let x = [9.0, 1.0, 5.0, 5.0, 1.0];
        let y = [9.0, 1.0, 5.0, 5.0, 9.0];
        let h = hexbin(&x, &y, &setup(6)).unwrap();
        let keys: Vec<(usize, usize)> = h.bins.iter().map(|b| (b.row, b.col)).collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted);
        assert_eq!(h.max_count, 2);
    }
}
