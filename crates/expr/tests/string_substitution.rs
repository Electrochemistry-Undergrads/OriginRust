use sheaf_expr::{ColumnLabel, ColumnRef, Expr, Host, SimpleHost, Value};

#[test]
fn substitution_preserves_literals_unicode_quoted_arguments_and_parentheses_in_strings() {
    for (source, expected) in [
        (r#""before %(upper("µ")) %(text(2.5,".2")) after""#, "before Μ 2.50 after"),
        (r#""left %("a)b(c") right""#, "left a)b(c right"),
        (r#""a%%(col(A)[L]$)""#, "a%(col(A)[L]$)"),
    ] {
        assert_eq!(
            Expr::parse(source).unwrap().eval_values(&SimpleHost::new(), 1).unwrap(),
            [Value::Text(expected.into())]
        );
    }
}

#[test]
fn standalone_substitution_converts_numbers_and_keeps_remainder_an_operator() {
    assert_eq!(Expr::parse(r#"%(trim("12.5")) + 1"#).unwrap().eval(&SimpleHost::new()).unwrap(), 13.5);
    assert_eq!(Expr::parse(r#"10 % (3) + %("4")"#).unwrap().eval(&SimpleHost::new()).unwrap(), 5.0);
    assert!(Expr::parse(r#"%("2+3")"#).unwrap().eval(&SimpleHost::new()).unwrap().is_nan());
}

#[test]
fn control_keywords_are_inserted_into_strings() {
    let expr = Expr::parse(r#""a%(TAB)b%(CRLF)c""#).unwrap();
    assert_eq!(expr.eval_values(&SimpleHost::new(), 1).unwrap(), [Value::Text("a\tb\r\nc".into())]);
}

struct Labels;
impl Host for Labels {
    fn variable(&self, _: &str) -> Option<f64> {
        None
    }
    fn cell(&self, _: &ColumnRef, _: usize) -> Option<f64> {
        None
    }
    fn has_column(&self, col: &ColumnRef) -> bool {
        matches!(col, ColumnRef::Short(s) if s == "A")
    }
    fn column_label(&self, _: &ColumnRef, label: ColumnLabel) -> Option<String> {
        (label == ColumnLabel::Units).then(|| "mV".into())
    }
    fn column_parameter(&self, _: &ColumnRef, name: &str) -> Option<String> {
        (name == "Bath Temp").then(|| "37".into())
    }
}

#[test]
fn interpolated_labels_and_space_containing_parameters_keep_metadata_dependencies() {
    let expr = Expr::parse(r#""%(col(A)[Bath Temp]$) %(col(A)[U]$)""#).unwrap();
    let resolved = expr.resolve_parameters(&Labels);
    assert!(resolved.data_columns().is_empty());
    assert_eq!(resolved.label_columns(), [ColumnRef::Short("A".into())]);
    assert_eq!(expr.eval_values(&Labels, 1).unwrap(), [Value::Text("37 mV".into())]);
}

#[test]
fn errors_inside_substitution_report_offsets_in_the_original_utf8_formula() {
    let source = r#""µ %(col(Nope)$)""#;
    let error = Expr::parse(source).unwrap().eval_values(&SimpleHost::new(), 1).unwrap_err();
    assert_eq!(error.position(), source.find("col("));
    assert!(error.to_string().contains("unknown column col(Nope)"));
}

#[test]
fn malformed_and_legacy_substitutions_fail_with_an_error() {
    for source in [
        r#""x %(col(A)$""#,
        r#""x %("unterminated)"#,
        r#""%()""#,
        r#"%()"#,
        r#""%([Book]Sheet,A,1)""#,
        r#""%(session$)""#,
    ] {
        assert!(Expr::parse(source).is_err(), "{source}");
    }
}

#[test]
fn nested_substitutions_are_bounded_and_plain_strings_still_work() {
    let mut source = r#""x""#.to_string();
    for _ in 0..5 {
        source = format!("\"%({source})\"");
    }
    assert_eq!(Expr::parse(&source).unwrap().eval_values(&SimpleHost::new(), 1).unwrap(), [Value::Text("x".into())]);
    for _ in 0..100 {
        source = format!("\"%({source})\"");
    }
    let error = Expr::parse(&source).unwrap_err().to_string();
    assert!(error.contains("deeply"), "{error}");
    assert_eq!(
        Expr::parse(r#""µ 50% (raw)""#).unwrap().eval_values(&SimpleHost::new(), 1).unwrap(),
        [Value::Text("µ 50% (raw)".into())]
    );
}
