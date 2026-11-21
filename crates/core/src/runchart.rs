//! Plot > Statistics > Run Chart (T270): Origin 2025b's Run Chart (`plot_runchart`, also in Origin
//! 2026's Graph Maker), written from OriginLab's help page on a computer whose Origin (2016) has no
//! such chart, so nothing here was measured:
//!
//! - Documented: a column is cut into subgroups by a size; the mean or median of each subgroup is a
//!   Line+Symbol plot over the subgroup sequence; the individual values can be shown at their
//!   subgroup; a horizontal dashed line at the mean or median of the whole data is the reference.
//! - Sheaf's choices: subgroups of `size` consecutive rows (missing values left out, the last
//!   subgroup may be shorter, an empty one gets no point), subgroup numbers 1, 2, ... on X, the
//!   reference statistic the same as the connected one, black squares and line for the statistic,
//!   small grey circles for the values, a blue dashed reference line labelled "Mean = 4.5" right of
//!   the frame. The data go to a new workbook `RunChart<n>` (Subgroup, Mean or Median, and the
//!   values with their subgroup numbers), the graph is `RunChart<n>` too.

use crate::Color;
use crate::column::{ColumnData, Designation};
use crate::graph::{Dash, Font, PlotKind, SymbolShape, TextLabel};
use crate::ids::{ColumnId, WindowId};
use crate::project::{Project, ProjectError};
use crate::straight_line::StraightLine;
use crate::template::Template;

/// The statistic a run chart connects (and draws its reference line at).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunStat {
    Mean,
    Median,
}

impl RunStat {
    pub fn label(self) -> &'static str {
        match self {
            RunStat::Mean => "Mean",
            RunStat::Median => "Median",
        }
    }

    /// The statistic of `v` (no missing values in it); NaN when empty.
    pub fn of(self, v: &[f64]) -> f64 {
        if v.is_empty() {
            return f64::NAN;
        }
        match self {
            RunStat::Mean => v.iter().sum::<f64>() / v.len() as f64,
            RunStat::Median => {
                let mut s = v.to_vec();
                s.sort_by(f64::total_cmp);
                let m = s.len() / 2;
                if s.len() % 2 == 1 { s[m] } else { 0.5 * (s[m - 1] + s[m]) }
            }
        }
    }
}

/// The largest subgroup size the dialog offers.
pub const MAX_SIZE: usize = 1000;

/// What a run chart shows: one statistic per subgroup (NaN for a subgroup with no value), the
/// individual values with their subgroup number, and the statistic of all values.
#[derive(Clone, Debug, PartialEq)]
pub struct RunChartData {
    pub stats: Vec<f64>,
    pub points: Vec<(f64, f64)>,
    pub reference: f64,
}

/// Cuts `values` into subgroups of `size` consecutive rows; `None` when there is no value or the size is 0.
pub fn run_chart_data(values: &[f64], size: usize, stat: RunStat) -> Option<RunChartData> {
    if size == 0 {
        return None;
    }
    let all: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();
    if all.is_empty() {
        return None;
    }
    let mut stats = Vec::new();
    let mut points = Vec::new();
    for (k, chunk) in values.chunks(size).enumerate() {
        let good: Vec<f64> = chunk.iter().copied().filter(|v| v.is_finite()).collect();
        points.extend(good.iter().map(|v| ((k + 1) as f64, *v)));
        stats.push(stat.of(&good));
    }
    Some(RunChartData { stats, points, reference: stat.of(&all) })
}

impl Project {
    /// Makes a run chart of column `y` in subgroups of `size` connecting their `stat`, with the individual
    /// values when `points`. Returns the new workbook and the graph.
    pub fn plot_run_chart(
        &mut self,
        y: ColumnId,
        size: usize,
        stat: RunStat,
        points: bool,
    ) -> Result<(WindowId, WindowId), ProjectError> {
        let values = self.numeric(y).ok_or(ProjectError::NoSuchColumn(y))?.to_vec();
        let source = self.column(y).map(|c| c.display_name().to_string()).unwrap_or_default();
        let d = run_chart_data(&values, size, stat).ok_or(ProjectError::QcSubgroup(size))?;
        let name = self.unique_name("RunChart");
        let book = self.add_workbook("");
        if let Some(b) = self.workbook_mut(book) {
            b.name = name.clone();
        }
        let groups: Vec<f64> = (1..=d.stats.len()).map(|k| k as f64).collect();
        let mut columns = vec![("Subgroup", Designation::X, groups), (stat.label(), Designation::Y, d.stats.clone())];
        if points {
            columns.push(("Point Subgroup", Designation::X, d.points.iter().map(|p| p.0).collect()));
            columns.push(("Data", Designation::Y, d.points.iter().map(|p| p.1).collect()));
        }
        let mut ids = Vec::new();
        for (short, des, data) in columns {
            let c = self.add_column(book, 0, des, ColumnData::numeric(data))?;
            if let Some(col) = self.column_mut(c) {
                col.short_name = short.into();
            }
            ids.push(c);
        }
        let graph = self.add_graph(Template::Scatter);
        if points {
            self.add_plot_xy(graph, 0, Some(ids[2]), ids[3], None)?;
        }
        self.add_plot_xy(graph, 0, Some(ids[0]), ids[1], None)?;
        let g = self.graph_mut(graph).expect("the graph was just added");
        g.name = name;
        let l = &mut g.layers[0];
        for p in &mut l.plots {
            p.auto_color = false;
        }
        if points {
            let p = &mut l.plots[0];
            p.kind = PlotKind::Scatter;
            p.symbol.shape = SymbolShape::Circle;
            p.symbol.color = Color::rgb(128, 128, 128);
            p.symbol.filled = true;
            p.symbol.size_pt = 4.0;
            p.label = Some("Data".into());
        }
        let p = l.plots.last_mut().expect("the statistic's plot");
        p.kind = PlotKind::LineSymbol;
        p.line.color = Color::BLACK;
        p.symbol.shape = SymbolShape::Square;
        p.symbol.color = Color::BLACK;
        p.symbol.filled = true;
        p.symbol.size_pt = 6.0;
        p.label = Some(stat.label().into());
        let groups = d.stats.len() as f64;
        l.x_axis.min = Some(0.0);
        l.x_axis.max = Some(groups + 1.0);
        if groups <= 20.0 {
            l.x_axis.major_step = Some(1.0);
        }
        l.x_axis.title = Some("Subgroup".into());
        l.y_axis.title = Some(if source.is_empty() { stat.label().to_string() } else { source });
        let mut line = StraightLine::new(false, d.reference);
        line.color = Color::rgb(0, 0, 255);
        line.dash = Dash::Dash;
        line.show_label = false;
        line.name = StraightLine::next_name(&l.straight_lines, false);
        l.straight_lines.push(line);
        let font = Font { color: Color::rgb(0, 0, 255), ..l.x_axis.tick_font.clone() };
        let text = format!("{} = {}", stat.label(), crate::qc::four_digits(d.reference));
        // Right of the frame; the vertical place follows the automatic range, so it is the middle.
        l.labels.push(TextLabel::new(text, 1.01, 0.5, font));
        Ok((book, graph))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subgroups_take_consecutive_rows_and_skip_missing_values() {
        let v = [1.0, 2.0, 3.0, 4.0, f64::NAN, 6.0, 7.0];
        let d = run_chart_data(&v, 3, RunStat::Mean).unwrap();
        assert_eq!(d.stats, [2.0, 5.0, 7.0], "the last subgroup is shorter; NaN is left out");
        assert_eq!(d.points.len(), 6);
        assert_eq!(d.points[3], (2.0, 4.0));
        assert_eq!(d.reference, 23.0 / 6.0);
        let m = run_chart_data(&v, 3, RunStat::Median).unwrap();
        assert_eq!(m.stats, [2.0, 5.0, 7.0]);
        assert_eq!(m.reference, 3.5);
        let gap = run_chart_data(&[1.0, f64::NAN, f64::NAN, 2.0], 1, RunStat::Mean).unwrap();
        assert!(gap.stats[1].is_nan(), "an empty subgroup has no point");
        assert!(run_chart_data(&[f64::NAN], 1, RunStat::Mean).is_none());
        assert!(run_chart_data(&v, 0, RunStat::Mean).is_none());
    }

    #[test]
    fn the_chart_has_the_statistic_the_values_and_a_dashed_reference_line() {
        let mut p = Project::new();
        let book = p.add_workbook("d");
        let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric((1..=10).map(f64::from).collect())).unwrap();
        let (rb, g) = p.plot_run_chart(y, 2, RunStat::Mean, true).unwrap();
        let ws = p.workbook(rb).unwrap().sheet();
        assert_eq!(ws.columns.len(), 4);
        assert_eq!(ws.columns[1].numeric().unwrap(), [1.5, 3.5, 5.5, 7.5, 9.5]);
        let l = &p.graph(g).unwrap().layers[0];
        assert_eq!(l.plots.len(), 2);
        assert_eq!(l.plots[1].kind, PlotKind::LineSymbol);
        assert_eq!(l.plots[0].kind, PlotKind::Scatter);
        assert_eq!(l.straight_lines.len(), 1);
        assert_eq!((l.straight_lines[0].value, l.straight_lines[0].dash), (5.5, Dash::Dash));
        assert!(l.labels.iter().any(|t| t.text == "Mean = 5.5"));
        assert!(p.window(g).unwrap().title().starts_with("RunChart"));
        let (_, g2) = p.plot_run_chart(y, 1, RunStat::Median, false).unwrap();
        assert_eq!(p.graph(g2).unwrap().layers[0].plots.len(), 1);
    }
}
