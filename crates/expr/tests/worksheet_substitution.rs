use sheaf_expr::{ColumnRef, Expr, Host, SimpleHost, Value, WorksheetRef};

#[test]
fn a_qualified_worksheet_cell_can_be_interpolated() {
    let values = [12.5, 3.25];
    let reference = ColumnRef::Scoped {
        book: Some("Src".into()),
        sheet: WorksheetRef::Name("Sheet1".into()),
        column: Box::new(ColumnRef::Short("A".into())),
    };
    let host = SimpleHost::new().column(reference, &values);
    let expression = Expr::parse(r#""cell=%([Src]Sheet1,A,2)""#).unwrap();
    assert_eq!(expression.eval_values(&host, 1).unwrap(), [Value::Text("cell=3.25".into())]);
}

const NUMBERS: [f64; 5] = [std::f64::consts::PI, 12.345678901234567, 1.2345678901234567e-10, f64::NAN, -42.0];

struct Cells;
impl Host for Cells {
    fn variable(&self, _: &str) -> Option<f64> {
        None
    }
    fn cell(&self, col: &ColumnRef, row: usize) -> Option<f64> {
        let ColumnRef::Scoped { column, .. } = col else { return None };
        match &**column {
            ColumnRef::Short(s) if s == "B" => (row < 3).then_some(f64::NAN),
            _ => NUMBERS.get(row).copied(),
        }
    }
    fn text_cell(&self, col: &ColumnRef, row: usize) -> Option<String> {
        let ColumnRef::Scoped { column, .. } = col else { return None };
        if matches!(&**column, ColumnRef::Short(s) if s == "B") {
            ["sample", "2+3", "µ"].get(row).map(|s| (*s).into())
        } else {
            None
        }
    }
    fn formatted_numeric_cell(&self, _: &ColumnRef, row: usize) -> Option<String> {
        NUMBERS.get(row).map(|x| format!("{x:.2}"))
    }
    fn column_parameter(&self, _: &ColumnRef, name: &str) -> Option<String> {
        (name == "i").then(|| "wrong metadata".into())
    }
}

// Scalar captures from Origin 2016, scratch/formula-worksheet-substitution/probe.lines.txt.
#[test]
fn origin_cell_substitution_formats_and_text_are_preserved() {
    for (arguments, expected) in [
        ("1,1", "3.14159265358979"),
        ("A,1", "3.14159265358979"),
        ("A,1,W", "3.14"),
        ("A,1,.2", "3.14"),
        ("A,1,*4", "3.142"),
        ("A,1,E*3", "3.14"),
        ("A,2", "12.3456789012346"),
        ("A,3", "1.23456789012346E-10"),
        ("A,4", "--"),
        ("A,5", "-42"),
        ("B,1", "sample"),
        ("B,1,W", "sample"),
        ("B,1,.2", "sample"),
        ("B,2", "2+3"),
        ("B,3", "µ"),
        ("B,4", ""),
        ("A,50", ""),
        ("A,0", "###"),
        ("A,-1", "###"),
        ("A,1+1", "12.3456789012346"),
        ("A,1.6", "12.3456789012346"),
        ("A,.5", "###"),
        ("A,.6", "###"),
        ("A,.99", "###"),
        ("A,.999999", "###"),
        ("A,1.499999", "3.14159265358979"),
        ("A,1.5", "12.3456789012346"),
        ("A,2.5", "1.23456789012346E-10"),
        ("A,3.5", "--"),
        ("A,4.5", "-42"),
        (r#"A,1,".2""#, "3.14"),
        (r#"A,1,"""#, "3.14159265358979"),
        (r#""Voltage V",1"#, "3.14159265358979"),
    ] {
        let source = format!("\"%([SubSrc]Sheet1,{arguments})\"");
        let expr = Expr::parse(&source).unwrap();
        assert_eq!(expr.eval_values(&Cells, 1).unwrap(), [Value::Text(expected.into())], "{source}");
    }
}

#[test]
fn origin_default_precision_handles_large_small_and_boundary_values() {
    for (number, expected) in [
        (0.0, "0"),
        (12.5, "12.5"),
        (1e14, "100000000000000"),
        (1e15, "1000000000000000"),
        (1.2345678901234567e15, "1.23456789012346E15"),
        (0.0009999999999999999, "1E-3"),
        (0.001, "0.001"),
        (0.0001, "1E-4"),
        (1e-100, "1E-100"),
        (1e100, "1E100"),
    ] {
        let r = ColumnRef::Scoped {
            book: Some("Src".into()),
            sheet: WorksheetRef::Index(1),
            column: Box::new(ColumnRef::Index(1)),
        };
        let values = [number];
        let host = SimpleHost::new().column(r, &values);
        let expr = Expr::parse(r#""%([Src]1,1,1)""#).unwrap();
        assert_eq!(expr.eval_values(&host, 1).unwrap(), [Value::Text(expected.into())], "{number}");
    }
}

#[test]
fn rows_remain_numeric_when_a_parameter_has_the_same_name() {
    let expr = Expr::parse(r#""%([SubSrc]Sheet1,A,i)""#).unwrap();
    assert_eq!(
        expr.eval_values(&Cells, 2).unwrap(),
        [Value::Text("3.14159265358979".into()), Value::Text("12.3456789012346".into())]
    );
    assert!(expr.label_columns().is_empty());
    assert_eq!(expr.data_columns().len(), 1);
}

#[test]
fn standalone_cells_convert_numeric_text_without_executing_generated_expressions() {
    assert!((Expr::parse("%([SubSrc]Sheet1,A,1,.2)+1").unwrap().eval(&Cells).unwrap() - 4.14).abs() < 1e-12);
    assert!(Expr::parse("%([SubSrc]Sheet1,B,2)").unwrap().eval(&Cells).unwrap().is_nan());
    assert_eq!(Expr::parse("%([SubSrc]Sheet1,B,50)").unwrap().eval(&Cells).unwrap(), 0.0);
    assert_eq!(Expr::parse("10 % (3)").unwrap().eval(&Cells).unwrap(), 1.0);
}

#[test]
fn malformed_static_references_formats_and_rows_report_errors() {
    for source in [
        r#""%([Book]Sheet,A)""#,
        r#""%([Book]Sheet,A,)""#,
        r#""%([Book]Sheet,0,1)""#,
        r#""%([Book]Sheet,1.5,1)""#,
        r#""%([Book]0,A,1)""#,
        r#""%([Book]Sheet,A,1,)""#,
        r#""%([Book]Sheet,A,1,.2,extra)""#,
        r#""%([Book]Sheet,A,1,text(2))""#,
    ] {
        assert!(Expr::parse(source).is_err(), "{source}");
    }
    let source = r#""µ %([Absent]Sheet,A,1)""#;
    let error = Expr::parse(source).unwrap().eval_values(&SimpleHost::new(), 1).unwrap_err();
    assert_eq!(error.position(), source.find('['));
}
