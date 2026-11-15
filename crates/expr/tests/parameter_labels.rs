//! Parameter-label observations from Origin, 2026-11-14; only synthetic metadata and cells.
use sheaf_expr::{ColumnRef, Expr, Host, Value};

struct Labels;

impl Host for Labels {
    fn variable(&self, name: &str) -> Option<f64> {
        match name {
            "Temperature" | "j" => Some(2.0),
            _ => None,
        }
    }

    fn cell(&self, col: &ColumnRef, row: usize) -> Option<f64> {
        self.has_column(col).then(|| [1.0, 2.0, 3.0].get(row).copied()).flatten()
    }

    fn has_column(&self, col: &ColumnRef) -> bool {
        matches!(col, ColumnRef::Short(s) if s == "A" || s == "B") || matches!(col, ColumnRef::Index(1))
    }

    fn column_parameter(&self, _col: &ColumnRef, name: &str) -> Option<String> {
        match name.to_ascii_lowercase().as_str() {
            "temperature" | "d1" => Some("2.5".into()),
            "sample" => Some("calibrated".into()),
            "bath temp" | "group 1" => Some("37".into()),
            "blank" => Some(String::new()),
            _ => None,
        }
    }
}

#[test]
fn parameter_text_is_scalar_and_value_converts_it_for_calculation() {
    for source in ["col(A)[Temperature]$", "col(A)[temperature]$", "wcol(1)[D1]$"] {
        assert_eq!(Expr::parse(source).unwrap().eval_values(&Labels, 3).unwrap(), vec![Value::Text("2.5".into()); 3]);
    }
    assert_eq!(
        Expr::parse("value(col(A)[Temperature]$)*col(A)").unwrap().eval_rows(&Labels, 3).unwrap(),
        [2.5, 5.0, 7.5]
    );
    assert!(Expr::parse("col(A)[Temperature]").unwrap().eval(&Labels).unwrap().is_nan());
    assert!(Expr::parse("col(A)[Temperature]").unwrap().compile(&Labels).unwrap().eval_row(0).is_nan());
    assert!(Expr::parse("value(col(A)[Sample]$)").unwrap().eval(&Labels).unwrap().is_nan());
    assert!(
        Expr::parse("col(A)[U]").unwrap().eval(&Labels).unwrap().is_nan(),
        "empty built-in headers retain numeric missing values"
    );
    for name in ["Missing", "Blank"] {
        assert_eq!(Expr::parse(&format!("value(col(A)[{name}]$)")).unwrap().eval(&Labels).unwrap(), 0.0);
    }
}

#[test]
fn parameter_names_can_contain_spaces_and_numbers() {
    for source in ["col(A)[Bath Temp]$", "col(A)[Group 1]$"] {
        assert_eq!(Expr::parse(source).unwrap().eval_values(&Labels, 2).unwrap(), vec![Value::Text("37".into()); 2]);
    }
    assert_eq!(Expr::parse("value(col(A)[Bath Temp]$)+i").unwrap().eval_rows(&Labels, 3).unwrap(), [38.0, 39.0, 40.0]);
    assert!(Expr::parse("col(A)[0 1]$").is_err(), "invalid numeric row syntax stays invalid");
}

#[test]
fn labels_win_over_variables_but_known_row_indices_keep_working() {
    // Temperature is both a variable and a label; j is only a variable.
    assert_eq!(
        Expr::parse("col(A)[Temperature]$").unwrap().eval_values(&Labels, 1).unwrap(),
        [Value::Text("2.5".into())]
    );
    assert_eq!(Expr::parse("col(A)[j]").unwrap().eval_rows(&Labels, 3).unwrap(), [2.0; 3]);
    assert_eq!(Expr::parse("col(A)[j]$").unwrap().eval_values(&Labels, 1).unwrap(), [Value::Text("2".into())]);
    assert_eq!(Expr::parse("col(A)[Temperature+0]").unwrap().eval(&Labels).unwrap(), 2.0);
    assert_eq!(Expr::parse("col(A)[i]").unwrap().eval_rows(&Labels, 3).unwrap(), [1.0, 2.0, 3.0]);
}

#[test]
fn resolved_label_dependencies_exclude_cells_and_unknown_columns_still_fail() {
    let expr = Expr::parse("value(col(A)[Temperature]$)*col(B)").unwrap().resolve_parameters(&Labels);
    assert!(expr.variables().is_empty());
    assert_eq!(expr.label_columns(), [ColumnRef::Short("A".into())]);
    assert_eq!(expr.data_columns(), [ColumnRef::Short("B".into())]);
    let rows = Expr::parse("col(A)[j]$").unwrap().resolve_parameters(&Labels);
    assert_eq!(rows.variables(), ["j"]);
    assert_eq!(rows.data_columns(), [ColumnRef::Short("A".into())]);
    assert!(rows.label_columns().is_empty());
    for source in ["col(Q)[Sample]$", "col(Q)[Temperature]"] {
        assert!(
            Expr::parse(source).unwrap().eval_values(&Labels, 1).unwrap_err().to_string().contains("unknown column")
        );
    }
    assert_eq!(Expr::parse("col(A)[Missing]$").unwrap().eval_values(&Labels, 1).unwrap(), [Value::Text(String::new())]);
}

#[test]
fn resolving_parameters_keeps_deep_expressions_within_the_stack_limit() {
    let source = format!("value(col(A)[Temperature]$){}", "+0".repeat(600));
    let expr = Expr::parse(&source).unwrap();
    assert_eq!(expr.eval(&Labels).unwrap(), 2.5);
}
