//! Graphs with three or more Y axes over one X axis (T61, graphing spec 21.5 and 21.6): Origin's
//! "3Ys Y-YY", "3Ys Y-Y-Y", "4Ys Y-YYY" and "4Ys YY-YY" templates and the Multiple Y Axes dialog.
//!
//! Every Y axis is one layer, linked to the first layer's X scale, with its own Y scale. A plan
//! ([`MultiYPlan`]) says where each axis stands and in which colour; the templates and the dialog
//! options ([`MultiYOptions`]) both make plans, and [`MultiYPlan::graph`] turns a plan into an empty graph.

use crate::color::Color;
use crate::graph::*;
use crate::ids::WindowId;
use crate::template::{ORIGIN_COLOR_LIST, Template, right_y_layer};

/// Origin's frame of a plain Line graph as fractions of the page: top, width, height (the left edge is
/// computed from the axes).
/// The left of the frame of a plain Line graph, which a single Y axis keeps (graphing spec 21.6, 17.91 %).
const ORDINARY_FRAME_LEFT: f64 = 0.1791;
const LINE_FRAME: (f64, f64, f64) = (0.1158, 0.6816, 0.7183);
/// Default distance between neighbouring Y axes, in percent of the X axis length (21.6).
pub const DEFAULT_GAP_PCT: f64 = 10.0;
/// The gap Origin chooses on its own when Y titles or the top X axis are on (21.6).
pub const TITLED_GAP_PCT: f64 = 14.0;
/// Where "Add Middle Y Axis" puts the extra left axis: the middle of the plot.
pub const MIDDLE_OFFSET_PCT: f64 = -50.0;

/// The four named templates of Plot > Multi-Y that have more than two Y axes.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum MultiYKind {
    /// 3Ys Y-YY: one left axis, two right ones.
    ThreeYYy,
    /// 3Ys Y-Y-Y: a left axis, a left axis in the middle of the plot and a right axis.
    ThreeYYY,
    /// 4Ys Y-YYY: one left axis, three right ones.
    FourYYyy,
    /// 4Ys YY-YY: two left axes and two right ones.
    FourYyYy,
}

/// One Y axis of a plan.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct YAxisPlan {
    /// `Normal` is the left edge, `Opposite` the right edge.
    pub side: AxisSide,
    /// Distance from the frame in percent of its width, outward positive.
    pub offset_pct: f64,
    /// Colour of the layer's first plot and so of its axis.
    pub color: Color,
}

/// Where the Y axes stand and how the graph looks; see the module description.
#[derive(Clone, Debug, PartialEq)]
pub struct MultiYPlan {
    /// The first layer's frame as fractions of the page.
    pub frame: Frame,
    /// One entry per layer, in layer order.
    pub axes: Vec<YAxisPlan>,
    /// Axis titles on the Y axes (the templates show none).
    pub y_titles: bool,
    /// Axis line, ticks and labels in the colour of the layer's plot.
    pub axis_color: bool,
    /// A top X axis line with ticks on the first layer.
    pub top_x: bool,
    /// The legend of the first layer.
    pub legend: LegendPosition,
}

const fn axis(side: AxisSide, offset_pct: f64, rgb: (u8, u8, u8)) -> YAxisPlan {
    YAxisPlan { side, offset_pct, color: Color::rgb(rgb.0, rgb.1, rgb.2) }
}

const BLACK: (u8, u8, u8) = (0, 0, 0);
const RED: (u8, u8, u8) = (255, 0, 0);
const BLUE: (u8, u8, u8) = (0, 0, 255);
const GREEN: (u8, u8, u8) = (0, 128, 0);
const ORANGE: (u8, u8, u8) = (255, 128, 0);
const MAGENTA: (u8, u8, u8) = (255, 0, 255);

/// Origin 2016's automatic stock legend lists the first plot from each layer before
/// appending the other plots in the legend's own layer (graphing 21.12).
pub(crate) fn stock_legend_order(owner: usize, ids: &[String]) -> Vec<usize> {
    let owner = (owner + 1).to_string();
    let mut first = Vec::new();
    let mut extras = Vec::new();
    for (i, id) in ids.iter().enumerate() {
        match id.split_once('.') {
            None | Some((_, "1")) => first.push(i),
            Some((layer, _)) if layer == owner => extras.push(i),
            Some(_) => {}
        }
    }
    first.extend(extras);
    first
}

impl Layer {
    /// Model slots of the plotted subset, in model order. A hidden plot keeps its slot
    /// when a stock legend looks up the first plot of a linked layer.
    pub fn stock_legend_plot_slots<'a>(&'a self, drawn: impl IntoIterator<Item = &'a Plot>) -> Vec<usize> {
        let mut drawn = drawn.into_iter().peekable();
        self.plots
            .iter()
            .enumerate()
            .filter_map(|(slot, plot)| {
                if drawn.peek().is_some_and(|&next| std::ptr::eq(plot, next)) {
                    let _ = drawn.next();
                    Some(slot)
                } else {
                    None
                }
            })
            .collect()
    }
}

impl Legend {
    /// Indices into the supplied drawable plot IDs in automatic legend order.
    pub fn automatic_entry_order(&self, owner: usize, ids: &[String]) -> Vec<usize> {
        if self.stock_multi_y { stock_legend_order(owner, ids) } else { (0..ids.len()).collect() }
    }

    /// The reconstructible legend text, including each selected plot's symbol and label.
    pub fn default_text(&self, owner: usize, ids: &[String]) -> String {
        if !self.stock_multi_y {
            return Self::automatic_text(ids);
        }
        let indices = self.automatic_entry_order(owner, ids);
        let displayed: Vec<_> = indices.into_iter().map(|index| ids[index].as_str()).collect();
        Self::automatic_text(&displayed)
    }
}

impl MultiYKind {
    pub const ALL: [MultiYKind; 4] =
        [MultiYKind::ThreeYYy, MultiYKind::ThreeYYY, MultiYKind::FourYYyy, MultiYKind::FourYyYy];

    /// The number of layers (Y axes) the template makes whatever the number of columns.
    pub fn layers(&self) -> usize {
        match self {
            MultiYKind::ThreeYYy | MultiYKind::ThreeYYY => 3,
            MultiYKind::FourYYyy | MultiYKind::FourYyYy => 4,
        }
    }

    /// Stock legend properties shared by empty templates and plots made from columns.
    /// The independently constructed Multiple Y Axes dialog plans do not use these defaults.
    pub(crate) fn apply_legend_defaults(&self, graph: &mut Graph) {
        graph.layers[0].legend.font.size_pt = if *self == Self::FourYyYy { 20.0 } else { 22.0 };
        graph.layers[0].legend.background = None;
        graph.layers[0].legend.frame = false;
        graph.layers[0].legend.stock_multi_y = true;
    }

    /// The measured frame, axes and colours (graphing spec 21.5).
    pub fn plan(&self) -> MultiYPlan {
        use AxisSide::{Normal as L, Opposite as R};
        let (frame, axes, legend) = match self {
            MultiYKind::ThreeYYy => (
                (0.09593, 0.1989, 0.7573, 0.6652),
                vec![axis(L, 0.0, BLACK), axis(R, 0.0, RED), axis(R, 10.0, BLUE)],
                LegendPosition::ManualTopLeft { x: 0.12482036542804353, y: -0.21129770992366412 },
            ),
            MultiYKind::ThreeYYY => (
                (0.09764, 0.157, 0.8169, 0.6884),
                vec![axis(L, 0.0, BLACK), axis(L, MIDDLE_OFFSET_PCT, RED), axis(R, 0.0, BLUE)],
                LegendPosition::ManualTopLeft { x: 0.15074229158736202, y: -0.19563293006786664 },
            ),
            MultiYKind::FourYYyy => (
                (0.06856, 0.2147, 0.6772, 0.6407),
                vec![axis(L, 0.0, BLUE), axis(R, 0.0, GREEN), axis(R, 12.0, ORANGE), axis(R, 23.0, MAGENTA)],
                LegendPosition::ManualTopLeft { x: 1.002754820936639, y: -0.3123018389346861 },
            ),
            MultiYKind::FourYyYy => (
                (0.1491, 0.2125, 0.685, 0.6348),
                vec![axis(L, 0.0, BLUE), axis(L, 11.0, GREEN), axis(R, 0.0, MAGENTA), axis(R, 11.0, ORANGE)],
                LegendPosition::ManualTopLeft { x: 0.05832955061280075, y: -0.27712 },
            ),
        };
        MultiYPlan {
            frame: Frame { left: frame.0, top: frame.1, width: frame.2, height: frame.3 },
            axes,
            y_titles: false,
            axis_color: true,
            top_x: false,
            legend,
        }
    }
}

impl MultiYPlan {
    /// An empty graph with one layer per axis of the plan: the first layer is the root with the bottom X axis, every
    /// other layer is linked to it (straight X, own Y), covers it and shows only its Y axis. `base` gives the page,
    /// fonts and the kind of plot (Line, Scatter or Line + Symbol).
    pub fn graph(&self, base: Template, id: WindowId, name: &str) -> Graph {
        let mut g = base.new_graph(id, name);
        let mut first = g.layers[0].clone();
        first.frame = self.frame;
        first.x_axis.show_opposite = self.top_x;
        first.x_axis.opposite_ticks = self.top_x;
        first.y_axis.show_opposite = false;
        first.y_axis.opposite_ticks = false;
        first.legend.scope = LegendScope::Graph;
        first.legend.position = self.legend;
        let mut layers = Vec::new();
        for (i, a) in self.axes.iter().enumerate() {
            let mut l = if i == 0 { first.clone() } else { right_y_layer(&first, 0) };
            l.y_axis.side = a.side;
            l.y_axis.offset_pct = a.offset_pct;
            l.y_axis.show_title = self.y_titles;
            l.y_axis.color_follows_plot = self.axis_color;
            // The layer's own colour first, then the rest of Origin's list.
            let own = a.color;
            let mut colors = vec![own];
            colors.extend(ORIGIN_COLOR_LIST.iter().map(|&(r, g, b)| Color::rgb(r, g, b)).filter(|c| *c != own));
            l.palette = Palette { colors, mode: PaletteMode::Cycle, list: None };
            if i > 0 {
                // The symbol shape changes from layer to layer as in Origin's scatter multi-Y graph.
                l.default_symbol.shape = SymbolShape::ALL[i % SymbolShape::ALL.len()];
            }
            layers.push(l);
        }
        g.layers = layers;
        g
    }
}

/// The kind of plot of the Multiple Y Axes dialog (Plot Type).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum MultiYPlotType {
    #[default]
    Line,
    Scatter,
    LineSymbol,
}

impl MultiYPlotType {
    pub const ALL: [MultiYPlotType; 3] = [MultiYPlotType::Line, MultiYPlotType::Scatter, MultiYPlotType::LineSymbol];

    pub fn template(&self) -> Template {
        match self {
            MultiYPlotType::Line => Template::Line,
            MultiYPlotType::Scatter => Template::Scatter,
            MultiYPlotType::LineSymbol => Template::LineSymbol,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            MultiYPlotType::Line => "Line",
            MultiYPlotType::Scatter => "Scatter",
            MultiYPlotType::LineSymbol => "Line + Symbol",
        }
    }

    /// Reads `line`, `scatter` or `line+symbol` (also `line-symbol`, `linesymb`).
    pub fn from_name(name: &str) -> Option<MultiYPlotType> {
        match name.trim().to_ascii_lowercase().as_str() {
            "line" => Some(MultiYPlotType::Line),
            "scatter" => Some(MultiYPlotType::Scatter),
            "line+symbol" | "line-symbol" | "linesymb" | "line + symbol" => Some(MultiYPlotType::LineSymbol),
            _ => None,
        }
    }
}

/// The controls of Plot > Multi-Y > Multiple Y Axes... (graphing spec 21.6).
#[derive(Clone, Debug, PartialEq)]
pub struct MultiYOptions {
    pub plot_type: MultiYPlotType,
    /// Number of left Y axes; `None` is Auto (half of the layers, rounded down).
    pub left: Option<usize>,
    /// Number of right Y axes; `None` is Auto (the rest).
    pub right: Option<usize>,
    /// Add Middle Y Axis: one of the left axes stands in the middle of the plot.
    pub middle: bool,
    /// Number of plots in each layer; empty is one plot per layer.
    pub per_layer: Vec<usize>,
    /// Link Axis Color to Plot.
    pub axis_color: bool,
    pub y_titles: bool,
    pub top_x: bool,
    /// Distance between Y axes in percent; `None` is Auto.
    pub gap_pct: Option<f64>,
}

impl Default for MultiYOptions {
    fn default() -> Self {
        MultiYOptions {
            plot_type: MultiYPlotType::Line,
            left: None,
            right: None,
            middle: false,
            per_layer: Vec::new(),
            axis_color: true,
            y_titles: false,
            top_x: false,
            gap_pct: None,
        }
    }
}

/// Why Multiple Y Axes cannot make a graph.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum MultiYError {
    #[error("a multiple Y axes graph needs at least one Y column")]
    NoColumns,
    #[error("the plots per layer add up to {sum} but {columns} Y columns were selected")]
    PlotsPerLayer { sum: usize, columns: usize },
    #[error("a layer cannot hold no plots; every number of plots per layer must be 1 or more")]
    EmptyLayer,
    #[error("{left} left and {right} right Y axes{middle} do not match the {layers} layers")]
    AxisCount { left: usize, right: usize, middle: &'static str, layers: usize },
    #[error("the gap between Y axes must be a number from 0 to 100 (got {0})")]
    Gap(String),
}

impl MultiYOptions {
    /// The number of plots in each layer for `columns` Y columns.
    pub fn plots_per_layer(&self, columns: usize) -> Result<Vec<usize>, MultiYError> {
        if columns == 0 {
            return Err(MultiYError::NoColumns);
        }
        if self.per_layer.is_empty() {
            return Ok(vec![1; columns]);
        }
        if self.per_layer.contains(&0) {
            return Err(MultiYError::EmptyLayer);
        }
        // A sum too large for usize is also too large for any number of columns.
        let sum = self.per_layer.iter().try_fold(0usize, |a, &n| a.checked_add(n)).unwrap_or(usize::MAX);
        if sum != columns {
            return Err(MultiYError::PlotsPerLayer { sum, columns });
        }
        Ok(self.per_layer.clone())
    }

    /// The gap in effect: the given one, else 14 % when Y titles or the top X axis are shown, else 10 %.
    pub fn gap(&self) -> f64 {
        self.gap_pct.unwrap_or(if self.y_titles || self.top_x { TITLED_GAP_PCT } else { DEFAULT_GAP_PCT })
    }

    /// The plan for `layers` layers: the first left axis at the frame, the other left axes at the gap and its multiples,
    /// the middle axis at -50 %, then the right axes at 0, the gap, ... . The frame of the first layer moves so that the
    /// frame and the outward offsets are centred on the page (21.6).
    pub fn plan(&self, layers: usize) -> Result<MultiYPlan, MultiYError> {
        use AxisSide::{Normal, Opposite};
        let gap = self.gap();
        if !(0.0..=100.0).contains(&gap) {
            return Err(MultiYError::Gap(gap.to_string()));
        }
        let mid = usize::from(self.middle && layers > 1);
        let free = layers - mid;
        let (left, right) = match (self.left, self.right) {
            // N = 1 makes an ordinary one-layer graph with a left axis (graphing spec 21.6).
            (None, None) if layers == 1 => (1, 0),
            (None, None) => (free / 2, free - free / 2),
            (Some(l), None) => (l, free.saturating_sub(l)),
            (None, Some(r)) => (free.saturating_sub(r), r),
            (Some(l), Some(r)) => (l, r),
        };
        if left + right + mid != layers {
            let middle = if mid == 1 { " and a middle axis" } else { "" };
            return Err(MultiYError::AxisCount { left, right, middle, layers });
        }
        let colour = |i: usize| {
            let (r, g, b) = ORIGIN_COLOR_LIST[i % ORIGIN_COLOR_LIST.len()];
            Color::rgb(r, g, b)
        };
        let mut axes = Vec::new();
        for i in 0..left {
            axes.push(YAxisPlan { side: Normal, offset_pct: gap * i as f64, color: colour(axes.len()) });
        }
        if mid == 1 {
            axes.push(YAxisPlan { side: Normal, offset_pct: MIDDLE_OFFSET_PCT, color: colour(axes.len()) });
        }
        for i in 0..right {
            axes.push(YAxisPlan { side: Opposite, offset_pct: gap * i as f64, color: colour(axes.len()) });
        }
        let (t, w, h) = LINE_FRAME;
        let step = gap * w / 100.0;
        let left_room = left.saturating_sub(1) as f64 * step;
        let right_room = right.saturating_sub(1) as f64 * step;
        let mut frame =
            Frame { left: (1.0 - w - left_room - right_room) / 2.0 + left_room, top: t, width: w, height: h };
        if layers == 1 && left == 1 {
            frame.left = ORDINARY_FRAME_LEFT;
        }
        Ok(MultiYPlan {
            frame,
            axes,
            y_titles: self.y_titles,
            axis_color: self.axis_color,
            top_x: self.top_x,
            legend: LegendPosition::TopRight,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts() -> MultiYOptions {
        MultiYOptions::default()
    }

    #[test]
    fn stock_legend_keeps_first_plots_then_adds_the_owners_extras() {
        let ids = ["4.1", "4.2", "4.3", "5.1", "5.2", "5.3", "6.1", "6.2"].map(str::to_owned);
        assert_eq!(stock_legend_order(3, &ids), [0, 3, 6, 1, 2]);
        assert_eq!(stock_legend_order(3, &ids[3..]), [0, 3]);
        let single_layer = ["1", "2", "3"].map(str::to_owned);
        assert_eq!(stock_legend_order(3, &single_layer), [0, 1, 2]);
    }

    #[test]
    fn templates_have_the_measured_axes() {
        for (kind, offsets) in [
            (MultiYKind::ThreeYYy, vec![0.0, 0.0, 10.0]),
            (MultiYKind::ThreeYYY, vec![0.0, -50.0, 0.0]),
            (MultiYKind::FourYYyy, vec![0.0, 0.0, 12.0, 23.0]),
            (MultiYKind::FourYyYy, vec![0.0, 11.0, 0.0, 11.0]),
        ] {
            let plan = kind.plan();
            assert_eq!(plan.axes.len(), kind.layers());
            assert_eq!(plan.axes.iter().map(|a| a.offset_pct).collect::<Vec<_>>(), offsets);
            let g = plan.graph(Template::Line, WindowId(1), "g");
            assert_eq!(g.layers.len(), kind.layers());
            assert_eq!(g.layers[0].link, None);
            for l in &g.layers[1..] {
                assert_eq!(l.link.map(|k| k.parent), Some(0));
                assert_eq!(l.x_axis.side, AxisSide::Hidden);
            }
            assert!(g.layers.iter().all(|l| !l.y_axis.show_title && l.y_axis.color_follows_plot));
            assert!(!g.layers[0].x_axis.show_opposite && !g.layers[0].y_axis.show_opposite);
        }
    }

    #[test]
    fn templates_put_the_axes_on_the_measured_sides() {
        let sides = |k: MultiYKind| k.plan().axes.iter().map(|a| a.side).collect::<Vec<_>>();
        use AxisSide::{Normal as L, Opposite as R};
        assert_eq!(sides(MultiYKind::ThreeYYy), [L, R, R]);
        assert_eq!(sides(MultiYKind::ThreeYYY), [L, L, R]);
        assert_eq!(sides(MultiYKind::FourYYyy), [L, R, R, R]);
        assert_eq!(sides(MultiYKind::FourYyYy), [L, L, R, R]);
    }

    #[test]
    fn auto_split_is_half_left_rest_right() {
        for (n, l) in [(2, 1), (3, 1), (4, 2), (5, 2), (6, 3)] {
            let plan = opts().plan(n).unwrap();
            let left = plan.axes.iter().filter(|a| a.side == AxisSide::Normal).count();
            assert_eq!(left, l, "{n} layers");
        }
    }

    #[test]
    fn offsets_follow_the_gap_and_the_middle_axis_stands_in_the_plot() {
        let plan = opts().plan(4).unwrap();
        let o: Vec<f64> = plan.axes.iter().map(|a| a.offset_pct).collect();
        assert_eq!(o, [0.0, 10.0, 0.0, 10.0]);
        let plan = MultiYOptions { middle: true, ..opts() }.plan(3).unwrap();
        let o: Vec<(AxisSide, f64)> = plan.axes.iter().map(|a| (a.side, a.offset_pct)).collect();
        assert_eq!(o, [(AxisSide::Normal, 0.0), (AxisSide::Normal, -50.0), (AxisSide::Opposite, 0.0)]);
        // ly = 0: layer 1 has a right axis; ry = 0: all left.
        let plan = MultiYOptions { left: Some(0), ..opts() }.plan(3).unwrap();
        assert!(plan.axes.iter().all(|a| a.side == AxisSide::Opposite));
        assert_eq!(plan.axes.iter().map(|a| a.offset_pct).collect::<Vec<_>>(), [0.0, 10.0, 20.0]);
        let plan = MultiYOptions { right: Some(0), ..opts() }.plan(3).unwrap();
        assert!(plan.axes.iter().all(|a| a.side == AxisSide::Normal));
        let plan = MultiYOptions { gap_pct: Some(20.0), ..opts() }.plan(3).unwrap();
        assert_eq!(plan.axes[2].offset_pct, 20.0);
    }

    #[test]
    fn titles_and_top_x_raise_the_automatic_gap_to_14() {
        assert_eq!(MultiYOptions { y_titles: true, ..opts() }.gap(), 14.0);
        assert_eq!(MultiYOptions { top_x: true, gap_pct: Some(12.0), ..opts() }.gap(), 12.0);
        assert_eq!(opts().gap(), 10.0);
    }

    #[test]
    fn the_first_frame_is_centred_with_its_offsets() {
        // Measured left edges in percent of the page (graphing spec 21.6): left and right axis counts.
        for (n, left, right, want) in
            [(2, 1, 1, 15.92), (3, 1, 2, 12.52), (4, 1, 3, 9.111), (4, 2, 2, 15.92), (3, 3, 0, 22.73)]
        {
            let plan = MultiYOptions { left: Some(left), right: Some(right), ..opts() }.plan(n).unwrap();
            assert!((plan.frame.left * 100.0 - want).abs() < 0.01, "{n}: {}", plan.frame.left * 100.0);
        }
    }

    #[test]
    fn one_column_is_an_ordinary_graph_with_a_left_axis() {
        let p = opts().plan(1).unwrap();
        assert_eq!(p.axes.len(), 1);
        assert_eq!(p.axes[0].side, AxisSide::Normal);
        assert!((p.frame.left - 0.1791).abs() < 1e-9);
    }

    #[test]
    fn a_huge_plots_per_layer_sum_does_not_overflow() {
        let o = MultiYOptions { per_layer: vec![usize::MAX, 4], ..opts() };
        assert!(matches!(o.plots_per_layer(3), Err(MultiYError::PlotsPerLayer { .. })));
    }

    #[test]
    fn plots_per_layer_and_axis_counts_are_validated() {
        let o = MultiYOptions { per_layer: vec![2, 1, 1], ..opts() };
        assert_eq!(o.plots_per_layer(4), Ok(vec![2, 1, 1]));
        assert_eq!(o.plots_per_layer(5), Err(MultiYError::PlotsPerLayer { sum: 4, columns: 5 }));
        assert_eq!(opts().plots_per_layer(0), Err(MultiYError::NoColumns));
        assert_eq!(MultiYOptions { per_layer: vec![1, 0], ..opts() }.plots_per_layer(1), Err(MultiYError::EmptyLayer));
        assert!(matches!(
            MultiYOptions { left: Some(3), right: Some(3), ..opts() }.plan(4),
            Err(MultiYError::AxisCount { .. })
        ));
        assert!(matches!(MultiYOptions { gap_pct: Some(-1.0), ..opts() }.plan(3), Err(MultiYError::Gap(_))));
    }

    #[test]
    fn plot_colours_follow_origins_list_in_the_dialog() {
        let plan = opts().plan(4).unwrap();
        let g = plan.graph(Template::Line, WindowId(1), "g");
        let first: Vec<Color> = g.layers.iter().map(|l| l.palette.colors[0]).collect();
        assert_eq!(first, [Color::rgb(0, 0, 0), Color::rgb(255, 0, 0), Color::rgb(0, 255, 0), Color::rgb(0, 0, 255)]);
    }
}
