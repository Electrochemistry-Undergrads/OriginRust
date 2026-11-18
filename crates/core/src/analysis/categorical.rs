//! Cross tabulation: a statistics tool that works on a pair of columns. The numerics are in `sheaf_calc`
//! (`Crosstab`); this module reads the columns and writes the result sheet. What the tool reports is
//! described in `docs/spec/statistics.md`, section 15.

use super::statistics::{Output, check_alpha, label_column, level_text, num, number_column};
use super::*;
use sheaf_calc::Crosstab;

/// Levels in the order a table lists them: numbers from small to large when every level is a number,
/// otherwise text in character order ignoring case (A before a).
fn ordered_levels(mut levels: Vec<String>) -> Vec<String> {
    let numbers: Option<Vec<f64>> = levels.iter().map(|l| l.parse::<f64>().ok()).collect();
    if numbers.is_some() {
        levels.sort_by(|a, b| a.parse::<f64>().unwrap_or(0.0).total_cmp(&b.parse::<f64>().unwrap_or(0.0)));
    } else {
        levels.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()).then_with(|| a.cmp(b)));
    }
    levels
}

/// Most distinct levels one variable of a cross tabulation may have. A column of numbers or identifiers
/// would otherwise make a table with millions of cells.
const MAX_LEVELS: usize = 200;

pub(super) fn crosstab(
    spec: &CrosstabSpec,
    sheet: &Worksheet,
    columns: &[ColumnId],
    rows: RowRange,
) -> Result<Output, AnalysisError> {
    check_alpha(spec.alpha)?;
    if columns.len() != 2 {
        return Err(AnalysisError::BadOption(format!(
            "a cross tabulation needs exactly two columns (the row variable and the column variable), {} were chosen",
            columns.len()
        )));
    }
    let column = |id: ColumnId| sheet.column(id).ok_or(AnalysisError::NoSuchColumn(id));
    let (cr, cc) = (column(columns[0])?, column(columns[1])?);
    let len = cr.data.len().max(cc.data.len());
    if len == 0 {
        return Err(AnalysisError::Empty(cr.short_name.clone()));
    }
    let (first, last) = match rows {
        RowRange::All => (0, len - 1),
        RowRange::Rows(..) => resolve_rows(rows, &cr.short_name, &[], len)?,
        RowRange::XBetween(..) => {
            return Err(AnalysisError::BadOption(
                "the cross tabulation has no X column to limit; choose rows with a row range".into(),
            ));
        }
    };
    let mut pairs: Vec<(String, String)> = Vec::new();
    let mut left_out = 0usize;
    for row in first..=last.min(len - 1) {
        // Rows hidden by the sheet filter and masked cells are excluded, as in every other tool.
        if !sheet.row_shown(row) {
            continue;
        }
        let level =
            |c: &Column| if c.active_mask().is_some_and(|m| m.contains(&row)) { None } else { level_text(c, row) };
        match (level(cr), level(cc)) {
            (Some(a), Some(b)) => pairs.push((a, b)),
            (None, None) => {}
            _ => left_out += 1,
        }
    }
    if pairs.is_empty() {
        return Err(AnalysisError::BadOption("no row has a level in both columns".to_string()));
    }
    let levels_r = ordered_levels(
        pairs.iter().map(|p| p.0.clone()).collect::<std::collections::BTreeSet<_>>().into_iter().collect(),
    );
    let levels_c = ordered_levels(
        pairs.iter().map(|p| p.1.clone()).collect::<std::collections::BTreeSet<_>>().into_iter().collect(),
    );
    for (levels, c) in [(&levels_r, cr), (&levels_c, cc)] {
        if levels.len() > MAX_LEVELS {
            return Err(AnalysisError::BadOption(format!(
                "column {} has {} distinct levels, too many for a categorical variable (at most {MAX_LEVELS}); \
                 choose a column of categories",
                c.display_name(),
                levels.len()
            )));
        }
    }
    let mut counts = vec![vec![0usize; levels_c.len()]; levels_r.len()];
    for (a, b) in &pairs {
        let i = levels_r.iter().position(|l| l == a).expect("collected above");
        let j = levels_c.iter().position(|l| l == b).expect("collected above");
        counts[i][j] += 1;
    }
    let t = Crosstab::new(counts).map_err(|e| AnalysisError::BadOption(e.to_string()))?;
    let (name_r, name_c) = (cr.display_name().to_string(), cc.display_name().to_string());
    let (nr, nc) = (t.rows(), t.cols());

    // Block 1: the table of counts with its totals.
    let mut on_sheet = vec![label_column(
        &format!("{name_r} \\ {name_c}"),
        levels_r.iter().cloned().chain(std::iter::once("Total".to_string())).collect(),
    )];
    for (j, level) in levels_c.iter().enumerate().take(nc) {
        let mut v: Vec<f64> = (0..nr).map(|i| t.counts[i][j] as f64).collect();
        v.push(t.col_totals[j] as f64);
        let mut col = number_column(level, "", v);
        col.comments = name_c.clone();
        on_sheet.push(col);
    }
    let mut totals: Vec<f64> = t.row_totals.iter().map(|&x| x as f64).collect();
    totals.push(t.total as f64);
    on_sheet.push(number_column("Total", "", totals));

    // Block 2: one row per cell.
    let cells: Vec<(usize, usize)> = (0..nr).flat_map(|i| (0..nc).map(move |j| (i, j))).collect();
    on_sheet.push(label_column("Row level", cells.iter().map(|&(i, _)| levels_r[i].clone()).collect()));
    on_sheet.push(label_column("Column level", cells.iter().map(|&(_, j)| levels_c[j].clone()).collect()));
    on_sheet.push(number_column("Count", "", cells.iter().map(|&(i, j)| t.counts[i][j] as f64).collect()));
    if spec.expected {
        on_sheet.push(number_column("Expected", "", cells.iter().map(|&(i, j)| t.expected[i][j]).collect()));
    }
    if spec.row_percent {
        on_sheet.push(number_column("Row %", "%", cells.iter().map(|&(i, j)| t.row_percent(i, j)).collect()));
    }
    if spec.column_percent {
        on_sheet.push(number_column("Column %", "%", cells.iter().map(|&(i, j)| t.col_percent(i, j)).collect()));
    }
    if spec.total_percent {
        on_sheet.push(number_column("Total %", "%", cells.iter().map(|&(i, j)| t.total_percent(i, j)).collect()));
    }

    // Block 3: the tests.
    let (pearson, lr) = (t.pearson(), t.likelihood_ratio());
    let mut names: Vec<String> = ["Pearson chi-square", "DF", "Prob>Pearson chi-square"].map(str::to_string).to_vec();
    let mut values = vec![pearson.statistic, pearson.df, pearson.p];
    names.extend(["Likelihood ratio chi-square", "DF", "Prob>Likelihood ratio chi-square"].map(str::to_string));
    values.extend([lr.statistic, lr.df, lr.p]);
    let yates = t.yates();
    if let Some(y) = yates {
        names.extend(["Yates chi-square", "Prob>Yates chi-square"].map(str::to_string));
        values.extend([y.statistic, y.p]);
    }
    let fisher = t.fisher();
    if let Some(f) = fisher {
        names.extend(
            ["Fisher exact Prob (two-sided)", "Fisher exact Prob (left)", "Fisher exact Prob (right)", "Odds ratio"]
                .map(str::to_string),
        );
        values.extend([f.p_two_sided, f.p_less, f.p_greater, f.odds_ratio]);
    }
    on_sheet.push(label_column("Statistic", names));
    on_sheet.push(number_column("Value", "", values));

    let mut table = ResultTable::new(&[
        "N",
        "Rows",
        "Columns",
        "Pearson chi-square",
        "DF",
        "Prob>Pearson",
        "Likelihood ratio chi-square",
        "Prob>Likelihood ratio",
        "Fisher Prob (two-sided)",
    ]);
    table.push_row(
        format!("{name_r} by {name_c}"),
        vec![
            t.total as f64,
            nr as f64,
            nc as f64,
            pearson.statistic,
            pearson.df,
            pearson.p,
            lr.statistic,
            lr.p,
            fisher.map_or(f64::NAN, |f| f.p_two_sided),
        ],
        vec![String::new(); 9],
    );

    let verdict = |p: f64| if p < spec.alpha { "associated" } else { "not shown to be associated" };
    let mut notes = vec![format!(
        "Cross tabulation of {name_r} ({nr} levels) by {name_c} ({nc} levels), {} rows: Pearson chi-square({}) = {}, p = {}; likelihood ratio chi-square({}) = {}, p = {}; the variables are {} at the {} level.",
        t.total,
        num(pearson.df),
        num(pearson.statistic),
        num(pearson.p),
        num(lr.df),
        num(lr.statistic),
        num(lr.p),
        verdict(pearson.p),
        num(spec.alpha)
    )];
    if let Some(f) = fisher {
        notes.push(format!(
            "Fisher's exact test: two-sided p = {}, odds ratio {}.",
            num(f.p_two_sided),
            num(f.odds_ratio)
        ));
    }
    let small = t.cells_expected_below_5();
    if small > 0 {
        notes.push(format!(
            "{small} of {} cells have an expected count below 5: the chi-square approximation is rough{}.",
            nr * nc,
            if fisher.is_some() { "; the exact test is the safer one" } else { "" }
        ));
    }
    if left_out > 0 {
        notes.push(format!("{left_out} rows with a level in only one of the two columns were left out."));
    }
    Ok(Output { table: Some(table), on_sheet, notes, plots: Vec::new(), comparison_pairs: Vec::new() })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(v: &[&str]) -> ColumnData {
        ColumnData::text(v.iter().map(|s| s.to_string()).collect())
    }

    fn project(columns: Vec<ColumnData>) -> (Project, Vec<ColumnId>) {
        let mut p = Project::new();
        let book = p.add_workbook("tab");
        let ids = columns.into_iter().map(|d| p.add_column(book, 0, Designation::Y, d).unwrap()).collect();
        (p, ids)
    }

    fn request(columns: &[ColumnId], tool: Tool) -> Request {
        Request { columns: columns.to_vec(), rows: RowRange::All, tool }
    }

    fn column(p: &Project, r: &Report, name: &str) -> Column {
        let ws = &p.workbook(r.book).unwrap().sheets[r.new_sheet.unwrap()];
        ws.columns.iter().find(|c| c.long_name == name).unwrap_or_else(|| panic!("no column {name}")).clone()
    }

    #[test]
    fn crosstab_of_two_text_columns() {
        // 2 x 2: treated/control against better/same; one row has no second level.
        let a = ["treated", "treated", "treated", "control", "control", "control", "control", "treated", "control"];
        let b = ["better", "better", "same", "same", "same", "better", "same", "better", ""];
        let (mut p, ids) = project(vec![text(&a), text(&b)]);
        let r = p.analyze(&request(&ids, Tool::Crosstab(CrosstabSpec::default()))).unwrap();
        assert!(r.message.contains("Cross tabulation") && r.message.contains("Crosstab1"), "{}", r.message);
        // Rows control, treated; columns better, same.
        let better = column(&p, &r, "better");
        assert_eq!(better.numeric().unwrap(), [1.0, 3.0, 4.0]);
        assert_eq!(column(&p, &r, "same").numeric().unwrap(), [3.0, 1.0, 4.0]);
        assert_eq!(column(&p, &r, "Total").numeric().unwrap(), [4.0, 4.0, 8.0]);
        assert_eq!(column(&p, &r, "Row %").numeric().unwrap(), [25.0, 75.0, 75.0, 25.0]);
        assert_eq!(column(&p, &r, "Total %").numeric().unwrap(), [12.5, 37.5, 37.5, 12.5]);
        let stats = column(&p, &r, "Value").numeric().unwrap().to_vec();
        // Pearson chi-square 2, df 1 for this table, as for the tea tasting table.
        assert!((stats[0] - 2.0).abs() < 1e-12 && stats[1] == 1.0);
        let fisher = column(&p, &r, "Statistic")
            .text()
            .unwrap()
            .iter()
            .position(|s| s.starts_with("Fisher exact Prob (two"))
            .unwrap();
        assert!((stats[fisher] - 34.0 / 70.0).abs() < 1e-12);
        assert!(r.notes.iter().any(|n| n.contains("1 rows with a level in only one")), "{:?}", r.notes);
        assert!(r.notes.iter().any(|n| n.contains("expected count below 5")), "{:?}", r.notes);
    }

    #[test]
    fn crosstab_levels_are_ordered_and_numbers_may_be_levels() {
        let a = ColumnData::numeric(vec![10.0, 2.0, 2.0, 10.0, 2.0, 10.0]);
        let (mut p, ids) = project(vec![a, text(&["b", "a", "B", "a", "b", "A"])]);
        let r =
            p.analyze(&request(&ids, Tool::Crosstab(CrosstabSpec { expected: false, ..Default::default() }))).unwrap();
        let ws = &p.workbook(r.book).unwrap().sheets[r.new_sheet.unwrap()];
        // Numbers from small to large (2 before 10), text ignoring case.
        assert_eq!(ws.columns[0].text().unwrap(), ["2", "10", "Total"]);
        let heads: Vec<&str> = ws.columns[1..5].iter().map(|c| c.long_name.as_str()).collect();
        assert_eq!(heads, ["A", "a", "B", "b"]);
        assert!(ws.columns.iter().all(|c| c.long_name != "Expected"));
    }

    #[test]
    fn crosstab_needs_two_columns_and_two_levels() {
        let (mut p, ids) = project(vec![text(&["a", "b"]), text(&["x", "y"]), text(&["u", "v"])]);
        assert!(p.analyze(&request(&ids, Tool::Crosstab(CrosstabSpec::default()))).is_err());
        let (mut p, ids) = project(vec![text(&["a", "a"]), text(&["x", "y"])]);
        let err = p.analyze(&request(&ids, Tool::Crosstab(CrosstabSpec::default()))).unwrap_err();
        assert!(err.to_string().contains("at least two levels"), "{err}");
    }

    #[test]
    fn crosstab_leaves_out_filtered_rows_and_masked_cells() {
        let mut p = Project::new();
        let book = p.add_workbook("tab");
        let a = ["x", "x", "y", "y", "x", "y"];
        let b = ["u", "v", "u", "v", "u", "u"];
        let ca = p.add_column(book, 0, Designation::Y, text(&a)).unwrap();
        let cb = p.add_column(book, 0, Designation::Y, text(&b)).unwrap();
        let keep =
            p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0, 1.0, 1.0, 1.0, 0.0, 0.0])).unwrap();
        let n = |p: &mut Project| {
            let r = p.analyze(&request(&[ca, cb], Tool::Crosstab(CrosstabSpec::default()))).unwrap();
            let t = r.table.unwrap();
            let at = t.headers.iter().position(|h| h == "N").unwrap();
            t.rows[0].1[at]
        };
        assert_eq!(n(&mut p), 6.0);
        // Rows 4 and 5 are hidden by the filter.
        let short = p.column(keep).unwrap().short_name.clone();
        assert_eq!(p.set_row_filter(book, 0, &format!("col({short}) > 0.5")).unwrap(), 4);
        assert_eq!(n(&mut p), 4.0);
        // A masked cell takes its row out of the table as well.
        p.column_mut(cb).unwrap().mask.insert(0);
        let r = p.analyze(&request(&[ca, cb], Tool::Crosstab(CrosstabSpec::default()))).unwrap();
        assert_eq!(r.table.unwrap().rows[0].1[0], 3.0);
        assert!(r.notes.iter().any(|n| n.contains("1 rows with a level in only one")), "{:?}", r.notes);
    }

    #[test]
    fn crosstab_refuses_a_column_with_too_many_levels() {
        let ids: Vec<f64> = (0..(MAX_LEVELS + 1) as u32).map(f64::from).collect();
        let few: Vec<&str> = (0..=MAX_LEVELS).map(|i| if i % 2 == 0 { "a" } else { "b" }).collect();
        let (mut p, cols) = project(vec![ColumnData::numeric(ids), text(&few)]);
        let err = p.analyze(&request(&cols, Tool::Crosstab(CrosstabSpec::default()))).unwrap_err();
        assert!(err.to_string().contains("too many for a categorical variable"), "{err}");
        // Exactly the limit is accepted.
        let ids: Vec<f64> = (0..MAX_LEVELS as u32).map(f64::from).collect();
        let few: Vec<&str> = (0..MAX_LEVELS).map(|i| if i % 2 == 0 { "a" } else { "b" }).collect();
        let (mut p, cols) = project(vec![ColumnData::numeric(ids), text(&few)]);
        assert!(p.analyze(&request(&cols, Tool::Crosstab(CrosstabSpec::default()))).is_ok());
    }
}
