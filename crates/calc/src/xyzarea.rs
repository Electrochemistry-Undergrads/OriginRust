//! XYZ Surface Area (Analysis > Mathematics > XYZ Surface Area): the area of the surface through scattered
//! (X, Y, Z) points, measured in Origin 2016 (`docs/spec/analysis-tools.md`, section 18).
//!
//! The surface is the Delaunay triangulation of the points projected on the XY plane, lifted to the Z values;
//! its area is the sum of the areas of the 3D triangles `|(p2 - p1) x (p3 - p1)| / 2`. Points with the same
//! (X, Y) are merged first, with the mean of their Z values (Origin does this; it was checked on five points
//! where one is repeated). The triangulation ([`delaunay`]) is also usable on its own, for gridding.
//!
//! Where four or more points lie on one circle the Delaunay triangulation is not unique (both diagonals of a
//! grid cell qualify); which one is taken changes the area of a curved surface, and Origin's choice was not
//! measured. The areas of the cases measured, all of them symmetric under that choice, are reproduced to 1e-15.

use std::collections::HashMap;

/// Why no area could be computed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XyzAreaError {
    /// The three columns have different lengths.
    LengthMismatch,
    /// Fewer than three different (X, Y) positions with numbers: carries the number found.
    TooFewPoints(usize),
    /// All the points lie on one line in the XY plane, so there is no triangle.
    Collinear,
    /// The triangulation could not be completed (extremely thin point sets).
    NoTriangulation,
}

impl std::fmt::Display for XyzAreaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            XyzAreaError::LengthMismatch => write!(f, "the X, Y and Z columns have different lengths"),
            XyzAreaError::TooFewPoints(n) => {
                write!(f, "a surface needs at least three different (X, Y) points with numbers, found {n}")
            }
            XyzAreaError::Collinear => write!(f, "all the points lie on a line in the XY plane: there is no surface"),
            XyzAreaError::NoTriangulation => write!(f, "the points could not be triangulated"),
        }
    }
}

impl std::error::Error for XyzAreaError {}

/// The area of the surface and what went into it.
#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceArea {
    /// The sum of the 3D triangle areas.
    pub area: f64,
    /// The area of the same triangles projected on the XY plane (the area of the convex hull of the points).
    pub projected_area: f64,
    /// The points used: different (X, Y) positions after merging.
    pub points: usize,
    /// The triangles of the triangulation.
    pub triangles: usize,
    /// Rows merged into another row with the same (X, Y).
    pub merged: usize,
    /// Rows left out because X, Y or Z is missing or not finite.
    pub skipped: usize,
}

/// The area of the surface through the points `(x[i], y[i], z[i])`; rows with a missing (NaN or infinite)
/// coordinate are skipped and counted, rows with equal X and Y are merged with the mean of their Z.
pub fn xyz_surface_area(x: &[f64], y: &[f64], z: &[f64]) -> Result<SurfaceArea, XyzAreaError> {
    if x.len() != y.len() || x.len() != z.len() {
        return Err(XyzAreaError::LengthMismatch);
    }
    let mut skipped = 0;
    // (x, y) -> (sum of z, count); the first occurrence keeps its place so that the result does not depend on a hash.
    let mut index: HashMap<(u64, u64), usize> = HashMap::new();
    let mut pts: Vec<(f64, f64, f64, usize)> = Vec::new();
    let mut used = 0;
    for i in 0..x.len() {
        if !(x[i].is_finite() && y[i].is_finite() && z[i].is_finite()) {
            skipped += 1;
            continue;
        }
        used += 1;
        // +0.0 and -0.0 are the same position.
        let key = ((x[i] + 0.0).to_bits(), (y[i] + 0.0).to_bits());
        match index.get(&key) {
            Some(&k) => {
                pts[k].2 += z[i];
                pts[k].3 += 1;
            }
            None => {
                index.insert(key, pts.len());
                pts.push((x[i], y[i], z[i], 1));
            }
        }
    }
    if pts.len() < 3 {
        return Err(XyzAreaError::TooFewPoints(pts.len()));
    }
    let merged = used - pts.len();
    let xy: Vec<(f64, f64)> = pts.iter().map(|p| (p.0, p.1)).collect();
    let zs: Vec<f64> = pts.iter().map(|p| p.2 / p.3 as f64).collect();
    let tris = delaunay(&xy)?;
    let mut area = 0.0;
    let mut flat = 0.0;
    for t in &tris {
        let [a, b, c] = t.map(|i| (xy[i].0, xy[i].1, zs[i]));
        let u = (b.0 - a.0, b.1 - a.1, b.2 - a.2);
        let v = (c.0 - a.0, c.1 - a.1, c.2 - a.2);
        let cross = (u.1 * v.2 - u.2 * v.1, u.2 * v.0 - u.0 * v.2, u.0 * v.1 - u.1 * v.0);
        area += 0.5 * (cross.0 * cross.0 + cross.1 * cross.1 + cross.2 * cross.2).sqrt();
        flat += 0.5 * cross.2.abs();
    }
    Ok(SurfaceArea { area, projected_area: flat, points: pts.len(), triangles: tris.len(), merged, skipped })
}

/// Twice the signed area of the triangle `a, b, c` (positive when counterclockwise).
fn orient(a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> f64 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

/// Positive when `d` is inside the circle through the counterclockwise triangle `a, b, c`.
fn in_circle(a: (f64, f64), b: (f64, f64), c: (f64, f64), d: (f64, f64)) -> f64 {
    let (ax, ay) = (a.0 - d.0, a.1 - d.1);
    let (bx, by) = (b.0 - d.0, b.1 - d.1);
    let (cx, cy) = (c.0 - d.0, c.1 - d.1);
    let (a2, b2, c2) = (ax * ax + ay * ay, bx * bx + by * by, cx * cx + cy * cy);
    ax * (by * c2 - b2 * cy) - ay * (bx * c2 - b2 * cx) + a2 * (bx * cy - by * cx)
}

/// The Delaunay triangulation of distinct points: triangles as indices into `points`, each counterclockwise.
/// Points that are not distinct are the caller's business (see [`xyz_surface_area`], which merges them). Where
/// points are cocircular, one of the valid triangulations is returned.
///
/// Bowyer-Watson insertion inside a far-away super triangle, on coordinates scaled to the unit square; the
/// result is checked by the area of the triangles against the area of the convex hull, and a failed check (a
/// hull triangle lost to the super triangle, or rounding in the circle test) is retried with a larger super
/// triangle and the other insertion order.
pub fn delaunay(points: &[(f64, f64)]) -> Result<Vec<[usize; 3]>, XyzAreaError> {
    if points.len() < 3 {
        return Err(XyzAreaError::TooFewPoints(points.len()));
    }
    let (mut x0, mut x1, mut y0, mut y1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
    for p in points {
        x0 = x0.min(p.0);
        x1 = x1.max(p.0);
        y0 = y0.min(p.1);
        y1 = y1.max(p.1);
    }
    let scale = (x1 - x0).max(y1 - y0);
    if !scale.is_finite() || scale <= 0.0 {
        return Err(XyzAreaError::TooFewPoints(1));
    }
    let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    let unit: Vec<(f64, f64)> = points.iter().map(|p| ((p.0 - cx) / scale, (p.1 - cy) / scale)).collect();
    let hull = convex_hull_area(&unit);
    // Collinear: every point on the line through the two farthest apart (relative tolerance).
    if hull <= 1e-13 {
        return Err(XyzAreaError::Collinear);
    }
    for (factor, reversed) in [(1e3, false), (1e5, true), (1e4, false), (1e6, true)] {
        let mut order: Vec<usize> = (0..unit.len()).collect();
        if reversed {
            order.reverse();
        }
        if let Some(tris) = bowyer_watson(&unit, &order, factor) {
            let flat: f64 = tris.iter().map(|t| 0.5 * orient(unit[t[0]], unit[t[1]], unit[t[2]]).abs()).sum();
            if (flat - hull).abs() <= 1e-9 * hull {
                return Ok(tris);
            }
        }
    }
    Err(XyzAreaError::NoTriangulation)
}

/// Area of the convex hull (Andrew's monotone chain).
fn convex_hull_area(p: &[(f64, f64)]) -> f64 {
    let mut s: Vec<(f64, f64)> = p.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).expect("finite points"));
    s.dedup();
    if s.len() < 3 {
        return 0.0;
    }
    let half = |it: &mut dyn Iterator<Item = &(f64, f64)>| {
        let mut h: Vec<(f64, f64)> = Vec::new();
        for &q in it {
            while h.len() >= 2 && orient(h[h.len() - 2], h[h.len() - 1], q) <= 0.0 {
                h.pop();
            }
            h.push(q);
        }
        h.pop();
        h
    };
    let mut hull = half(&mut s.iter());
    hull.extend(half(&mut s.iter().rev()));
    (0..hull.len())
        .map(|i| (hull[i].0 * hull[(i + 1) % hull.len()].1 - hull[(i + 1) % hull.len()].0 * hull[i].1) / 2.0)
        .sum::<f64>()
        .abs()
}

struct Mesh {
    /// Vertex coordinates: the points, then the three super-triangle corners.
    v: Vec<(f64, f64)>,
    tris: Vec<[usize; 3]>,
    alive: Vec<bool>,
    /// Directed edge (a, b) of a live triangle -> that triangle.
    edge: HashMap<(usize, usize), usize>,
}

impl Mesh {
    fn add(&mut self, t: [usize; 3]) -> usize {
        let id = self.tris.len();
        self.tris.push(t);
        self.alive.push(true);
        for k in 0..3 {
            self.edge.insert((t[k], t[(k + 1) % 3]), id);
        }
        id
    }

    fn remove(&mut self, id: usize) {
        self.alive[id] = false;
        let t = self.tris[id];
        for k in 0..3 {
            self.edge.remove(&(t[k], t[(k + 1) % 3]));
        }
    }

    /// A live triangle that contains `p` (inside or on its boundary), walking from `start`.
    fn locate(&self, p: (f64, f64), start: usize) -> Option<usize> {
        let mut t = start;
        if !self.alive[t] {
            t = (0..self.tris.len()).rev().find(|&i| self.alive[i])?;
        }
        for _ in 0..self.tris.len() + 8 {
            let tri = self.tris[t];
            let mut moved = false;
            for k in 0..3 {
                let (a, b) = (tri[k], tri[(k + 1) % 3]);
                if orient(self.v[a], self.v[b], p) < 0.0
                    && let Some(&n) = self.edge.get(&(b, a))
                {
                    t = n;
                    moved = true;
                    break;
                }
            }
            if !moved {
                return Some(t);
            }
        }
        // The walk cycled (rounding): scan.
        (0..self.tris.len()).find(|&i| {
            self.alive[i] && {
                let t = self.tris[i];
                (0..3).all(|k| orient(self.v[t[k]], self.v[t[(k + 1) % 3]], p) >= 0.0)
            }
        })
    }
}

fn bowyer_watson(p: &[(f64, f64)], order: &[usize], factor: f64) -> Option<Vec<[usize; 3]>> {
    let n = p.len();
    let m = factor;
    let mut mesh = Mesh { v: p.to_vec(), tris: Vec::new(), alive: Vec::new(), edge: HashMap::with_capacity(n * 6) };
    mesh.v.extend([(0.0, m), (-m, -m), (m, -m)]);
    let mut last = mesh.add([n + 1, n + 2, n]);
    // (n + 1, n + 2, n) is counterclockwise: (-m,-m) -> (m,-m) -> (0,m).
    debug_assert!(orient(mesh.v[n + 1], mesh.v[n + 2], mesh.v[n]) > 0.0);
    for &i in order {
        let q = mesh.v[i];
        let first = mesh.locate(q, last)?;
        let mut bad = vec![first];
        let mut seen = std::collections::HashSet::from([first]);
        let mut boundary: Vec<(usize, usize)> = Vec::new();
        let mut k = 0;
        while k < bad.len() {
            let t = mesh.tris[bad[k]];
            k += 1;
            for e in 0..3 {
                let (a, b) = (t[e], t[(e + 1) % 3]);
                match mesh.edge.get(&(b, a)) {
                    Some(&nb) if seen.contains(&nb) => {}
                    Some(&nb)
                        if in_circle(
                            mesh.v[mesh.tris[nb][0]],
                            mesh.v[mesh.tris[nb][1]],
                            mesh.v[mesh.tris[nb][2]],
                            q,
                        ) > 0.0 =>
                    {
                        seen.insert(nb);
                        bad.push(nb);
                    }
                    _ => boundary.push((a, b)),
                }
            }
        }
        for &t in &bad {
            mesh.remove(t);
        }
        for (a, b) in boundary {
            if orient(mesh.v[a], mesh.v[b], q) <= 0.0 {
                // A degenerate or inverted triangle: the cavity was not star-shaped (rounding).
                return None;
            }
            last = mesh.add([a, b, i]);
        }
    }
    Some(
        (0..mesh.tris.len())
            .filter(|&t| mesh.alive[t] && mesh.tris[t].iter().all(|&v| v < n))
            .map(|t| mesh.tris[t])
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(pts: &[(f64, f64, f64)]) -> Result<SurfaceArea, XyzAreaError> {
        let x: Vec<f64> = pts.iter().map(|p| p.0).collect();
        let y: Vec<f64> = pts.iter().map(|p| p.1).collect();
        let z: Vec<f64> = pts.iter().map(|p| p.2).collect();
        xyz_surface_area(&x, &y, &z)
    }

    /// A triangle with legs 1 along X and 1 along Y, rising by 2 at (0, 1): the cross product of (1,0,0) and
    /// (0,1,2) is (0*2-0*1, 0*0-1*2, 1*1-0) = (0, -2, 1), length sqrt(5), area sqrt(5)/2.
    #[test]
    fn one_triangle_is_half_the_cross_product() {
        let r = area(&[(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (0.0, 1.0, 2.0)]).unwrap();
        assert!((r.area - 5f64.sqrt() / 2.0).abs() < 1e-15, "{r:?}");
        assert_eq!((r.points, r.triangles, r.merged, r.skipped), (3, 1, 0, 0));
        assert!((r.projected_area - 0.5).abs() < 1e-15);
    }

    #[test]
    fn a_flat_unit_square_has_area_one() {
        let r = area(&[(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (1.0, 1.0, 0.0), (0.0, 1.0, 0.0)]).unwrap();
        assert!((r.area - 1.0).abs() < 1e-15, "{r:?}");
        assert_eq!(r.triangles, 2, "the four corners are cocircular: either diagonal gives two triangles");
    }

    /// The plane z = 2x + 3y has the normal (-2, -3, 1): the area is sqrt(14) times the projected area.
    #[test]
    fn a_tilted_plane_is_sqrt_14_times_its_shadow() {
        let mut pts = Vec::new();
        for j in 0..5 {
            for i in 0..5 {
                let (x, y) = (i as f64 / 4.0, j as f64 / 4.0);
                pts.push((x, y, 2.0 * x + 3.0 * y));
            }
        }
        let r = area(&pts).unwrap();
        assert!((r.area - 14f64.sqrt()).abs() < 1e-14, "{r:?}");
        assert_eq!(r.triangles, 32, "a 4 by 4 grid of cells, two triangles each");
    }

    /// Five rows where (1, 1) is given twice, with z = 3 and 5: the merged point has z = 4, so the surface is
    /// that of the square (0,0,0), (1,0,1), (0,1,2), (1,1,4) (either diagonal, the four corners are cocircular).
    /// Origin's measured value for this case is checked in `tests/origin_xyzarea.rs`.
    #[test]
    fn rows_with_the_same_xy_are_merged_with_the_mean_of_z() {
        let r = area(&[(0.0, 0.0, 0.0), (1.0, 0.0, 1.0), (0.0, 1.0, 2.0), (1.0, 1.0, 3.0), (1.0, 1.0, 5.0)]).unwrap();
        assert_eq!((r.points, r.merged), (4, 1));
        let tri = |a: (f64, f64, f64), b: (f64, f64, f64), c: (f64, f64, f64)| {
            let u = (b.0 - a.0, b.1 - a.1, b.2 - a.2);
            let v = (c.0 - a.0, c.1 - a.1, c.2 - a.2);
            let w = (u.1 * v.2 - u.2 * v.1, u.2 * v.0 - u.0 * v.2, u.0 * v.1 - u.1 * v.0);
            0.5 * (w.0 * w.0 + w.1 * w.1 + w.2 * w.2).sqrt()
        };
        let (o, p, q, s) = ((0.0, 0.0, 0.0), (1.0, 0.0, 1.0), (0.0, 1.0, 2.0), (1.0, 1.0, 4.0));
        let a = tri(o, p, s) + tri(o, s, q);
        let b = tri(o, p, q) + tri(p, s, q);
        assert!((r.area - a).abs() < 1e-14 || (r.area - b).abs() < 1e-14, "{} vs {a} or {b}", r.area);
        // The first row's z (3) or the last row's (5) would give other areas.
        let z3 = area(&[(0.0, 0.0, 0.0), (1.0, 0.0, 1.0), (0.0, 1.0, 2.0), (1.0, 1.0, 3.0)]).unwrap();
        assert!((z3.area - r.area).abs() > 1e-3);
    }

    #[test]
    fn missing_values_are_skipped_and_counted() {
        let x = [0.0, 1.0, 0.0, f64::NAN, 5.0];
        let y = [0.0, 0.0, 1.0, 1.0, f64::INFINITY];
        let z = [0.0, 0.0, 0.0, 0.0, 0.0];
        let r = xyz_surface_area(&x, &y, &z).unwrap();
        assert_eq!((r.points, r.skipped), (3, 2));
        assert!((r.area - 0.5).abs() < 1e-15);
    }

    #[test]
    fn too_few_or_collinear_points_have_no_area() {
        assert_eq!(area(&[(0.0, 0.0, 0.0), (1.0, 1.0, 1.0)]).unwrap_err(), XyzAreaError::TooFewPoints(2));
        // Three rows but two positions.
        assert_eq!(
            area(&[(0.0, 0.0, 0.0), (0.0, 0.0, 1.0), (1.0, 1.0, 1.0)]).unwrap_err(),
            XyzAreaError::TooFewPoints(2)
        );
        let line = [(0.0, 0.0, 0.0), (1.0, 1.0, 1.0), (2.0, 2.0, 0.0), (3.0, 3.0, 1.0)];
        assert_eq!(area(&line).unwrap_err(), XyzAreaError::Collinear);
        assert_eq!(xyz_surface_area(&[0.0], &[0.0, 1.0], &[0.0]).unwrap_err(), XyzAreaError::LengthMismatch);
    }

    /// Every triangulation of scattered points covers the convex hull exactly once and satisfies the empty
    /// circle property.
    #[test]
    fn delaunay_of_scattered_points_is_empty_circle_and_covers_the_hull() {
        let mut s = 12345u64;
        let mut rnd = || {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (s >> 11) as f64 / (1u64 << 53) as f64
        };
        let pts: Vec<(f64, f64)> = (0..200).map(|_| (rnd() * 3.0 - 1.0, rnd() * 50.0)).collect();
        let tris = delaunay(&pts).unwrap();
        let flat: f64 = tris.iter().map(|t| 0.5 * orient(pts[t[0]], pts[t[1]], pts[t[2]])).sum();
        assert!((flat - convex_hull_area(&pts)).abs() < 1e-9 * flat);
        for t in &tris {
            assert!(orient(pts[t[0]], pts[t[1]], pts[t[2]]) > 0.0, "counterclockwise");
            for (i, &q) in pts.iter().enumerate() {
                if !t.contains(&i) {
                    let d = in_circle(pts[t[0]], pts[t[1]], pts[t[2]], q);
                    assert!(d < 1e-9, "point {i} inside the circle of {t:?}: {d}");
                }
            }
        }
        // Euler: for n points with h on the hull, 2n - 2 - h triangles.
        let mut h = 0;
        let mut edges = std::collections::HashMap::new();
        for t in &tris {
            for k in 0..3 {
                *edges.entry((t[k].min(t[(k + 1) % 3]), t[k].max(t[(k + 1) % 3]))).or_insert(0) += 1;
            }
        }
        for c in edges.values() {
            if *c == 1 {
                h += 1;
            }
        }
        assert_eq!(tris.len(), 2 * pts.len() - 2 - h);
    }

    #[test]
    fn a_large_regular_grid_triangulates_in_reasonable_time() {
        let mut x = Vec::new();
        let mut y = Vec::new();
        let mut z = Vec::new();
        for j in 0..60 {
            for i in 0..60 {
                x.push(i as f64);
                y.push(j as f64);
                z.push(0.0);
            }
        }
        let r = xyz_surface_area(&x, &y, &z).unwrap();
        assert!((r.area - 59.0 * 59.0).abs() < 1e-9, "{r:?}");
        assert_eq!(r.triangles, 2 * 59 * 59);
    }
}
