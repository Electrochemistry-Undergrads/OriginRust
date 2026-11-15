//! K-means cluster analysis against Origin's `kmeans` X-Function (Origin 2016). The data are ours (12 and 15 cases
//! in two variables, ten cases in one); the runs are `tools/origin-ref/kmeans-suite1.lines.txt` and
//! `kmeans-suite2.lines.txt`, raw output in `re-workspace/scratch/t29/km2.out` and `km4.out` (local). Origin prints
//! 17 digits: tolerance 1e-12 relative, 1e-9 for the probabilities.

#![allow(clippy::excessive_precision)]

use sheaf_calc::{KmeansOptions, kmeans};

fn close(a: f64, b: f64, tol: f64, what: &str) {
    assert!((a - b).abs() <= tol * b.abs().max(1.0), "{what}: {a} vs {b}");
}

fn run(columns: &[Vec<f64>], k: usize) -> sheaf_calc::Kmeans {
    kmeans(columns, &KmeansOptions { clusters: k, max_passes: 100 }).unwrap()
}

fn twelve() -> Vec<Vec<f64>> {
    vec![
        vec![1.0, 1.5, 3.0, 5.0, 3.5, 4.5, 3.5, 6.0, 7.5, 8.0, 7.0, 9.0],
        vec![1.0, 2.0, 4.0, 7.0, 5.0, 5.0, 4.5, 8.0, 7.0, 9.0, 6.5, 8.5],
    ]
}

fn fifteen() -> Vec<Vec<f64>> {
    vec![
        vec![3.1, 8.4, 5.5, 1.2, 9.9, 4.4, 7.7, 2.2, 6.6, 0.5, 8.8, 3.3, 5.1, 9.2, 1.9],
        vec![7.2, 1.3, 5.5, 2.4, 9.1, 8.8, 3.3, 6.1, 0.9, 9.5, 6.4, 1.1, 2.9, 4.7, 8.3],
    ]
}

fn ten() -> Vec<f64> {
    vec![2.0, 9.0, 4.0, 15.0, 7.0, 12.0, 1.0, 20.0, 6.0, 11.0]
}

#[test]
fn two_clusters_with_every_table() {
    let r = run(&twelve(), 2);
    close(r.mean[0], 4.9583333333333339, 1e-12, "mean A");
    close(r.sd[0], 2.5801544468688244, 1e-12, "SD A");
    close(r.mean[1], 5.625, 1e-12, "mean B");
    close(r.sd[1], 2.5056753762028094, 1e-12, "SD B");
    assert_eq!(r.count, vec![12, 12]);
    let (a, b) = (&r.clusters[0], &r.clusters[1]);
    assert_eq!((a.initial.clone(), b.initial.clone()), (vec![1.0, 1.0], vec![9.0, 8.5]));
    close(a.centre[0], 2.8333333333333335, 1e-12, "centre 1 A");
    close(a.centre[1], 3.5833333333333335, 1e-12, "centre 1 B");
    close(b.centre[0], 7.083333333333333, 1e-12, "centre 2 A");
    close(b.centre[1], 7.666666666666667, 1e-12, "centre 2 B");
    assert_eq!((a.count, b.count), (6, 6));
    close(a.sum_squares, 23.041666666666668, 1e-12, "SS 1");
    close(a.mean_distance, 1.7621719354858907, 1e-12, "mean dist 1");
    close(a.max_distance, 3.1677629681247024, 1e-12, "max dist 1");
    close(b.sum_squares, 15.041666666666668, 1e-12, "SS 2");
    close(b.mean_distance, 1.4974484754463908, 1e-12, "mean dist 2");
    close(b.max_distance, 2.1874007914011142, 1e-12, "max dist 2");
    close(r.centre_distances[0][1], 5.8937349033623079, 1e-12, "centre distance");
    close(r.centre_distances[1][0], 5.8937349033623079, 1e-12, "centre distance");
    let anova = [
        (54.187500000000014, 1.9041666666666663, 28.457330415754935, 0.0003308109458772801),
        (50.020833333333343, 1.9041666666666668, 26.2691466083151, 0.00044719617924082744),
    ];
    for (v, (msb, mse, f, p)) in r.anova.iter().zip(anova) {
        assert_eq!((v.df_between, v.df_error), (1.0, 10.0));
        close(v.ms_between, msb, 1e-12, "MS between");
        close(v.ms_error, mse, 1e-12, "MS error");
        close(v.f, f, 1e-12, "F");
        close(v.p, p, 1e-9, "p");
    }
    // Membership sheet: cluster and distance to the centre.
    let m = |i: usize| r.membership[i].unwrap();
    assert_eq!(m(0).0, 0);
    close(m(0).1, 3.1677629681247024, 1e-12, "distance of case 1");
    assert_eq!(m(3).0, 1);
    close(m(3).1, 2.1874007914011142, 1e-12, "distance of case 4");
    assert_eq!(m(11).0, 1);
    close(m(11).1, 2.0899893673307424, 1e-12, "distance of case 12");
}

#[test]
fn three_clusters_with_a_cluster_of_two() {
    let r = run(&twelve(), 3);
    let init: Vec<Vec<f64>> = r.clusters.iter().map(|c| c.initial.clone()).collect();
    assert_eq!(init, vec![vec![1.0, 1.0], vec![9.0, 8.5], vec![3.0, 4.0]]);
    let counts: Vec<usize> = r.clusters.iter().map(|c| c.count).collect();
    assert_eq!(counts, vec![2, 6, 4]);
    close(r.clusters[0].centre[0], 1.25, 1e-12, "c1 A");
    close(r.clusters[2].centre[0], 3.625, 1e-12, "c3 A");
    close(r.clusters[2].centre[1], 4.625, 1e-12, "c3 B");
    close(r.clusters[0].sum_squares, 0.625, 1e-12, "SS 1");
    close(r.clusters[2].sum_squares, 1.875, 1e-12, "SS 3");
    close(r.clusters[2].mean_distance, 0.60197912938346432, 1e-12, "mean dist 3");
    close(r.clusters[2].max_distance, 0.95197163823298858, 1e-12, "max dist 3");
    close(r.centre_distances[0][1], 8.4885543854978955, 1e-12, "d12");
    close(r.centre_distances[0][2], 3.9250796170268956, 1e-12, "d13");
    close(r.centre_distances[1][2], 4.60562759627345, 1e-12, "d23");
    let v = &r.anova[0];
    assert_eq!((v.df_between, v.df_error), (2.0, 9.0));
    close(v.ms_between, 30.854166666666661, 1e-12, "MS between");
    close(v.ms_error, 1.2800925925925928, 1e-12, "MS error");
    close(v.f, 24.103074141048815, 1e-12, "F");
    close(v.p, 0.00024299599110699788, 1e-9, "p");
}

/// One variable, ten cases: the initial centres (Origin's replacement rule) and the centres Origin ends with, which
/// are not what Lloyd's iterations give from the same start for k = 3 and k = 4.
#[test]
fn one_variable_two_three_and_four_clusters() {
    let check = |data: Vec<f64>, k: usize, initial: &[f64], centre: &[f64], counts: &[usize]| {
        let r = run(&[data], k);
        let i: Vec<f64> = r.clusters.iter().map(|c| c.initial[0]).collect();
        assert_eq!(i, initial, "initial centres, k = {k}");
        for (c, e) in r.clusters.iter().zip(centre) {
            close(c.centre[0], *e, 1e-12, "centre");
        }
        let n: Vec<usize> = r.clusters.iter().map(|c| c.count).collect();
        assert_eq!(n, counts, "k = {k}");
        r
    };
    let r = check(ten(), 2, &[1.0, 20.0], &[4.833333333333333, 14.5], &[6, 4]);
    close(r.mean[0], 8.7000000000000011, 1e-12, "mean");
    close(r.sd[0], 5.9637795622127641, 1e-12, "SD");
    close(r.clusters[0].sum_squares, 46.833333333333343, 1e-12, "SS 1");
    close(r.clusters[0].max_distance, 4.166666666666667, 1e-12, "max 1");
    close(r.clusters[1].mean_distance, 3.0, 1e-12, "mean 2");
    close(r.centre_distances[0][1], 9.6666666666666679, 1e-12, "distance");
    check(ten(), 3, &[1.0, 11.0, 20.0], &[3.25, 9.75, 17.5], &[4, 4, 2]);
    let r = check(
        ten(),
        4,
        &[1.0, 9.0, 20.0, 15.0],
        &[2.3333333333333335, 7.333333333333333, 20.0, 12.666666666666666],
        &[3, 3, 1, 3],
    );
    close(r.clusters[3].sum_squares, 8.6666666666666679, 1e-12, "SS 4");
    // The same cases in the reverse order: the cluster numbers follow the order of the cases.
    let reversed = vec![11.0, 6.0, 20.0, 1.0, 12.0, 7.0, 15.0, 4.0, 9.0, 2.0];
    check(reversed, 3, &[11.0, 1.0, 20.0], &[9.75, 3.25, 17.5], &[4, 4, 2]);
}

#[test]
fn two_variables_fifteen_cases() {
    let check = |k: usize, initial: &[[f64; 2]], centre: &[[f64; 2]], counts: &[usize]| {
        let r = run(&fifteen(), k);
        for (c, (i, e)) in r.clusters.iter().zip(initial.iter().zip(centre)) {
            assert_eq!(c.initial, i.to_vec(), "initial centres, k = {k}");
            close(c.centre[0], e[0], 1e-12, "centre A");
            close(c.centre[1], e[1], 1e-12, "centre B");
        }
        let n: Vec<usize> = r.clusters.iter().map(|c| c.count).collect();
        assert_eq!(n, counts, "k = {k}");
        r
    };
    let r = check(
        3,
        &[[0.5, 9.5], [3.3, 1.1], [9.9, 9.1]],
        &[[2.42, 7.98], [5.4, 2.4857142857142853], [9.3, 6.7333333333333334]],
        &[5, 7, 3],
    );
    close(r.clusters[0].sum_squares, 15.616000000000007, 1e-12, "SS 1");
    close(r.centre_distances[0][1], 6.2504060276276538, 1e-12, "d12");
    check(
        4,
        &[[0.5, 9.5], [8.4, 1.3], [9.9, 9.1], [1.2, 2.4]],
        &[[2.42, 7.98], [6.66, 2.78], [9.3, 6.7333333333333334], [2.25, 1.75]],
        &[5, 5, 3, 2],
    );
    // Five clusters: Lloyd's iterations from these centres end with sizes 4, 5, 2, 2, 2; Origin's are 4, 4, 2, 2, 3.
    check(
        5,
        &[[4.4, 8.8], [8.4, 1.3], [0.5, 9.5], [1.2, 2.4], [9.9, 9.1]],
        &[[3.8, 6.9], [6.95, 2.1], [1.2, 8.9], [2.25, 1.75], [9.3, 6.7333333333333334]],
        &[4, 4, 2, 2, 3],
    );
}
