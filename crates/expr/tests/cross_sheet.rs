use sheaf_expr::{ColumnRef, Expr, SimpleHost, WorksheetRef};

fn scoped(book: Option<&str>, sheet: WorksheetRef, column: ColumnRef) -> ColumnRef {
    ColumnRef::Scoped { book: book.map(str::to_owned), sheet, column: Box::new(column) }
}

#[test]
fn qualified_columns_keep_their_scope_in_dependencies_and_evaluation() {
    let remote = scoped(Some("Src"), WorksheetRef::Name("Other".into()), ColumnRef::Short("A".into()));
    let local = scoped(None, WorksheetRef::Index(1), ColumnRef::Index(2));
    let expr = Expr::parse("total([Src]Other!col(A)[2:3]) + 1!wcol(2)[i]").unwrap();
    assert_eq!(expr.data_columns(), [remote.clone(), local.clone()]);
    assert!(expr.variables().contains(&"i".into()));
    let a = [10.0, 20.0, 30.0, 40.0];
    let b = [1.0, 2.0, 3.0];
    let host = SimpleHost::new().column(remote, &a).column(local, &b);
    assert_eq!(expr.eval_rows(&host, 3).unwrap(), [51.0, 52.0, 53.0]);
}

#[test]
fn quoted_sheet_and_column_names_and_labels_keep_their_scope() {
    let expr = Expr::parse("[\"My Book\"]\"Raw Data\"!col(\"Current µA\")[U]$").unwrap();
    let column = scoped(Some("My Book"), WorksheetRef::Name("Raw Data".into()), ColumnRef::Long("Current µA".into()));
    assert_eq!(expr.label_columns(), [column]);
    assert!(expr.data_columns().is_empty());
    let expr = Expr::parse("\"Raw Data\"!col(A)$").unwrap();
    assert_eq!(expr.columns(), [scoped(None, WorksheetRef::Name("Raw Data".into()), ColumnRef::Short("A".into()))]);
}

#[test]
fn qualified_vector_functions_and_unary_operators_remain_unambiguous() {
    let expr = Expr::parse("-[Src]2!col(A) + sum([Src]2!col(A))").unwrap();
    let column = scoped(Some("Src"), WorksheetRef::Index(2), ColumnRef::Short("A".into()));
    let a = [2.0, 4.0, 8.0];
    let host = SimpleHost::new().column(column, &a);
    assert_eq!(expr.eval_rows(&host, 3).unwrap(), [0.0, 2.0, 6.0]);
    assert_eq!(Expr::parse("!0 + (1!=2)").unwrap().eval(&SimpleHost::new()).unwrap(), 2.0);
}

#[test]
fn malformed_qualifiers_are_refused_and_unknown_qualified_columns_are_errors() {
    for source in [
        "[]Sheet1!col(A)",
        "[Src]!col(A)",
        "[Src]0!col(A)",
        "0!col(A)",
        "1.5!col(A)",
        "[Src]Sheet1col(A)",
        "[Src]Sheet1!sqrt(1)",
        "[Src]Sheet1!col(A)col(B)",
        "[Src]Sheet1!col(A",
    ] {
        assert!(Expr::parse(source).is_err(), "{source}");
    }
    let error = Expr::parse("[Missing]Sheet1!col(A)").unwrap().eval(&SimpleHost::new()).unwrap_err().to_string();
    assert!(error.contains("unknown column") && error.contains("[Missing]Sheet1!col(A)"), "{error}");
}
