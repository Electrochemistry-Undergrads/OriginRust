//! Partial correlation coefficient (Origin's Statistics > Descriptive Statistics > Partial Correlation
//! Coefficient, `pcorrcoef`): reads the variables and the controlling variables from the sheet and lays the
//! report out; the numbers are in `sheaf_calc::partial_correlation`. See `docs/spec/statistics.md`, section 24.

use super::statistics::{Output, label_column, num, number_column};
use super::*;

/// The significance level of the `*` flags (Origin's dialog has no field for it).
const FLAG_LEVEL: f64 = 0.05;

pub(super) fn partial_correlation(
    spec: &PartialCorrSpec,
    sheet: &Worksheet,
    columns: &[ColumnId],
    rows: RowRange,
) -> Result<Output, AnalysisError> {
    if columns.len() < 2 {
        return Err(AnalysisError::BadOption("the partial correlation needs at least two variables".into()));
    }
    if spec.controls.is_empty() {
        return Err(AnalysisError::BadOption("choose at least one controlling variable".into()));
    }
    if let Some(id) = spec.controls.iter().find(|c| columns.contains(c)) {
        let name = sheet.column(*id).map_or_else(String::new, |c| c.display_name().to_string());
        return Err(AnalysisError::BadOption(format!("column {name} is both a variable and a controlling variable")));
    }
    let read = |id: ColumnId| -> Result<(String, Vec<f64>), AnalysisError> {
        let c = sheet.column(id).ok_or(AnalysisError::NoSuchColumn(id))?;
        let data = sheet.values_of(c).ok_or_else(|| AnalysisError::NotNumeric(c.short_name.clone()))?;
        if data.is_empty() {
            return Err(AnalysisError::Empty(c.short_name.clone()));
        }
        let data = match rows {
            RowRange::All => data.into_owned(),
            RowRange::Rows(..) => {
                let (first, last) = resolve_rows(rows, &c.short_name, &[], data.len())?;
                (0..data.len()).map(|i| if (first..=last).contains(&i) { data[i] } else { f64::NAN }).collect()
            }
            RowRange::XBetween(..) => {
                return Err(AnalysisError::BadOption(
                    "the partial correlation has no X column to limit; choose rows with a row range".into(),
                ));
            }
        };
        Ok((c.display_name().to_string(), data))
    };
    let vars: Vec<(String, Vec<f64>)> = columns.iter().map(|&id| read(id)).collect::<Result<_, _>>()?;
    let controls: Vec<(String, Vec<f64>)> = spec.controls.iter().map(|&id| read(id)).collect::<Result<_, _>>()?;
    let var_data: Vec<&[f64]> = vars.iter().map(|v| v.1.as_slice()).collect();
    let control_data: Vec<&[f64]> = controls.iter().map(|v| v.1.as_slice()).collect();
    let r = calc::partial_correlation(&var_data, &control_data, spec.listwise, spec.one_tailed)
        .map_err(|e| AnalysisError::BadOption(e.to_string()))?;
    let m = vars.len();
    let names: Vec<String> = vars.iter().map(|v| v.0.clone()).collect();

    // Descriptive statistics of the variables, then of the controlling variables (over the complete rows when
    // missing values are left out listwise).
    let all: Vec<&[f64]> = var_data.iter().chain(&control_data).copied().collect();
    let len = all.iter().map(|c| c.len()).max().unwrap_or(0);
    let value = |c: &[f64], i: usize| c.get(i).copied().unwrap_or(f64::NAN);
    let complete: Vec<usize> = (0..len).filter(|&i| all.iter().all(|c| !value(c, i).is_nan())).collect();
    let described: Vec<calc::Stats> = all
        .iter()
        .map(|c| {
            if spec.listwise {
                calc::stats(&complete.iter().map(|&i| value(c, i)).collect::<Vec<f64>>())
            } else {
                calc::stats(c)
            }
        })
        .collect();
    let labels: Vec<String> =
        vars.iter().map(|v| v.0.clone()).chain(controls.iter().map(|c| format!("{} (control)", c.0))).collect();
    let mut on_sheet = vec![
        label_column("Variable", labels),
        number_column("N", "", described.iter().map(|s| s.n as f64).collect()),
        number_column("Mean", "", described.iter().map(|s| s.mean).collect()),
        number_column("SD", "", described.iter().map(|s| s.sd).collect()),
        number_column("Sum", "", described.iter().map(|s| s.sum).collect()),
        number_column("Minimum", "", described.iter().map(|s| s.min).collect()),
        number_column("Maximum", "", described.iter().map(|s| s.max).collect()),
    ];

    // The matrix: one block of rows per variable in Origin (Partial Corr., p-value, df); here the matrix of
    // coefficients (text with a `*` when flags are on) and, with "Show Significance", the matrices of p and df.
    let flagged = |i: usize, j: usize| spec.flag_significant && i != j && r.p(i, j) < FLAG_LEVEL;
    on_sheet.push(label_column("Partial Corr.", names.clone()));
    for (j, name) in names.iter().enumerate() {
        if spec.flag_significant {
            let cells = (0..m).map(|i| format!("{}{}", num(r.r(i, j)), if flagged(i, j) { "*" } else { "" })).collect();
            on_sheet.push(label_column(name, cells));
        } else {
            on_sheet.push(number_column(name, "", (0..m).map(|i| r.r(i, j)).collect()));
        }
    }
    if spec.show_significance {
        for (j, name) in names.iter().enumerate() {
            on_sheet.push(number_column(&format!("p-value ({name})"), "", (0..m).map(|i| r.p(i, j)).collect()));
        }
        for (j, name) in names.iter().enumerate() {
            on_sheet.push(number_column(&format!("df ({name})"), "", (0..m).map(|i| r.df(i, j)).collect()));
        }
    }

    // The pairs, for the report table and the command line.
    let mut table = ResultTable::new(&["N", "Partial Corr.", "p-value", "df", "Significant"]);
    for i in 0..m {
        for j in i + 1..m {
            table.push_row(
                format!("{} vs {}", names[i], names[j]),
                vec![
                    r.n[i * m + j] as f64,
                    r.r(i, j),
                    r.p(i, j),
                    r.df(i, j),
                    f64::from(u8::from(r.p(i, j) < FLAG_LEVEL)),
                ],
                vec![String::new(); 5],
            );
        }
    }
    let control_names: Vec<&str> = controls.iter().map(|c| c.0.as_str()).collect();
    let mut notes = vec![format!("Controlling variables: {}", control_names.join(", "))];
    notes.push(format!("{} test of significance is used.", if spec.one_tailed { "One-tailed" } else { "Two-tailed" }));
    notes.push(format!(
        "Missing values are left out {}.",
        if spec.listwise {
            "row by row over all variables and controls (listwise)"
        } else {
            "pair by pair (pairwise; n of a pair counts the rows where both variables have a number)"
        }
    ));
    if spec.flag_significant {
        notes.push(format!(
            "*: The partial correlation is significantly different from zero at the {} level.",
            num(FLAG_LEVEL)
        ));
    }
    for i in 0..m {
        for j in i + 1..m {
            if r.df(i, j) == 0.0 {
                notes.push(format!(
                    "{} vs {}: no degrees of freedom are left, so there is no p-value.",
                    names[i], names[j]
                ));
            }
        }
    }
    Ok(Output { table: Some(table), on_sheet, notes, plots: Vec::new(), comparison_pairs: Vec::new() })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project() -> (Project, Vec<ColumnId>) {
        let mut p = Project::new();
        let book = p.add_workbook("pc");
        let cols = [
            vec![1.0, 2.0, 3.0, 4.0, 5.0, 7.0, 8.0],
            vec![2.0, 1.0, 4.0, 3.0, 7.0, 6.0, 9.0],
            vec![1.0, 3.0, 2.0, 5.0, 4.0, 6.0, 8.5],
            vec![5.0, 1.0, 4.0, 2.0, 7.0, 3.0, 2.5],
        ];
        let ids =
            cols.into_iter().map(|v| p.add_column(book, 0, Designation::Y, ColumnData::numeric(v)).unwrap()).collect();
        (p, ids)
    }

    fn request(ids: &[ColumnId], spec: PartialCorrSpec) -> Request {
        Request { columns: ids.to_vec(), rows: RowRange::All, tool: Tool::PartialCorrelation(spec) }
    }

    #[test]
    fn partial_correlation_writes_the_matrix_table_and_footer_lines() {
        let (mut p, ids) = project();
        let spec = PartialCorrSpec { controls: vec![ids[2]], ..Default::default() };
        let r = p.analyze(&request(&ids[..2], spec)).unwrap();
        let t = r.table.as_ref().unwrap();
        assert_eq!(t.rows.len(), 1);
        // n = 7, one control: df = 4.
        assert_eq!(t.rows[0].1[0], 7.0);
        assert_eq!(t.rows[0].1[3], 4.0);
        let direct = sheaf_calc::partial_correlation(
            &[p.column(ids[0]).unwrap().numeric().unwrap(), p.column(ids[1]).unwrap().numeric().unwrap()],
            &[p.column(ids[2]).unwrap().numeric().unwrap()],
            false,
            false,
        )
        .unwrap();
        assert!((t.rows[0].1[1] - direct.r(0, 1)).abs() < 1e-15);
        assert!(r.notes.iter().any(|n| n.starts_with("Controlling variables:")), "{:?}", r.notes);
        assert!(r.notes.iter().any(|n| n == "Two-tailed test of significance is used."), "{:?}", r.notes);
        assert!(!r.notes.iter().any(|n| n.starts_with("*:")));
        let ws = &p.workbook(r.book).unwrap().sheets[r.new_sheet.unwrap()];
        assert!(ws.name.starts_with("PartialCorr"), "{}", ws.name);
        // Without "Show Significance" the sheet has the coefficient matrix only.
        assert!(!ws.columns.iter().any(|c| c.long_name.starts_with("p-value")));
    }

    #[test]
    fn options_add_significance_blocks_flags_and_one_tailed_wording() {
        let (mut p, ids) = project();
        let spec = PartialCorrSpec {
            controls: vec![ids[3]],
            one_tailed: true,
            show_significance: true,
            flag_significant: true,
            ..Default::default()
        };
        let r = p.analyze(&request(&ids[..3], spec)).unwrap();
        let ws = &p.workbook(r.book).unwrap().sheets[r.new_sheet.unwrap()];
        assert!(ws.columns.iter().any(|c| c.long_name.starts_with("p-value")));
        assert!(ws.columns.iter().any(|c| c.long_name.starts_with("df")));
        assert!(r.notes.iter().any(|n| n == "One-tailed test of significance is used."));
        assert!(r.notes.iter().any(|n| n.starts_with("*: The partial correlation is significantly different")));
        assert_eq!(r.table.as_ref().unwrap().rows.len(), 3);
    }

    #[test]
    fn refusals_are_reported_with_a_reason() {
        let (mut p, ids) = project();
        // No control, one variable, a variable that is also a control.
        let none = PartialCorrSpec::default();
        assert!(p.analyze(&request(&ids[..2], none)).is_err());
        let spec = PartialCorrSpec { controls: vec![ids[2]], ..Default::default() };
        assert!(p.analyze(&request(&ids[..1], spec.clone())).is_err());
        let same = PartialCorrSpec { controls: vec![ids[1]], ..Default::default() };
        let err = p.analyze(&request(&ids[..2], same)).unwrap_err().to_string();
        assert!(err.contains("both a variable and a controlling variable"), "{err}");
        // A text column.
        let book = p.locate(ids[0]).unwrap().0.id;
        let text = p.add_column(book, 0, Designation::Y, ColumnData::text(vec!["a".into(); 7])).unwrap();
        assert!(
            p.analyze(&request(&[ids[0], ids[1]], PartialCorrSpec { controls: vec![text], ..Default::default() }))
                .is_err()
        );
        // Row ranges work, an X range does not.
        let rows = Request {
            columns: ids[..2].to_vec(),
            rows: RowRange::Rows(1, 6),
            tool: Tool::PartialCorrelation(spec.clone()),
        };
        assert_eq!(p.analyze(&rows).unwrap().table.unwrap().rows[0].1[0], 6.0);
        let x = Request {
            columns: ids[..2].to_vec(),
            rows: RowRange::XBetween(0.0, 1.0),
            tool: Tool::PartialCorrelation(spec),
        };
        assert!(p.analyze(&x).is_err());
    }
}
