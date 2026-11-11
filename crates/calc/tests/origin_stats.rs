//! Descriptive statistics against numbers from Origin 2016 (`colstats`, `stats`).
//!
//! Reference scripts: `tools/origin-ref/colstats-suite1.labtalk.txt` .. `colstats-suite3.labtalk.txt`
//! (run with `run-labtalk-data.ps1`). Inputs are closed-form columns of different lengths.
//! Tolerance: relative 1e-11 (skewness and kurtosis of tiny samples: 1e-9).

use sheaf_calc::{quantile_origin, stats};
#[allow(dead_code)]
fn grid(n: usize, start: f64, step: f64) -> Vec<f64> {
    (0..n).map(|i| start + i as f64 * step).collect()
}

#[allow(dead_code)]
fn close(name: &str, got: f64, want: f64, tol: f64) {
    if want.is_nan() {
        assert!(got.is_nan(), "{name}: got {got:?}, Origin gave NaN");
        return;
    }
    let ok = (got - want).abs() <= tol * want.abs().max(1.0);
    assert!(ok, "{name}: got {got:?}, Origin gave {want:?}");
}

#[allow(dead_code)]
fn check(name: &str, got: &[f64], samples: &[(usize, f64)], tol: f64) {
    for &(i, w) in samples {
        close(&format!("{name}[{i}]"), got[i], w, tol);
    }
}

struct Want {
    mean: f64,
    sd: f64,
    sem: f64,
    n: f64,
    min: f64,
    q1: f64,
    median: f64,
    q3: f64,
    max: f64,
    iqr: f64,
    sum: f64,
    variance: f64,
    skewness: f64,
    kurtosis: f64,
    cv: f64,
    missing: f64,
}

fn compare(name: &str, data: &[f64], w: &Want) {
    let s = stats(data);
    let t = 1e-11;
    close(&format!("{name} n"), s.n as f64, w.n, 0.0);
    close(&format!("{name} missing"), s.missing as f64, w.missing, 0.0);
    close(&format!("{name} mean"), s.mean, w.mean, t);
    close(&format!("{name} sd"), s.sd, w.sd, t);
    close(&format!("{name} sem"), s.sem, w.sem, t);
    close(&format!("{name} min"), s.min, w.min, t);
    close(&format!("{name} q1"), s.q1, w.q1, t);
    close(&format!("{name} median"), s.median, w.median, t);
    close(&format!("{name} q3"), s.q3, w.q3, t);
    close(&format!("{name} max"), s.max, w.max, t);
    close(&format!("{name} iqr"), s.iqr, w.iqr, t);
    close(&format!("{name} sum"), s.sum, w.sum, t);
    close(&format!("{name} variance"), s.variance, w.variance, t);
    close(&format!("{name} skewness"), s.skewness, w.skewness, 1e-9);
    close(&format!("{name} kurtosis"), s.kurtosis, w.kurtosis, 1e-9);
    close(&format!("{name} cv"), s.cv, w.cv, t);
}

#[test]
fn four_columns_with_a_missing_value() {
    let ramp = |n: usize| -> Vec<f64> { (1..=n).map(|i| i as f64).collect() };
    let a: Vec<f64> = ramp(9).iter().map(|i| i.sin() * 3.0 + 1.0).collect();
    let mut b: Vec<f64> = ramp(11).iter().map(|i| i.powf(1.5) - 10.0 * i / 3.0).collect();
    b[4] = f64::NAN;
    let c: Vec<f64> = ramp(10).iter().map(|i| (i * 37.0) % 11.0).collect();
    let d: Vec<f64> = ramp(4).iter().map(|i| 2.0_f64.powf(*i)).collect();
    let want = [
        Want {
            mean: 1.6517364940357933,
            sd: 2.194493132269025,
            sem: 0.7314977107563417,
            n: 9.0,
            min: -1.8767728239894153,
            q1: 0.16175350540322242,
            median: 2.2363554557252696,
            q3: 3.5244129544236893,
            max: 3.9680747398701453,
            iqr: 3.362659449020467,
            sum: 14.86562844632214,
            variance: 4.8158001075759165,
            skewness: -0.6350975383054819,
            kurtosis: -1.156585049895583,
            cv: 1.3285975942246575,
            missing: 0.0,
        },
        Want {
            mean: -3.53584898581666,
            sd: 1.7074708299328247,
            sem: 0.5399496860885733,
            n: 10.0,
            min: -5.333333333333334,
            q1: -4.813074155881196,
            median: -3.938744605308811,
            q3: -2.3333333333333335,
            max: -0.1837939727572646,
            iqr: 2.4797408225478628,
            sum: -35.3584898581666,
            variance: 2.9154566350714894,
            skewness: 0.8379700901753718,
            kurtosis: -0.10154061303620754,
            cv: -0.482902645667843,
            missing: 1.0,
        },
        Want {
            mean: 5.500000000000001,
            sd: 3.0276503540974913,
            sem: 0.9574271077563379,
            n: 10.0,
            min: 1.0,
            q1: 3.0,
            median: 5.5,
            q3: 8.0,
            max: 10.0,
            iqr: 5.0,
            sum: 55.00000000000001,
            variance: 9.166666666666664,
            skewness: 0.0,
            kurtosis: -1.2000000000000002,
            cv: 0.5504818825631801,
            missing: 0.0,
        },
        Want {
            mean: 7.5,
            sd: 6.191391873668903,
            sem: 3.0956959368344514,
            n: 4.0,
            min: 2.0,
            q1: 3.0,
            median: 6.0,
            q3: 12.0,
            max: 16.0,
            iqr: 9.0,
            sum: 30.0,
            variance: 38.33333333333333,
            skewness: 1.1376243669576886,
            kurtosis: 0.7576559546313781,
            cv: 0.8255189164891871,
            missing: 0.0,
        },
    ];
    for (name, data, w) in [("A", &a, &want[0]), ("B", &b, &want[1]), ("C", &c, &want[2]), ("D", &d, &want[3])] {
        compare(name, data, w);
    }
    // the scalar `stats` run on A and B gave the same mean / sd / sum
    close("stats A mean", stats(&a).mean, 1.6517364940357933, 1e-12);
    close("stats B sd", stats(&b).sd, 1.7074708299328247, 1e-12);
    close("stats B sum", stats(&b).sum, -35.3584898581666, 1e-12);
}

#[test]
fn small_samples_and_the_quartile_rule() {
    let lens = [1usize, 2, 3, 5, 7, 11, 13];
    let want = [
        Want {
            mean: 7.1,
            sd: f64::NAN,
            sem: f64::NAN,
            n: 1.0,
            min: 7.1,
            q1: 7.1,
            median: 7.1,
            q3: 7.1,
            max: 7.1,
            iqr: 0.0,
            sum: 7.1,
            variance: f64::NAN,
            skewness: f64::NAN,
            kurtosis: f64::NAN,
            cv: f64::NAN,
            missing: 0.0,
        },
        Want {
            mean: 4.65,
            sd: 3.4648232278140823,
            sem: 2.4499999999999993,
            n: 2.0,
            min: 2.2,
            q1: 2.2,
            median: 4.65,
            q3: 7.1,
            max: 7.1,
            iqr: 4.8999999999999995,
            sum: 9.3,
            variance: 12.004999999999995,
            skewness: f64::NAN,
            kurtosis: f64::NAN,
            cv: 0.7451232747987273,
            missing: 0.0,
        },
        Want {
            mean: 6.866666666666667,
            sd: 4.554484969053398,
            sem: 2.6295331229030836,
            n: 3.0,
            min: 2.2,
            q1: 2.2,
            median: 7.1,
            q3: 11.3,
            max: 11.3,
            iqr: 9.100000000000001,
            sum: 20.6,
            variance: 20.743333333333332,
            skewness: -0.22993688673998372,
            kurtosis: f64::NAN,
            cv: 0.6632745100563201,
            missing: 0.0,
        },
        Want {
            mean: 7.1,
            sd: 3.305298776207682,
            sem: 1.4781745499094483,
            n: 5.0,
            min: 2.2,
            q1: 6.5,
            median: 7.1,
            q3: 8.4,
            max: 11.3,
            iqr: 1.9000000000000004,
            sum: 35.5,
            variance: 10.925000000000002,
            skewness: -0.47977882314133025,
            kurtosis: 1.3275390246584475,
            cv: 0.4655350389024905,
            missing: 0.0,
        },
        Want {
            mean: 6.685714285714285,
            sd: 2.790118618403509,
            sem: 1.0545657132381154,
            n: 7.0,
            min: 2.2,
            q1: 5.6,
            median: 6.5,
            q3: 8.4,
            max: 11.3,
            iqr: 2.8000000000000007,
            sum: 46.8,
            variance: 7.784761904761906,
            skewness: 0.11656581271867042,
            kurtosis: 1.293341767259438,
            cv: 0.41732543437659325,
            missing: 0.0,
        },
        Want {
            mean: 7.054545454545454,
            sd: 3.0203852854772144,
            sem: 0.9106804285671274,
            n: 11.0,
            min: 2.2,
            q1: 5.6,
            median: 6.8,
            q3: 8.9,
            max: 12.0,
            iqr: 3.3000000000000007,
            sum: 77.6,
            variance: 9.122727272727275,
            skewness: 0.10732695073454875,
            kurtosis: -0.3208820667275889,
            cv: 0.42814739871455365,
            missing: 0.0,
        },
        Want {
            mean: 6.699999999999999,
            sd: 3.2148094811357018,
            sem: 0.8916277250063505,
            n: 13.0,
            min: 1.3,
            q1: 5.6,
            median: 6.8,
            q3: 8.4,
            max: 12.0,
            iqr: 2.8000000000000007,
            sum: 87.1,
            variance: 10.334999999999999,
            skewness: -0.09595031372258017,
            kurtosis: -0.43622123693323367,
            cv: 0.479822310617269,
            missing: 0.0,
        },
    ];
    for (n, w) in lens.iter().zip(&want) {
        let data: Vec<f64> = (1..=*n).map(|i| ((i * i * 7) % 13) as f64 + i as f64 / 10.0).collect();
        compare(&format!("n={n}"), &data, w);
    }
}

#[test]
fn quartile_positions_for_squares() {
    // values i^2, so that any interpolation between neighbours is visible in the result
    let lens = [4usize, 6, 8, 9, 10, 12, 14, 15, 16, 20, 21];
    let want = [
        Want {
            mean: 7.5,
            sd: 6.557438524302,
            sem: 3.278719262151,
            n: 4.0,
            min: 1.0,
            q1: 2.5,
            median: 6.5,
            q3: 12.5,
            max: 16.0,
            iqr: 10.0,
            sum: 30.0,
            variance: 42.99999999999999,
            skewness: 0.7092956759656032,
            kurtosis: -0.5916711736073577,
            cv: 0.8743251365736,
            missing: 0.0,
        },
        Want {
            mean: 15.166666666666668,
            sd: 13.377842377105011,
            sem: 5.461481280548143,
            n: 6.0,
            min: 1.0,
            q1: 4.0,
            median: 12.5,
            q3: 25.0,
            max: 36.0,
            iqr: 21.0,
            sum: 91.0,
            variance: 178.96666666666667,
            skewness: 0.6921205708772438,
            kurtosis: -0.6768612025722041,
            cv: 0.8820555413475831,
            missing: 0.0,
        },
        Want {
            mean: 25.5,
            sd: 22.58317958127243,
            sem: 7.984359711335655,
            n: 8.0,
            min: 1.0,
            q1: 6.5,
            median: 20.5,
            q3: 42.5,
            max: 64.0,
            iqr: 36.0,
            sum: 204.0,
            variance: 509.99999999999994,
            skewness: 0.6815017404918067,
            kurtosis: -0.7208067226890771,
            cv: 0.8856148855400953,
            missing: 0.0,
        },
        Want {
            mean: 31.666666666666668,
            sd: 28.080242164197944,
            sem: 9.360080721399315,
            n: 9.0,
            min: 1.0,
            q1: 9.0,
            median: 25.0,
            q3: 49.0,
            max: 81.0,
            iqr: 40.0,
            sum: 285.0,
            variance: 788.5,
            skewness: 0.677613056720495,
            kurtosis: -0.7356415425266607,
            cv: 0.8867444893957245,
            missing: 0.0,
        },
        Want {
            mean: 38.5,
            sd: 34.17357653704589,
            sem: 10.80663376511545,
            n: 10.0,
            min: 1.0,
            q1: 9.0,
            median: 30.5,
            q3: 64.0,
            max: 100.0,
            iqr: 55.0,
            sum: 385.0,
            variance: 1167.8333333333333,
            skewness: 0.6743668131337159,
            kurtosis: -0.747573126581873,
            cv: 0.8876253645985945,
            missing: 0.0,
        },
        Want {
            mean: 54.166666666666664,
            sd: 48.149074065171106,
            sem: 13.899440436378883,
            n: 12.0,
            min: 1.0,
            q1: 12.5,
            median: 42.5,
            q3: 90.5,
            max: 144.0,
            iqr: 78.0,
            sum: 650.0,
            variance: 2318.333333333333,
            skewness: 0.6692610333856905,
            kurtosis: -0.7655725390863828,
            cv: 0.8889059827416205,
            missing: 0.0,
        },
        Want {
            mean: 72.5,
            sd: 64.50968919472486,
            sem: 17.24093964956667,
            n: 14.0,
            min: 1.0,
            q1: 16.0,
            median: 56.5,
            q3: 121.0,
            max: 196.0,
            iqr: 105.0,
            sum: 1015.0,
            variance: 4161.500000000001,
            skewness: 0.6654322754325552,
            kurtosis: -0.778500950935257,
            cv: 0.8897888164789636,
            missing: 0.0,
        },
        Want {
            mean: 82.66666666666667,
            sd: 73.58441864054284,
            sem: 18.999415195678466,
            n: 15.0,
            min: 1.0,
            q1: 16.0,
            median: 64.0,
            q3: 144.0,
            max: 225.0,
            iqr: 128.0,
            sum: 1240.0,
            variance: 5414.666666666668,
            skewness: 0.663856699174109,
            kurtosis: -0.7836883384439659,
            cv: 0.8901340964581794,
            missing: 0.0,
        },
        Want {
            mean: 93.5,
            sd: 83.25543025332742,
            sem: 20.813857563331855,
            n: 16.0,
            min: 1.0,
            q1: 20.5,
            median: 72.5,
            q3: 156.5,
            max: 256.0,
            iqr: 136.0,
            sum: 1496.0,
            variance: 6931.466666666666,
            skewness: 0.662456904820869,
            kurtosis: -0.7882345903152275,
            cv: 0.8904324091264965,
            missing: 0.0,
        },
        Want {
            mean: 143.5,
            sd: 127.90230646864818,
            sem: 28.59982517429084,
            n: 20.0,
            min: 1.0,
            q1: 30.5,
            median: 110.5,
            q3: 240.5,
            max: 400.0,
            iqr: 210.0,
            sum: 2870.0,
            variance: 16359.000000000002,
            skewness: 0.6581359770935882,
            kurtosis: -0.8019127836707729,
            cv: 0.8913052715585239,
            missing: 0.0,
        },
        Want {
            mean: 157.66666666666666,
            sd: 140.55473429711762,
            sem: 30.671557580997487,
            n: 21.0,
            min: 1.0,
            q1: 36.0,
            median: 121.0,
            q3: 256.0,
            max: 441.0,
            iqr: 220.0,
            sum: 3311.0,
            variance: 19755.63333333333,
            skewness: 0.6572918039533433,
            kurtosis: -0.8045246430004895,
            cv: 0.8914676593897524,
            missing: 0.0,
        },
    ];
    for (n, w) in lens.iter().zip(&want) {
        let data: Vec<f64> = (1..=*n).map(|i| (i * i) as f64).collect();
        compare(&format!("n={n}"), &data, w);
        let s: Vec<f64> = data.clone();
        close(&format!("n={n} q1 direct"), quantile_origin(&s, 0.25), w.q1, 1e-12);
        close(&format!("n={n} q3 direct"), quantile_origin(&s, 0.75), w.q3, 1e-12);
    }
}

#[test]
fn empty_and_all_missing() {
    let s = stats(&[f64::NAN, f64::NAN]);
    assert_eq!((s.n, s.missing), (0, 2));
    assert!(s.mean.is_nan() && s.median.is_nan() && s.q1.is_nan());
}
