//! Plot > Statistics > Scatter Matrix (T114), after Origin's `plot_matrix` (`docs/spec/graphing.md`, "Scatter
//! matrix"; Origin's documentation only, not measured):
//!
//! - Each selected numeric column is a variable; every column is plotted against every other one, each pair in
//!   its own layer of one graph, the layers in an n x n grid. The cell in row `r`, column `c` holds column `c` on
//!   X and column `r` on Y, so one grid column shares its X variable and one grid row its Y variable.
//! - Origin's Matrix Display: Square (every cell, the default), Upper or Lower Triangular (the cells above or
//!   below the diagonal, and the diagonal itself).
//! - Origin's Show in Diagonal Cells: None (the default) or a histogram of the variable; Variables in Diagonal
//!   Cells (on by default) writes the column's name in its diagonal cell. Gap is a percentage of the page between
//!   neighbouring cells (0 by default).
//!
//! Sheaf's own choices (unmeasured): a square page; tick labels only on the outer cells (the bottom row and left
//! column, or for an upper matrix the top row and right column with the axes on those sides); no axis titles or
//! legends; inward ticks and a closed frame in every cell; tick labels, symbols and names shrinking with the number of variables; every
//! cell keeps its variables' automatic ranges (with at most five steps) as fixed limits, so the grid lines up.

use crate::autorange::linear_range;
use crate::graph::{AxisSide, Frame, HistogramStyle, PlotKind, TextLabel, TickDirection};
use crate::ids::{ColumnId, WindowId};
use crate::panels::{PanelLayout, PanelOrder, PanelPage};
use crate::project::{Project, ProjectError, Window};
use crate::template::Template;
use serde::{Deserialize, Serialize};

/// The most variables a scatter matrix takes: 8 x 8 cells is the largest grid of layers (64).
pub const MAX_VARIABLES: usize = 8;

/// Origin's Matrix Display.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatrixDisplay {
    /// Every cell: each pair twice, once each way round.
    #[default]
    Square,
    /// The diagonal and the cells right of it.
    Upper,
    /// The diagonal and the cells left of it.
    Lower,
}

impl MatrixDisplay {
    pub const ALL: [MatrixDisplay; 3] = [MatrixDisplay::Square, MatrixDisplay::Upper, MatrixDisplay::Lower];

    /// The name in Origin's dialog.
    pub fn label(self) -> &'static str {
        match self {
            MatrixDisplay::Square => "Square",
            MatrixDisplay::Upper => "Upper Triangular",
            MatrixDisplay::Lower => "Lower Triangular",
        }
    }

    /// The name on the command line.
    pub fn name(self) -> &'static str {
        match self {
            MatrixDisplay::Square => "square",
            MatrixDisplay::Upper => "upper",
            MatrixDisplay::Lower => "lower",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|d| d.name().eq_ignore_ascii_case(name))
    }

    /// Whether the cell in grid row `r`, column `c` is drawn.
    pub fn shows(self, r: usize, c: usize) -> bool {
        match self {
            MatrixDisplay::Square => true,
            MatrixDisplay::Upper => c >= r,
            MatrixDisplay::Lower => c <= r,
        }
    }
}

/// Origin's Show in Diagonal Cells (the two Sheaf has; Origin also offers a box chart and distribution curves).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatrixDiagonal {
    #[default]
    None,
    Histogram,
}

impl MatrixDiagonal {
    pub const ALL: [MatrixDiagonal; 2] = [MatrixDiagonal::None, MatrixDiagonal::Histogram];

    pub fn label(self) -> &'static str {
        match self {
            MatrixDiagonal::None => "None",
            MatrixDiagonal::Histogram => "Histogram",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            MatrixDiagonal::None => "none",
            MatrixDiagonal::Histogram => "histogram",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|d| d.name().eq_ignore_ascii_case(name))
    }
}

/// The settings of a scatter matrix (Origin's dialog defaults).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScatterMatrixOptions {
    pub display: MatrixDisplay,
    pub diagonal: MatrixDiagonal,
    /// Write each variable's name in its diagonal cell.
    pub names: bool,
    /// Space between neighbouring cells, in percent of the page (0 to 10).
    pub gap_pct: f64,
}

impl Default for ScatterMatrixOptions {
    fn default() -> Self {
        ScatterMatrixOptions {
            display: MatrixDisplay::Square,
            diagonal: MatrixDiagonal::None,
            names: true,
            gap_pct: 0.0,
        }
    }
}

/// The largest gap between cells, in percent of the page.
pub const MAX_GAP_PCT: f64 = 10.0;

/// The page margins around the grid, in percent of the page: left, right, top, bottom (Sheaf's choice; the
/// tick labels are left and below). An upper matrix, labelled right and above, mirrors them.
const MARGINS: (f64, f64, f64, f64) = (14.0, 4.0, 4.0, 12.0);

/// The margins of a matrix shown as `display`: left, right, top, bottom in percent of the page.
pub fn margins(display: MatrixDisplay) -> (f64, f64, f64, f64) {
    let (left, right, top, bottom) = MARGINS;
    if display == MatrixDisplay::Upper { (right, left, bottom, top) } else { (left, right, top, bottom) }
}

impl Project {
    /// Plots the scatter matrix of `columns` (two to [`MAX_VARIABLES`] numeric columns, in selection order) with
    /// `options`: one undoable change of the project when called through the document's edit. Layers are the shown
    /// cells row by row from the top left.
    pub fn plot_scatter_matrix(
        &mut self,
        columns: &[ColumnId],
        options: &ScatterMatrixOptions,
    ) -> Result<WindowId, ProjectError> {
        let n = columns.len();
        if !(2..=MAX_VARIABLES).contains(&n) {
            return Err(ProjectError::ScatterMatrixColumns(n));
        }
        if !(0.0..=MAX_GAP_PCT).contains(&options.gap_pct) {
            return Err(ProjectError::ScatterMatrixGap);
        }
        let mut variables: Vec<(String, Vec<f64>)> = Vec::with_capacity(n);
        for &c in columns {
            let col = self.column(c).ok_or(ProjectError::NoSuchColumn(c))?;
            let name = col.display_name().to_string();
            let values = col.numeric().ok_or(ProjectError::ScatterMatrixText(name.clone()))?.to_vec();
            variables.push((name, values));
        }
        let long_name = self.locate(columns[0]).map(|(b, _, _)| b.title_for_graph());

        let (left, right, top, bottom) = margins(options.display);
        let layout = PanelLayout {
            rows: n,
            cols: n,
            h_gap: options.gap_pct,
            v_gap: options.gap_pct,
            margin_left: left,
            margin_right: right,
            margin_top: top,
            margin_bottom: bottom,
            order: PanelOrder::TopDown,
            link_x: false,
            link_y: false,
            page: PanelPage::Keep,
        };
        let frames = layout.frames()?;
        let cells: Vec<(usize, usize)> =
            (0..n).flat_map(|r| (0..n).map(move |c| (r, c))).filter(|&(r, c)| options.display.shows(r, c)).collect();

        let mut g = Template::ScatterMatrix.new_graph(WindowId(0), "");
        let mut base = g.layers[0].clone();
        let scale = (2.0 / n as f64).min(1.0);
        base.default_symbol.size_pt = (base.default_symbol.size_pt * scale).max(2.5);
        for axis in [&mut base.x_axis, &mut base.y_axis] {
            axis.tick_font.size_pt = (axis.tick_font.size_pt * scale).max(9.0);
            axis.major_tick_len_pt *= scale.max(0.5);
            axis.minor_tick_len_pt *= scale.max(0.5);
        }
        let page_w = g.page.width_pt;
        let ranges: Vec<Option<(f64, f64, f64)>> =
            variables.iter().map(|(_, v)| variable_range(v, base.x_axis.rescale_margin_pct)).collect();
        // The outer cells carry the tick labels: the bottom row (X) and the left column (Y) of a square or lower
        // matrix; the top row and the right column of an upper one, whose axes move to the opposite sides.
        let upper = options.display == MatrixDisplay::Upper;
        if upper {
            base.x_axis.side = AxisSide::Opposite;
            base.y_axis.side = AxisSide::Opposite;
        }
        let labels_x = |r: usize| if upper { r == 0 } else { r == n - 1 };
        let labels_y = |c: usize| if upper { c == n - 1 } else { c == 0 };
        g.layers = cells
            .iter()
            .map(|&(r, c)| {
                let mut l = base.clone();
                l.frame = frames[r * n + c];
                l.x_axis.show_tick_labels = labels_x(r);
                l.y_axis.show_tick_labels = labels_y(c) && !(r == c && options.diagonal == MatrixDiagonal::Histogram);
                // Every cell of a grid column has its variable's limits and step on X, every cell of a grid row its
                // variable's on Y, so the cells line up; a diagonal histogram counts from 0 on an automatic Y.
                let histogram = r == c && options.diagonal == MatrixDiagonal::Histogram;
                if let Some((lo, hi, step)) = ranges[c] {
                    (l.x_axis.min, l.x_axis.max, l.x_axis.major_step) = (Some(lo), Some(hi), Some(step));
                }
                if histogram {
                    l.y_axis.min = Some(0.0);
                } else if let Some((lo, hi, step)) = ranges[r] {
                    (l.y_axis.min, l.y_axis.max, l.y_axis.major_step) = (Some(lo), Some(hi), Some(step));
                }
                if r == c && options.names {
                    l.labels.push(name_label(&l, &variables[r].0, page_w, options.diagonal));
                }
                l
            })
            .collect();

        let graph = WindowId(self.fresh_id());
        g.id = graph;
        g.name = self.unique_name("Graph");
        self.windows.push(Window::Graph(g));
        for (k, &(r, c)) in cells.iter().enumerate() {
            if r == c {
                if options.diagonal == MatrixDiagonal::Histogram {
                    self.add_plot_xy(graph, k, None, columns[r], None)?;
                    let plot = self.graph_mut(graph).and_then(|g| g.layers[k].plots.last_mut()).expect("just added");
                    plot.kind = PlotKind::Histogram;
                    plot.hist = HistogramStyle::default();
                }
            } else {
                self.add_plot_xy(graph, k, Some(columns[c]), columns[r], None)?;
            }
        }
        if let Some(g) = self.graph_mut(graph) {
            for layer in &mut g.layers {
                layer.apply_palette();
            }
            if let Some(name) = long_name {
                g.long_name = name;
            }
        }
        Ok(graph)
    }
}

/// The most major steps across one cell: the automatic step is widened (1, 2, 5, 10, ...) until the variable's
/// range has at most this many, so that the tick labels of a small cell do not run into each other.
pub const MAX_STEPS: f64 = 5.0;

/// The limits and major step of a variable's axes: the automatic linear range of its finite values (the limits a
/// scatter of them gets) with the step widened to at most [`MAX_STEPS`] steps, or `None` when it has no finite
/// value. A constant variable gets one unit either side.
pub fn variable_range(values: &[f64], margin_pct: f64) -> Option<(f64, f64, f64)> {
    let finite = values.iter().copied().filter(|v| v.is_finite());
    let (lo, hi) = finite.fold(None, |acc: Option<(f64, f64)>, v| match acc {
        None => Some((v, v)),
        Some((lo, hi)) => Some((lo.min(v), hi.max(v))),
    })?;
    let (from, to, mut step) = if hi > lo { linear_range(lo, hi, margin_pct) } else { (lo - 1.0, hi + 1.0, 0.5) };
    while (to - from) / step > MAX_STEPS + 1e-9 {
        step = wider_step(step);
    }
    Some((from, to, step))
}

/// The next step of the 1, 2, 5, 10 series above `step`.
fn wider_step(step: f64) -> f64 {
    let p = 10f64.powf(step.log10().floor());
    let m = step / p;
    let next = if m < 1.5 {
        2.0
    } else if m < 3.5 {
        5.0
    } else {
        10.0
    };
    next * p
}

/// The variable's name in its diagonal cell: centred (in the middle, or near the top above a histogram), in a
/// size that fits the cell. Text labels are anchored at their start, so the centring uses an average glyph width of
/// 0.55 em.
fn name_label(layer: &crate::graph::Layer, name: &str, page_w: f64, diagonal: MatrixDiagonal) -> TextLabel {
    const EM: f64 = 0.55;
    let mut font = layer.new_label_font();
    let cell_w = (layer.frame.width * page_w).max(1.0);
    let chars = name.chars().count().max(1) as f64;
    // A quarter larger than the tick labels, smaller when needed to fit 80 % of the cell's width.
    font.size_pt = (1.25 * layer.x_axis.tick_font.size_pt).min(0.8 * cell_w / (chars * EM));
    let width = chars * EM * font.size_pt / cell_w;
    let y = if diagonal == MatrixDiagonal::Histogram { 0.1 } else { 0.5 };
    TextLabel::new(name, ((1.0 - width) / 2.0).max(0.0), y, font)
}

/// The template's one layer: Origin's default scatter symbol (a black square) in a box with inward ticks, no
/// titles and no legend, on a square page (Sheaf's choice).
pub(crate) fn template_layer(g: &mut crate::graph::Graph) {
    let side = g.page.height_pt;
    g.page.width_pt = side;
    let layer = &mut g.layers[0];
    let (left, right, top, bottom) = MARGINS;
    layer.frame = Frame {
        left: left / 100.0,
        top: top / 100.0,
        width: (100.0 - left - right) / 100.0,
        height: (100.0 - top - bottom) / 100.0,
    };
    layer.legend.visible = false;
    layer.default_symbol.size_pt = 6.0;
    for axis in [&mut layer.x_axis, &mut layer.y_axis] {
        axis.side = AxisSide::Normal;
        axis.show_title = false;
        axis.show_opposite = true;
        axis.opposite_ticks = false;
        axis.major_tick = TickDirection::In;
        axis.minor_tick = TickDirection::In;
        axis.minor_ticks = 0;
    }
}

#[cfg(test)]
mod tests;
