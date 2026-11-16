//! Plot > Statistics > QC (X bar R) Chart (T111), as Origin 2016's, measured on 2026-11-16
//! (`docs/spec/graphing.md`, "QC (X bar R) chart"):
//!
//! - Origin asks for the subgroup size (3 by default), writes a new workbook `QC1` with the columns
//!   Mean, R and Sigma (one row per subgroup), and plots it in a graph `XbarR1` with two layers.
//! - Layer 1 (the range chart) sits at left 13.99 %, top 47.95 %, 65.01 % x 37 % of the page; layer 2
//!   (the mean chart) is linked to it at top -107 %, the same size, sharing X (0 to subgroups + 1).
//! - The means are black squares with a line down (or up) to the centre line; the ranges are red,
//!   black-edged bars from the centre line, 0.8 of the spacing wide.
//! - Each chart has a blue centre line (CL) and red upper and lower control limits (UCL, LCL), labelled
//!   right of the frame ("UCL = 10.82", four significant digits). The mean chart's Y axis runs from
//!   LCL - 0.3 (UCL - LCL) to UCL + 0.3 (UCL - LCL), the range chart's from 0 to 1.5 UCL.
//! - Titles: "Subgroup Mean (XBAR)", "Range" and "Subgroup Number". No legends.

use crate::Color;
use crate::column::{ColumnData, Designation};
use crate::graph::{Font, PlotKind, SymbolShape, TextLabel};
use crate::ids::{ColumnId, WindowId};
use crate::project::{Project, ProjectError};
use crate::straight_line::StraightLine;
use crate::template::Template;
use sheaf_calc::qc::XbarR;

/// Origin's default subgroup size.
pub const DEFAULT_GROUP: usize = 3;

/// The default number of standard deviations of the control limits.
pub const NUM_SIGMA: f64 = 3.0;

/// `v` to four significant digits without trailing zeros, as Origin labels the limits ("10.82",
/// "9.444", "0.6714", "0").
pub fn four_digits(v: f64) -> String {
    if v == 0.0 || !v.is_finite() {
        return if v == 0.0 { "0".into() } else { v.to_string() };
    }
    let decimals = (3 - v.abs().log10().floor() as i32).max(0) as usize;
    let s = format!("{v:.decimals$}");
    if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s }
}

impl Project {
    /// Makes an X bar R chart of column `y` in subgroups of `n`: a new QC workbook with the subgroups'
    /// Mean, R and Sigma, and the two-layer graph. Returns the workbook and the graph.
    pub fn plot_qc_xbar_r(&mut self, y: ColumnId, n: usize) -> Result<(WindowId, WindowId), ProjectError> {
        let values = self.numeric(y).ok_or(ProjectError::NoSuchColumn(y))?.to_vec();
        let q = sheaf_calc::qc::xbar_r(&values, n, NUM_SIGMA).ok_or(ProjectError::QcSubgroup(n))?;
        let book = self.add_workbook("");
        let name = self.unique_name("QC");
        if let Some(b) = self.workbook_mut(book) {
            b.name = name;
        }
        let mut cols = Vec::new();
        for (short, data) in [("Mean", &q.means), ("R", &q.ranges), ("Sigma", &q.sds)] {
            let c = self.add_column(book, 0, Designation::Y, ColumnData::numeric(data.clone()))?;
            if let Some(col) = self.column_mut(c) {
                col.short_name = short.into();
            }
            cols.push(c);
        }
        let graph = self.add_graph(Template::QcXbarR);
        let gname = self.unique_name("XbarR");
        self.add_plot_xy(graph, 1, None, cols[0], None)?;
        self.add_plot_xy(graph, 0, None, cols[1], None)?;
        let g = self.graph_mut(graph).expect("the graph was just added");
        g.name = gname;
        style_charts(g, &q);
        Ok((book, graph))
    }
}

/// The plots, limits, labels and axis ranges of the two charts.
fn style_charts(g: &mut crate::graph::Graph, q: &XbarR) {
    let groups = q.means.len() as f64;
    let mean_span = q.ucl - q.lcl;
    let charts = [
        (1, PlotKind::QcMeans, q.xbar, q.ucl, q.lcl, q.lcl - 0.3 * mean_span, q.ucl + 0.3 * mean_span),
        (0, PlotKind::QcRanges, q.rbar, q.r_ucl, q.r_lcl, 0.0, 1.5 * q.r_ucl),
    ];
    for (layer, kind, cl, ucl, lcl, lo, hi) in charts {
        let l = &mut g.layers[layer];
        l.x_axis.min = Some(0.0);
        l.x_axis.max = Some(groups + 1.0);
        l.x_axis.major_step = Some(1.0);
        l.y_axis.min = Some(lo);
        l.y_axis.max = Some(hi);
        let plot = l.plots.last_mut().expect("a plot was just added");
        plot.kind = kind;
        plot.auto_color = false;
        plot.decor.qc_center = Some(cl);
        plot.line.color = Color::BLACK;
        plot.line.width_pt = 0.5;
        if kind == PlotKind::QcMeans {
            plot.symbol.shape = SymbolShape::Square;
            plot.symbol.color = Color::BLACK;
            plot.symbol.filled = true;
            plot.symbol.size_pt = 5.0;
        } else {
            plot.symbol.color = Color::RED;
        }
        let font = l.x_axis.tick_font.clone();
        for (tag, value, color) in
            [("UCL", ucl, Color::RED), ("CL", cl, Color::rgb(0, 0, 255)), ("LCL", lcl, Color::RED)]
        {
            let mut line = StraightLine::new(false, value);
            line.color = color;
            line.show_label = false;
            line.name = StraightLine::next_name(&l.straight_lines, false);
            l.straight_lines.push(line);
            let y = if hi > lo { (hi - value) / (hi - lo) } else { 0.5 };
            let text = format!("{tag} = {}", four_digits(value));
            l.labels.push(TextLabel::new(text, 1.01, y, Font { color, ..font.clone() }));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_take_four_significant_digits() {
        assert_eq!(four_digits(10.81804), "10.82");
        assert_eq!(four_digits(9.443862), "9.444");
        assert_eq!(four_digits(0.6714286), "0.6714");
        assert_eq!(four_digits(1.728929), "1.729");
        assert_eq!(four_digits(10.13095), "10.13");
        assert_eq!(four_digits(0.0), "0");
    }

    #[test]
    fn the_qc_workbook_holds_the_subgroups_and_the_graph_both_charts() {
        let mut p = Project::new();
        let b = p.add_workbook("");
        let v = vec![
            10.2, 9.8, 10.5, 10.1, 9.7, 10.4, 10.0, 9.9, 10.6, 10.3, 9.6, 10.2, 10.8, 10.1, 9.9, 10.0, 10.4, 9.5, 10.2,
            10.3,
        ];
        let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(v)).unwrap();
        assert!(p.plot_qc_xbar_r(y, 1).is_err());
        let (book, g) = p.plot_qc_xbar_r(y, 3).unwrap();
        let wb = p.workbook(book).unwrap();
        assert_eq!(wb.name, "QC1");
        let names: Vec<&str> = wb.sheet().columns.iter().map(|c| c.short_name.as_str()).collect();
        assert_eq!(names, ["Mean", "R", "Sigma"]);
        assert_eq!(wb.sheet().columns[0].data.len(), 7);
        let graph = p.graph(g).unwrap();
        assert_eq!(graph.name, "XbarR1");
        assert_eq!(graph.layers.len(), 2);
        let (range, mean) = (&graph.layers[0], &graph.layers[1]);
        assert_eq!((mean.plots[0].kind, range.plots[0].kind), (PlotKind::QcMeans, PlotKind::QcRanges));
        assert_eq!((range.x_axis.min, range.x_axis.max), (Some(0.0), Some(8.0)));
        // Origin's ranges: 9.0316 to 11.2303 for the means, 0 to 2.5934 for the ranges.
        assert!((mean.y_axis.min.unwrap() - 9.0316).abs() < 1e-3 && (mean.y_axis.max.unwrap() - 11.2303).abs() < 1e-3);
        assert!((range.y_axis.max.unwrap() - 2.59339).abs() < 1e-4);
        let texts: Vec<&str> = mean.labels.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(texts, ["UCL = 10.82", "CL = 10.13", "LCL = 9.444"]);
        let texts: Vec<&str> = range.labels.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(texts, ["UCL = 1.729", "CL = 0.6714", "LCL = 0"]);
        assert_eq!(mean.straight_lines.len(), 3);
        assert_eq!(mean.straight_lines[1].color, Color::rgb(0, 0, 255));
    }
}
