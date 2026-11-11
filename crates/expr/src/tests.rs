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
