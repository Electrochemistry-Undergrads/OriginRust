//! Statistics > Manhattan Plot as a tool of `Project::analyze` (T362; Origin's Manhattan Plot app of 2017, not in
//! Origin 2016, so the layout is Sheaf's own; `docs/spec/statistics.md` section 33).
//!
//! The chosen columns are the group (chromosome; numbers or text), the position inside the group and the p-value,
//! in that order. Groups are placed one after the other along X (numeric groups in increasing order, text groups in
//! the order they first appear): each group's positions are moved by the sum of the largest positions of the groups
//! before it. Y is `-log10 p`. Rows with a missing position, or a p outside (0, 1], are left out.

use super::dist::{GraphPlot, Output, XyGraph};
use super::statistics::num;
use super::*;
use crate::graph::{PlotKind, ScaleKind};

/// What [`Tool::Manhattan`] does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ManhattanSpec {
    /// The genome-wide significance level drawn as a line (default 5e-8).
    pub threshold: f64,
    pub graph: bool,
}

impl Default for ManhattanSpec {
    fn default() -> Self {
        ManhattanSpec { threshold: 5e-8, graph: true }
    }
}

fn number_column(name: &str, comments: &str, designation: Designation, values: Vec<f64>) -> SheetColumn {
    SheetColumn {
        designation,
        long_name: name.to_string(),
        units: String::new(),
        comments: comments.to_string(),
        data: ColumnData::numeric(values),
    }
}

/// The text of a group cell: the text, or the number written plainly; `None` for a missing cell.
fn group_text(c: &Column, row: usize) -> Option<String> {
    if let Some(t) = c.data.text_cell(row) {
        let t = t.trim();
        return (!t.is_empty()).then(|| t.to_string());
    }
    c.numeric()?.get(row).copied().filter(|v| v.is_finite()).map(|v| format!("{v}"))
}

pub(super) fn run(
    spec: &ManhattanSpec,
    sheet: &Worksheet,
    columns: &[ColumnId],
    rows: RowRange,
) -> Result<Output, AnalysisError> {
    if !(spec.threshold > 0.0 && spec.threshold < 1.0) {
        return Err(AnalysisError::BadOption("the significance level must lie between 0 and 1".into()));
    }
    let [g_id, pos_id, p_id] = columns else {
        return Err(AnalysisError::BadOption(
            "the Manhattan plot needs three columns: the group (chromosome), the position and the p-value".into(),
        ));
    };
    let column = |id: ColumnId| sheet.column(id).ok_or(AnalysisError::NoSuchColumn(id));
    let (gc, pc, vc) = (column(*g_id)?, column(*pos_id)?, column(*p_id)?);
    let pos = sheet.values_of(pc).ok_or_else(|| AnalysisError::NotNumeric(pc.short_name.clone()))?.into_owned();
    let pv = sheet.values_of(vc).ok_or_else(|| AnalysisError::NotNumeric(vc.short_name.clone()))?.into_owned();
    let n = gc.data.len().max(pos.len()).max(pv.len());
    if n == 0 {
        return Err(AnalysisError::Empty(vc.short_name.clone()));
    }
    let (first, last) = resolve_rows(rows, &vc.short_name, &[], n)?;
    // The points of each group, groups in first-appearance order.
    let mut groups: Vec<(String, Vec<(usize, f64, f64)>)> = Vec::new();
    for r in first..=last {
        let (Some(g), Some(&x), Some(&p)) = (group_text(gc, r), pos.get(r), pv.get(r)) else { continue };
        if !(x.is_finite() && p > 0.0 && p <= 1.0) {
            continue;
        }
        match groups.iter_mut().find(|(name, _)| *name == g) {
            Some((_, pts)) => pts.push((r, x, p)),
            None => groups.push((g, vec![(r, x, p)])),
        }
    }
    if groups.is_empty() {
        return Err(AnalysisError::BadOption("no row has a group, a position and a p-value between 0 and 1".into()));
    }
    if groups.iter().all(|(g, _)| g.parse::<f64>().is_ok()) {
        groups.sort_by(|a, b| a.0.parse::<f64>().unwrap_or(0.0).total_cmp(&b.0.parse::<f64>().unwrap_or(0.0)));
    }
    let line = -spec.threshold.log10();
    let mut table = ResultTable::new(&["N", "Offset", "Smallest p", "Largest -log10 p", "Above the line"]);
    let mut on_sheet = Vec::new();
    let mut plots = Vec::new();
    let mut offset = 0.0;
    let (mut x_lo, mut x_hi) = (f64::INFINITY, f64::NEG_INFINITY);
    let mut hits = 0;
    for (name, pts) in &groups {
        let xs: Vec<f64> = pts.iter().map(|p| offset + p.1).collect();
        let ys: Vec<f64> = pts.iter().map(|p| -p.2.log10()).collect();
        let above = ys.iter().filter(|y| **y >= line).count();
        hits += above;
        let smallest = pts.iter().map(|p| p.2).fold(f64::INFINITY, f64::min);
        table.push_row(
            name.clone(),
            vec![pts.len() as f64, offset, smallest, -smallest.log10(), above as f64],
            vec![String::new(); 5],
        );
        x_lo = x_lo.min(xs.iter().copied().fold(f64::INFINITY, f64::min));
        x_hi = x_hi.max(xs.iter().copied().fold(f64::NEG_INFINITY, f64::max));
        let label = format!("{} {name}", column_label(gc));
        let x = on_sheet.len();
        on_sheet.push(number_column("Cumulative position", &label, Designation::X, xs));
        on_sheet.push(number_column("-log10 p", &label, Designation::Y, ys));
        plots.push(GraphPlot { x, y: x + 1, kind: PlotKind::Scatter, label, line_of: None });
        offset += pts.iter().map(|p| p.1).fold(0.0, f64::max);
    }
    let x = on_sheet.len();
    on_sheet.push(number_column("Line X", "Significance line", Designation::X, vec![x_lo, x_hi]));
    on_sheet.push(number_column("Line Y", "Significance line", Designation::Y, vec![line, line]));
    plots.push(GraphPlot {
        x,
        y: x + 1,
        kind: PlotKind::Line,
        label: format!("p = {}", num(spec.threshold)),
        line_of: None,
    });
    let notes = vec![format!(
        "{} groups, {} points; {hits} at or above the line p = {} (-log10 p = {}).",
        groups.len(),
        groups.iter().map(|g| g.1.len()).sum::<usize>(),
        num(spec.threshold),
        num(line)
    )];
    let mut sheet_cols = vec![SheetColumn {
        designation: Designation::Label,
        long_name: "Group".into(),
        units: String::new(),
        comments: String::new(),
        data: ColumnData::text(table.rows.iter().map(|(l, _)| l.clone()).collect()),
    }];
    for (i, h) in table.headers.iter().enumerate() {
        sheet_cols.push(number_column(h, "", Designation::Y, table.rows.iter().map(|(_, v)| v[i]).collect()));
    }
    sheet_cols.extend(on_sheet);
    let graph = spec.graph.then(|| XyGraph {
        plots,
        x_scale: ScaleKind::Linear,
        y_scale: ScaleKind::Linear,
        x_title: format!("{} position", column_label(gc)),
        y_title: "-log10 p".into(),
        look: Default::default(),
    });
    Ok(Output { table: Some(table), on_sheet: sheet_cols, notes, graph, more_graphs: Vec::new() })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(columns: Vec<ColumnId>, spec: ManhattanSpec) -> Request {
        Request { columns, rows: RowRange::All, tool: Tool::Manhattan(spec) }
    }

    #[test]
    fn groups_are_laid_end_to_end_and_p_becomes_minus_log10() {
        let mut p = Project::new();
        let book = p.add_workbook("gwas");
        // Chromosomes 2 then 1 in the sheet; numeric groups are sorted, so 1 comes first.
        let chrom = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![2.0, 1.0, 1.0, 2.0, 1.0])).unwrap();
        let pos =
            p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![50.0, 10.0, 100.0, 20.0, 30.0])).unwrap();
        let pv = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1e-9, 0.5, 1e-3, 0.01, 2.0])).unwrap();
        let out = p.analyze(&request(vec![chrom, pos, pv], ManhattanSpec::default())).unwrap();
        let t = out.table.as_ref().unwrap();
        assert_eq!(t.rows.iter().map(|(l, _)| l.as_str()).collect::<Vec<_>>(), ["1", "2"]);
        // Group 1: two usable points (p = 2 is left out), offset 0; group 2 starts at 100.
        assert_eq!((t.rows[0].1[0], t.rows[0].1[1]), (2.0, 0.0));
        assert_eq!((t.rows[1].1[0], t.rows[1].1[1], t.rows[1].1[4]), (2.0, 100.0, 1.0));
        assert!((t.rows[1].1[3] - 9.0).abs() < 1e-12);
        assert!(out.new_graph.is_some());
        let bad = ManhattanSpec { threshold: 0.0, graph: false };
        assert!(p.analyze(&request(vec![chrom, pos, pv], bad)).is_err());
        assert!(p.analyze(&request(vec![chrom, pos], ManhattanSpec::default())).is_err());
    }
}
