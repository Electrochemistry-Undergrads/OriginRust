//! Making a graph from a request, and styling it.
//!
//! The text of the errors names the command-line options (`--y2`, `--x-min`, ...): the options of
//! the command line and the fields of [`PlotSpec`] and [`Style`] have the same names.

use crate::pick::*;
use anyhow::{Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use sheaf_core::graph::{Graph, LineStyle, Stack, TextLabel, TickDirection, TickLabelSource, TrendKind, Trendline};
use sheaf_core::panels::{MAX_PANELS, PanelLayout, PanelOrder};
use sheaf_core::template::{STACK_AUTO_DEFAULT, Template};
use sheaf_core::{Color, ColumnId, Project, WindowId};

/// Whether the axis colors follow the plots.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AxisColors {
    Plot,
    Fixed,
}

/// The order of the panels of a multi-panel graph.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PanelOrderSetting {
    BottomUp,
    TopDown,
}

/// Which axes of a multi-panel graph are linked.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PanelLinkSetting {
    X,
    Y,
    Xy,
    None,
}

/// Axis, size, color, label and drawing settings applied to a graph.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Style {
    pub x_title: Option<String>,
    pub y_title: Option<String>,
    pub x_min: Option<f64>,
    pub x_max: Option<f64>,
    pub y_min: Option<f64>,
    pub y_max: Option<f64>,
    pub x_step: Option<f64>,
    pub y_step: Option<f64>,
    pub y2_title: Option<String>,
    pub y2_min: Option<f64>,
    pub y2_max: Option<f64>,
    pub y2_step: Option<f64>,
    pub axis_colors: Option<AxisColors>,
    /// Page width in inches.
    pub width: Option<f64>,
    /// Page height in inches.
    pub height: Option<f64>,
    /// Line width in points.
    pub line_width: Option<f64>,
    /// Connect the points of line and line+symbol plots in increasing X order (the worksheet is
    /// not sorted).
    pub sort_x: bool,
    /// Trendlines for every curve: linear, poly:N, power, log, exp or ma:N (replaces the curve's own).
    pub trendline: Vec<String>,
    /// Leave the equation and R-squared label off the trendlines.
    pub no_trendline_label: bool,
    pub colors: Vec<String>,
    pub labels: Vec<String>,
    pub no_legend: bool,
    /// none, auto, auto:GAP or step:STEP.
    pub stack: Option<String>,
    pub offsets: Vec<f64>,
    pub waterfall: Vec<f64>,
    pub no_y_tick_labels: bool,
    pub no_y_ticks: bool,
    /// auto, numbers or a column name.
    pub x_labels: Option<String>,
    pub x_label_angle: Option<f64>,
    pub bar_gap: Option<f64>,
    pub bar_overlap: Option<f64>,
    /// Hexbin plots: the number of bins along X (and, for Derive X from Y, along Y).
    pub hex_bins: Option<usize>,
    /// Treemaps: the layout (squarified, squarified-plus, split, pivot-size, pivot-middle, pivot-split-size).
    pub treemap_layout: Option<String>,
    /// Treemaps: the order of the rectangles (categorical, alphabetical, size, or column:NAME).
    pub treemap_order: Option<String>,
    /// Treemaps: the levels shown (default: all).
    pub treemap_depth: Option<usize>,
    /// Treemaps: the gap between rectangles in pt (default 1.5).
    pub treemap_gap: Option<f64>,
    /// Sunburst plots: the start angle in degrees counter-clockwise from 3 o'clock (default 90).
    pub sunburst_start: Option<f64>,
    /// Sunburst plots: the hole as a percentage of the radius, 0 to 90 (default 30).
    pub sunburst_hole: Option<f64>,
    /// Sunburst plots: the gap between rings in pt, 0 to 20 (default 0).
    pub sunburst_gap: Option<f64>,
    /// Sunburst plots: names, values, both or none.
    pub sunburst_labels: Option<String>,
    /// Sunburst plots: worksheet or size.
    pub sunburst_order: Option<String>,
    /// Text labels, each "TEXT@X,Y".
    pub label: Vec<String>,
    /// Shapes, each "KIND@X1,Y1,X2,Y2".
    pub draw: Vec<String>,
    /// Slope annotations on the first plot of the first layer, each "ROW_A,ROW_B" (worksheet rows from 1).
    pub slope: Vec<String>,
}

/// A graph with several panels.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PanelsSpec {
    /// `stack` or ROWSxCOLUMNS such as `2x2`.
    pub shape: String,
    pub panel_y: Vec<String>,
    pub per_panel: Vec<usize>,
    pub gap: Vec<f64>,
    pub margins: Vec<f64>,
    pub order: Option<PanelOrderSetting>,
    pub link: Option<PanelLinkSetting>,
}

/// What to plot and how it looks.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PlotSpec {
    /// A template name; the file's suggested one when absent.
    pub template: Option<String>,
    pub y: Vec<String>,
    pub y2: Vec<String>,
    pub book: Option<String>,
    pub sheet: Option<String>,
    /// Style an existing graph instead of making one.
    pub graph: Option<String>,
    pub panels: Option<PanelsSpec>,
    /// The project holds one workbook per input file, drawn together in one graph.
    pub overlay: bool,
    /// With `overlay`: draw only these workbooks (by name). Empty: every workbook of the project.
    pub overlay_books: Vec<String>,
    /// Parallel plots: a parallel index's group column (default: the first text column of the sheet) or a parallel
    /// plot's numeric colour column (default: the last Y column); `none` colours every line in the plot colour.
    pub group: Option<String>,
    pub style: Style,
}

impl PlotSpec {
    /// Refuses the combinations that the command line refuses with its option conflicts.
    pub fn validate(&self) -> Result<()> {
        if self.graph.is_some() && !self.y2.is_empty() {
            bail!("--graph cannot be combined with --y2");
        }
        if let Some(p) = &self.panels {
            if self.graph.is_some() || !self.y2.is_empty() {
                bail!("--panels cannot be combined with --graph or --y2");
            }
            if !p.panel_y.is_empty() && (!self.y.is_empty() || !p.per_panel.is_empty()) {
                bail!("--panel-y cannot be combined with --y or --per-panel");
            }
        }
        if !self.y2.is_empty() && self.template.as_deref().and_then(Template::from_name) == Some(Template::Lollipop) {
            bail!("a lollipop plot cannot have a right Y axis; leave out --y2 or choose another template");
        }
        if !self.y2.is_empty() && self.template.as_deref().and_then(Template::from_name).is_some_and(|t| t.is_violin())
        {
            bail!("a violin plot cannot have a right Y axis; leave out --y2 or choose another template");
        }
        if !self.y2.is_empty() && self.template.as_deref().and_then(Template::from_name) == Some(Template::Ridgeline) {
            bail!("a ridgeline chart cannot have a right Y axis; leave out --y2 or choose another template");
        }
        if self.template.as_deref().and_then(Template::from_name) == Some(Template::Hexbin) {
            if !self.y2.is_empty() {
                bail!("a hexbin plot cannot have a right Y axis; leave out --y2 or choose another template");
            }
            if self.panels.is_some() {
                bail!("a hexbin plot cannot be made in panels; leave out --panels or choose another template");
            }
        }
        if self.template.as_deref().and_then(Template::from_name).is_some_and(|t| t.is_parallel()) {
            if !self.y2.is_empty() {
                bail!("a parallel plot cannot have a right Y axis; leave out --y2 or choose another template");
            }
            if self.panels.is_some() {
                bail!("a parallel plot cannot be made in panels; leave out --panels or choose another template");
            }
        }
        let st = &self.style;
        let treemap_option = st.treemap_layout.is_some()
            || st.treemap_order.is_some()
            || st.treemap_depth.is_some()
            || st.treemap_gap.is_some();
        let is_treemap = self.template.as_deref().and_then(Template::from_name) == Some(Template::Treemap);
        if is_treemap {
            if !self.y2.is_empty() {
                bail!("a treemap cannot have a right Y axis; leave out --y2 or choose another template");
            }
            if self.panels.is_some() || self.graph.is_some() || self.overlay {
                bail!(
                    "a treemap cannot be made in panels, as an overlay or by styling a graph; leave those options out"
                );
            }
        } else if treemap_option {
            bail!("--treemap-layout, --treemap-order, --treemap-depth and --treemap-gap need -t treemap");
        }
        let sunburst_option = st.sunburst_start.is_some()
            || st.sunburst_hole.is_some()
            || st.sunburst_gap.is_some()
            || st.sunburst_labels.is_some()
            || st.sunburst_order.is_some();
        if self.template.as_deref().and_then(Template::from_name) == Some(Template::Sunburst) {
            if !self.y2.is_empty() {
                bail!("a sunburst plot cannot have a right Y axis; leave out --y2 or choose another template");
            }
            if self.panels.is_some() || self.graph.is_some() || self.overlay {
                bail!(
                    "a sunburst plot cannot be made in panels, as an overlay or by styling a graph; leave those options out"
                );
            }
        } else if sunburst_option {
            bail!(
                "--sunburst-start, --sunburst-hole, --sunburst-gap, --sunburst-labels and --sunburst-order need -t sunburst"
            );
        }
        if self.group.is_some() && (self.graph.is_some() || self.panels.is_some()) {
            bail!("--group cannot be combined with --graph or --panels");
        }
        if !self.y2.is_empty()
            && matches!(
                self.template.as_deref().and_then(Template::from_name),
                Some(Template::Waterfall | Template::WaterfallY | Template::WaterfallZ)
            )
        {
            bail!("a waterfall cannot have a right Y axis; leave out --y2 or choose another template");
        }
        if !self.y2.is_empty() && self.template.as_deref().and_then(Template::from_name) == Some(Template::StackedHistograms)
        {
            bail!("stacked histograms cannot have a right Y axis; leave out --y2 or choose another template");
        }
        Ok(())
    }
}

/// Makes the graph a request asks for, or finds the existing one it names, and styles it.
/// `suggested` is the template the data suggests, used when the request names none.
pub fn build_graph(project: &mut Project, a: &PlotSpec, suggested: Template) -> Result<WindowId> {
    a.validate()?;
    let graph = if let Some(name) = &a.graph {
        pick_graph(project, Some(name))?
    } else if let Some(p) = &a.panels {
        panel_graph(project, template_from(a.template.as_deref(), suggested)?, p, a)?
    } else if a.overlay {
        overlay_files(project, template_from(a.template.as_deref(), suggested)?, a)?
    } else {
        let template = template_from(a.template.as_deref(), suggested)?;
        let book = pick_book(project, a.book.as_deref())?;
        let sheet = pick_sheet(project, book, a.sheet.as_deref())?;
        let (left, right) = axis_columns(project, book, sheet, template, a)?;
        let ys: Vec<ColumnId> = left.iter().chain(&right).copied().collect();
        if ys.is_empty() {
            bail!("there are no numeric Y columns to plot");
        }
        if template.is_parallel() {
            parallel_graph(project, book, sheet, template, &ys, a.group.as_deref())?
        } else if template == Template::Treemap {
            treemap_graph(project, book, sheet, &ys, a.group.as_deref(), &a.style)?
        } else if template == Template::Sunburst {
            sunburst_graph(project, book, sheet, &ys, a.group.as_deref(), &a.style)?
        } else if a.group.is_some() {
            bail!("--group needs -t parallel, -t parallel-index, -t treemap or -t sunburst");
        } else if a.y2.is_empty() && template != Template::DoubleY {
            project.plot_columns(template, &ys)?
        } else {
            require_two_axes(&left, &right)?;
            project.plot_columns_double_y(template, &left, &right)?
        }
    };
    apply_style(project, graph, &a.style, a.panels.is_some())?;
    Ok(graph)
}

/// A Parallel Plot or Parallel Index of the numeric columns among `ys`; `group` names the group column of a parallel
/// index or the colour column of a parallel plot (`none`: the plot colour), see [`PlotSpec::group`].
fn parallel_graph(
    project: &mut Project,
    book: WindowId,
    sheet: usize,
    template: Template,
    ys: &[ColumnId],
    group: Option<&str>,
) -> Result<WindowId> {
    let Some(name) = group else {
        return Ok(project.plot_columns(template, ys)?);
    };
    let axes: Vec<ColumnId> = ys.iter().copied().filter(|y| project.numeric(*y).is_some()).collect();
    let color = if name.trim().eq_ignore_ascii_case("none") {
        None
    } else {
        let id = named_columns(project, book, sheet, &[name.to_string()])?[0];
        if template == Template::Parallel && project.numeric(id).is_none() {
            bail!(
                "--group {name}: the colour column of a parallel plot must be numeric (it colours the lines through a \
                 colour ramp); to group by a text column use -t parallel-index"
            );
        }
        Some(id)
    };
    Ok(project.plot_parallel(template, &axes, color)?)
}

/// A treemap: the last numeric column among `ys` is the value, the category columns (first = top level) are those
/// named by `group` (comma-separated), else the text columns among `ys`, else every text column of the sheet.
fn treemap_graph(
    project: &mut Project,
    book: WindowId,
    sheet: usize,
    ys: &[ColumnId],
    group: Option<&str>,
    style: &Style,
) -> Result<WindowId> {
    use sheaf_core::graph::{TreemapLayout, TreemapOrder};
    let value = ys.iter().rev().copied().find(|y| project.numeric(*y).is_some()).ok_or_else(|| {
        anyhow!("a treemap needs a numeric value column (the last numeric --y column; default: the last Y column)")
    })?;
    let mut categories: Vec<ColumnId> = match group {
        Some(g) => {
            let names: Vec<String> = g.split(',').map(|n| n.trim().to_string()).filter(|n| !n.is_empty()).collect();
            if names.is_empty() {
                bail!("--group needs one or more category column names, comma-separated");
            }
            named_columns(project, book, sheet, &names)?
        }
        None => ys.iter().copied().filter(|y| project.numeric(*y).is_none()).collect(),
    };
    if categories.is_empty() {
        categories = project.treemap_default_categories(value);
    }
    if categories.contains(&value) {
        bail!("the value column of a treemap cannot also be a category column");
    }
    let layout = match &style.treemap_layout {
        Some(name) => Some(TreemapLayout::from_name(name).ok_or_else(|| {
            let all: Vec<&str> = TreemapLayout::ALL.iter().map(|l| l.name()).collect();
            anyhow!("--treemap-layout {name}: not a layout (choose one of {})", all.join(", "))
        })?),
        None => None,
    };
    let (order, order_column) = match style.treemap_order.as_deref().map(str::trim) {
        None => (None, None),
        Some(o) if o.eq_ignore_ascii_case("categorical") => (Some(TreemapOrder::Categorical), None),
        Some(o) if o.eq_ignore_ascii_case("alphabetical") => (Some(TreemapOrder::Alphabetical), None),
        Some(o) if o.eq_ignore_ascii_case("size") => (Some(TreemapOrder::BySize), None),
        Some(o) => match o.strip_prefix("column:") {
            Some(name) => {
                (Some(TreemapOrder::ByColumn), Some(named_columns(project, book, sheet, &[name.to_string()])?[0]))
            }
            None => bail!("--treemap-order {o}: choose categorical, alphabetical, size or column:NAME"),
        },
    };
    if let Some(d) = style.treemap_depth
        && d == 0
    {
        bail!("--treemap-depth must be 1 or more (leave it out to show every level)");
    }
    if let Some(g) = style.treemap_gap
        && !(g.is_finite() && (0.0..=20.0).contains(&g))
    {
        bail!("--treemap-gap must be between 0 and 20 pt (got {g})");
    }
    let graph = project.plot_treemap(&categories, value)?;
    if let Some(p) = project.graph_mut(graph).and_then(|g| g.layers.first_mut()).and_then(|l| l.plots.first_mut()) {
        let t = &mut p.treemap;
        t.layout = layout.unwrap_or(t.layout);
        t.order = order.unwrap_or(t.order);
        t.order_column = order_column.or(t.order_column);
        t.depth = style.treemap_depth.map(|d| d as u32).or(t.depth);
        t.gap_pt = style.treemap_gap.unwrap_or(t.gap_pt);
    }
    Ok(graph)
}

/// A sunburst plot: the last numeric column among `ys` is the value, the category columns (first = innermost ring)
/// are those named by `group` (comma-separated), else the text columns among `ys`, else every text column of the sheet.
fn sunburst_graph(
    project: &mut Project,
    book: WindowId,
    sheet: usize,
    ys: &[ColumnId],
    group: Option<&str>,
    style: &Style,
) -> Result<WindowId> {
    use sheaf_core::graph::SunburstOrder;
    let value = ys.iter().rev().copied().find(|y| project.numeric(*y).is_some()).ok_or_else(|| {
        anyhow!(
            "a sunburst plot needs a numeric value column (the last numeric --y column; default: the last Y column)"
        )
    })?;
    let mut categories: Vec<ColumnId> = match group {
        Some(g) => {
            let names: Vec<String> = g.split(',').map(|n| n.trim().to_string()).filter(|n| !n.is_empty()).collect();
            if names.is_empty() {
                bail!("--group needs one or more category column names, comma-separated");
            }
            named_columns(project, book, sheet, &names)?
        }
        None => ys.iter().copied().filter(|y| project.numeric(*y).is_none()).collect(),
    };
    if categories.is_empty() {
        categories = project.treemap_default_categories(value);
    }
    if categories.contains(&value) {
        bail!("the value column of a sunburst plot cannot also be a category column");
    }
    if let Some(s) = style.sunburst_start
        && !s.is_finite()
    {
        bail!("--sunburst-start must be a number of degrees");
    }
    if let Some(h) = style.sunburst_hole
        && !(h.is_finite() && (0.0..=90.0).contains(&h))
    {
        bail!("--sunburst-hole must be between 0 and 90 percent (got {h})");
    }
    if let Some(g) = style.sunburst_gap
        && !(g.is_finite() && (0.0..=20.0).contains(&g))
    {
        bail!("--sunburst-gap must be between 0 and 20 pt (got {g})");
    }
    let (names, values) = match style.sunburst_labels.as_deref().map(str::trim) {
        None => (None, None),
        Some(l) if l.eq_ignore_ascii_case("names") => (Some(true), Some(false)),
        Some(l) if l.eq_ignore_ascii_case("values") => (Some(false), Some(true)),
        Some(l) if l.eq_ignore_ascii_case("both") => (Some(true), Some(true)),
        Some(l) if l.eq_ignore_ascii_case("none") => (Some(false), Some(false)),
        Some(l) => bail!("--sunburst-labels {l}: choose names, values, both or none"),
    };
    let order = match style.sunburst_order.as_deref().map(str::trim) {
        None => None,
        Some(o) if o.eq_ignore_ascii_case("worksheet") => Some(SunburstOrder::Worksheet),
        Some(o) if o.eq_ignore_ascii_case("size") => Some(SunburstOrder::BySize),
        Some(o) => bail!("--sunburst-order {o}: choose worksheet or size"),
    };
    let graph = project.plot_sunburst(&categories, value)?;
    if let Some(p) = project.graph_mut(graph).and_then(|g| g.layers.first_mut()).and_then(|l| l.plots.first_mut()) {
        let s = &mut p.sunburst;
        s.start_angle_deg = style.sunburst_start.unwrap_or(s.start_angle_deg);
        s.hole = style.sunburst_hole.map_or(s.hole, |h| h / 100.0);
        s.ring_gap_pt = style.sunburst_gap.unwrap_or(s.ring_gap_pt);
        s.show_names = names.unwrap_or(s.show_names);
        s.show_values = values.unwrap_or(s.show_values);
        s.order = order.unwrap_or(s.order);
    }
    Ok(graph)
}

/// Smallest and largest page side the program accepts, in inches (0.1 in is 7.2 pt).
const PAGE_SIDE_RANGE_IN: (f64, f64) = (0.1, 100.0);

/// Like [`require_positive`] for a page side, which must also lie within `PAGE_SIDE_RANGE_IN`,
/// so that a tiny or huge value is refused here and not later by the renderer.
fn require_page_size(option: &str, value: Option<f64>) -> Result<()> {
    require_positive(option, value)?;
    let (lo, hi) = PAGE_SIDE_RANGE_IN;
    match value {
        Some(v) if v < lo || v > hi => {
            bail!("{option} must be between {lo} and {hi} inches (got {v})")
        }
        _ => Ok(()),
    }
}

/// Fails when the two limits of an axis are the same number, which has no width to draw.
/// A lower limit above the upper one is allowed: it draws a reversed axis.
fn check_limits(axis: &str, min: Option<f64>, max: Option<f64>) -> Result<()> {
    for (name, v) in [("min", min), ("max", max)] {
        if matches!(v, Some(v) if !v.is_finite()) {
            bail!("--{axis}-{name} must be a finite number");
        }
    }
    if let (Some(a), Some(b)) = (min, max)
        && a == b
    {
        bail!("--{axis}-min and --{axis}-max are both {a}; the axis would have no length");
    }
    if let (Some(a), Some(b)) = (min, max)
        && !(b - a).is_finite()
    {
        bail!("--{axis}-min and --{axis}-max are too far apart ({a} and {b}); use limits closer together");
    }
    Ok(())
}

/// Applies the style options of the command line to a graph.
///
/// The options act on the first layer and the layers linked to it (its right Y axis, or the other
/// panels of a stack); with `every_panel` (a graph just made with `--panels`) they act on every
/// layer. Among those layers, the panels ([`Graph::panels`]) take `--x-*` and `--y-*` (with
/// `--x-labels`, `--x-label-angle`, `--stack`, `--no-y-tick-labels` and `--no-y-ticks`); a linked
/// X axis follows the first panel's. `--y2-*` set the first panel's right Y axis; `--bar-gap` and
/// `--bar-overlap` set every layer; colors, labels and line widths count the plots in legend order,
/// layer by layer. `--offsets` and `--waterfall` need a graph of one panel. Further layers of a
/// graph read from an Origin project are left alone.
pub fn apply_style(project: &mut Project, graph: WindowId, s: &Style, every_panel: bool) -> Result<()> {
    require_page_size("--width", s.width)?;
    require_page_size("--height", s.height)?;
    require_positive("--line-width", s.line_width)?;
    check_limits("x", s.x_min, s.x_max)?;
    check_limits("y", s.y_min, s.y_max)?;
    check_limits("y2", s.y2_min, s.y2_max)?;
    let colors: Vec<Color> =
        s.colors.iter().map(|c| c.parse::<Color>().map_err(|e| anyhow!("{e}"))).collect::<Result<_>>()?;
    let stack = stack_option(s)?;
    let trends: Vec<TrendKind> = s
        .trendline
        .iter()
        .map(|t| TrendKind::parse(t).map_err(|e| anyhow!("--trendline: {e}")))
        .collect::<Result<_>>()?;
    require_range("--x-label-angle", s.x_label_angle, -90.0, 90.0)?;
    require_range("--bar-gap", s.bar_gap, 0.0, 100.0)?;
    require_range("--bar-overlap", s.bar_overlap, -100.0, 100.0)?;
    if let Some(n) = s.hex_bins
        && !(2..=1000).contains(&n)
    {
        bail!("--hex-bins must be between 2 and 1000 (got {n})");
    }
    let labels: Vec<(String, f64, f64)> = s.label.iter().map(|v| parse_label(v)).collect::<Result<_>>()?;
    let shapes: Vec<sheaf_core::shape::Shape> = s.draw.iter().map(|v| parse_draw(v)).collect::<Result<_>>()?;
    let slopes: Vec<(usize, usize)> = s.slope.iter().map(|v| parse_slope(v)).collect::<Result<_>>()?;
    let slope_plot = project.graph(graph).and_then(|g| g.layers.first()).and_then(|l| l.plots.first()).cloned();
    if !slopes.is_empty() {
        let Some(plot) = &slope_plot else { bail!("--slope needs a plot in the first layer to mark the slope on") };
        let rows = project.column(plot.y).map_or(0, |c| c.len());
        for (v, &(a, b)) in s.slope.iter().zip(&slopes) {
            if a >= rows || b >= rows {
                bail!("--slope \"{v}\": the first plot has only {rows} rows");
            }
            for row in [a, b] {
                if let Err(why) = sheaf_plot::slope_check_point(project, graph, 0, 0, Some(row)) {
                    bail!("--slope \"{v}\": {why}");
                }
            }
        }
    }
    let x_labels = s.x_labels.as_deref().map(|text| x_label_source(project, graph, text)).transpose()?;
    let g = project.graph_mut(graph).ok_or_else(|| anyhow!("graph not found"))?;
    // The layers the options act on, in legend order, and the panels among them.
    let roots: Vec<usize> =
        if every_panel { (0..g.layers.len()).filter(|&i| g.root_of(i) == i).collect() } else { vec![0] };
    let styled: Vec<usize> = roots.iter().flat_map(|&r| g.group(r)).collect();
    let panels = g.panels();
    let mains: Vec<usize> = styled.iter().copied().filter(|i| panels.contains(i)).collect();
    check_stack_fits(g, s, mains.len())?;
    let right = g.right_y_member(0);
    let wants_y2 = s.y2_title.is_some() || s.y2_min.is_some() || s.y2_max.is_some() || s.y2_step.is_some();
    if wants_y2 && right.is_none() {
        bail!(
            "the graph has no right Y axis for --y2-title, --y2-min, --y2-max or --y2-step; \
             make one with --y2 COLUMNS or -t double-y"
        );
    }
    if let Some(w) = s.width {
        g.page.width_pt = w * 72.0;
    }
    if let Some(h) = s.height {
        g.page.height_pt = h * 72.0;
    }
    if let Some(r) = right {
        let y2 = &mut g.layers[r].y_axis;
        if let Some(t) = &s.y2_title {
            y2.title = Some(t.clone());
        }
        y2.min = s.y2_min.or(y2.min);
        y2.max = s.y2_max.or(y2.max);
        y2.major_step = s.y2_step.or(y2.major_step);
    }
    if let Some(mode) = s.axis_colors {
        for &i in &styled {
            g.layers[i].y_axis.color_follows_plot = mode == AxisColors::Plot;
        }
    }
    for &i in &styled {
        let bars = &mut g.layers[i].bars;
        bars.gap_pct = s.bar_gap.unwrap_or(bars.gap_pct);
        bars.overlap_pct = s.bar_overlap.unwrap_or(bars.overlap_pct);
    }
    if let Some(n) = s.hex_bins {
        for &i in &styled {
            for p in g.layers[i].plots.iter_mut().filter(|p| p.kind.is_hexbin()) {
                p.hexbin.x.by = sheaf_core::graph::HexbinBy::Bins(n as u32);
                p.hexbin.y.by = sheaf_core::graph::HexbinBy::Bins(n as u32);
            }
        }
    }
    style_plots(g, &styled, s, &colors, &trends);
    for &i in &mains {
        let layer = &mut g.layers[i];
        if let Some(t) = &s.x_title {
            layer.x_axis.title = Some(t.clone());
        }
        if let Some(t) = &s.y_title {
            layer.y_axis.title = Some(t.clone());
        }
        layer.x_axis.min = s.x_min.or(layer.x_axis.min);
        layer.x_axis.max = s.x_max.or(layer.x_axis.max);
        layer.y_axis.min = s.y_min.or(layer.y_axis.min);
        layer.y_axis.max = s.y_max.or(layer.y_axis.max);
        layer.x_axis.major_step = s.x_step.or(layer.x_axis.major_step);
        layer.y_axis.major_step = s.y_step.or(layer.y_axis.major_step);
        if let Some(stack) = stack {
            layer.stack = stack;
        }
        if s.no_y_tick_labels {
            layer.y_axis.show_tick_labels = false;
        }
        if s.no_y_ticks {
            layer.y_axis.major_tick = TickDirection::None;
            layer.y_axis.minor_tick = TickDirection::None;
        }
        if let Some(source) = x_labels {
            layer.x_axis.tick_label_source = source;
        }
        layer.x_axis.tick_label_angle_deg = s.x_label_angle.unwrap_or(layer.x_axis.tick_label_angle_deg);
    }
    if !s.offsets.is_empty()
        && let Some(layer) = g.layers.first_mut()
    {
        layer.stack = Stack::Individual;
        for (column, y) in stack_members(layer).into_iter().zip(&s.offsets) {
            layer.plot_offset_mut(column).y = *y;
        }
    }
    if let Some(layer) = g.layers.first_mut() {
        layer.shapes.extend(shapes);
        if let Some(plot) = &slope_plot {
            layer.slopes.extend(slopes.iter().map(|&(a, b)| sheaf_core::graph::SlopeAnnotation::for_plot(plot, a, b)));
        }
        for (text, x, y) in labels {
            let font = layer.new_label_font();
            layer.labels.push(TextLabel::new(text, x, y, font));
        }
    }
    Ok(())
}

/// Reads one `--draw` value, "KIND@X1,Y1,X2,Y2".
pub fn parse_draw(value: &str) -> Result<sheaf_core::shape::Shape> {
    use sheaf_core::shape::{Shape, ShapeKind};
    let usage = "--draw must look like \"KIND@X1,Y1,X2,Y2\" with KIND line, arrow, rectangle or ellipse and four fractions of the layer frame, for example \"arrow@0.2,0.2,0.5,0.4\"";
    let Some((kind, at)) = value.split_once('@') else {
        bail!("--draw \"{value}\": there is no @ after the kind; {usage}");
    };
    let kind = match kind.trim().to_ascii_lowercase().as_str() {
        "line" => ShapeKind::Line,
        "arrow" => ShapeKind::Arrow,
        "rectangle" | "rect" => ShapeKind::Rectangle,
        "ellipse" | "circle" => ShapeKind::Ellipse,
        other => bail!("--draw \"{value}\": \"{other}\" is not a kind; {usage}"),
    };
    let n: Vec<f64> = at.split(',').filter_map(|v| v.trim().parse::<f64>().ok().filter(|v| v.is_finite())).collect();
    if n.len() != 4 || at.split(',').count() != 4 {
        bail!("--draw \"{value}\": \"{at}\" is not four numbers X1,Y1,X2,Y2; {usage}");
    }
    Ok(Shape::new(kind, (n[0], n[1]), (n[2], n[3])))
}

/// Reads one `--slope` value, "ROW_A,ROW_B": two different row numbers as the worksheet shows them (from 1);
/// the result counts from 0.
pub fn parse_slope(value: &str) -> Result<(usize, usize)> {
    let usage = "--slope must look like \"ROW_A,ROW_B\" with two different worksheet row numbers (counted from 1), for example --slope 3,8";
    let row = |s: &str| s.trim().parse::<usize>().ok().filter(|&r| r >= 1);
    let Some((a, b)) = value.split_once(',').and_then(|(a, b)| Some((row(a)?, row(b)?))) else {
        bail!("--slope \"{value}\": not two row numbers; {usage}");
    };
    if a == b {
        bail!("--slope \"{value}\": the two rows are the same; {usage}");
    }
    Ok((a - 1, b - 1))
}

/// Reads one `--label` value, "TEXT@X,Y": the text up to the last `@`, then two fractions of the
/// layer frame.
pub fn parse_label(value: &str) -> Result<(String, f64, f64)> {
    let usage = "--label must look like \"TEXT@X,Y\", with X and Y fractions of the layer frame (0,0 is the top left corner), for example \"Zn-LMO@0.85,0.1\"";
    let Some((text, at)) = value.rsplit_once('@') else {
        bail!("--label \"{value}\": there is no @ before the position; {usage}");
    };
    if text.is_empty() {
        bail!("--label \"{value}\": the text is empty; {usage}");
    }
    let number = |s: &str| s.trim().parse::<f64>().ok().filter(|v| v.is_finite());
    let Some((x, y)) = at.split_once(',').and_then(|(x, y)| Some((number(x)?, number(y)?))) else {
        bail!("--label \"{value}\": the position \"{at}\" is not two numbers X,Y; {usage}");
    };
    Ok((text.to_string(), x, y))
}

/// Fails with a clear message unless `value` (when given) is a number from `lo` to `hi`.
fn require_range(option: &str, value: Option<f64>, lo: f64, hi: f64) -> Result<()> {
    match value {
        Some(v) if !(v.is_finite() && (lo..=hi).contains(&v)) => {
            bail!("{option} must be between {lo} and {hi} (got {v})")
        }
        _ => Ok(()),
    }
}

/// Reads the value of `--x-labels`: `auto`, `numbers`, or the short or long name of a column in the
/// sheet that holds the Y column of the graph's first curve.
fn x_label_source(project: &Project, graph: WindowId, text: &str) -> Result<TickLabelSource> {
    let name = text.trim();
    if name.eq_ignore_ascii_case("auto") {
        return Ok(TickLabelSource::Auto);
    }
    if name.eq_ignore_ascii_case("numbers") {
        return Ok(TickLabelSource::Numbers);
    }
    let first = project
        .graph(graph)
        .and_then(|g| g.layers.first())
        .and_then(|l| l.plots.first())
        .ok_or_else(|| anyhow!("--x-labels {name}: the graph has no curve whose sheet holds the column"))?;
    let (_, sheet, _) = project
        .locate(first.y)
        .ok_or_else(|| anyhow!("--x-labels {name}: the first curve's column is missing from the project"))?;
    match sheet.column_by_name(name) {
        Some(c) => Ok(TickLabelSource::Column { column: c.id }),
        None => {
            let have: Vec<&str> = sheet.columns.iter().map(|c| c.short_name.as_str()).collect();
            bail!(
                "--x-labels: no column named '{name}' in sheet '{}' (columns: {}); use auto, numbers or a column name",
                sheet.name,
                have.join(", ")
            )
        }
    }
}

/// The stack mode asked for with `--stack` or `--waterfall`, checked (`--offsets` is applied
/// separately because it also sets offsets).
fn stack_option(s: &Style) -> Result<Option<Stack>> {
    if let Some(text) = &s.stack {
        return parse_stack(text).map(Some);
    }
    if s.waterfall.is_empty() {
        return Ok(None);
    }
    let [dx, dy] = s.waterfall[..] else {
        bail!("--waterfall needs two numbers, DX,DY in percent (got {} number(s))", s.waterfall.len());
    };
    for v in [dx, dy] {
        if !(v.is_finite() && (-100.0..=100.0).contains(&v)) {
            bail!("--waterfall takes percentages from -100 to 100 (got {v})");
        }
    }
    Ok(Some(Stack::Waterfall { dx_percent: dx, dy_percent: dy }))
}

/// Reads the value of `--stack`: `none`, `cumulative`, `auto`, `auto:GAP` or `step:STEP`.
pub fn parse_stack(text: &str) -> Result<Stack> {
    let t = text.trim().to_ascii_lowercase();
    let number = |v: &str| v.trim().parse::<f64>().ok().filter(|v| v.is_finite());
    if t == "none" {
        return Ok(Stack::None);
    }
    if t == "cumulative" {
        return Ok(Stack::Cumulative);
    }
    if t == "auto" {
        return Ok(STACK_AUTO_DEFAULT);
    }
    if let Some(gap) = t.strip_prefix("auto:") {
        return match number(gap) {
            Some(g) if g >= 0.0 => Ok(Stack::Auto { gap_percent: g }),
            _ => bail!("--stack auto:GAP needs a gap in percent that is a number of at least 0 (got '{gap}')"),
        };
    }
    if let Some(step) = t.strip_prefix("step:") {
        return match number(step) {
            Some(v) => Ok(Stack::Constant { step: v }),
            None => bail!("--stack step:STEP needs a step that is a number (got '{step}')"),
        };
    }
    bail!("--stack must be none, cumulative, auto, auto:GAP or step:STEP (got '{text}')")
}

/// The Y columns that have a place of their own in a layer's stack, in plot order: every plotted
/// column once, except curves that follow another one (a fitted curve drawn over its data).
/// `--offsets` gives its values to these, so a fit curve does not take one.
fn stack_members(layer: &sheaf_core::graph::Layer) -> Vec<ColumnId> {
    let mut out: Vec<ColumnId> = Vec::new();
    for p in &layer.plots {
        if layer.follows(p.y).is_none() && !out.contains(&p.y) {
            out.push(p.y);
        }
    }
    out
}

/// Refuses stack options that do not fit the graph: `--offsets` or `--waterfall` on a graph of
/// several panels (`panels` is the number of panels the options act on), more `--offsets` than
/// the first layer has curves, and a waterfall in a graph with a right Y axis (a waterfall is drawn
/// only for a layer on its own).
fn check_stack_fits(g: &Graph, s: &Style, panels: usize) -> Result<()> {
    if panels > 1 && (!s.offsets.is_empty() || !s.waterfall.is_empty()) {
        bail!("--offsets and --waterfall work on a graph of one panel; this graph has {panels} panels");
    }
    let plots = g.layers.first().map_or(0, |l| stack_members(l).len());
    if s.offsets.len() > plots {
        bail!("--offsets gives {} values but the graph has {} curve(s)", s.offsets.len(), plots);
    }
    if !s.waterfall.is_empty() && g.group(0).len() > 1 {
        bail!("--waterfall cannot be used on a graph with a right Y axis");
    }
    Ok(())
}

/// Line widths, colors, labels and `--no-legend` for the plots of the layers in `group`, counted
/// in legend order (layer by layer, plots in order).
fn style_plots(g: &mut Graph, group: &[usize], s: &Style, colors: &[Color], trends: &[TrendKind]) {
    let mut i = 0;
    for &li in group {
        let layer = &mut g.layers[li];
        if s.no_legend {
            layer.legend.visible = false;
        }
        let followers: Vec<bool> = layer.plots.iter().map(|p| layer.follows(p.y).is_some()).collect();
        for (k, plot) in layer.plots.iter_mut().enumerate() {
            // A fitted curve drawn over its data gets no trendline of its own.
            if !trends.is_empty() && !followers[k] {
                plot.decor.trendlines = trends
                    .iter()
                    .map(|&kind| {
                        let mut t = Trendline::new(kind);
                        t.show_equation = !s.no_trendline_label;
                        t.show_r2 = !s.no_trendline_label;
                        t
                    })
                    .collect();
            }
            if let Some(w) = s.line_width {
                plot.line = LineStyle { width_pt: w, ..plot.line.clone() };
            }
            if s.sort_x {
                plot.line.sort_x = true;
            }
            if let Some(c) = colors.get(i) {
                // A column plot's color is its fill; its outline keeps its own color.
                if !plot.kind.is_bar() {
                    plot.line.color = *c;
                }
                plot.symbol.color = *c;
            }
            if let Some(l) = s.labels.get(i) {
                plot.label = Some(l.clone());
            }
            i += 1;
        }
    }
}
pub fn template_from(name: Option<&str>, fallback: Template) -> Result<Template> {
    match name {
        None => Ok(fallback),
        Some(n) => Template::from_name(n).ok_or_else(|| {
            let names: Vec<&str> = Template::ALL.iter().map(Template::name).collect();
            anyhow!("unknown template '{}'; choose one of: {}", n, names.join(", "))
        }),
    }
}
/// The shape asked for with `--panels`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PanelShape {
    Stack,
    Grid { rows: usize, cols: usize },
}

/// Reads the value of `--panels`: `stack`, or ROWSxCOLUMNS such as `2x2`.
pub fn parse_panels(text: &str) -> Result<PanelShape> {
    let t = text.trim().to_ascii_lowercase();
    if t == "stack" {
        return Ok(PanelShape::Stack);
    }
    let bad = || anyhow!("--panels must be 'stack' or ROWSxCOLUMNS such as 2x2 (got '{text}')");
    let (r, c) = t.split_once('x').ok_or_else(bad)?;
    let rows: usize = r.trim().parse().map_err(|_| bad())?;
    let cols: usize = c.trim().parse().map_err(|_| bad())?;
    if rows == 0 || cols == 0 || rows.saturating_mul(cols) > MAX_PANELS {
        bail!("--panels {text}: rows x columns must be from 1 to {MAX_PANELS} panels (got {rows} x {cols})");
    }
    Ok(PanelShape::Grid { rows, cols })
}

/// The curves of each panel, before `--per-panel`, and the legend text of the curves that need
/// one (several files: the file name).
///
/// With several files, each file is one group (its `-y` or default columns), or with `--panel-y`
/// each group takes the named columns of every file. With one input, each plotted Y column (`-y`
/// or the default columns) is a group, or `--panel-y` names the columns of each group.
fn panel_groups(
    project: &Project,
    template: Template,
    p: &PanelsSpec,
    a: &PlotSpec,
) -> Result<(Vec<Vec<ColumnId>>, Vec<(ColumnId, String)>)> {
    let split = |spec: &String| -> Vec<String> { spec.split(',').map(|n| n.trim().to_string()).collect() };
    let mut labels = Vec::new();
    if a.overlay {
        let books = overlay_book_ids(project, a);
        let mut sheets = Vec::new();
        for &book in &books {
            sheets.push((book, pick_sheet(project, book, a.sheet.as_deref())?));
        }
        let mut groups = Vec::new();
        if p.panel_y.is_empty() {
            for &(book, sheet) in &sheets {
                let ys = if a.y.is_empty() {
                    project.default_plot_columns(book, sheet, template)
                } else {
                    named_columns(project, book, sheet, &a.y)?
                };
                labels.extend(ys.iter().copied().zip(overlay_labels(project, book, &ys)));
                groups.push(ys);
            }
        } else {
            for spec in &p.panel_y {
                let mut group = Vec::new();
                for &(book, sheet) in &sheets {
                    let ys = named_columns(project, book, sheet, &split(spec))?;
                    labels.extend(ys.iter().copied().zip(overlay_labels(project, book, &ys)));
                    group.extend(ys);
                }
                groups.push(group);
            }
        }
        return Ok((groups, labels));
    }
    let book = pick_book(project, a.book.as_deref())?;
    let sheet = pick_sheet(project, book, a.sheet.as_deref())?;
    if !p.panel_y.is_empty() {
        let groups =
            p.panel_y.iter().map(|spec| named_columns(project, book, sheet, &split(spec))).collect::<Result<_>>()?;
        return Ok((groups, labels));
    }
    let ys = if a.y.is_empty() {
        project.default_plot_columns(book, sheet, template)
    } else {
        named_columns(project, book, sheet, &a.y)?
    };
    Ok((ys.into_iter().map(|y| vec![y]).collect(), labels))
}

/// Puts the curves of `groups`, in order, into panels of `--per-panel` curves each.
fn regroup(groups: Vec<Vec<ColumnId>>, per_panel: &[usize]) -> Result<Vec<Vec<ColumnId>>> {
    if per_panel.is_empty() {
        return Ok(groups);
    }
    if per_panel.contains(&0) {
        bail!("--per-panel counts must be 1 or more");
    }
    let curves: Vec<ColumnId> = groups.into_iter().flatten().collect();
    let sum: usize = per_panel.iter().sum();
    if sum != curves.len() {
        bail!("--per-panel counts add up to {sum} but there are {} curves", curves.len());
    }
    let mut rest = &curves[..];
    Ok(per_panel
        .iter()
        .map(|&n| {
            let (head, tail) = rest.split_at(n);
            rest = tail;
            head.to_vec()
        })
        .collect())
}

/// The layout of `--panels` with the `--panel-*` options applied.
fn panel_layout(shape: PanelShape, panels: usize, template: Template, p: &PanelsSpec) -> Result<PanelLayout> {
    let mut layout = match shape {
        PanelShape::Stack => {
            if panels > MAX_PANELS {
                bail!("--panels stack can hold at most {MAX_PANELS} panels, but there are {panels} to stack");
            }
            PanelLayout::stack(panels)
        }
        PanelShape::Grid { rows, cols } => {
            PanelLayout::grid(rows, cols, &template.new_graph(WindowId(0), "").layers[0].frame)
        }
    };
    let check = |option: &str, v: f64, hi: f64| -> Result<f64> {
        if v.is_finite() && (0.0..=hi).contains(&v) {
            Ok(v)
        } else {
            bail!("{option} must be from 0 to {hi} percent of the page (got {v})")
        }
    };
    match p.gap[..] {
        [] => {}
        [both] => {
            layout.h_gap = check("--panel-gap", both, 50.0)?;
            layout.v_gap = layout.h_gap;
        }
        [h, v] => {
            layout.h_gap = check("--panel-gap", h, 50.0)?;
            layout.v_gap = check("--panel-gap", v, 50.0)?;
        }
        _ => bail!("--panel-gap takes one number, or two as H,V (got {} numbers)", p.gap.len()),
    }
    match p.margins[..] {
        [] => {}
        [l, r, t, b] => {
            layout.margin_left = check("--panel-margins", l, 90.0)?;
            layout.margin_right = check("--panel-margins", r, 90.0)?;
            layout.margin_top = check("--panel-margins", t, 90.0)?;
            layout.margin_bottom = check("--panel-margins", b, 90.0)?;
        }
        _ => bail!("--panel-margins takes four numbers, L,R,T,B (got {} numbers)", p.margins.len()),
    }
    if let Some(order) = p.order {
        layout.order = match order {
            PanelOrderSetting::BottomUp => PanelOrder::BottomUp,
            PanelOrderSetting::TopDown => PanelOrder::TopDown,
        };
    }
    if let Some(link) = p.link {
        (layout.link_x, layout.link_y) = match link {
            PanelLinkSetting::X => (true, false),
            PanelLinkSetting::Y => (false, true),
            PanelLinkSetting::Xy => (true, true),
            PanelLinkSetting::None => (false, false),
        };
    }
    Ok(layout)
}

/// One graph with several panels (`--panels`), graphing spec section 15.6.
fn panel_graph(project: &mut Project, template: Template, p: &PanelsSpec, a: &PlotSpec) -> Result<WindowId> {
    let shape = parse_panels(&p.shape)?;
    if matches!(
        template,
        Template::DoubleY
            | Template::Waterfall
            | Template::WaterfallY
            | Template::WaterfallZ
            | Template::Lollipop
            | Template::StackedHistograms
    ) {
        bail!("the {} template cannot be used with --panels; choose another template", template.name());
    }
    let (groups, labels) = panel_groups(project, template, p, a)?;
    if groups.iter().all(|g| g.is_empty()) {
        bail!("there are no numeric Y columns to plot");
    }
    let groups = regroup(groups, &p.per_panel)?;
    let layout = panel_layout(shape, groups.len(), template, p)?;
    let graph = project.plot_panels(template, &groups, &layout)?;
    if let Some(g) = project.graph_mut(graph) {
        for plot in g.layers.iter_mut().flat_map(|l| l.plots.iter_mut()) {
            if let Some((_, label)) = labels.iter().find(|(c, _)| *c == plot.y) {
                plot.label = Some(label.clone());
            }
        }
    }
    Ok(graph)
}

/// The Y columns of one sheet for the left and the right axis.
///
/// With `--y2`, the right axis gets those columns and the left axis the `-y` columns (or the
/// default columns without the `--y2` ones). With the double-y template and no `--y2`, the
/// columns (`-y`, or every numeric Y column) alternate: 1st, 3rd, ... left; 2nd, 4th, ... right.
/// Otherwise every column is on the left axis.
fn axis_columns(
    project: &Project,
    book: WindowId,
    sheet: usize,
    template: Template,
    a: &PlotSpec,
) -> Result<(Vec<ColumnId>, Vec<ColumnId>)> {
    let named = |names: &[String]| named_columns(project, book, sheet, names);
    if a.y2.is_empty() {
        let ys = if a.y.is_empty() { project.default_plot_columns(book, sheet, template) } else { named(&a.y)? };
        if template == Template::DoubleY {
            return Ok(alternate(&ys));
        }
        return Ok((ys, Vec::new()));
    }
    let right = named(&a.y2)?;
    let left = if a.y.is_empty() {
        let mut ys = project.default_plot_columns(book, sheet, template);
        ys.retain(|c| !right.contains(c));
        ys
    } else {
        named(&a.y)?
    };
    if left.iter().any(|c| right.contains(c)) {
        bail!("a column cannot be on both axes; a column named with --y2 must not be named with -y");
    }
    Ok((left, right))
}

/// Splits columns as Origin's Double-Y command does: 1st, 3rd, ... left; 2nd, 4th, ... right.
fn alternate(ys: &[ColumnId]) -> (Vec<ColumnId>, Vec<ColumnId>) {
    (ys.iter().step_by(2).copied().collect(), ys.iter().skip(1).step_by(2).copied().collect())
}

/// A double-Y graph needs a curve for each axis.
fn require_two_axes(left: &[ColumnId], right: &[ColumnId]) -> Result<()> {
    if left.is_empty() || right.is_empty() {
        bail!(
            "a double-Y graph needs at least two Y columns, one for each axis (left: {}, right: {})",
            left.len(),
            right.len()
        );
    }
    Ok(())
}

/// The legend text of each column of one file in an overlay: the file name, plus the column when
/// the file gives several curves.
fn overlay_labels(project: &Project, book: WindowId, columns: &[ColumnId]) -> Vec<String> {
    let file = project.workbook(book).map(|b| b.long_name.clone()).unwrap_or_default();
    columns
        .iter()
        .map(|y| {
            let column = project
                .column(*y)
                .map(|c| c.comments.lines().next().filter(|l| !l.is_empty()).unwrap_or(c.display_name()).to_string());
            match (columns.len(), column) {
                (1, _) | (_, None) => file.clone(),
                (_, Some(c)) => format!("{file} {c}"),
            }
        })
        .collect()
}

/// One graph with the chosen (or default) Y columns of every imported file.
/// Legend entries name the file, plus the column when a file gives several curves.
/// With `--y2` (or the double-y template) each file's columns are split between the axes as for a
/// single file, and the right axis lists its curves after the left axis' curves.
/// The workbooks an overlay draws: the ones named in `overlay_books`, else all.
fn overlay_book_ids(project: &Project, a: &PlotSpec) -> Vec<WindowId> {
    project
        .workbooks()
        .filter(|b| a.overlay_books.is_empty() || a.overlay_books.iter().any(|n| n.eq_ignore_ascii_case(&b.name)))
        .map(|b| b.id)
        .collect()
}

fn overlay_files(project: &mut Project, template: Template, a: &PlotSpec) -> Result<WindowId> {
    let books = overlay_book_ids(project, a);
    let (mut left, mut right) = (Vec::new(), Vec::new());
    let (mut left_labels, mut right_labels) = (Vec::new(), Vec::new());
    for book in books {
        let sheet = pick_sheet(project, book, a.sheet.as_deref())?;
        let (l, r) = axis_columns(project, book, sheet, template, a)?;
        // Labels are made over the file's columns together, so that a file with one curve per
        // axis still names the column.
        let all: Vec<ColumnId> = l.iter().chain(&r).copied().collect();
        let labels = overlay_labels(project, book, &all);
        left_labels.extend_from_slice(&labels[..l.len()]);
        right_labels.extend_from_slice(&labels[l.len()..]);
        left.extend(l);
        right.extend(r);
    }
    if left.is_empty() && right.is_empty() {
        bail!("there are no numeric Y columns to plot");
    }
    let two_axes = !a.y2.is_empty() || template == Template::DoubleY;
    let graph = if two_axes {
        require_two_axes(&left, &right)?;
        project.plot_columns_double_y(template, &left, &right)?
    } else {
        project.plot_columns(template, &left)?
    };
    if let Some(g) = project.graph_mut(graph) {
        let right_layer = g.right_y_member(0);
        let mut set = |layer: Option<usize>, labels: Vec<String>| {
            if let Some(layer) = layer.and_then(|i| g.layers.get_mut(i)) {
                for (plot, label) in layer.plots.iter_mut().zip(labels) {
                    plot.label = Some(label);
                }
            }
        };
        set(Some(0), left_labels);
        set(right_layer.filter(|_| two_axes), right_labels);
    }
    Ok(graph)
}
