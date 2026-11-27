//! Statistics > Manhattan Plot as a tool of `Project::analyze` (T362; Origin's Manhattan Plot app of 2017, not in
//! Origin 2016, so the layout is Sheaf's own; `docs/spec/statistics.md` section 33).
//!
//! The chosen columns are the group (chromosome; numbers or text), the position inside the group and the p-value,
//! in that order; with two chosen columns (position, p-value) the group is the sheet's X column for them. Groups are placed one after the other along X (numeric groups in increasing order, text groups in
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
    /// Colour the groups in two alternating colours (T505; default) instead of the palette.
    pub alternate: bool,
    /// A gap between groups, in percent of the summed group spans (T505; default 0).
    pub gap_percent: f64,
    /// Label the points at or above the line with their position in the group (T505).
    pub label_hits: bool,
}

impl Default for ManhattanSpec {
    fn default() -> Self {
        ManhattanSpec { threshold: 5e-8, graph: true, alternate: true, gap_percent: 0.0, label_hits: false }
    }
}

/// The two alternating group colours (T505): a dark blue and a mid grey.
const ALTERNATE: [crate::color::Color; 2] =
    [crate::color::Color::rgb(31, 64, 145), crate::color::Color::rgb(140, 140, 140)];

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
    if !(spec.gap_percent >= 0.0 && spec.gap_percent <= 100.0) {
        return Err(AnalysisError::BadOption("the gap between groups must be 0 to 100 percent".into()));
    }
    let column = |id: ColumnId| sheet.column(id).ok_or(AnalysisError::NoSuchColumn(id));
    let (gc, pc, vc) = match columns {
        [g, pos, p] => (column(*g)?, column(*pos)?, column(*p)?),
        // Two columns: the position and the p-value, the group being the sheet's X column for them.
        [pos, p] => {
            let index = sheet.column_index(*pos).ok_or(AnalysisError::NoSuchColumn(*pos))?;
            let g = sheet.x_for(index).filter(|x| x.id != *pos).ok_or_else(|| {
                AnalysisError::BadOption(
                    "choose the group column too: the sheet has no X column for the positions".into(),
                )
            })?;
            (g, column(*pos)?, column(*p)?)
        }
        _ => {
            return Err(AnalysisError::BadOption(
                "the Manhattan plot needs three columns: the group (chromosome), the position and the p-value".into(),
            ));
        }
    };
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
    // T505: the gap between groups, a share of the summed group spans.
    let total: f64 = groups.iter().map(|(_, pts)| pts.iter().map(|p| p.1).fold(0.0, f64::max)).sum();
    let gap = total * spec.gap_percent / 100.0;
    let mut colors = Vec::new();
    let mut labels = Vec::new();
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
        if spec.alternate {
            colors.push((plots.len(), ALTERNATE[plots.len() % 2]));
        }
        if spec.label_hits && above > 0 {
            // T505: the position of each point at or above the line, the others unlabelled.
            labels.push((plots.len(), on_sheet.len(), crate::color::Color::BLACK));
            on_sheet.push(SheetColumn {
                designation: Designation::Label,
                long_name: "Hit label".into(),
                units: String::new(),
                comments: label.clone(),
                data: ColumnData::text(
                    pts.iter().map(|p| if -p.2.log10() >= line { num(p.1) } else { String::new() }).collect(),
                ),
            });
        }
        plots.push(GraphPlot { x, y: x + 1, kind: PlotKind::Scatter, label, line_of: None });
        offset += pts.iter().map(|p| p.1).fold(0.0, f64::max) + gap;
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
    let shift = sheet_cols.len();
    sheet_cols.extend(on_sheet);
    let graph = spec.graph.then(|| XyGraph {
        plots,
        x_scale: ScaleKind::Linear,
        y_scale: ScaleKind::Linear,
        x_title: format!("{} position", column_label(gc)),
        y_title: "-log10 p".into(),
        look: super::dist::Look { colors, labels, ..Default::default() },
    });
    Ok(Output { table: Some(table), on_sheet: sheet_cols, notes, graph, more_graphs: Vec::new() }.shift_graphs(shift))
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
        let cols = super::super::dist::plot_columns(&p, out.new_graph.unwrap());
        assert_eq!((cols[0].0.as_str(), cols[0].1.as_str()), ("Cumulative position", "-log10 p"), "{cols:?}");
        assert_eq!(cols.last().unwrap().1, "Line Y");
        // T505: two alternating colours, a gap between groups, labels of the hits.
        let g = p.graph(out.new_graph.unwrap()).unwrap();
        let (a, b) = (g.layers[0].plots[0].symbol.color, g.layers[0].plots[1].symbol.color);
        assert_eq!((a, b), (ALTERNATE[0], ALTERNATE[1]));
        let spaced = ManhattanSpec { gap_percent: 10.0, label_hits: true, ..Default::default() };
        let out = p.analyze(&request(vec![chrom, pos, pv], spaced)).unwrap();
        let t = out.table.as_ref().unwrap();
        assert_eq!(t.rows[1].1[1], 100.0 + 15.0, "the gap is 10 % of 100 + 50");
        let g = p.graph(out.new_graph.unwrap()).unwrap();
        let labelled = &g.layers[0].plots[1].decor.labels;
        assert!(labelled.enable, "group 2 has a hit");
        assert!(!g.layers[0].plots[0].decor.labels.enable, "group 1 has none");
        let bad = ManhattanSpec { threshold: 0.0, graph: false, ..Default::default() };
        assert!(p.analyze(&request(vec![chrom, pos, pv], bad)).is_err());
        assert!(p.analyze(&request(vec![chrom], ManhattanSpec::default())).is_err());
    }
}
