//! K-means cluster analysis of the rows of a data table (Statistics > Multivariate Analysis > K-Means Cluster
//! Analysis).
//!
//! What Origin's `kmeans` X-Function does, measured on synthetic data (`docs/spec/statistics.md`, section 18;
//! fixtures in `crates/calc/tests/origin_kmeans.rs`):
//!
//! - **Initial centres** are `k` of the cases, chosen by a replacement rule over the cases in sheet order: the
//!   first `k` cases are the centres; every later case `x` first looks at the distance `d1` to its nearest centre
//!   `m` and the smallest distance `dmin` between two centres. If `d1 > dmin`, `x` replaces the one of the two
//!   closest centres that is nearer to `x`. Otherwise, if the distance from `x` to its second-nearest centre is
//!   larger than the distance from `m` to the centre nearest to `m`, `x` replaces `m`. (The rule of the SPSS
//!   quick-cluster procedure; it reproduced Origin's centres, in Origin's order, in all nine measured runs.) The
//!   cluster numbers are the positions of the centres, so they depend on the order of the rows.
//! - **Iteration**: every case goes to its nearest centre (ties to the lower cluster number), the centres become the
//!   means, then single cases are moved to another cluster for as long as a move lowers the sum of squares
//!   (Hartigan-Wong's criterion `n_b/(n_b+1) |x - c_b|^2 < n_a/(n_a-1) |x - c_a|^2`, centres updated after each
//!   move). Origin's final centres and cluster sizes are such a local optimum in the nine measured runs, and
//!   some of them differ from what plain Lloyd iterations from the same centres give.
//! - Distances are Euclidean on the raw (not standardized) variables. Per cluster Origin reports the number of
//!   cases, the sum of squared distances to the centre, and the mean and the largest distance to the centre; the
//!   Euclidean distances between the final centres form a `k x k` table.
//! - Per variable: N, missing, mean, sample standard deviation, and a one-way analysis of variance of the variable
//!   by cluster (`k - 1` and `n - k` degrees of freedom, mean squares, `F` and its probability).
//! - Per case the cluster number and the distance to its centre.
//!
//! Not measured: missing values (a row with a missing number in any column is left out here), the meaning of the
//! iteration-limit option (the run with the limit set to 1 gave the same centres as the default).

use statrs::distribution::{ContinuousCDF, FisherSnedecor};

/// How a k-means cluster analysis is done.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KmeansOptions {
    /// Number of clusters (at least 2, at most the number of cases).
    pub clusters: usize,
    /// Upper limit of the passes of single-case moves (a pass looks at every case once).
    pub max_passes: usize,
}

impl Default for KmeansOptions {
    fn default() -> Self {
        KmeansOptions { clusters: 3, max_passes: 100 }
    }
}

/// Why an analysis could not be done.
#[derive(Debug, Clone, PartialEq)]
pub enum KmeansError {
    NoColumns,
    /// Fewer than two clusters asked for.
    TooFewClusters,
    /// More clusters than usable rows.
    TooManyClusters { clusters: usize, rows: usize },
    /// A number is infinite.
    NotFinite,
    /// Two of the initial centres are the same row values, so a cluster stays empty.
    EmptyCluster,
}

impl std::fmt::Display for KmeansError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KmeansError::NoColumns => write!(f, "a k-means analysis needs at least one column"),
            KmeansError::TooFewClusters => write!(f, "the number of clusters must be at least 2"),
            KmeansError::TooManyClusters { clusters, rows } => {
                write!(f, "{clusters} clusters cannot be made from {rows} rows with numbers")
            }
            KmeansError::NotFinite => write!(f, "a number is infinite"),
            KmeansError::EmptyCluster => {
                write!(f, "the rows chosen as initial centres include identical ones, so a cluster would be empty")
            }
        }
    }
}

impl std::error::Error for KmeansError {}

/// One cluster of the result.
#[derive(Debug, Clone, PartialEq)]
pub struct Cluster {
    /// The initial centre, one value per variable.
    pub initial: Vec<f64>,
    pub centre: Vec<f64>,
    pub count: usize,
    /// Sum of the squared distances of the cases to the centre.
    pub sum_squares: f64,
    pub mean_distance: f64,
    pub max_distance: f64,
}

/// Analysis of variance of one variable by cluster.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VariableAnova {
    pub df_between: f64,
    pub ms_between: f64,
    pub df_error: f64,
    pub ms_error: f64,
    /// `NaN` when the error mean square is 0.
    pub f: f64,
    pub p: f64,
}

/// The result of a k-means cluster analysis.
#[derive(Debug, Clone, PartialEq)]
pub struct Kmeans {
    /// Per variable: cases with a number, cases without, mean and sample standard deviation of the cases used.
    pub count: Vec<usize>,
    pub missing: Vec<usize>,
    pub mean: Vec<f64>,
    pub sd: Vec<f64>,
    pub clusters: Vec<Cluster>,
    /// Euclidean distances between the final centres.
    pub centre_distances: Vec<Vec<f64>>,
    pub anova: Vec<VariableAnova>,
    /// Per input row: the cluster (0-based) and the distance to its centre; `None` for a row left out.
    pub membership: Vec<Option<(usize, f64)>>,
}

fn dist2(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| (x - y) * (x - y)).sum()
}

/// The initial centres by the replacement rule in the module documentation.
fn initial_centres(rows: &[Vec<f64>], k: usize) -> Vec<Vec<f64>> {
    let mut c: Vec<Vec<f64>> = rows[..k].to_vec();
    let d = |a: &[f64], b: &[f64]| dist2(a, b).sqrt();
    for x in &rows[k..] {
        let dist: Vec<f64> = c.iter().map(|ci| d(x, ci)).collect();
        let mut order: Vec<usize> = (0..k).collect();
        order.sort_by(|&a, &b| dist[a].total_cmp(&dist[b]));
        let (m, s) = (order[0], order[1]);
        // The closest pair of centres.
        let mut best = (f64::INFINITY, 0, 1);
        for i in 0..k {
            for j in i + 1..k {
                let dij = d(&c[i], &c[j]);
                if dij < best.0 {
                    best = (dij, i, j);
                }
            }
        }
        if dist[m] > best.0 {
            let r = if dist[best.1] < dist[best.2] { best.1 } else { best.2 };
            c[r] = x.clone();
            continue;
        }
        let nearest_to_m = (0..k).filter(|&q| q != m).map(|q| d(&c[m], &c[q])).fold(f64::INFINITY, f64::min);
        if dist[s] > nearest_to_m {
            c[m] = x.clone();
        }
    }
    c
}

/// K-means cluster analysis of `columns` (the variables; missing values are `NaN`).
pub fn kmeans(columns: &[Vec<f64>], options: &KmeansOptions) -> Result<Kmeans, KmeansError> {
    let p = columns.len();
    if p == 0 {
        return Err(KmeansError::NoColumns);
    }
    let k = options.clusters;
    if k < 2 {
        return Err(KmeansError::TooFewClusters);
    }
    let len = columns.iter().map(Vec::len).max().unwrap_or(0);
    let value = |j: usize, i: usize| columns[j].get(i).copied().unwrap_or(f64::NAN);
    if columns.iter().flatten().any(|v| v.is_infinite()) {
        return Err(KmeansError::NotFinite);
    }
    let used: Vec<usize> = (0..len).filter(|&i| (0..p).all(|j| !value(j, i).is_nan())).collect();
    let n = used.len();
    if n < k {
        return Err(KmeansError::TooManyClusters { clusters: k, rows: n });
    }
    let rows: Vec<Vec<f64>> = used.iter().map(|&i| (0..p).map(|j| value(j, i)).collect()).collect();

    let initial = initial_centres(&rows, k);
    let mut centre = initial.clone();
    let nearest = |x: &[f64], c: &[Vec<f64>]| {
        let mut best = (f64::INFINITY, 0);
        for (q, cq) in c.iter().enumerate() {
            let v = dist2(x, cq);
            if v < best.0 {
                best = (v, q);
            }
        }
        best.1
    };
    let mut label: Vec<usize> = rows.iter().map(|x| nearest(x, &centre)).collect();
    let mut size = vec![0usize; k];
    for &l in &label {
        size[l] += 1;
    }
    if size.contains(&0) {
        return Err(KmeansError::EmptyCluster);
    }
    let mut sums = vec![vec![0.0; p]; k];
    for (x, &l) in rows.iter().zip(&label) {
        for j in 0..p {
            sums[l][j] += x[j];
        }
    }
    let refresh = |centre: &mut [Vec<f64>], sums: &[Vec<f64>], size: &[usize]| {
        for q in 0..k {
            for j in 0..p {
                centre[q][j] = sums[q][j] / size[q] as f64;
            }
        }
    };
    refresh(&mut centre, &sums, &size);
    // Single-case moves while one lowers the sum of squares.
    for _ in 0..options.max_passes {
        let mut moved = false;
        for (i, x) in rows.iter().enumerate() {
            let a = label[i];
            if size[a] <= 1 {
                continue;
            }
            let na = size[a] as f64;
            let mut best = (na / (na - 1.0) * dist2(x, &centre[a]), None);
            for b in 0..k {
                if b == a {
                    continue;
                }
                let nb = size[b] as f64;
                let rb = nb / (nb + 1.0) * dist2(x, &centre[b]);
                if rb < best.0 - 1e-12 * best.0.abs() {
                    best = (rb, Some(b));
                }
            }
            if let Some(b) = best.1 {
                for j in 0..p {
                    sums[a][j] -= x[j];
                    sums[b][j] += x[j];
                }
                size[a] -= 1;
                size[b] += 1;
                label[i] = b;
                refresh(&mut centre, &sums, &size);
                moved = true;
            }
        }
        if !moved {
            break;
        }
    }

    // Statistics of the result.
    let nf = n as f64;
    let mean: Vec<f64> = (0..p).map(|j| rows.iter().map(|r| r[j]).sum::<f64>() / nf).collect();
    let sd: Vec<f64> = (0..p)
        .map(|j| (rows.iter().map(|r| (r[j] - mean[j]).powi(2)).sum::<f64>() / (nf - 1.0).max(1.0)).sqrt())
        .collect();
    let count: Vec<usize> = (0..p).map(|j| (0..len).filter(|&i| !value(j, i).is_nan()).count()).collect();
    let missing: Vec<usize> = count.iter().map(|&c| len - c).collect();
    let mut clusters: Vec<Cluster> = (0..k)
        .map(|q| Cluster {
            initial: initial[q].clone(),
            centre: centre[q].clone(),
            count: size[q],
            sum_squares: 0.0,
            mean_distance: 0.0,
            max_distance: 0.0,
        })
        .collect();
    let mut membership = vec![None; len];
    for (r, (x, &l)) in rows.iter().zip(&label).enumerate() {
        let d2 = dist2(x, &centre[l]);
        let c = &mut clusters[l];
        c.sum_squares += d2;
        c.mean_distance += d2.sqrt();
        c.max_distance = c.max_distance.max(d2.sqrt());
        membership[used[r]] = Some((l, d2.sqrt()));
    }
    for c in &mut clusters {
        c.mean_distance /= c.count as f64;
    }
    let centre_distances: Vec<Vec<f64>> =
        (0..k).map(|a| (0..k).map(|b| dist2(&centre[a], &centre[b]).sqrt()).collect()).collect();
    let anova = (0..p)
        .map(|j| {
            let within: f64 = rows.iter().zip(&label).map(|(r, &l)| (r[j] - centre[l][j]).powi(2)).sum();
            let between: f64 = (0..k).map(|q| size[q] as f64 * (centre[q][j] - mean[j]).powi(2)).sum();
            let (df_b, df_e) = ((k - 1) as f64, (n - k) as f64);
            let (ms_b, ms_e) = (between / df_b, if df_e > 0.0 { within / df_e } else { f64::NAN });
            let f = if ms_e > 0.0 { ms_b / ms_e } else { f64::NAN };
            let pv = if f.is_nan() {
                f64::NAN
            } else {
                FisherSnedecor::new(df_b, df_e).map_or(f64::NAN, |d| 1.0 - d.cdf(f))
            };
            VariableAnova { df_between: df_b, ms_between: ms_b, df_error: df_e, ms_error: ms_e, f, p: pv }
        })
        .collect();
    Ok(Kmeans { count, missing, mean, sd, clusters, centre_distances, anova, membership })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-12, "{a} vs {b}");
    }

    /// Two well separated groups, hand worked: {1, 2, 3} (centre 2, SS 2) and {10, 11, 12} (centre 11, SS 2).
    #[test]
    fn two_separated_groups_by_hand() {
        let x = vec![vec![1.0, 10.0, 2.0, 11.0, 3.0, 12.0]];
        let r = kmeans(&x, &KmeansOptions { clusters: 2, max_passes: 10 }).unwrap();
        assert_eq!(r.clusters[0].initial, vec![1.0]);
        // 11 and then 12 each replace the second centre (they are farther from 1 than that centre is).
        assert_eq!(r.clusters[1].initial, vec![12.0]);
        close(r.clusters[0].centre[0], 2.0);
        close(r.clusters[1].centre[0], 11.0);
        assert_eq!((r.clusters[0].count, r.clusters[1].count), (3, 3));
        close(r.clusters[0].sum_squares, 2.0);
        close(r.clusters[0].mean_distance, 2.0 / 3.0);
        close(r.clusters[0].max_distance, 1.0);
        close(r.centre_distances[0][1], 9.0);
        close(r.mean[0], 6.5);
        // ANOVA: between SS = 3 * 4.5^2 * 2 = 121.5 on 1 df; error SS = 4 on 4 df.
        close(r.anova[0].ms_between, 121.5);
        close(r.anova[0].ms_error, 1.0);
        close(r.anova[0].f, 121.5);
        assert_eq!(r.membership[2], Some((0, 0.0)));
        assert_eq!(r.membership[1].unwrap().0, 1);
    }

    #[test]
    fn missing_rows_are_left_out_and_errors() {
        let x = vec![vec![1.0, f64::NAN, 2.0, 9.0, 10.0], vec![1.0, 5.0, 2.0, 9.0, 11.0]];
        let r = kmeans(&x, &KmeansOptions { clusters: 2, max_passes: 10 }).unwrap();
        assert_eq!(r.membership[1], None);
        assert_eq!(r.count, vec![4, 5]);
        assert_eq!(r.missing, vec![1, 0]);
        assert_eq!(kmeans(&x, &KmeansOptions { clusters: 1, max_passes: 10 }), Err(KmeansError::TooFewClusters));
        assert!(matches!(
            kmeans(&x, &KmeansOptions { clusters: 5, max_passes: 10 }),
            Err(KmeansError::TooManyClusters { .. })
        ));
        assert_eq!(kmeans(&[], &KmeansOptions::default()), Err(KmeansError::NoColumns));
        let same = vec![vec![1.0, 1.0, 1.0, 1.0]];
        assert_eq!(kmeans(&same, &KmeansOptions { clusters: 2, max_passes: 5 }), Err(KmeansError::EmptyCluster));
    }
}
