//! Statistics > qPCR > Relative Quantification and Standard Curve as tools of `Project::analyze` (T397; Origin
//! 2027's assay apps, not in Origin 2016, so the layout is Sheaf's own). The numerics are in `sheaf_calc::qpcr`;
//! `docs/spec/statistics.md` section 50 says what is computed and written.

use super::dist::{GraphPlot, Output, XyGraph};
use super::statistics::num;
use super::survival::group_cell;
use super::*;
use crate::graph::{PlotKind, ScaleKind};
use calc::qpcr::{relative_quantification, standard_curve};

/// What [`Tool::Qpcr`] does: the chosen columns are the group, the target Ct and the reference Ct.
#[derive(Debug, Clone, PartialEq)]
pub struct QpcrSpec {
    /// The control group (empty: the first group).
    pub control: String,
    /// Amplification factors (2 = 100 % efficiency).
    pub e_target: f64,
    pub e_reference: f64,
    pub confidence: f64,
    pub graph: bool,
}

impl Default for QpcrSpec {
    fn default() -> Self {
        QpcrSpec { control: String::new(), e_target: 2.0, e_reference: 2.0, confidence: 0.95, graph: true }
    }
}

/// What [`Tool::QpcrCurve`] does: the chosen columns are the quantity and the Ct of a dilution series.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QpcrCurveSpec {
    pub graph: bool,
}

impl Default for QpcrCurveSpec {
    fn default() -> Self {
        QpcrCurveSpec { graph: true }
    }
}

fn column_of(name: &str, designation: Designation, values: Vec<f64>) -> SheetColumn {
    SheetColumn {
        designation,
        long_name: name.to_string(),
        units: String::new(),
        comments: String::new(),
        data: ColumnData::numeric(values),
    }
}

fn label_column(name: &str, labels: Vec<String>) -> SheetColumn {
    SheetColumn {
        designation: Designation::Label,
        long_name: name.to_string(),
        units: String::new(),
        comments: String::new(),
        data: ColumnData::text(labels),
    }
}

pub(super) fn run(
    spec: &QpcrSpec,
    sheet: &Worksheet,
    columns: &[ColumnId],
    rows: RowRange,
) -> Result<Output, AnalysisError> {
    let [g_id, t_id, r_id] = columns else {
        return Err(AnalysisError::BadOption(
            "qPCR quantification takes three columns: the group, the target Ct and the reference Ct".into(),
        ));
    };
    let column = |id: ColumnId| sheet.column(id).ok_or(AnalysisError::NoSuchColumn(id));
    let read = |c: &Column| -> Result<Vec<f64>, AnalysisError> {
        sheet.values_of(c).map(|v| v.into_owned()).ok_or_else(|| AnalysisError::NotNumeric(c.short_name.clone()))
    };
    let (gc, tc, rc) = (column(*g_id)?, column(*t_id)?, column(*r_id)?);
    let (ts, rs) = (read(tc)?, read(rc)?);
    let n = gc.data.len().max(ts.len()).max(rs.len());
    if n == 0 {
        return Err(AnalysisError::Empty(tc.short_name.clone()));
    }
    let (first, last) = resolve_rows(rows, &tc.short_name, &[], n)?;
    let (mut groups, mut t, mut r) = (Vec::new(), Vec::new(), Vec::new());
    for row in first..=last {
        let Some(g) = group_cell(gc, row) else { continue };
        groups.push(g);
        t.push(ts.get(row).copied().unwrap_or(f64::NAN));
        r.push(rs.get(row).copied().unwrap_or(f64::NAN));
    }
    let control = if spec.control.trim().is_empty() {
        groups.first().cloned().ok_or_else(|| AnalysisError::Empty(gc.short_name.clone()))?
    } else {
        spec.control.trim().to_string()
    };
    let res = relative_quantification(&groups, &t, &r, &control, spec.e_target, spec.e_reference, spec.confidence)
        .map_err(|e| AnalysisError::BadOption(e.to_string()))?;
    let pct = num(100.0 * spec.confidence);
    let low = format!("Ratio lower {pct}%");
    let high = format!("Ratio upper {pct}%");
    let mut table = ResultTable::new(&[
        "N",
        "Mean target Ct",
        "Mean reference Ct",
        "Mean dCt",
        "SD dCt",
        "ddCt",
        "SE ddCt",
        "Expression ratio",
        &low,
        &high,
        "Prob (Welch)",
    ]);
    for (name, g) in &res {
        table.push_row(
            name.clone(),
            vec![
                g.n as f64,
                g.mean_target,
                g.mean_reference,
                g.mean_dct,
                g.sd_dct,
                g.ddct,
                g.se_ddct,
                g.ratio,
                g.ratio_low,
                g.ratio_high,
                g.p,
            ],
            vec![String::new(); 11],
        );
    }
    let livak = spec.e_target == 2.0 && spec.e_reference == 2.0;
    let mut notes = vec![format!(
        "{} groups against the control {control}: {} (dCt = target Ct - reference Ct, ddCt against the control's mean dCt).",
        res.len(),
        if livak {
            "Livak ratio 2^-ddCt".to_string()
        } else {
            format!(
                "Pfaffl ratio with amplification factors {} (target) and {} (reference)",
                num(spec.e_target),
                num(spec.e_reference)
            )
        }
    )];
    for (name, g) in res.iter().filter(|(n, _)| *n != control) {
        notes.push(format!(
            "{name}: ratio {} ({pct}% {} to {}), Prob {}.",
            num(g.ratio),
            num(g.ratio_low),
            num(g.ratio_high),
            num(g.p)
        ));
    }
    let names: Vec<String> = res.iter().map(|(n, _)| n.clone()).collect();
    let mut sheet_cols = vec![label_column("Group", names.clone())];
    for (i, h) in table.headers.iter().enumerate() {
        sheet_cols.push(column_of(h, Designation::Y, table.rows.iter().map(|(_, v)| v[i]).collect()));
    }
    let shift = sheet_cols.len();
    let k = res.len();
    let on_sheet = vec![
        column_of("Group index", Designation::X, (1..=k).map(|i| i as f64).collect()),
        column_of("Ratio (plot)", Designation::Y, res.iter().map(|(_, g)| g.ratio).collect()),
        column_of("Lower (plot)", Designation::Y, res.iter().map(|(_, g)| g.ratio_low).collect()),
        column_of("Upper (plot)", Designation::Y, res.iter().map(|(_, g)| g.ratio_high).collect()),
    ];
    let graph = spec.graph.then(|| XyGraph {
        plots: vec![
            GraphPlot { x: 0, y: 1, kind: PlotKind::Column, label: "Expression ratio".into(), line_of: None },
            GraphPlot { x: 0, y: 2, kind: PlotKind::Scatter, label: format!("Lower {pct}%"), line_of: Some(0) },
            GraphPlot { x: 0, y: 3, kind: PlotKind::Scatter, label: format!("Upper {pct}%"), line_of: Some(0) },
        ],
        x_scale: ScaleKind::Linear,
        y_scale: ScaleKind::Linear,
        x_title: "Group (row order of the table)".into(),
        y_title: "Expression ratio".into(),
        look: Default::default(),
    });
    sheet_cols.extend(on_sheet);
    Ok(Output { table: Some(table), on_sheet: sheet_cols, notes, graph, more_graphs: Vec::new() }.shift_graphs(shift))
}

pub(super) fn run_curve(
    spec: &QpcrCurveSpec,
    sheet: &Worksheet,
    columns: &[ColumnId],
    rows: RowRange,
) -> Result<Output, AnalysisError> {
    let [q_id, c_id] = columns else {
        return Err(AnalysisError::BadOption("the standard curve takes two columns: the quantity and the Ct".into()));
    };
    let column = |id: ColumnId| sheet.column(id).ok_or(AnalysisError::NoSuchColumn(id));
    let read = |c: &Column| -> Result<Vec<f64>, AnalysisError> {
        sheet.values_of(c).map(|v| v.into_owned()).ok_or_else(|| AnalysisError::NotNumeric(c.short_name.clone()))
    };
    let (qc, cc) = (column(*q_id)?, column(*c_id)?);
    let (qs, cs) = (read(qc)?, read(cc)?);
    let n = qs.len().max(cs.len());
    if n == 0 {
        return Err(AnalysisError::Empty(cc.short_name.clone()));
    }
    let (first, last) = resolve_rows(rows, &cc.short_name, &[], n)?;
    let q: Vec<f64> = (first..=last).map(|r| qs.get(r).copied().unwrap_or(f64::NAN)).collect();
    let c: Vec<f64> = (first..=last).map(|r| cs.get(r).copied().unwrap_or(f64::NAN)).collect();
    let s = standard_curve(&q, &c).map_err(|e| AnalysisError::BadOption(e.to_string()))?;
    let mut table = ResultTable::new(&["Slope", "Intercept", "R^2", "Amplification factor", "Efficiency (%)"]);
    table.push_row(column_label(cc), vec![s.slope, s.intercept, s.r2, s.factor, s.efficiency], vec![String::new(); 5]);
    let notes = vec![format!(
        "Standard curve of {} on log10 {}: slope {}, R^2 {}; amplification factor {} (efficiency {} %).",
        column_label(cc),
        column_label(qc),
        num(s.slope),
        num(s.r2),
        num(s.factor),
        num(s.efficiency)
    )];
    let mut sheet_cols = vec![label_column("Data", vec![column_label(cc)])];
    for (i, h) in table.headers.iter().enumerate() {
        sheet_cols.push(column_of(h, Designation::Y, table.rows.iter().map(|(_, v)| v[i]).collect()));
    }
    let shift = sheet_cols.len();
    let pts: Vec<(f64, f64)> = q
        .iter()
        .zip(&c)
        .filter(|(a, b)| a.is_finite() && **a > 0.0 && b.is_finite())
        .map(|(a, b)| (a.log10(), *b))
        .collect();
    let (lo, hi) = pts.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), p| (a.min(p.0), b.max(p.0)));
    let on_sheet = vec![
        column_of("log10 quantity", Designation::X, pts.iter().map(|p| p.0).collect()),
        column_of("Ct", Designation::Y, pts.iter().map(|p| p.1).collect()),
        column_of("Line X", Designation::X, vec![lo, hi]),
        column_of("Fitted Ct", Designation::Y, vec![s.intercept + s.slope * lo, s.intercept + s.slope * hi]),
    ];
    let graph = spec.graph.then(|| XyGraph {
        plots: vec![
            GraphPlot { x: 0, y: 1, kind: PlotKind::Scatter, label: column_label(cc), line_of: None },
            GraphPlot {
                x: 2,
                y: 3,
                kind: PlotKind::Line,
                label: format!("Ct = {} {} log10 q", num(s.intercept), num(s.slope)),
                line_of: Some(0),
            },
        ],
        x_scale: ScaleKind::Linear,
        y_scale: ScaleKind::Linear,
        x_title: format!("log10 {}", column_label(qc)),
        y_title: "Ct".into(),
        look: Default::default(),
    });
    sheet_cols.extend(on_sheet);
    Ok(Output { table: Some(table), on_sheet: sheet_cols, notes, graph, more_graphs: Vec::new() }.shift_graphs(shift))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_against_the_control_and_a_standard_curve() {
        let mut p = Project::new();
        let book = p.add_workbook("qpcr");
        let g = p
            .add_column(
                book,
                0,
                Designation::Y,
                ColumnData::text(["c", "c", "c", "t", "t", "t"].iter().map(|s| s.to_string()).collect()),
            )
            .unwrap();
        let t = p
            .add_column(book, 0, Designation::Y, ColumnData::numeric(vec![25.0, 25.2, 24.8, 23.0, 23.1, 22.9]))
            .unwrap();
        let r = p
            .add_column(book, 0, Designation::Y, ColumnData::numeric(vec![18.0, 18.1, 17.9, 18.0, 18.0, 18.0]))
            .unwrap();
        let out = p
            .analyze(&Request { columns: vec![g, t, r], rows: RowRange::All, tool: Tool::Qpcr(Default::default()) })
            .unwrap();
        let tab = out.table.as_ref().unwrap();
        assert_eq!(tab.rows.iter().map(|r| r.0.as_str()).collect::<Vec<_>>(), ["c", "t"]);
        assert!((tab.rows[1].1[7] - 4.0).abs() < 1e-12, "{:?}", tab.rows[1]);
        let cols = super::super::dist::plot_columns(&p, out.new_graph.unwrap());
        assert_eq!(cols[0].1, "Ratio (plot)");
        let q = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0, 10.0, 100.0, 1000.0])).unwrap();
        let ct = p
            .add_column(
                book,
                0,
                Designation::Y,
                ColumnData::numeric(
                    [1.0f64, 10.0, 100.0, 1000.0].iter().map(|v| 30.0 - v.log10() / 2f64.log10()).collect(),
                ),
            )
            .unwrap();
        let out = p
            .analyze(&Request { columns: vec![q, ct], rows: RowRange::All, tool: Tool::QpcrCurve(Default::default()) })
            .unwrap();
        assert!((out.table.as_ref().unwrap().rows[0].1[4] - 100.0).abs() < 1e-9);
        let bad = QpcrSpec { control: "x".into(), ..Default::default() };
        assert!(p.analyze(&Request { columns: vec![g, t, r], rows: RowRange::All, tool: Tool::Qpcr(bad) }).is_err());
    }
}
