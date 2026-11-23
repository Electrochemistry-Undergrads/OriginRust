//! Mixture designs (T386, Origin 2026b's "mixture designs with several totals"; Origin 2016 has no
//! design of experiments, so these are the textbook designs of Cornell, *Experiments with Mixtures*,
//! not measured in Origin; `docs/spec/statistics.md` section 45a).
//!
//! The blends are made in pseudo-components `z` (non-negative, summing to 1):
//!
//! - **Simplex lattice {q, m}**: every `z` whose parts are multiples of `1/m`.
//! - **Simplex centroid**: for every non-empty subset of the components, equal parts of that subset
//!   (`2^q - 1` blends).
//! - **Axial check blends** (optional): for each component, `(1 + q) / (2q)` of it and `1 / (2q)` of
//!   each other one. **Overall centroid** (optional): `1/q` of each, added when the design lacks it.
//!
//! Each blend is scaled to real amounts `x_i = L_i + (T - sum L) z_i` for each total `T` (several
//! totals repeat the design once per total) and the lower bounds `L_i` (0 by default). Replicates
//! repeat everything; a seeded shuffle (the factorial designs' generator) gives the run order.

/// Which simplex design.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum MixtureKind {
    #[default]
    SimplexLattice,
    SimplexCentroid,
}

/// The settings of [`mixture_design`].
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MixtureOptions {
    /// Number of components q (2 to 10).
    pub components: usize,
    pub kind: MixtureKind,
    /// The lattice degree m (1 to 10; the simplex lattice only).
    pub degree: usize,
    pub axial: bool,
    pub centroid: bool,
    pub replicates: usize,
    /// The totals the components add up to (at least one; each above the lower bounds' sum).
    pub totals: Vec<f64>,
    /// Lower bound of each component (empty: all 0).
    pub lower: Vec<f64>,
    pub randomize: bool,
    pub seed: u64,
}

impl Default for MixtureOptions {
    fn default() -> Self {
        MixtureOptions {
            components: 3,
            kind: MixtureKind::SimplexLattice,
            degree: 2,
            axial: false,
            centroid: false,
            replicates: 1,
            totals: vec![1.0],
            lower: Vec::new(),
            randomize: true,
            seed: 1,
        }
    }
}

/// A mixture design, rows in run order.
#[derive(Debug, Clone, PartialEq)]
pub struct MixtureDesign {
    /// The real amounts of each run.
    pub runs: Vec<Vec<f64>>,
    /// The pseudo-component blend of each run.
    pub blends: Vec<Vec<f64>>,
    /// 1-based standard order (blends in design order, then the next total, then the next replicate).
    pub standard: Vec<usize>,
    /// The total of each run.
    pub total: Vec<f64>,
    /// "Vertex", "Binary blend", "Blend of 3", "Overall centroid", "Axial check blend".
    pub point_type: Vec<String>,
    /// Distinct blends of the design (before totals and replicates).
    pub blend_count: usize,
}

/// Why a mixture design could not be made.
#[derive(Debug, Clone, PartialEq)]
pub enum MixtureError {
    Components,
    Degree,
    Replicates,
    Totals,
    LowerBounds(String),
    TooLarge(usize),
}

impl std::fmt::Display for MixtureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MixtureError::Components => f.write_str("a mixture design takes 2 to 10 components"),
            MixtureError::Degree => f.write_str("the lattice degree m must be 1 to 10"),
            MixtureError::Replicates => f.write_str("give at least one replicate"),
            MixtureError::Totals => f.write_str("give at least one total, each above 0"),
            MixtureError::LowerBounds(why) => f.write_str(why),
            MixtureError::TooLarge(n) => write!(f, "the design would have {n} runs; at most 10000"),
        }
    }
}

impl std::error::Error for MixtureError {}

/// Every way of writing `m` as an ordered sum of `q` non-negative parts, in lexicographic order with
/// the first component falling fastest from `m` (so the first blend is the first vertex).
fn compositions(q: usize, m: usize) -> Vec<Vec<usize>> {
    fn rec(q: usize, left: usize, cur: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
        if cur.len() + 1 == q {
            cur.push(left);
            out.push(cur.clone());
            cur.pop();
            return;
        }
        for v in (0..=left).rev() {
            cur.push(v);
            rec(q, left - v, cur, out);
            cur.pop();
        }
    }
    let mut out = Vec::new();
    rec(q, m, &mut Vec::new(), &mut out);
    out
}

fn kind_of(z: &[f64], q: usize) -> String {
    let nonzero = z.iter().filter(|v| **v > 1e-12).count();
    match nonzero {
        1 => "Vertex".into(),
        2 => "Binary blend".into(),
        k if k == q && z.iter().all(|v| (v - 1.0 / q as f64).abs() < 1e-12) => "Overall centroid".into(),
        k => format!("Blend of {k}"),
    }
}

/// Makes the design (see the module text).
pub fn mixture_design(o: &MixtureOptions) -> Result<MixtureDesign, MixtureError> {
    let q = o.components;
    if !(2..=10).contains(&q) {
        return Err(MixtureError::Components);
    }
    if o.kind == MixtureKind::SimplexLattice && !(1..=10).contains(&o.degree) {
        return Err(MixtureError::Degree);
    }
    if o.replicates == 0 {
        return Err(MixtureError::Replicates);
    }
    if o.totals.is_empty() || o.totals.iter().any(|t| !(t.is_finite() && *t > 0.0)) {
        return Err(MixtureError::Totals);
    }
    let lower: Vec<f64> = if o.lower.is_empty() { vec![0.0; q] } else { o.lower.clone() };
    if lower.len() != q {
        return Err(MixtureError::LowerBounds(format!("give {q} lower bounds, one per component, or none")));
    }
    if lower.iter().any(|l| !(l.is_finite() && *l >= 0.0)) {
        return Err(MixtureError::LowerBounds("the lower bounds must be numbers of at least 0".into()));
    }
    let sum_lower: f64 = lower.iter().sum();
    if let Some(t) = o.totals.iter().find(|t| **t <= sum_lower) {
        return Err(MixtureError::LowerBounds(format!(
            "the lower bounds add up to {sum_lower}, which leaves nothing to vary in the total {t}"
        )));
    }
    let mut blends: Vec<(Vec<f64>, String)> = match o.kind {
        MixtureKind::SimplexLattice => compositions(q, o.degree)
            .into_iter()
            .map(|c| {
                let z: Vec<f64> = c.iter().map(|&v| v as f64 / o.degree as f64).collect();
                let k = kind_of(&z, q);
                (z, k)
            })
            .collect(),
        MixtureKind::SimplexCentroid => {
            let mut out: Vec<(Vec<f64>, String)> = Vec::new();
            for size in 1..=q {
                for mask in 1u32..(1u32 << q) {
                    if mask.count_ones() as usize != size {
                        continue;
                    }
                    let z: Vec<f64> =
                        (0..q).map(|i| if mask & (1 << i) != 0 { 1.0 / size as f64 } else { 0.0 }).collect();
                    let k = kind_of(&z, q);
                    out.push((z, k));
                }
            }
            out
        }
    };
    if o.axial {
        for i in 0..q {
            let z: Vec<f64> = (0..q)
                .map(|j| if i == j { (1.0 + q as f64) / (2.0 * q as f64) } else { 1.0 / (2.0 * q as f64) })
                .collect();
            blends.push((z, "Axial check blend".into()));
        }
    }
    if o.centroid {
        let c = vec![1.0 / q as f64; q];
        if !blends.iter().any(|(z, _)| z.iter().zip(&c).all(|(a, b)| (a - b).abs() < 1e-12)) {
            blends.push((c, "Overall centroid".into()));
        }
    }
    let blend_count = blends.len();
    let n = blend_count * o.totals.len() * o.replicates;
    if n > 10_000 {
        return Err(MixtureError::TooLarge(n));
    }
    let mut rows: Vec<(usize, f64, Vec<f64>, Vec<f64>, String)> = Vec::with_capacity(n);
    for _ in 0..o.replicates {
        for &t in &o.totals {
            for (z, k) in &blends {
                let x: Vec<f64> = (0..q).map(|i| lower[i] + (t - sum_lower) * z[i]).collect();
                rows.push((rows.len() + 1, t, x, z.clone(), k.clone()));
            }
        }
    }
    if o.randomize {
        let mut s = o.seed;
        for i in (1..rows.len()).rev() {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let u = (s >> 11) as f64 / (1u64 << 53) as f64;
            let j = ((u * (i + 1) as f64) as usize).min(i);
            rows.swap(i, j);
        }
    }
    Ok(MixtureDesign {
        standard: rows.iter().map(|r| r.0).collect(),
        total: rows.iter().map(|r| r.1).collect(),
        runs: rows.iter().map(|r| r.2.clone()).collect(),
        blends: rows.iter().map(|r| r.3.clone()).collect(),
        point_type: rows.into_iter().map(|r| r.4).collect(),
        blend_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(q: usize, kind: MixtureKind, degree: usize) -> MixtureOptions {
        MixtureOptions { components: q, kind, degree, randomize: false, ..MixtureOptions::default() }
    }

    #[test]
    fn lattice_and_centroid_have_the_textbook_points() {
        // {3, 2}: 6 blends (3 vertices, 3 binary midpoints); {3, 3}: 10; {4, 2}: 10.
        let d = mixture_design(&plain(3, MixtureKind::SimplexLattice, 2)).unwrap();
        assert_eq!(d.runs.len(), 6);
        assert_eq!(d.runs[0], [1.0, 0.0, 0.0]);
        assert_eq!(d.point_type.iter().filter(|t| *t == "Vertex").count(), 3);
        assert!(d.runs.contains(&vec![0.5, 0.5, 0.0]));
        assert_eq!(mixture_design(&plain(3, MixtureKind::SimplexLattice, 3)).unwrap().runs.len(), 10);
        assert_eq!(mixture_design(&plain(4, MixtureKind::SimplexLattice, 2)).unwrap().runs.len(), 10);
        // Simplex centroid: 2^q - 1 blends, the last the overall centroid.
        let c = mixture_design(&plain(3, MixtureKind::SimplexCentroid, 0)).unwrap();
        assert_eq!(c.runs.len(), 7);
        assert_eq!(c.point_type.last().unwrap(), "Overall centroid");
        for r in c.runs.iter().chain(&d.runs) {
            assert!((r.iter().sum::<f64>() - 1.0).abs() < 1e-12, "{r:?}");
        }
    }

    #[test]
    fn axial_centroid_totals_bounds_and_replicates() {
        let o = MixtureOptions {
            axial: true,
            centroid: true,
            totals: vec![1.0, 100.0],
            lower: vec![0.1, 0.0, 0.2],
            replicates: 2,
            randomize: false,
            ..MixtureOptions::default()
        };
        let d = mixture_design(&o).unwrap();
        // 6 lattice blends + 3 axial + the centroid, for 2 totals and 2 replicates.
        assert_eq!(d.blend_count, 10);
        assert_eq!(d.runs.len(), 40);
        assert_eq!(d.point_type.iter().filter(|t| *t == "Axial check blend").count(), 3 * 4);
        // The axial blend of A: (1+q)/(2q) = 2/3 of the free part.
        let axial_a = d.blends.iter().position(|z| (z[0] - 2.0 / 3.0).abs() < 1e-12).unwrap();
        assert!((d.blends[axial_a][1] - 1.0 / 6.0).abs() < 1e-12);
        for (x, t) in d.runs.iter().zip(&d.total) {
            assert!((x.iter().sum::<f64>() - t).abs() < 1e-9, "{x:?} {t}");
            assert!(x[0] >= 0.1 - 1e-12 && x[2] >= 0.2 - 1e-12);
        }
        // Total 100, vertex A: its lower bound 0.1 plus the free part 100 - 0.3 = 99.7.
        let v = d.runs.iter().zip(&d.total).find(|(x, t)| **t == 100.0 && x[0] > 99.0).unwrap().0;
        assert!((v[0] - 99.8).abs() < 1e-9 && (v[2] - 0.2).abs() < 1e-12, "{v:?}");
        // The standard order numbers every run once; a shuffle keeps the same runs.
        let mut s = d.standard.clone();
        s.sort_unstable();
        assert_eq!(s, (1..=40).collect::<Vec<_>>());
        let shuffled = mixture_design(&MixtureOptions { randomize: true, seed: 7, ..o }).unwrap();
        assert_ne!(shuffled.standard, d.standard);
        let mut a: Vec<String> = shuffled.runs.iter().map(|r| format!("{r:?}")).collect();
        let mut b: Vec<String> = d.runs.iter().map(|r| format!("{r:?}")).collect();
        a.sort();
        b.sort();
        assert_eq!(a, b);
    }

    #[test]
    fn bad_settings_are_explained() {
        let bad = |o: MixtureOptions| mixture_design(&o).unwrap_err().to_string();
        assert!(bad(MixtureOptions { components: 1, ..Default::default() }).contains("2 to 10"));
        assert!(bad(MixtureOptions { degree: 0, ..Default::default() }).contains("degree"));
        assert!(bad(MixtureOptions { totals: vec![], ..Default::default() }).contains("total"));
        assert!(bad(MixtureOptions { lower: vec![0.5, 0.6, 0.0], ..Default::default() }).contains("nothing to vary"));
        assert!(bad(MixtureOptions { lower: vec![0.1], ..Default::default() }).contains("3 lower bounds"));
        assert!(bad(MixtureOptions { components: 10, degree: 10, ..Default::default() }).contains("at most 10000"));
    }
}
