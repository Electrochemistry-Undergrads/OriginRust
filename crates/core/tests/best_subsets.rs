//! Best subset selection on worksheet columns (T347).

use sheaf_core::analysis::RowRange;
use sheaf_core::fitting::BestSubsetRequest;
use sheaf_core::{ColumnData, Designation, Project};

#[test]
fn every_subset_goes_to_a_new_sheet_with_the_preferred_ones_marked() {
    let mut p = Project::new();
    let book = p.add_workbook("Book1");
    let n = 30;
    let x1: Vec<f64> = (0..n).map(|i| f64::from(i) / 3.0).collect();
    let x2: Vec<f64> = (0..n).map(|i| (f64::from(i) * 1.7).sin()).collect();
    let x3: Vec<f64> = (0..n).map(|i| (f64::from(i) * 0.4).cos() * 2.0).collect();
    let y: Vec<f64> =
        (0..n as usize).map(|i| 3.0 + 1.5 * x1[i] + 0.8 * x3[i] + 0.02 * (i as f64 * 5.1).sin()).collect();
    let c1 = p.add_column(book, 0, Designation::X, ColumnData::numeric(x1)).unwrap();
    let c2 = p.add_column(book, 0, Designation::Y, ColumnData::numeric(x2)).unwrap();
    let c3 = p.add_column(book, 0, Designation::Y, ColumnData::numeric(x3)).unwrap();
    let cy = p.add_column(book, 0, Designation::Y, ColumnData::numeric(y)).unwrap();
    let sheets = p.workbook(book).unwrap().sheets.len();
    let o = p.best_subsets(&BestSubsetRequest { y: cy, x: vec![c1, c2, c3], rows: RowRange::All }).unwrap();
    assert_eq!(o.result.models.len(), 7);
    let book_now = p.workbook(book).unwrap();
    assert_eq!(book_now.sheets.len(), sheets + 1);
    let ws = &book_now.sheets[o.sheet];
    assert!(ws.name.starts_with("BestSubsets"), "{}", ws.name);
    let labels: Vec<&str> = ws.columns.iter().map(|c| c.long_name.as_str()).collect();
    assert_eq!(
        labels,
        [
            "Predictors",
            "Number of Predictors",
            "R-Square",
            "Adj. R-Square",
            "Mallows Cp",
            "AIC",
            "BIC",
            "Residual Sum of Squares",
            "Best by"
        ]
    );
    let names: Vec<String> = (0..7).map(|r| ws.columns[0].cell_text(r)).collect();
    let best_bic = names[o.result.best_bic].clone();
    assert!(best_bic.contains('A') && best_bic.contains('C') && !best_bic.contains('B'), "{best_bic}");
    assert!(ws.columns[8].cell_text(o.result.best_bic).contains("BIC"));
    assert!(o.message.contains("best by BIC"), "{}", o.message);
    assert!(p.best_subsets(&BestSubsetRequest { y: cy, x: vec![], rows: RowRange::All }).is_err());
}
