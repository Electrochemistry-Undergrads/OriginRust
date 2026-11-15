use sheaf_expr::{Expr, SimpleHost, Value};

fn text(source: &str) -> String {
    match Expr::parse(source).unwrap().eval_values(&SimpleHost::new(), 1).unwrap().remove(0) {
        Value::Text(value) => value,
        value => panic!("{source}: expected text, got {value:?}"),
    }
}

// Origin 2016, @SD=14, scratch/formula-format-defaults/probe.lines.txt.
#[test]
fn an_empty_format_retains_the_default_fourteen_significant_digits() {
    for function in ["text", "format"] {
        for (value, expected) in [
            ("3.141592653589793", "3.1415926535898"),
            ("12.345678901234567", "12.345678901235"),
            ("1234.5678901234567", "1234.5678901235"),
            ("1234567.8901234567", "1234567.8901235"),
            ("1000000", "1000000"),
            ("10000000", "10000000"),
            ("0.00012345678901234567", "1.2345678901235E-4"),
            ("0.0012345678901234567", "0.0012345678901235"),
            ("1.2345678901234567e-10", "1.2345678901235E-10"),
            ("1.2345678901234567e15", "1.2345678901235E15"),
            ("-42", "-42"),
            ("-12.345678901234567", "-12.345678901235"),
            ("0", "0"),
            ("na()", "--"),
            ("1e-100", "1E-100"),
            ("1e100", "1E100"),
            // Independent controls at notation and rounding boundaries.
            ("1e13", "10000000000000"),
            ("1e14", "100000000000000"),
            ("1.2345678901234567e14", "1.2345678901235E14"),
            ("9.99999999999995e13", "100000000000000"),
            ("0.0009999999999999999", "1E-3"),
            ("-0", "0"),
            ("1.2300000000000", "1.23"),
            ("9.99999999999999", "10"),
        ] {
            let source = format!(r#"{function}({value},"")$"#);
            assert_eq!(text(&source), expected, "{source}");
        }
    }
}

#[test]
fn omitted_global_and_explicit_formats_remain_distinct_from_empty() {
    for function in ["text", "format"] {
        for (arguments, expected) in [
            ("12.345678901234567", "12.34568"),
            (r#"12.345678901234567,"*""#, "12.34568"),
            (r#"12.345678901234567,".2""#, "12.35"),
            (r#"12.345678901234567,"*3""#, "12.3"),
            (r#"42,"*3""#, "42.0"),
        ] {
            let source = format!("{function}({arguments})$");
            assert_eq!(text(&source), expected, "{source}");
        }
    }
}

#[test]
fn a_row_expression_can_choose_an_empty_format() {
    let values = [12.345678901234567, 12.345678901234567];
    let host = SimpleHost::new().short("A", &values);
    for function in ["text", "format"] {
        let source = format!(r#"{function}(col(A),i==1 ? "" : "*3")$"#);
        assert_eq!(
            Expr::parse(&source).unwrap().eval_values(&host, 2).unwrap(),
            [Value::Text("12.345678901235".into()), Value::Text("12.3".into())]
        );
    }
}
