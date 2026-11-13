use crate::{ColumnRef, Expr, Host, SimpleHost};

fn ev(src: &str) -> f64 {
    Expr::parse(src)
        .unwrap_or_else(|e| panic!("{src}: {e}"))
        .eval(&SimpleHost::new())
        .unwrap_or_else(|e| panic!("{src}: {e}"))
}

fn near(src: &str, want: f64) {
    let got = ev(src);
    assert!((got - want).abs() <= 1e-12 * want.abs().max(1.0), "{src} = {got}, want {want}");
}

fn err(src: &str) -> crate::ExprError {
    match Expr::parse(src) {
        Err(e) => e,
        Ok(e) => e.eval(&SimpleHost::new()).expect_err(src),
    }
}

#[test]
fn numbers() {
    assert_eq!(ev("42"), 42.0);
    assert_eq!(ev(".5"), 0.5);
    assert_eq!(ev("5."), 5.0);
    assert_eq!(ev("1e-3"), 0.001);
    assert_eq!(ev("2.5E+2"), 250.0);
}

#[test]
fn precedence_and_associativity() {
    assert_eq!(ev("1+2*3"), 7.0);
    assert_eq!(ev("(1+2)*3"), 9.0);
    assert_eq!(ev("10-4-3"), 3.0);
    assert_eq!(ev("100/10/5"), 2.0);
    assert_eq!(ev("2^3^2"), 512.0);
    assert_eq!(ev("-2^2"), -4.0);
    assert_eq!(ev("(-2)^2"), 4.0);
    assert_eq!(ev("2^-1"), 0.5);
    assert_eq!(ev("2*3^2"), 18.0);
    assert_eq!(ev("7 % 4"), 3.0);
    assert_eq!(ev("--3"), 3.0);
    assert_eq!(ev("1 - -1"), 2.0);
    // Comparison binds looser than arithmetic, && looser than comparison, || loosest.
    assert_eq!(ev("1+1 == 2"), 1.0);
    assert_eq!(ev("1 < 2 && 2 < 3"), 1.0);
    assert_eq!(ev("1 || 0 && 0"), 1.0);
    assert_eq!(ev("!0 + 1"), 2.0);
    assert_eq!(ev("!(0 + 1)"), 0.0);
}

#[test]
fn comparisons_and_logic() {
    assert_eq!(ev("2 < 3"), 1.0);
    assert_eq!(ev("3 <= 3"), 1.0);
    assert_eq!(ev("3 > 3"), 0.0);
    assert_eq!(ev("3 >= 4"), 0.0);
    assert_eq!(ev("3 == 3"), 1.0);
    assert_eq!(ev("3 != 3"), 0.0);
    assert_eq!(ev("1 && 0"), 0.0);
    assert_eq!(ev("0 || 2"), 1.0);
    assert_eq!(ev("!5"), 0.0);
}

#[test]
fn conditional() {
    assert_eq!(ev("1 ? 10 : 20"), 10.0);
    assert_eq!(ev("0 ? 10 : 20"), 20.0);
    // Right associative: a ? b : c ? d : e
    assert_eq!(ev("0 ? 1 : 0 ? 2 : 3"), 3.0);
    assert_eq!(ev("1 ? 0 ? 1 : 2 : 3"), 2.0);
    assert_eq!(ev("2 > 1 ? 5 : 6"), 5.0);
    assert!(err("1 ? 2").position().is_some());
}

#[test]
fn constants_and_case() {
    near("pi", std::f64::consts::PI);
    near("PI*2", 2.0 * std::f64::consts::PI);
    near("e", std::f64::consts::E);
    near("E^2", std::f64::consts::E * std::f64::consts::E);
    near("SIN(0.5)+Sin(0.5)", 2.0 * 0.5f64.sin());
    assert_eq!(Expr::parse("pi*e").unwrap().variables(), Vec::<String>::new());
}

#[test]
fn functions() {
    assert_eq!(ev("abs(-2.5)"), 2.5);
    assert_eq!(ev("sqrt(81)"), 9.0);
    near("exp(1.5)", 1.5f64.exp());
    near("ln(7)", 7f64.ln());
    assert_eq!(ev("log(1000)"), 3.0);
    assert_eq!(ev("log10(100)"), 2.0);
    assert_eq!(ev("log2(8)"), 3.0);
    near("atan2(1, 2)", 1f64.atan2(2.0));
    near("tanh(0.3)", 0.3f64.tanh());
    near("asin(0.5)", 0.5f64.asin());
    assert_eq!(ev("int(-2.7)"), -2.0);
    assert_eq!(ev("int(2.7)"), 2.0);
    assert_eq!(ev("nint(2.5)"), 3.0);
    assert_eq!(ev("round(2.4)"), 2.0);
    near("round(1.2345, 2)", 1.23);
    assert_eq!(ev("floor(-1.5)"), -2.0);
    assert_eq!(ev("ceil(-1.5)"), -1.0);
    assert_eq!(ev("mod(7, 3)"), 1.0);
    assert_eq!(ev("mod(-7, 3)"), -1.0);
    assert_eq!(ev("sign(-9)"), -1.0);
    assert_eq!(ev("sign(0)"), 0.0);
    assert_eq!(ev("min(4, 2, 8)"), 2.0);
    assert_eq!(ev("max(4, 2, 8)"), 8.0);
    assert_eq!(ev("pow(2, 10)"), 1024.0);
    assert_eq!(ev("ifelse(1 > 2, 7, 8)"), 8.0);
    near("erf(0.7)", 0.677_801_193_837_418_5);
    near("erfc(0.7)", 1.0 - 0.677_801_193_837_418_5);
    near("gamma(6)", 120.0);
    near("gamma(0.5)", std::f64::consts::PI.sqrt());
}

#[test]
fn missing_values_propagate() {
    assert!(ev("na()").is_nan());
    assert!(ev("na() + 1").is_nan());
    assert!(ev("sin(na())").is_nan());
    assert!(ev("max(1, na())").is_nan());
    assert!(ev("1/0").is_nan());
    assert!(ev("sqrt(-1)").is_nan());
    assert!(ev("ln(0)").is_nan());
    assert!(ev("0^-1").is_nan());
    assert!(ev("mod(5, 0)").is_nan());
    assert_eq!(ev("isna(1/0)"), 1.0);
    assert_eq!(ev("isna(2)"), 0.0);
    // The documented way to find gaps.
    assert_eq!(ev("(1/0) == (1/0)"), 1.0);
    assert_eq!(ev("(1/0) == 1"), 0.0);
    assert_eq!(ev("(1/0) != 1"), 1.0);
    // Ordered comparisons and logic treat missing as false; a missing condition takes the else branch.
    assert_eq!(ev("(1/0) < 1"), 0.0);
    assert_eq!(ev("(1/0) && 1"), 0.0);
    assert_eq!(ev("!(1/0)"), 1.0);
    assert_eq!(ev("(1/0) ? 1 : 2"), 2.0);
    // IEEE pow gives 1 for x^0 and 1^x even when x is missing; formulas must not fill gaps.
    assert!(ev("na()^0").is_nan());
    assert!(ev("1^na()").is_nan());
    assert!(ev("pow(na(), 0)").is_nan());
    assert!(ev("round(2.5, na())").is_nan());
    assert_eq!(ev("round(2.345, 1/0 == 1/0)"), 2.3);
}

#[test]
fn statistics_of_infinite_data_are_not_zero() {
    let a = [1.0, f64::INFINITY, 3.0];
    let host = SimpleHost::new().short("A", &a);
    let e = |s: &str| Expr::parse(s).unwrap().eval(&host).unwrap();
    assert!(e("sd(col(A))").is_nan(), "the spread of data with infinity is undefined");
    assert!(e("var(col(A))").is_nan());
    assert_eq!(e("sum(col(A))"), f64::INFINITY);
    assert_eq!(e("count(col(A))"), 3.0);
}

/// The longest accepted formulas must parse and evaluate on a small stack. A review found that an
/// unoptimised build overflowed the 1 MB Windows main thread at about 200 terms before the stack
/// was grown on demand. This workspace's test profile optimises a little, so the test uses a
/// 256 KiB thread instead: without the stack growth it overflows there (checked on 2026-11-11).
#[test]
fn longest_accepted_formulas_run_on_a_small_stack() {
    let run = std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(|| {
            let a = [2.0, 3.0];
            let host = SimpleHost::new().short("A", &a);
            let chain = format!("0{}", "+col(A)".repeat(900));
            let v = Expr::parse(&chain).unwrap().eval_rows(&host, 2).unwrap();
            assert_eq!(v, [1800.0, 2700.0]);
            let nested = format!("{}col(A){}", "abs(".repeat(63), ")".repeat(63));
            let v = Expr::parse(&nested).unwrap().eval_rows(&host, 2).unwrap();
            assert_eq!(v, [2.0, 3.0]);
        })
        .unwrap();
    run.join().expect("no stack overflow");
}

#[test]
fn variables_and_hosts() {
    let host = SimpleHost::from_vars([("A", 2.0), ("t", 4.0)]);
    let e = Expr::parse("A*exp(-x/t) + y0").unwrap();
    assert_eq!(e.variables(), ["A", "x", "t", "y0"]);
    let e = Expr::parse("A*t").unwrap();
    assert_eq!(e.eval(&host).unwrap(), 8.0);
    // Case matters for variables.
    let e = Expr::parse("a").unwrap();
    let er = e.eval(&host).unwrap_err();
    assert_eq!(er.position(), Some(0));
    assert!(er.to_string().contains("unknown variable 'a'"), "{er}");
    // `i` is the 1-based row number unless the host defines it.
    let e = Expr::parse("i").unwrap();
    assert_eq!(e.eval(&SimpleHost::new().at_row(4)).unwrap(), 5.0);
    assert_eq!(e.eval(&SimpleHost::new().var("i", 9.0)).unwrap(), 9.0);
}

#[test]
fn column_references() {
    let a = [1.0, 2.0, 4.0, f64::NAN, 8.0];
    let b = [10.0, 20.0, 30.0];
    let host = SimpleHost::new().short("A", &a).short("B", &b).long("Long Name", &b).column(ColumnRef::Index(1), &a);
    let rows = |s: &str, n: usize| Expr::parse(s).unwrap().eval_rows(&host, n).unwrap();
    assert_eq!(rows("col(A)*2", 3), [2.0, 4.0, 8.0]);
    assert_eq!(rows("col(a)+0", 2), [1.0, 2.0]);
    assert_eq!(rows("col(\"Long Name\")", 3), b);
    assert_eq!(rows("col(1)+wcol(1)", 3), [2.0, 4.0, 8.0]);
    assert_eq!(rows("col(A) + col(B)", 3), [11.0, 22.0, 34.0]);
    // Past the end of a shorter column gives missing.
    let r = rows("col(B)", 5);
    assert_eq!(&r[..3], &b);
    assert!(r[3].is_nan() && r[4].is_nan());
    // Stored missing values propagate.
    assert!(rows("col(A)*2", 5)[3].is_nan());
}

#[test]
fn column_subscripts() {
    let a = [1.0, 2.0, 4.0, 8.0];
    let host = SimpleHost::new().short("A", &a);
    let rows = |s: &str| Expr::parse(s).unwrap().eval_rows(&host, 4).unwrap();
    let prev = rows("col(A)[i-1]");
    assert!(prev[0].is_nan());
    assert_eq!(&prev[1..], &[1.0, 2.0, 4.0]);
    assert_eq!(rows("col(A)[i]"), a);
    assert_eq!(rows("col(A)[2]"), [2.0; 4]);
    let next = rows("col(A)[i+1] - col(A)");
    assert_eq!(&next[..3], &[1.0, 2.0, 4.0]);
    assert!(next[3].is_nan());
    assert!(rows("col(A)[0]").iter().all(|x| x.is_nan()));
    assert!(rows("col(A)[99]").iter().all(|x| x.is_nan()));
    assert_eq!(rows("col(A)[i>1 ? i-1 : 1]"), [1.0, 1.0, 2.0, 4.0]);
}

#[test]
fn statistics_over_columns_and_ranges() {
    let a = [1.0, 2.0, f64::NAN, 5.0];
    let host = SimpleHost::new().short("A", &a);
    let one = |s: &str| Expr::parse(s).unwrap().eval(&host).unwrap();
    assert_eq!(one("sum(col(A))"), 8.0);
    assert_eq!(one("total(col(A))"), 8.0);
    assert_eq!(one("count(col(A))"), 3.0);
    near_eq(one("mean(col(A))"), 8.0 / 3.0);
    assert_eq!(one("min(col(A))"), 1.0);
    assert_eq!(one("max(col(A))"), 5.0);
    assert_eq!(one("median(col(A))"), 2.0);
    assert_eq!(one("mean(col(A)[1:2])"), 1.5);
    assert_eq!(one("sum(col(A)[2:99])"), 7.0);
    assert!(one("mean(col(A)[3:3])").is_nan());
    near_eq(one("sd(col(A)[1:2])"), 0.5f64.sqrt());
    // The documented gap-filling formula.
    let rows = Expr::parse("col(A)==1/0 ? mean(col(A)) : col(A)").unwrap().eval_rows(&host, 4).unwrap();
    near_eq(rows[2], 8.0 / 3.0);
    assert_eq!(rows[3], 5.0);
    // A moving window: the bounds depend on the row.
    let m = Expr::parse("mean(col(A)[i-1:i+1])").unwrap().eval_rows(&host, 2).unwrap();
    assert_eq!(m, [1.5, 1.5]);
    // Baseline subtraction, as in the owner's examples.
    let d = Expr::parse("col(A) - mean(col(A)[1:2])").unwrap().eval_rows(&host, 2).unwrap();
    assert_eq!(d, [-0.5, 0.5]);
    // The scalar min/max with one non-column argument is that argument.
    assert_eq!(ev("min(3)"), 3.0);
}

fn near_eq(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-12, "{a} vs {b}");
}

/// A host with no slices, so every access goes through `cell`.
struct CellHost(Vec<f64>);

impl Host for CellHost {
    fn variable(&self, name: &str) -> Option<f64> {
        (name == "k").then_some(10.0)
    }
    fn cell(&self, col: &ColumnRef, row: usize) -> Option<f64> {
        matches!(col, ColumnRef::Short(s) if s == "A").then(|| self.0.get(row).copied()).flatten()
    }
    fn has_column(&self, col: &ColumnRef) -> bool {
        matches!(col, ColumnRef::Short(s) if s == "A")
    }
}

#[test]
fn host_without_slices() {
    let h = CellHost(vec![1.0, 2.0, 3.0]);
    let e = Expr::parse("col(A)*k + col(A)[i-1]").unwrap();
    let v = e.eval_rows(&h, 3).unwrap();
    assert!(v[0].is_nan());
    assert_eq!(&v[1..], &[21.0, 32.0]);
    let m = Expr::parse("mean(col(A)[i:3])").unwrap().eval_rows(&h, 3).unwrap();
    assert_eq!(m, [2.0, 2.5, 3.0]);
    assert_eq!(Expr::parse("sum(col(A))").unwrap().eval(&h).unwrap(), 6.0);
}

#[test]
fn dependencies() {
    let e = Expr::parse("col(B)*k + col(\"Long\")[i-1] - wcol(3) + col(b) + col(B)").unwrap();
    assert_eq!(
        e.columns(),
        [
            ColumnRef::Short("B".into()),
            ColumnRef::Long("Long".into()),
            ColumnRef::Index(3),
            ColumnRef::Short("b".into()),
        ]
    );
    assert_eq!(e.variables(), ["k", "i"]);
    assert_eq!(e.source(), "col(B)*k + col(\"Long\")[i-1] - wcol(3) + col(b) + col(B)");
}

#[test]
fn error_messages_and_positions() {
    let cases: &[(&str, usize, &str)] = &[
        ("1 + foo(2)", 4, "unknown function 'foo'"),
        ("sin(1, 2)", 0, "function 'sin' takes 1 argument, but 2 were given"),
        ("atan2(1)", 0, "takes 2 arguments, but 1 was given"),
        ("na(1)", 0, "takes no arguments"),
        ("1 + * 2", 4, "expected a value"),
        ("1 +", 3, "unexpected end of expression"),
        ("(1 + 2", 6, "expected ')'"),
        ("1 + 2)", 5, "unmatched ')'"),
        ("1 2", 2, "found a number"),
        ("", 0, "empty"),
        ("   ", 0, "empty"),
        ("1 = 2", 2, "single '='"),
        ("1 & 2", 2, "'&' must be doubled"),
        ("2 $ 3", 2, "unexpected character '$'"),
        ("col(\"abc", 4, "missing its closing"),
        ("col(2.5)", 4, "whole number"),
        ("col()", 4, "expected a column name"),
        ("col(A", 5, "expected ')'"),
        ("(1+2)[1]", 5, "only a column reference"),
        ("col(A)[1", 8, "expected ']'"),
        ("sum(3)", 0, "needs a column"),
        ("1e999", 0, "too large"),
        ("\"text\" + 1", 0, "text strings"),
        ("1 ? 2 3", 6, "expected ':'"),
    ];
    for (src, pos, text) in cases {
        let e = Expr::parse(src).expect_err(src);
        assert_eq!(e.position(), Some(*pos), "{src}: {e}");
        assert!(e.to_string().contains(text), "{src}: {e}");
        assert!(e.to_string().ends_with(&format!("at position {pos}")), "{src}: {e}");
        // The place is named once only.
        assert_eq!(e.to_string().matches("position").count(), 1, "{src}: {e}");
        assert!(!e.message().contains("position"), "{src}: {e}");
    }
}

#[test]
fn evaluation_errors() {
    let e = err("1 + x*2");
    assert_eq!(e.position(), Some(4));
    assert!(e.to_string().contains("unknown variable 'x'"));
    let e = err("2*col(Q)");
    assert_eq!(e.position(), Some(2));
    assert!(e.to_string().contains("unknown column col(Q)"), "{e}");
    let e = err("1 + mean(col(Q)[1:3])");
    assert!(e.to_string().contains("unknown column"), "{e}");
    let e = Expr::parse("col(A)[1:3] + 1").unwrap();
    let e = e.eval(&SimpleHost::new().short("A", &[1.0])).unwrap_err();
    assert_eq!(e.position(), Some(6));
    assert!(e.to_string().contains("statistics function"), "{e}");
}

#[test]
fn garbage_and_deep_input_never_panic() {
    let mut inputs: Vec<String> = vec![
        "(".repeat(100_000),
        ")".repeat(1000),
        "-".repeat(100_000),
        "!".repeat(100_000),
        "1+".repeat(100_000),
        "2^".repeat(100_000),
        "a?".repeat(50_000),
        "f(".repeat(50_000),
        "col(A)[".repeat(10_000),
        format!("{}1{}", "(".repeat(5000), ")".repeat(5000)),
        format!("{}1", "-(".repeat(5000)),
        "1".repeat(100_000),
        "\"".to_string(),
        "col(\"".to_string(),
        "é€😀 + 1".to_string(),
        "x\u{0}y".to_string(),
        "1e".to_string(),
        "1e+".to_string(),
        ".".to_string(),
        "..5".to_string(),
        "[]:?,".to_string(),
        "col(99999999999999999999)".to_string(),
        "col(-1)".to_string(),
        "1 +\u{a0}\u{2003}2".to_string(),
    ];
    // Deterministic pseudo-random soup of tokens and bytes.
    let atoms = [
        "(", ")", "[", "]", ":", "?", ",", "+", "-", "*", "/", "^", "%", "<", ">", "=", "!", "&", "|", "1", "2.5",
        ".5", "e", "pi", "i", "x", "col(", "wcol(", "sin(", "max(", "mean(", "\"", "é", " ", "1e3", "A", "if(", "na(",
    ];
    let mut seed = 0x2545_f491_4f6c_dd1du64;
    for _ in 0..3000 {
        let mut s = String::new();
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let len = 1 + (seed % 24) as usize;
        for _ in 0..len {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            s.push_str(atoms[(seed % atoms.len() as u64) as usize]);
        }
        inputs.push(s);
    }
    let data = [1.0, 2.0, 3.0];
    let host = SimpleHost::new().short("A", &data).var("x", 1.5);
    for src in &inputs {
        if let Ok(e) = Expr::parse(src) {
            let _ = e.variables();
            let _ = e.columns();
            let _ = e.eval_rows(&host, 3);
        }
    }
    // Deeply nested but valid-looking input is rejected with an error, not a crash.
    let e = Expr::parse(&format!("{}1{}", "(".repeat(5000), ")".repeat(5000))).unwrap_err();
    assert!(e.to_string().contains("too deeply"), "{e}");
    // A long flat sum is fine up to the node-depth limit.
    assert!(Expr::parse(&vec!["1"; 500].join("+")).is_ok());
}

#[test]
fn million_rows() {
    let n = 1_000_000;
    let a: Vec<f64> = (0..n).map(|k| k as f64).collect();
    let host = SimpleHost::new().short("A", &a).var("k", 0.5);
    let e = Expr::parse("col(A)*2 + k - i").unwrap();
    let v = e.eval_rows(&host, n).unwrap();
    assert_eq!(v.len(), n);
    assert_eq!(v[0], 0.0 + 0.5 - 1.0);
    assert_eq!(v[n - 1], 2.0 * (n - 1) as f64 + 0.5 - n as f64);
}

#[test]
fn eval_rows_zero_and_empty_columns() {
    let host = SimpleHost::new().short("A", &[]);
    let e = Expr::parse("col(A)+1").unwrap();
    assert!(e.eval_rows(&host, 0).unwrap().is_empty());
    assert!(e.eval_rows(&host, 2).unwrap().iter().all(|x| x.is_nan()));
}

// ---------------------------------------------------------------------------------------------
// Functions added for T08. Numbers marked "Origin 2016" were measured with
// tools/origin-ref/run-labtalk-lines.ps1 (raw output in re-workspace/scratch/expr-funcs/, local
// only: g1e, g2, g3, g4). Origin 2016 has no cumsum/shift/rank/sem formula functions (the
// formula comes out empty); those follow the choices in docs/spec/expression-language.md and
// are "not checked against Origin".
// ---------------------------------------------------------------------------------------------

/// The Origin probe column: 3 1 4 1 5 (missing) 9 2 6.
const PROBE: [f64; 9] = [3.0, 1.0, 4.0, 1.0, 5.0, f64::NAN, 9.0, 2.0, 6.0];
const NAN: f64 = f64::NAN;

fn rows(src: &str) -> Vec<f64> {
    let host = SimpleHost::new().short("A", &PROBE).column(ColumnRef::Index(1), &PROBE);
    Expr::parse(src).unwrap_or_else(|e| panic!("{src}: {e}")).eval_rows(&host, PROBE.len()).unwrap()
}

fn same(got: &[f64], want: &[f64]) {
    assert_eq!(got.len(), want.len(), "{got:?}");
    for (k, (g, w)) in got.iter().zip(want).enumerate() {
        let ok = (g.is_nan() && w.is_nan()) || (g - w).abs() <= 1e-12 * w.abs().max(1.0);
        assert!(ok, "row {}: {g} vs {w} (all: {got:?})", k + 1);
    }
}

fn first(src: &str) -> f64 {
    rows(src)[0]
}

#[test]
fn stddevp_and_sem() {
    // Origin 2016: stddevp 2.5708704751503917 and stddev 2.7483761439387129 (n = 8, one missing).
    assert!((first("stddevp(col(A))") - 2.570_870_475_150_391_7).abs() < 1e-12);
    assert!((first("stddev(col(A))") - 2.748_376_143_938_713).abs() < 1e-12);
    // sem is not an Origin 2016 formula function (not checked against Origin): sd / sqrt(n).
    assert!((first("sem(col(A))") - 2.748_376_143_938_713 / 8f64.sqrt()).abs() < 1e-12);
    assert!(first("sem(col(A)[1:1])").is_nan());
    assert_eq!(first("stddevp(col(A)[1:1])"), 0.0);
}

#[test]
fn cumsum_matches_origin_sum_of_a_column() {
    // Origin 2016 evaluates `sum(col(A))` in a column formula as the running sum, skipping the
    // missing row and leaving it missing: 3 4 8 9 14 . 23 25 31.
    let want = [3.0, 4.0, 8.0, 9.0, 14.0, NAN, 23.0, 25.0, 31.0];
    same(&rows("cumsum(col(A))"), &want);
    // A row range gives the running sum within the range, from row 1 (Origin 2016: 1 5 6).
    same(&rows("cumsum(col(A)[2:4])"), &[1.0, 5.0, 6.0, NAN, NAN, NAN, NAN, NAN, NAN]);
}

#[test]
fn diff_is_a_forward_difference_with_the_missing_value_last() {
    // Origin 2016: -2 3 -3 4 . . -7 4 . (row i is A[i+1] - A[i]; missing inputs spread to both rows).
    same(&rows("diff(col(A))"), &[-2.0, 3.0, -3.0, 4.0, NAN, NAN, -7.0, 4.0, NAN]);
}

#[test]
fn shift_and_rank() {
    // Not checked against Origin (no such formula function in 2016).
    same(&rows("shift(col(A),1)"), &[NAN, 3.0, 1.0, 4.0, 1.0, 5.0, NAN, 9.0, 2.0]);
    same(&rows("shift(col(A),-2)"), &[4.0, 1.0, 5.0, NAN, 9.0, 2.0, 6.0, NAN, NAN]);
    same(&rows("shift(col(A),0)"), &PROBE);
}

#[test]
fn rank_orders_ascending() {
    same(&rows("rank(col(A))"), &[4.0, 1.5, 5.0, 1.5, 6.0, NAN, 8.0, 3.0, 7.0]);
}

#[test]
fn vector_functions_need_a_column() {
    assert!(Expr::parse("cumsum(3)").is_err());
    assert!(Expr::parse("diff(col(A)+1)").is_err());
    assert!(Expr::parse("shift(col(A))").is_err());
    assert!(Expr::parse("lookup(1, 2, col(A))").is_err());
}

#[test]
fn fact_truncates_and_overflows_to_missing() {
    // Origin 2016: fact(1.5)=1, fact(0.5)=1, fact(2.5)=2, fact(4.5)=24, fact(0)=1, fact(-1) missing,
    // fact(20) = 2.43290200817664E18, fact(170) = 7.257415615308004E306, fact(171) missing.
    same(&rows("fact(col(A)/2)"), &[1.0, 1.0, 2.0, 1.0, 2.0, NAN, 24.0, 1.0, 6.0]);
    same(&[ev("fact(0)"), ev("fact(20)"), ev("fact(170)")], &[1.0, 2.432_902_008_176_64e18, 7.257_415_615_308_004e306]);
    assert!(ev("fact(-1)").is_nan());
    assert!(ev("fact(171)").is_nan());
    assert!(ev("fact(na())").is_nan());
}

#[test]
fn dates_match_origin_numbers() {
    // Origin 2016: date(2020,1,1) = 2458849, date(1900,1,1) = 2415020, time(12,30,0) = 0.52083333333333337,
    // time(25,0,0) = 1.0416666666666667 (not wrapped), year/month/day of 2021-03-15 06:00 = 2021 3 15,
    // weekday of Sunday 2021-03-14 = 0, of Monday 2021-03-15 = 1, of Wednesday 2020-01-01 = 3.
    assert_eq!(ev("date(2020,1,1)"), 2_458_849.0);
    assert_eq!(ev("date(1900,1,1)"), 2_415_020.0);
    assert_eq!(ev("date(1970,1,1)"), 2_440_587.0);
    near("time(12,30,0)", 0.520_833_333_333_333_4);
    near("time(25,0,0)", 1.041_666_666_666_666_7);
    assert_eq!(ev("year(date(2021,3,15)+time(6,0,0))"), 2021.0);
    assert_eq!(ev("month(date(2021,3,15))"), 3.0);
    assert_eq!(ev("day(date(2021,3,15))"), 15.0);
    assert_eq!(ev("weekday(date(2021,3,14))"), 0.0);
    assert_eq!(ev("weekday(date(2021,3,15))"), 1.0);
    assert_eq!(ev("weekday(date(2020,1,1))"), 3.0);
    // Leap days and the century rules.
    assert_eq!(ev("day(date(2000,2,29))"), 29.0);
    assert_eq!(ev("month(date(1900,3,1)-1)"), 2.0);
    assert_eq!(ev("day(date(1900,3,1)-1)"), 28.0);
    // Impossible dates are missing (Origin 2016 gave missing for month 14; for 30 February it gave an
    // unrelated number, which we do not copy).
    assert!(ev("date(2021,14,1)").is_nan());
    assert!(ev("date(2021,2,30)").is_nan());
    assert!(ev("year(na())").is_nan());
}

#[test]
fn today_and_now_come_from_the_host() {
    struct Clock;
    impl Host for Clock {
        fn variable(&self, _: &str) -> Option<f64> {
            None
        }
        fn cell(&self, _: &ColumnRef, _: usize) -> Option<f64> {
            None
        }
        fn now(&self) -> f64 {
            2_461_356.928_877_315
        }
    }
    let get = |s: &str| Expr::parse(s).unwrap().eval(&Clock).unwrap();
    assert_eq!(get("now()"), 2_461_356.928_877_315);
    assert_eq!(get("today()"), 2_461_356.0);
    // The system clock default is a plausible date number (after 2020, before 2100).
    let sys = ev("now()");
    assert!((2_458_849.0..2_488_070.0).contains(&sys), "{sys}");
    assert_eq!(ev("today()").fract(), 0.0);
}

#[test]
fn random_numbers_are_seeded_and_distributed() {
    let h = SimpleHost::new();
    let e = Expr::parse("rnd()").unwrap();
    let a = e.eval_rows_seeded(&h, 20_000, 7).unwrap();
    let b = e.eval_rows_seeded(&h, 20_000, 7).unwrap();
    let c = e.eval_rows_seeded(&h, 20_000, 8).unwrap();
    assert_eq!(a, b);
    assert_ne!(a, c);
    assert!(a.iter().all(|v| (0.0..1.0).contains(v)));
    let mean = a.iter().sum::<f64>() / a.len() as f64;
    assert!((mean - 0.5).abs() < 0.01, "{mean}");
    // Every row gets its own draw (Origin 2016 does as well).
    assert_ne!(a[0], a[1]);
    // Two calls in one formula are independent streams.
    let two = Expr::parse("rnd() - rnd()").unwrap().eval_rows_seeded(&h, 100, 1).unwrap();
    assert!(two.iter().any(|v| *v != 0.0));
    let g = Expr::parse("grnd()").unwrap().eval_rows_seeded(&h, 20_000, 3).unwrap();
    let m = g.iter().sum::<f64>() / g.len() as f64;
    let var = g.iter().map(|v| (v - m) * (v - m)).sum::<f64>() / (g.len() - 1) as f64;
    assert!(m.abs() < 0.03, "{m}");
    assert!((var - 1.0).abs() < 0.05, "{var}");
}

#[test]
fn lookup_cell_and_distance() {
    // Origin 2016: lookup(4,A,A) = 4, lookup(9,A,A*10) = 90 (exact match only, 2.5 and 0 give missing).
    let b: Vec<f64> = PROBE.iter().map(|v| v * 10.0).collect();
    let host = SimpleHost::new().short("A", &PROBE).short("B", &b).column(ColumnRef::Index(1), &PROBE);
    let run = |s: &str| Expr::parse(s).unwrap().eval(&host).unwrap();
    assert_eq!(run("lookup(9,col(A),col(B))"), 90.0);
    assert_eq!(run("lookup(1,col(A),col(B))"), 10.0);
    assert!(run("lookup(2.5,col(A),col(B))").is_nan());
    assert!(run("lookup(0,col(A),col(B))").is_nan());
    assert!(run("lookup(na(),col(A),col(B))").is_nan());
    // Origin 2016: cell(2,1) = 1, cell(1,1) = 3, cell(1.6,1) = 1 (rounds to row 2), cell(6,1), cell(10,1),
    // cell(0,1) and cell(1,0) all missing.
    assert_eq!(run("cell(2,1)"), 1.0);
    assert_eq!(run("cell(1,1)"), 3.0);
    assert_eq!(run("cell(1.6,1)"), 1.0);
    for s in ["cell(6,1)", "cell(10,1)", "cell(0,1)", "cell(1,0)", "cell(1,2)"] {
        assert!(run(s).is_nan(), "{s}");
    }
    // Origin 2016: distance(x1,y1,x2,y2) is the Euclidean distance; (A,1)-(A+3,5) is 5.
    same(&rows("distance(col(A),1,col(A)+3,5)"), &[5.0, 5.0, 5.0, 5.0, 5.0, NAN, 5.0, 5.0, 5.0]);
    assert_eq!(ev("distance(0,0,3,4)"), 5.0);
}
