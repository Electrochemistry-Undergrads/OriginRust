//! The graph (figure) model. It describes *what* a figure shows and how it is
//! styled. It holds no pixels and no data: plots refer to worksheet columns
//! by [`ColumnId`]. The `sheaf-plot` crate turns a graph plus the project's
//! data into drawing commands.
//!
//! All sizes are in points (1/72 inch) so that a figure has the same
//! physical proportions on screen and in every export format.

use crate::color::Color;
pub use crate::graph_style::*;
use crate::ids::{ColumnId, WindowId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Graph {
    pub id: WindowId,
    /// Short name, unique among all windows of the project (for example `Graph1`).
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub long_name: String,
    pub page: Page,
    pub layers: Vec<Layer>,
}

impl Graph {
    pub fn title(&self) -> String {
        if self.long_name.is_empty() {
            return self.name.clone();
        }
        match self.page.style.title_display {
            TitleDisplay::Both => format!("{} - {}", self.name, self.long_name),
            TitleDisplay::Name => self.name.clone(),
            TitleDisplay::LongName => self.long_name.clone(),
        }
    }

    /// Every column this graph reads.
    pub fn referenced_columns(&self) -> Vec<ColumnId> {
        let mut out = Vec::new();
        for layer in &self.layers {
            for p in &layer.plots {
                out.extend(p.x);
                out.push(p.y);
                out.extend(p.y_error);
                if let LabelForm::Column { column } = p.decor.labels.form {
                    out.push(column);
                }
            }
            for axis in [&layer.x_axis, &layer.y_axis] {
                if let TickLabelSource::Column { column } = axis.tick_label_source {
                    out.push(column);
                }
            }
        }
        out
    }

    /// Renumbers every column id stored anywhere in this graph through `f`: the plots' X, Y, error,
    /// base, size and colour columns, label columns, treemap and sunburst categories, the treemap order column, the
    /// parallel colour/group column, the plot offsets and their follows, the tick-label columns of
    /// the axes, the plot choice of the rugs and the plot of the slope annotations. Every path that renumbers columns (File > Append,
    /// ...) goes through this; a new field that holds a [`ColumnId`] is added here.
    pub fn remap_columns(&mut self, f: &dyn Fn(ColumnId) -> ColumnId) {
        for layer in &mut self.layers {
            for p in &mut layer.plots {
                p.y = f(p.y);
                for id in [&mut p.x, &mut p.y_error, &mut p.x_error, &mut p.y_base, &mut p.size_from, &mut p.color_from]
                {
                    *id = id.map(f);
                }
                if let LabelForm::Column { column } = &mut p.decor.labels.form {
                    *column = f(*column);
                }
                p.parallel.color = p.parallel.color.map(f);
                for c in &mut p.treemap.categories {
                    *c = f(*c);
                }
                p.treemap.order_column = p.treemap.order_column.map(f);
                for c in &mut p.sunburst.categories {
                    *c = f(*c);
                }
            }
            for sl in &mut layer.slopes {
                sl.plot = f(sl.plot);
                sl.x = sl.x.map(f);
            }
            for o in &mut layer.plot_offsets {
                o.column = f(o.column);
                o.follows = o.follows.map(f);
            }
            for axis in [&mut layer.x_axis, &mut layer.y_axis] {
                if let TickLabelSource::Column { column } = &mut axis.tick_label_source {
                    *column = f(*column);
                }
                for rug in [&mut axis.rug, &mut axis.rug_opposite] {
                    rug.plot = rug.plot.map(f);
                }
            }
        }
    }

    /// The parent of layer `index` when the layer has a valid link: the parent comes earlier in
    /// the list and is not linked itself. A layer with an invalid link (to itself, to a later or
    /// missing layer, or to a linked layer) is drawn as an independent layer.
    pub fn link_parent(&self, index: usize) -> Option<usize> {
        let link = self.layers.get(index)?.link?;
        (link.parent < index && self.layers[link.parent].link.is_none()).then_some(link.parent)
    }

    /// The link of layer `index` when it is valid (see [`Graph::link_parent`]).
    pub fn valid_link(&self, index: usize) -> Option<LayerLink> {
        self.link_parent(index).and(self.layers[index].link)
    }

    /// The layer that layer `index` follows: its parent when validly linked, else itself.
    pub fn root_of(&self, index: usize) -> usize {
        self.link_parent(index).unwrap_or(index)
    }

    /// The layers drawn as one group with layer `index`: its root followed by every layer
    /// validly linked to that root, in layer order. Empty when there is no such layer.
    pub fn group(&self, index: usize) -> Vec<usize> {
        if index >= self.layers.len() {
            return Vec::new();
        }
        let root = self.root_of(index);
        (0..self.layers.len()).filter(|&i| i == root || self.link_parent(i) == Some(root)).collect()
    }

    /// The place of layer `index` inside the frame of its root, as fractions of that frame: the
    /// whole frame `{0, 0, 1, 1}` for a root, and the layer's own `frame` for a validly linked
    /// layer. A right Y layer has the place of the layer it belongs to; a panel linked to the first
    /// panel of a stack has a place of its own (graphing spec section 15).
    pub fn place(&self, index: usize) -> Frame {
        match self.link_parent(index) {
            Some(_) => self.layers[index].frame,
            None => Frame::WHOLE,
        }
    }

    /// Gives the page a new size. With `keep_layer_size` the layers that stand on the page keep their
    /// size and distance from the top left corner in points (their frames are fractions of the page,
    /// so these are scaled back); without it they grow and shrink with the page. Layers that follow
    /// another layer are fractions of it and need no change.
    pub fn resize_page(&mut self, width_pt: f64, height_pt: f64, keep_layer_size: bool) {
        if keep_layer_size && width_pt > 0.0 && height_pt > 0.0 {
            let (kx, ky) = (self.page.width_pt / width_pt, self.page.height_pt / height_pt);
            for layer in self.layers.iter_mut().filter(|l| l.link.is_none()) {
                layer.frame.left *= kx;
                layer.frame.width *= kx;
                layer.frame.top *= ky;
                layer.frame.height *= ky;
            }
        }
        self.page.width_pt = width_pt;
        self.page.height_pt = height_pt;
    }

    /// Links layer `index` to `parent`, or unlinks it with `None` (Plot Details > Link Axes Scales).
    /// The layer keeps its place on the page: its frame is converted between fractions of the page
    /// and fractions of the parent's frame. A new link starts with both scales the layer's own, as
    /// Origin's tab does (None and None); a change of parent keeps them. A parent that cannot be
    /// followed (a later layer, or one linked itself) is refused, and so is a link for a layer that
    /// other layers are linked to.
    pub fn set_link(&mut self, index: usize, parent: Option<usize>) -> bool {
        if index >= self.layers.len() {
            return false;
        }
        if let Some(p) = parent
            && (p >= index || self.layers[p].link.is_some() || self.has_children(index))
        {
            return false;
        }
        let on_page = match self.link_parent(index) {
            Some(p) => self.layers[index].frame.within(self.layers[p].frame),
            None => self.layers[index].frame,
        };
        let layer = &mut self.layers[index];
        let (x, y) = layer.link.map_or((ScaleLink::None, ScaleLink::None), |k| (k.x, k.y));
        layer.link = parent.map(|parent| LayerLink { parent, x, y });
        let frame = match parent {
            Some(p) => on_page.relative_to(self.layers[p].frame),
            None => on_page,
        };
        self.layers[index].frame = frame;
        true
    }

    /// Whether another layer is linked to layer `index`.
    pub fn has_children(&self, index: usize) -> bool {
        self.layers.iter().enumerate().any(|(i, l)| i != index && l.link.is_some_and(|k| k.parent == index))
    }

    /// The layers of layer `index`'s group that are drawn in the same frame as it (the same
    /// [`Graph::place`]), in layer order: a layer and its right Y layer, but not the other panels
    /// of a stack. Empty when there is no such layer.
    pub fn same_frame(&self, index: usize) -> Vec<usize> {
        if index >= self.layers.len() {
            return Vec::new();
        }
        let place = self.place(index);
        self.group(index).into_iter().filter(|&i| self.place(i) == place).collect()
    }

    /// The ids of the plots that the legend of layer `index` lists, in order, given how many
    /// plots each layer shows (`plots_in`). They are `1`, `2`, ... for the layer's own plots, and
    /// `l.p` (layer and plot, both from 1) when the legend lists the plots of several layers; they
    /// are what `\l(n)` and `%(n)` in the legend text refer to ([`Legend::text`]).
    pub fn legend_ids(&self, index: usize, plots_in: &dyn Fn(usize) -> usize) -> Vec<String> {
        let Some(layer) = self.layers.get(index) else { return Vec::new() };
        if layer.legend.scope == LegendScope::Graph {
            let layers = self.same_frame(index);
            if layers.len() > 1 {
                return layers.iter().flat_map(|&l| (1..=plots_in(l)).map(move |k| format!("{}.{k}", l + 1))).collect();
            }
            return layers.first().map_or_else(Vec::new, |&l| (1..=plots_in(l)).map(|k| k.to_string()).collect());
        }
        (1..=plots_in(index)).map(|k| k.to_string()).collect()
    }

    /// The panels of the graph, in layer order: every layer except those drawn in the frame of an
    /// earlier layer of their group (a right Y layer is part of the panel it lies over). A graph
    /// with one layer, or a double-Y graph, has one panel.
    pub fn panels(&self) -> Vec<usize> {
        (0..self.layers.len()).filter(|&i| self.same_frame(i).first() == Some(&i)).collect()
    }

    /// The linked layer that gives the frame of layer `index` its right Y axis, if there is one: a
    /// member of the layer's group, drawn in the same frame, whose Y axis is on the right.
    pub fn right_y_member(&self, index: usize) -> Option<usize> {
        self.same_frame(index)
            .into_iter()
            .find(|&i| self.link_parent(i).is_some() && self.layers[i].y_axis.side == AxisSide::Opposite)
    }
}

/// The sheet of "paper" a graph is drawn on.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Page {
    pub width_pt: f64,
    pub height_pt: f64,
    pub background: Color,
    /// The border line, layer icons, window title choice and notes of the page (Plot Details > graph node).
    #[serde(default, skip_serializing_if = "PageStyle::is_default")]
    pub style: PageStyle,
}

impl Page {
    pub fn inches(width: f64, height: f64) -> Self {
        Page { width_pt: width * 72.0, height_pt: height * 72.0, background: Color::WHITE, style: PageStyle::default() }
    }
}

/// The rectangle inside which data is drawn, as fractions of the page
/// (0..1, measured from the top-left corner). Axis titles and tick labels
/// are drawn outside this rectangle.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
}

impl Frame {
    /// The whole rectangle it is measured in: the whole page, or the whole frame of the parent
    /// layer for a linked layer.
    pub const WHOLE: Frame = Frame { left: 0.0, top: 0.0, width: 1.0, height: 1.0 };

    /// This frame, given as fractions of `outer`, as fractions of what `outer` is measured in.
    pub fn within(self, outer: Frame) -> Frame {
        Frame {
            left: outer.left + self.left * outer.width,
            top: outer.top + self.top * outer.height,
            width: self.width * outer.width,
            height: self.height * outer.height,
        }
    }

    /// The inverse of [`Frame::within`]: this frame as fractions of `outer`. An empty `outer` side
    /// leaves that side as it is.
    pub fn relative_to(self, outer: Frame) -> Frame {
        let (left, width) = if outer.width > 0.0 {
            ((self.left - outer.left) / outer.width, self.width / outer.width)
        } else {
            (self.left, self.width)
        };
        let (top, height) = if outer.height > 0.0 {
            ((self.top - outer.top) / outer.height, self.height / outer.height)
        } else {
            (self.top, self.height)
        };
        Frame { left, top, width, height }
    }
}

/// One set of axes with the plots drawn in it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Layer {
    pub frame: Frame,
    pub x_axis: Axis,
    pub y_axis: Axis,
    pub plots: Vec<Plot>,
    pub legend: Legend,
    /// Free text labels placed in the layer.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<TextLabel>,
    /// Drawn lines, arrows, rectangles and ellipses (graphing spec section 17), above the data.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shapes: Vec<crate::shape::Shape>,
    /// Slope annotations (graphing spec 17.6, T52): a slope marked between two rows of a plot.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub slopes: Vec<SlopeAnnotation>,
    /// Straight lines at an axis value with a value label (T67, graphing spec "Straight lines").
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub straight_lines: Vec<crate::straight_line::StraightLine>,
    /// Significance brackets between groups of a multiple comparisons plot (T64, graphing spec 17.7): the pairs
    /// that are shown, drawn above the groups at positions 1, 2, 3, ...
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub brackets: Vec<crate::compplot::SignificanceBracket>,
    /// The settings of a multiple comparisons plot with every compared pair, so that the labels can be re-styled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comparison: Option<crate::compplot::ComparisonPlotOptions>,
    /// The colors given to new plots, in order. See [`Layer::next_color`].
    pub palette: Palette,
    /// Style copied to each new plot added to this layer.
    pub default_line: LineStyle,
    pub default_symbol: SymbolStyle,
    pub default_kind: PlotKind,
    /// Set when this layer follows another layer of the graph: a double-Y graph is a layer with a
    /// right Y axis linked to the layer with the left one, and the panels of a stack are linked to
    /// the first panel to share its X axis. While the link is valid, `frame` is read as fractions
    /// of the parent's frame instead of the page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<LayerLink>,
    /// How the plots are moved against each other so that they do not overlap (stacked lines,
    /// waterfall). Only the drawing moves; the data stay as they are.
    #[serde(default, skip_serializing_if = "Stack::is_none")]
    pub stack: Stack,
    /// Colour mapping of the curves of a [`Stack::Waterfall`] layer (graphing spec 13.17).
    #[serde(default, skip_serializing_if = "WaterfallColor::is_none")]
    pub waterfall_color: WaterfallColor,
    /// Settings of single curves for [`Layer::stack`], found by the curve's Y column: the offsets
    /// of [`Stack::Individual`] mode and which curve a fitted curve moves with.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plot_offsets: Vec<PlotOffset>,
    /// Width and spacing of the bars of the layer's column plots. All visible column plots of a
    /// layer stand side by side as one group (graphing spec section 14.4).
    #[serde(default, skip_serializing_if = "BarSpacing::is_default")]
    pub bars: BarSpacing,
    /// The coordinate system the plots are drawn in: the usual rectangular axes, or a circle with
    /// angle and radius (graphing spec section 14.18).
    #[serde(default, skip_serializing_if = "Coords::is_cartesian")]
    pub coords: Coords,
    /// The colour behind the plot area (the Background tab of Origin's layer Plot Details,
    /// "Color", None by default); `None` leaves the page showing through.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<Color>,
    /// A gradient from that colour and a border line around the frame (the rest of the Background tab).
    #[serde(default, skip_serializing_if = "BackgroundStyle::is_default")]
    pub background_style: BackgroundStyle,
    /// The Group tab of the layer's plots: edit mode and increments (the colour increment is
    /// [`Layer::palette`]'s mode).
    #[serde(default, skip_serializing_if = "GroupStyle::is_default")]
    pub group_style: GroupStyle,
    /// The Display tab of Origin's layer Plot Details (graphing spec 18).
    #[serde(default, skip_serializing_if = "LayerDisplay::is_default")]
    pub display: LayerDisplay,
    /// Origin's Size/Speed "Link Axis Length to Scale with X:Y Ratio" (graphing spec 18.7a): `Some(r)` keeps
    /// one unit of X `r` times as long on the page as one unit of Y. The layout keeps the left, top and
    /// width of the frame and makes the height follow the axis ranges ([`xy_ratio_height`]); turning it on
    /// or changing `r` first shrinks the frame to fit ([`Layer::fit_xy_ratio`]). Ignored while an axis has
    /// a break and in polar and ternary layers. `None` (off) by default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub xy_ratio: Option<f64>,
}

/// The height (in the units of `width`) a frame of that width needs so that one unit of an X axis spanning
/// `span_x` is `ratio` times as long as one unit of a Y axis spanning `span_y` (spans in the axes' own
/// scale: decades on a log axis). `None` when an input is not a positive finite number.
pub fn xy_ratio_height(width: f64, span_x: f64, span_y: f64, ratio: f64) -> Option<f64> {
    let ok = |v: f64| v.is_finite() && v > 0.0;
    (ok(width) && ok(span_x) && ok(span_y) && ok(ratio)).then(|| width * span_y / (span_x * ratio)).filter(|h| ok(*h))
}

impl Layer {
    /// Origin's response to ticking "Link Axis Length to Scale" or changing its ratio (measured in Origin
    /// 2016, graphing spec 18.7a): the frame keeps its left and top and shrinks either its width or its height
    /// (never grows) until one unit of X is `ratio` times one unit of Y. `page` is the page size in points,
    /// `span_x` and `span_y` the current axis spans in their own scale. Sets [`Layer::xy_ratio`]; the frame
    /// is unchanged when the spans are not usable.
    pub fn fit_xy_ratio(&mut self, ratio: f64, page: (f64, f64), span_x: f64, span_y: f64) {
        self.xy_ratio = Some(ratio);
        let (w, h) = (self.frame.width * page.0, self.frame.height * page.1);
        let Some(need_h) = xy_ratio_height(w, span_x, span_y, ratio) else { return };
        if need_h <= h {
            self.frame.height = need_h / page.1;
        } else if let Some(need_w) = xy_ratio_height(h, span_y, span_x, 1.0 / ratio) {
            self.frame.width = need_w / page.0;
        }
    }
}

/// What a layer draws and how (Origin's Display tab; defaults measured: data shown, clipped to the
/// frame, grid under the data).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayerDisplay {
    /// Show Elements: Data.
    pub show_data: bool,
    /// Clip Data to Frame.
    pub clip_data: bool,
    /// Grid on Top of Data.
    #[serde(default)]
    pub grid_on_top: bool,
    /// Data on Top of Axes. Axes remain unclipped when drawn behind the data.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub data_on_top: bool,
    /// Show Elements: X Axes. Off hides the X axes of the layer (lines, ticks, tick labels, titles).
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub show_x_axes: bool,
    /// Show Elements: Y Axes.
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub show_y_axes: bool,
}

fn is_true(v: &bool) -> bool {
    *v
}

impl Default for LayerDisplay {
    fn default() -> Self {
        LayerDisplay {
            show_data: true,
            clip_data: true,
            grid_on_top: false,
            data_on_top: false,
            show_x_axes: true,
            show_y_axes: true,
        }
    }
}

impl LayerDisplay {
    pub fn is_default(&self) -> bool {
        *self == LayerDisplay::default()
    }
}

/// The coordinate system of a layer.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Coords {
    /// Rectangular X and Y axes.
    #[default]
    Cartesian,
    /// A circle: the angle runs round it, the radius from its centre to its edge. The plots are
    /// ordinary line and symbol plots whose two columns are read as angle and radius.
    Polar(PolarCoords),
    /// An equilateral triangle: three fractions that sum to 1 give the position of a point
    /// (graphing spec section 14.20). Plots take their X, Y and Z columns as the three parts.
    Ternary(TernaryCoords),
}

impl Coords {
    pub fn is_cartesian(&self) -> bool {
        *self == Coords::Cartesian
    }

    /// The polar settings, `None` for a rectangular layer.
    pub fn polar(&self) -> Option<&PolarCoords> {
        match self {
            Coords::Polar(p) => Some(p),
            _ => None,
        }
    }

    /// The ternary settings, `None` for any other layer.
    pub fn ternary(&self) -> Option<&TernaryCoords> {
        match self {
            Coords::Ternary(t) => Some(t),
            _ => None,
        }
    }
}

/// How the ticks of a ternary layer count: fractions (0 to 1) or percent (0 to 100).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TernaryScale {
    /// Percent when a row of the data sums to about 100 (99 to 101), else fractions.
    #[default]
    Auto,
    /// Ticks 0, 0.2, ... 1.
    Fraction,
    /// Ticks 0, 20, ... 100.
    Percent,
}

/// Settings of a ternary layer (graphing spec section 14.20). A plot of such a layer names its Z
/// column in `Plot::size_from` (no bubble sizes are drawn there).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TernaryCoords {
    #[serde(default)]
    pub scale: TernaryScale,
}

/// Which column of a plot holds the radius in a polar layer.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RadiusColumn {
    /// Origin's "theta(X) r(Y)": the angle comes from the X column, the radius from Y.
    #[default]
    Y,
    /// Origin's "r(X) theta(Y)": the radius comes from the X column, the angle from Y.
    X,
}

/// The unit of the angle of a polar layer (Origin's Units of the angular axis).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AngleUnit {
    /// A full turn is 360; the axis has a tick every 30 and plain number labels.
    #[default]
    Degrees,
    /// A full turn is 2 pi; the axis has a tick every quarter of pi and labels like 0.25 and the pi sign.
    Radians,
}

impl AngleUnit {
    /// The angle of a full turn in this unit.
    pub fn circle(self) -> f64 {
        match self {
            AngleUnit::Degrees => 360.0,
            AngleUnit::Radians => std::f64::consts::TAU,
        }
    }
}

/// Settings of a polar layer (graphing spec section 14.18).
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PolarCoords {
    /// Which column holds the radius.
    #[serde(default)]
    pub radius: RadiusColumn,
    /// Where angle 0 is, in degrees counter-clockwise from the right (3 o'clock). Origin's default 0.
    #[serde(default)]
    pub zero_deg: f64,
    /// True when the angle grows clockwise. Origin's default is counter-clockwise.
    #[serde(default)]
    pub clockwise: bool,
    /// True for a radar chart (graphing spec section 14.19): the rows are the spokes, evenly spaced
    /// from 12 o'clock clockwise, whatever `zero_deg` and `clockwise` say, and the grid is made of
    /// polygons.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub radar: bool,
    /// The unit the angle column is in (Origin's Units on the angular axis; degrees by default).
    #[serde(default, skip_serializing_if = "is_default_angle_unit")]
    pub angle_unit: AngleUnit,
}

fn is_default_angle_unit(u: &AngleUnit) -> bool {
    *u == AngleUnit::Degrees
}

impl Default for PolarCoords {
    fn default() -> Self {
        PolarCoords {
            radius: RadiusColumn::Y,
            zero_deg: 0.0,
            clockwise: false,
            radar: false,
            angle_unit: AngleUnit::Degrees,
        }
    }
}

/// How the bars of the column plots of a layer share the room at one X position (Origin's "Gap
/// Between Bars" and "Overlap"). The rules are in the graphing spec, section 14.4.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BarSpacing {
    /// The gap between neighbouring groups of bars, in percent of the X spacing (0 to 100). The
    /// bars at one X position together take `100 - gap_pct` percent of the spacing.
    #[serde(default = "default_gap_pct")]
    pub gap_pct: f64,
    /// How much neighbouring bars of the group cover each other, in percent of a bar's width
    /// (-100 to 100). Negative values leave a gap between them; 100 puts them on top of each other.
    #[serde(default)]
    pub overlap_pct: f64,
    /// Plot Details > Spacing > Width (in %): the bars at one X position together take this share of the
    /// X spacing, which is the same as a gap of `100 - width`; it overrides `gap_pct` while set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width_pct: Option<f64>,
}

/// Origin's default gap between bars, in percent.
pub const DEFAULT_BAR_GAP_PCT: f64 = 20.0;

fn default_gap_pct() -> f64 {
    DEFAULT_BAR_GAP_PCT
}

impl Default for BarSpacing {
    fn default() -> Self {
        BarSpacing { gap_pct: DEFAULT_BAR_GAP_PCT, overlap_pct: 0.0, width_pct: None }
    }
}

impl BarSpacing {
    pub fn is_default(&self) -> bool {
        *self == BarSpacing::default()
    }
}

/// Where the tick labels of an axis come from (Origin's tick label types "Numeric" and "Text from
/// dataset"). The rules are in the graphing spec, section 14.3.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum TickLabelSource {
    /// The X axis shows the cells of the first plot's X column when that column holds text (sample
    /// names), and numbers otherwise. A Y axis always shows numbers.
    #[default]
    Auto,
    /// Always numbers.
    Numbers,
    /// The major tick at value k shows the cell of row k (counted from 1) of this column; other
    /// ticks get no label.
    Column { column: ColumnId },
}

impl TickLabelSource {
    pub fn is_auto(&self) -> bool {
        *self == TickLabelSource::Auto
    }
}

/// How the plots of a layer are moved against each other (Origin's "Offset Grouped Data in
/// Layer" and its "Stacked Lines by Y Offsets" and "Waterfall" graphs). The rules are in the
/// graphing spec, section 13.
///
/// The members of the stack are the plots whose Y column holds numbers and that do not follow
/// another curve (see [`PlotOffset::follows`]), in plot order. Hidden plots are members too, so
/// that hiding a curve does not move the others.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Stack {
    /// No offsets.
    #[default]
    None,
    /// The first member stays where it is. Every later member is moved up until its lowest point
    /// lies one gap above the highest point of the member before it (after that one was moved).
    /// The gap is `gap_percent` percent of the height of the tallest member. Recomputed from the
    /// data at every drawing.
    Auto { gap_percent: f64 },
    /// Member k (counted from 0) is moved up by `k * step` axis units.
    Constant { step: f64 },
    /// Each member is moved by its own offset from [`Layer::plot_offsets`].
    Individual,
    /// Each member's drawn Y is its Y plus the sum of the Y values of all members before it at the
    /// same row (graphing spec 13.15). Pointwise by row index, so it is not a pair of offsets: see
    /// [`crate::stack::cumulative_add`]. Lines, scatter and line + symbol plots only.
    Cumulative,
    /// A simple two-dimensional waterfall: member k is drawn moved right by `k * dx_percent` and
    /// up by `k * dy_percent` percent of the front frame, which shrinks to make room. Drawn only
    /// for a layer that is not linked to another and has no linked layers.
    Waterfall { dx_percent: f64, dy_percent: f64 },
}

/// How the curves of a waterfall layer are coloured (Origin's Waterfall Y:Color Mapping and Waterfall Z:Color
/// Mapping). Sheaf's rules are in graphing spec 13.17.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WaterfallColor {
    /// Each curve has the colour of its plot.
    #[default]
    None,
    /// Each curve is coloured along its points by their Y values through the colour ramp.
    Y,
    /// Each curve is coloured as a whole by its Z value (see [`crate::stack::waterfall_z`]).
    Z,
}

impl WaterfallColor {
    pub fn is_none(&self) -> bool {
        *self == WaterfallColor::None
    }
}

impl Stack {
    pub fn is_none(&self) -> bool {
        *self == Stack::None
    }
}

/// Stack settings of the curves of one Y column in a layer (an entry of
/// [`Layer::plot_offsets`]).
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlotOffset {
    /// The Y column of the curve.
    pub column: ColumnId,
    /// Offset in X, in axis units; used in [`Stack::Individual`] mode.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub x: f64,
    /// Offset in Y, in axis units; used in [`Stack::Individual`] mode.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub y: f64,
    /// Scale of the original X values before adding `x`, in [`Stack::Individual`] mode.
    #[serde(default = "unit_multiplier", skip_serializing_if = "is_unit_multiplier")]
    pub x_multiplier: f64,
    /// Scale of the original Y values before adding `y`, in [`Stack::Individual`] mode.
    #[serde(default = "unit_multiplier", skip_serializing_if = "is_unit_multiplier")]
    pub y_multiplier: f64,
    /// When set, the curve has no place of its own in the stack: it takes the offset (and the
    /// waterfall shift) of the first member whose Y column is this one. A fitted curve drawn over
    /// its data uses it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub follows: Option<ColumnId>,
}

impl PlotOffset {
    /// Identity placement for a curve, without adding an entry to its layer.
    pub fn new(column: ColumnId) -> Self {
        Self { column, x: 0.0, y: 0.0, x_multiplier: 1.0, y_multiplier: 1.0, follows: None }
    }
}

fn unit_multiplier() -> f64 {
    1.0
}

fn is_unit_multiplier(value: &f64) -> bool {
    *value == 1.0
}

fn default_rescale_margin() -> f64 {
    crate::autorange::DEFAULT_RESCALE_MARGIN_PCT
}

fn is_default_rescale_margin(v: &f64) -> bool {
    *v == crate::autorange::DEFAULT_RESCALE_MARGIN_PCT
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

/// How a layer follows its parent layer.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayerLink {
    /// Index of the parent in [`Graph::layers`]; it must be smaller than the layer's own index.
    pub parent: usize,
    #[serde(default)]
    pub x: ScaleLink,
    #[serde(default)]
    pub y: ScaleLink,
}

/// How one axis scale of a linked layer relates to the parent's.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ScaleLink {
    /// The layer's own scale.
    #[default]
    None,
    /// Exactly the parent's scale: limits, scale type, ticks and direction. The automatic range
    /// covers the data of the parent and of every layer linked this way.
    Straight,
}

/// Which edge of the frame an axis is drawn on.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AxisSide {
    /// Bottom for X, left for Y.
    #[default]
    Normal,
    /// Top for X, right for Y: line, ticks, tick labels and title move there.
    Opposite,
    /// Nothing of the axis is drawn; its scale still places the data.
    Hidden,
}

impl AxisSide {
    fn is_normal(&self) -> bool {
        *self == AxisSide::Normal
    }
}

/// Which plots a legend lists.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LegendScope {
    /// The plots of its own layer.
    #[default]
    Layer,
    /// The plots of every layer drawn in its layer's frame ([`Graph::same_frame`]: the layer and
    /// its right Y layer), layer by layer. In a graph of one panel that is every layer.
    Graph,
}

impl LegendScope {
    fn is_layer(&self) -> bool {
        *self == LegendScope::Layer
    }
}

fn is_false(b: &bool) -> bool {
    !*b
}

impl Layer {
    /// The ridgeline settings a plot added to this layer starts with: those of its first ridgeline
    /// plot (so all ridges share bandwidth, display and look), else the defaults.
    pub fn inherited_ridge(&self) -> RidgelineStyle {
        self.plots.iter().find(|p| p.kind == PlotKind::Ridgeline).map(|p| p.ridge.clone()).unwrap_or_default()
    }

    /// The hexbin settings a plot added to this layer starts with: those of its first hexbin plot, else the
    /// defaults.
    pub fn inherited_hexbin(&self) -> HexbinStyle {
        self.plots.iter().find(|p| p.kind == PlotKind::Hexbin).map(|p| p.hexbin).unwrap_or_default()
    }

    /// The parallel settings a plot added to this layer starts with: those of its first parallel plot, else the
    /// defaults.
    pub fn inherited_parallel(&self) -> ParallelStyle {
        self.plots.iter().find(|p| p.kind.is_parallel()).map(|p| p.parallel).unwrap_or_default()
    }

    /// The sunburst settings a plot added to this layer starts with: those of its first sunburst plot, else the defaults.
    pub fn inherited_sunburst(&self) -> SunburstStyle {
        self.plots.iter().find(|p| p.kind == PlotKind::Sunburst).map(|p| p.sunburst.clone()).unwrap_or_default()
    }

    /// The treemap settings a plot added to this layer starts with: those of its first treemap plot, else the defaults.
    pub fn inherited_treemap(&self) -> TreemapStyle {
        self.plots.iter().find(|p| p.kind == PlotKind::Treemap).map(|p| p.treemap.clone()).unwrap_or_default()
    }

    /// Recolors the plots whose color still comes from the palette, in plot order, and leaves
    /// colors chosen by hand alone. Used when plots are added or removed: with a palette that
    /// spreads its colors over the number of plots, every automatic color moves along.
    ///
    /// A column plot gets the color as its fill only; its outline keeps its own color (black in
    /// the column template, as in Origin).
    pub fn recolor_automatic(&mut self) {
        let n = self.plots.len();
        let g = self.group_style;
        for (i, p) in self.plots.iter_mut().enumerate().filter(|(_, p)| p.auto_color) {
            // With subgroups the colour list runs over the plots of a subgroup (or over the subgroups).
            let ((ci, cn), _, _) = group_steps(g.subgrouping, g.subgroup_on, i, n);
            let c = self.palette.color(ci, cn);
            if !p.kind.is_bar() {
                p.line.color = c;
            }
            p.symbol.color = c;
        }
    }

    /// Gives the plots their line styles, symbol shapes and interiors by the increments of
    /// [`Layer::group_style`], starting from the first plot's, and their automatic colours.
    pub fn apply_group_increments(&mut self) {
        let Some(first) = self.plots.first() else { return };
        let dash0 = Dash::ALL.iter().position(|d| *d == first.line.dash).unwrap_or(0);
        let shape0 = SymbolShape::ALL.iter().position(|s| *s == first.symbol.shape).unwrap_or(0);
        let filled0 = first.symbol.filled;
        let g = self.group_style;
        let n = self.plots.len();
        for (i, p) in self.plots.iter_mut().enumerate() {
            let (_, line_step, shape_step) = group_steps(g.subgrouping, g.subgroup_on, i, n);
            if g.line_style == Increment::ByOne {
                p.line.dash = Dash::ALL[(dash0 + line_step) % Dash::ALL.len()];
            }
            if g.symbol_type == Increment::ByOne {
                p.symbol.shape = SymbolShape::ALL[(shape0 + shape_step) % SymbolShape::ALL.len()];
            }
            if g.symbol_interior == Increment::ByOne {
                p.symbol.filled = if shape_step % 2 == 0 { filled0 } else { !filled0 };
            }
        }
        self.recolor_automatic();
    }

    /// Origin's Dependent edit mode: the line, symbol and decorations of plot `leader` are given to
    /// every plot of the layer, except the properties that are incremented (colours unless the
    /// colour increment is None), then the increments are applied again from the first plot.
    pub fn follow_leader(&mut self, leader: usize) {
        let Some(src) = self.plots.get(leader).cloned() else { return };
        let g = self.group_style;
        let same_color = self.palette.mode == PaletteMode::Same;
        for (i, q) in self.plots.iter_mut().enumerate() {
            if i == leader {
                continue;
            }
            q.line.width_pt = src.line.width_pt;
            q.line.connect = src.line.connect;
            q.line.sort_x = src.line.sort_x;
            if g.line_style == Increment::None {
                q.line.dash = src.line.dash;
            }
            q.symbol.size_pt = src.symbol.size_pt;
            if g.symbol_type == Increment::None {
                q.symbol.shape = src.symbol.shape;
            }
            if g.symbol_interior == Increment::None {
                q.symbol.filled = src.symbol.filled;
            }
            q.decor.drop = src.decor.drop.clone();
            q.decor.symbol_line = src.decor.symbol_line;
            q.decor.labels = src.decor.labels.clone();
            q.decor.fill_under = src.decor.fill_under.map(|_| q.line.color);
            q.decor.fill_under_mode = src.decor.fill_under_mode;
            if same_color {
                q.line.color = src.line.color;
                q.symbol.color = src.symbol.color;
                q.auto_color = src.auto_color;
            }
        }
        self.apply_group_increments();
    }

    /// Recolors all plots from the palette, in plot order, including colors chosen by hand
    /// (the "Recolor plots" command); afterwards every color is automatic again.
    pub fn apply_palette(&mut self) {
        for p in &mut self.plots {
            p.auto_color = true;
        }
        self.recolor_automatic();
    }

    /// The stack settings of the curves of Y column `column`, if any were stored.
    pub fn plot_offset(&self, column: ColumnId) -> Option<&PlotOffset> {
        self.plot_offsets.iter().find(|o| o.column == column)
    }

    /// The stack settings of Y column `column`, added with no offset when missing.
    pub fn plot_offset_mut(&mut self, column: ColumnId) -> &mut PlotOffset {
        let i = match self.plot_offsets.iter().position(|o| o.column == column) {
            Some(i) => i,
            None => {
                self.plot_offsets.push(PlotOffset::new(column));
                self.plot_offsets.len() - 1
            }
        };
        &mut self.plot_offsets[i]
    }

    /// The column whose curve the curves of `column` move with, when they follow one.
    pub fn follows(&self, column: ColumnId) -> Option<ColumnId> {
        self.plot_offset(column).and_then(|o| o.follows).filter(|leader| *leader != column)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ScaleKind {
    #[default]
    Linear,
    Log10,
    /// Natural logarithm: ticks at powers of e.
    Ln,
    /// Base-2 logarithm: ticks at powers of 2.
    Log2,
    /// `1/x`, drawn with larger values at the top.
    Reciprocal,
    /// `1/(x + 273.14)`.
    OffsetReciprocal,
    /// Percent values placed by the normal quantile function, ticks from a list (0.01 .. 99.99).
    Probability,
    /// Percent values labelled with their probit (normal quantile plus 5).
    Probit,
    /// Percent values placed by `ln(p / (100 - p))`.
    Logit,
    /// Percent values placed by `ln(-ln(1 - p))`.
    Weibull,
}

/// How an axis picks its limits when `min`/`max` are not fixed.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AutoRange {
    /// As Origin's Normal rescale: the data span plus the Rescale Margin
    /// ([`Axis::rescale_margin_pct`], 8 % by default) on each side, rounded outwards to half a major
    /// step (a minor tick) on a linear axis and to the next 1..9 tick on a log axis
    /// (see [`crate::autorange`]). With a margin of 0 the limits are the data limits.
    #[default]
    Nice,
    /// Exactly the data range, no rounding.
    Tight,
    /// Symmetric about zero (`-L..+L`) so that zero sits in the middle.
    /// The limit is a whole number of tick steps, at most three per side.
    SymmetricZero,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TickDirection {
    #[default]
    In,
    Out,
    Both,
    None,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case", tag = "kind", content = "digits")]
pub enum TickFormat {
    /// As few digits as the tick step needs.
    #[default]
    Auto,
    Decimal(u8),
    Scientific(u8),
    /// The axis values are date numbers (see `datetime`): ticks fall on round times and labels
    /// are dates (with the given pattern) or times of day, depending on the tick step.
    DateTime(crate::datetime::DatePattern),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Font {
    pub family: String,
    pub size_pt: f64,
    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub italic: bool,
    pub color: Color,
}

impl Font {
    pub fn new(family: &str, size_pt: f64) -> Self {
        Font { family: family.to_string(), size_pt, bold: false, italic: false, color: Color::BLACK }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Axis {
    #[serde(default)]
    pub scale: ScaleKind,
    /// Fixed lower limit; `None` means automatic.
    pub min: Option<f64>,
    /// Fixed upper limit; `None` means automatic.
    pub max: Option<f64>,
    #[serde(default)]
    pub auto_range: AutoRange,
    /// Rescale Margin in percent of the data span, added on each side by [`AutoRange::Nice`].
    #[serde(default = "default_rescale_margin", skip_serializing_if = "is_default_rescale_margin")]
    pub rescale_margin_pct: f64,
    /// Distance between major ticks; `None` means automatic.
    pub major_step: Option<f64>,
    /// Number of minor ticks between two major ticks.
    pub minor_ticks: u32,
    /// Title text. `None` means "build it from the first plot's column":
    /// long name plus units in parentheses.
    pub title: Option<String>,
    pub show_title: bool,
    pub title_font: Font,
    pub show_tick_labels: bool,
    pub tick_font: Font,
    #[serde(default)]
    pub tick_format: TickFormat,
    /// Tick labels show the axis value divided by this factor (1 = unchanged).
    pub tick_label_divisor: f64,
    pub major_tick: TickDirection,
    pub minor_tick: TickDirection,
    pub major_tick_len_pt: f64,
    pub minor_tick_len_pt: f64,
    /// Width of the axis line and of the ticks.
    pub line_width_pt: f64,
    pub color: Color,
    /// Draw the axis line on the opposite side too (top for X, right for Y).
    /// With both axes set this closes the box around the plot.
    pub show_opposite: bool,
    /// Draw ticks (without labels) on the opposite side.
    pub opposite_ticks: bool,
    pub major_grid: Option<LineStyle>,
    pub minor_grid: Option<LineStyle>,
    /// The edge the axis is drawn on. With [`AxisSide::Opposite`], `show_opposite` and
    /// `opposite_ticks` refer to the bottom (X) or left (Y) edge.
    #[serde(default, skip_serializing_if = "AxisSide::is_normal")]
    pub side: AxisSide,
    /// How far the axis line stands outward from its edge of the frame, in percent of the frame's
    /// width (Origin's axis position "% From Left" / "% From Right", measured in percent of the X axis
    /// length, graphing spec 21.3). Negative values move it inward: -50 puts a Y axis in the middle.
    /// Used by the Y axis, whose line, ticks, tick labels and title move together; the frame, grid and
    /// the line on the opposite edge stay. 0 (the default) is the edge itself.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub offset_pct: f64,
    /// Draw the axis line, its ticks and its tick labels in the color of the layer's first
    /// visible plot instead of `color` and the tick font's color.
    #[serde(default, skip_serializing_if = "is_false")]
    pub color_follows_plot: bool,
    /// Draw the title in that color too.
    #[serde(default, skip_serializing_if = "is_false")]
    pub title_color_follows_plot: bool,
    /// Where the tick labels come from: numbers, or the cells of a text column such as sample names.
    #[serde(default, skip_serializing_if = "TickLabelSource::is_auto")]
    pub tick_label_source: TickLabelSource,
    /// Counter-clockwise rotation of the tick labels in degrees. Used for X axes; a Y axis ignores it.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub tick_label_angle_deg: f64,
    /// Leave out the tick label at the far end of the axis: the top end of a Y axis, the right
    /// end of an X axis. Panels that touch use it, so that the labels at the joint do not run into
    /// the labels of the next panel (graphing spec section 15).
    #[serde(default, skip_serializing_if = "is_false")]
    pub hide_end_label: bool,
    /// Draw the axis the other way round: larger values towards the origin of the page's axes (Origin's
    /// Reverse check box). It adds to a lower limit that is above the upper one.
    #[serde(default, skip_serializing_if = "is_false")]
    pub reverse: bool,
    /// A tick sits at this value and at whole steps from it (Origin's First Tick, "anchor"); `None` means
    /// the ticks are at multiples of the step. Linear axes only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_tick: Option<f64>,
    /// The breaks of the axis (Origin allows 1 to 7, [`MAX_AXIS_BREAKS`]): regions of values that are left
    /// out, each with a gap in the axis line. Files written before several breaks existed hold one break
    /// under the name `axis_break`; it is read as a list of one.
    #[serde(
        default,
        alias = "axis_break",
        deserialize_with = "breaks_one_or_many",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub axis_breaks: Vec<AxisBreak>,
    /// Origin's Grids tab "Additional Lines": a line across the layer at this value of the OTHER axis,
    /// in this axis' line style (for the X axis a horizontal line at Y = value, labelled "Y =" in
    /// Origin's dialog; for the Y axis a vertical line at X = value). `None` means off; Origin's
    /// default value is 0 (graphing.md 3.7).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub additional_line: Option<f64>,
    /// Extra ticks at chosen values, each with its own label (the Special Ticks tab of Origin).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub special_ticks: Vec<SpecialTick>,
    /// An expression of `x` (the tick value) whose result is written as the tick label, for example
    /// `x*100` or `1/x`; empty means the value itself.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub tick_label_formula: String,
    /// Write labels at the minor ticks too.
    #[serde(default, skip_serializing_if = "is_false")]
    pub minor_tick_labels: bool,
    /// Colour of the ticks; `None` means the colour of the axis line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tick_color: Option<Color>,
    /// Thickness of the ticks in points; `None` means the thickness of the axis line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tick_width_pt: Option<f64>,
    /// Text put before and after every tick label.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub tick_prefix: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub tick_suffix: String,
    /// The rug beside the axis' own edge (bottom or left; the edge the axis line is on, see [`Axis::side`]).
    #[serde(default, skip_serializing_if = "Rug::is_default")]
    pub rug: Rug,
    /// The rug beside the opposite edge (top for X, right for Y).
    #[serde(default, skip_serializing_if = "Rug::is_default")]
    pub rug_opposite: Rug,
}

/// Origin's Rug tab of the Axis dialog (2019b, graphing spec 18.10): a strip beside an axis edge with one
/// short line per plotted point, projected onto that axis. [`Rug::default`] is "no rug"; the numbers
/// are Sheaf's own choices (Origin's are unmeasured).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Rug {
    /// Show the rug panel.
    pub show: bool,
    /// Which plot's points: the Y column of the plot (the first plot of the layer with that Y column);
    /// `None` is Origin's `<All>`. A column no plot of the layer uses any more also means `<All>`.
    /// A column id, not a position, so removing, reordering or moving plots keeps the choice.
    pub plot: Option<ColumnId>,
    /// Line colour; `None` is Auto, the colour of each plot.
    pub color: Option<Color>,
    /// Line thickness in points.
    pub thickness_pt: f64,
    /// Transparency of the lines, 0 to 100 percent.
    pub transparency_pct: f64,
    /// Panel size across the axis, in percent of the average of the layer's width and height.
    pub size_pct: f64,
    /// All plots share one strip (Origin's Overlap); off gives each plot a sub-strip of its own.
    pub overlap: bool,
    /// Space between sub-strips when `overlap` is off, in percent of the panel size.
    pub gap_pct: f64,
    /// Space left empty at both ends of the lines across the panel, in percent of the panel size (below 50).
    pub margin_pct: f64,
    /// Border lines at the two ends of the panel along the axis (perpendicular to the axis).
    pub perpendicular_border: bool,
    /// Border lines along the panel's inner and outer edges (parallel to the axis).
    pub parallel_border: bool,
}

impl Default for Rug {
    fn default() -> Rug {
        Rug {
            show: false,
            plot: None,
            color: None,
            thickness_pt: 0.5,
            transparency_pct: 0.0,
            size_pct: 5.0,
            overlap: true,
            gap_pct: 10.0,
            margin_pct: 0.0,
            perpendicular_border: false,
            parallel_border: false,
        }
    }
}

impl Rug {
    pub fn is_default(&self) -> bool {
        *self == Rug::default()
    }

    /// A shown rug with the defaults (what the Scatter + Rug template puts on the bottom and left axes).
    pub fn shown() -> Rug {
        Rug { show: true, ..Rug::default() }
    }

    /// The values the drawing uses: percentages and the thickness limited to their ranges.
    pub fn size_frac(&self) -> f64 {
        self.size_pct.clamp(0.0, 100.0) / 100.0
    }
}

/// An extra tick of an axis.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpecialTick {
    pub value: f64,
    /// The label; empty means the value written as the axis writes its numbers.
    #[serde(default)]
    pub label: String,
}

/// The most breaks an axis has (Origin's Number of Breaks lists None and 1 to 7).
pub const MAX_AXIS_BREAKS: usize = 7;

/// The width of the gap of a break in percent of the axis length when the user has not set one. Origin
/// 2016's automatic Break Half Length left gaps of 1.67 % to 1.79 % of the axis length on 879 and 1090 px
/// axes with one to three breaks (graphing spec 3.6).
pub const BREAK_GAP_PCT: f64 = 1.75;

/// A break in an axis (graphing spec 3.6): the values from `from` to `to` are cut out; the axis
/// shows a gap there, and the part after it can have its own tick step.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AxisBreak {
    pub from: f64,
    pub to: f64,
    /// The middle of the gap, in percent of the axis length from its low end. Used when
    /// `auto_position` is off.
    pub position_pct: f64,
    /// Width of the gap in percent of the axis length.
    pub gap_pct: f64,
    /// Major tick step after the break; `None` keeps the step of the axis.
    pub step_after: Option<f64>,
    /// Origin's "Auto Position" (on for a new break): the breaks of the axis share it evenly, break k of
    /// n sitting at k / (n + 1) of its length, whatever `position_pct` says. Off in files written before
    /// the setting existed, so that their breaks stay where they were.
    #[serde(default)]
    pub auto_position: bool,
}

impl AxisBreak {
    /// A break of the values `from..to` placed automatically (in the middle when it is the only one).
    pub fn new(from: f64, to: f64) -> AxisBreak {
        AxisBreak { from, to, position_pct: 50.0, gap_pct: BREAK_GAP_PCT, step_after: None, auto_position: true }
    }

    /// The break Origin's dialog adds after `existing` on an axis from `min` to `max`: the middle half of
    /// what lies after the last break (or of the whole axis for the first one), as Origin 2016 filled the
    /// rows when the Number of Breaks went up one at a time (graphing spec 3.6). It takes the gap width of
    /// the breaks before it.
    pub fn next_after(existing: &[AxisBreak], min: f64, max: f64) -> AxisBreak {
        let (lo, hi) = (min.min(max), min.max(max));
        let start = existing.iter().map(|b| b.to).filter(|t| t.is_finite() && *t > lo && *t < hi).fold(lo, f64::max);
        let span = hi - start;
        let mut b = AxisBreak::new(start + span * 0.25, start + span * 0.75);
        if let Some(last) = existing.last() {
            b.gap_pct = last.gap_pct;
        }
        b
    }
}

/// Reads the breaks of an axis written either as a list or, by files from before several breaks, as a
/// single break (or `null`).
fn breaks_one_or_many<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<AxisBreak>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OneOrMany {
        Many(Vec<AxisBreak>),
        One(AxisBreak),
    }
    Ok(match Option::<OneOrMany>::deserialize(d)? {
        None => Vec::new(),
        Some(OneOrMany::One(b)) => vec![b],
        Some(OneOrMany::Many(v)) => v,
    })
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Dash {
    #[default]
    Solid,
    Dash,
    Dot,
    DashDot,
    DashDotDot,
    ShortDash,
    ShortDot,
    ShortDashDot,
}

impl Dash {
    /// Dash pattern as multiples of the line width (on, off, on, off, ...), as Origin draws the
    /// eight line styles (graphing spec section 2.1; measured on lines 1.5 to 2 pt wide, where the
    /// pattern grows with the width). Empty for a solid line.
    pub fn pattern(&self) -> &'static [f64] {
        match self {
            Dash::Solid => &[],
            Dash::Dash => &[5.1, 4.9],
            Dash::Dot => &[1.6, 4.3],
            Dash::DashDot => &[6.1, 2.4, 1.6, 2.4],
            Dash::DashDotDot => &[4.7, 2.35, 1.6, 2.4, 1.55, 2.5],
            Dash::ShortDash => &[2.6, 1.95],
            Dash::ShortDot => &[0.95, 1.05],
            Dash::ShortDashDot => &[3.95, 1.5, 0.95, 1.5],
        }
    }

    pub const ALL: [Dash; 8] = [
        Dash::Solid,
        Dash::Dash,
        Dash::Dot,
        Dash::DashDot,
        Dash::DashDotDot,
        Dash::ShortDash,
        Dash::ShortDot,
        Dash::ShortDashDot,
    ];
}

/// How consecutive data points are joined.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Connect {
    #[default]
    Straight,
    /// A natural cubic spline through every point; needs increasing X and at most 900 points,
    /// otherwise the points are joined by straight segments (as Origin does).
    Spline,
    /// A cubic B-spline that winds around the points, starting and ending at the end points.
    BSpline,
    /// Cubic Bezier segments over groups of four points (consecutive groups share an end point).
    Bezier,
    /// Horizontal first, then vertical.
    StepHorizontal,
    /// Vertical first, then horizontal.
    StepVertical,
    /// Origin's "2 Point Segment": points 1-2, 3-4, ... are joined, the pairs are not (LabTalk line
    /// connection 2, measured 2026-11-15).
    Segment2,
    /// Origin's "3 Point Segment": points 1-2-3, 4-5-6, ... are joined (line connection 3).
    Segment3,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LineStyle {
    pub color: Color,
    pub width_pt: f64,
    #[serde(default)]
    pub dash: Dash,
    #[serde(default)]
    pub connect: Connect,
    /// Connect the plotted points in increasing X order instead of worksheet order (Origin 2023's
    /// `layer.plotN.line.sortx`). Only the line path follows it; the worksheet is untouched.
    #[serde(default, skip_serializing_if = "is_false")]
    pub sort_x: bool,
}

impl LineStyle {
    pub fn solid(color: Color, width_pt: f64) -> Self {
        LineStyle { color, width_pt, dash: Dash::Solid, connect: Connect::Straight, sort_x: false }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SymbolShape {
    Square,
    #[default]
    Circle,
    TriangleUp,
    TriangleDown,
    Diamond,
    Plus,
    Cross,
    Star,
}

impl SymbolShape {
    pub const ALL: [SymbolShape; 8] = [
        SymbolShape::Square,
        SymbolShape::Circle,
        SymbolShape::TriangleUp,
        SymbolShape::TriangleDown,
        SymbolShape::Diamond,
        SymbolShape::Plus,
        SymbolShape::Cross,
        SymbolShape::Star,
    ];
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SymbolStyle {
    pub shape: SymbolShape,
    /// Width of the symbol's bounding box.
    pub size_pt: f64,
    pub color: Color,
    /// Filled with the color, or drawn as an outline.
    pub filled: bool,
    pub edge_width_pt: f64,
    /// Drop Lines > Skip Points: draw point decorations (symbols, drop lines, errors and labels)
    /// on the first of every n source points in the plot range (0 or 1 = every point).
    #[serde(default = "one")]
    pub every: u32,
}

fn one() -> u32 {
    1
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PlotKind {
    #[default]
    Line,
    Scatter,
    LineSymbol,
    /// Vertical bars from zero up to each value. The bar is filled with the
    /// symbol color and outlined with the line style.
    Column,
    /// A line with the area between it and Y = 0 filled with the symbol color (Origin's Area
    /// plot). Where 0 is outside the Y axis the fill reaches the edge of the frame.
    Area,
    /// Columns piled on each other: every plot of this kind in a layer shares one bar slot per
    /// X position, and each bar starts where the one before it (in plot order) ends (Origin's
    /// plot type 213).
    StackedColumn,
    /// Like [`PlotKind::StackedColumn`], with every pile scaled to 100 (Origin's "100 % Stacked
    /// Column"). The Y axis runs from 0 to 100.
    Stacked100Column,
    /// Horizontal bars (Origin's plot type 215): the same bars as [`PlotKind::Column`] with the
    /// roles of the axes exchanged: the horizontal axis (the layer's X axis here) carries the values and
    /// the vertical axis (the Y axis) the bar positions. Origin names them the other way round.
    Bar,
    /// A column from the plot's base column (`Plot::y_base`) up to its Y column (Origin's
    /// plot type 207).
    FloatingColumn,
    /// Bars piled on each other, lying down: the horizontal form of [`PlotKind::StackedColumn`]
    /// (Origin's plot type 216 with the stacked bar template). The piles grow from left to right.
    StackedBar,
    /// Like [`PlotKind::StackedBar`], with every pile scaled to 100; the value axis runs from 0 to 100.
    Stacked100Bar,
    /// The horizontal form of [`PlotKind::FloatingColumn`]: a bar from the plot's base column
    /// (`Plot::y_base`) to its Y column along the horizontal axis.
    FloatingBar,
    /// Areas piled on each other (Origin's plot type 214): each plot's area runs from the curve of
    /// the plots before it (in plot order) up to the sum of their values and its own.
    StackedArea,
    /// Like [`PlotKind::StackedArea`] with every pile scaled to 100; the Y axis runs from 0 to 100.
    Stacked100Area,
    /// A histogram of the plot's Y column (Origin's plot type 219): the values are counted into bins
    /// (`Plot::hist`) and drawn as touching bars; the bars stand at the bin centres.
    Histogram,
    /// A box chart of the plot's Y column (Origin's plot type 206): one box per plot, standing at
    /// 1, 2, 3, ... in plot order, with the median, the mean, whiskers and outliers.
    Box,
    /// Origin's violin plot (2019, template Violin.otpu): each plot's Y column as a mirrored
    /// kernel density silhouette standing at 1, 2, 3, ... in plot order like [`PlotKind::Box`],
    /// with the median marked inside.
    Violin,
    /// Origin's "Violin with Box" (ViolinBox.otpu): the silhouette with a narrow box, whiskers
    /// and median inside.
    ViolinBox,
    /// Origin's "Violin with Point" (ViolinData.otpu): the silhouette with every observation
    /// drawn as a point inside it.
    ViolinPoint,
    /// Origin's "Violin with Quartile" (ViolinQuartile.otpu): the silhouette with lines at the
    /// quartiles and the median inside.
    ViolinQuartile,
    /// Origin's "Violin with Stick" (ViolinStick.otpu): the silhouette with every observation
    /// drawn as a line across it.
    ViolinStick,
    /// Origin's "Half Violin" (HalfViolin.otpu): one half of the silhouette, with a half box on
    /// the other side.
    HalfViolin,
    /// Origin's ridgeline chart (2022, ridgeline.OTPU): the plot's Y column as a density curve on
    /// a baseline at its column index (graphing spec 14.23).
    Ridgeline,
    /// Origin's hexbin plot (2027, Hexbin.OTPU): the plot's X and Y columns binned into hexagons coloured by
    /// the number of points in each (graphing spec 14.24).
    Hexbin,
    /// Origin's Parallel Plot (2019, Parallel.otpu): the plot's Y column is one vertical axis; every row is a
    /// line through the axes of all the layer's parallel plots, coloured by a numeric column (graphing spec 14.25).
    Parallel,
    /// Origin's Parallel Index: like [`PlotKind::Parallel`], with the lines coloured by the group (the text of a
    /// column) of their row, and a legend of the groups.
    ParallelIndex,
    /// Origin's Treemap plot (2025b, treemap.otpu): the plot's Y column is the value, the category columns of its
    /// [`TreemapStyle`] the nested levels (graphing spec 14.26).
    Treemap,
    /// Origin's Sunburst plot (2022, Sunburst.OTP): the plot's Y column is the value, the category columns of its
    /// [`SunburstStyle`] the rings, the first innermost (graphing spec 14.27).
    Sunburst,
    /// A pie chart of the plot's Y column (Origin's plot type 225): one slice per row, clockwise
    /// from 12 o'clock, each labelled with its share. The layer draws no axes.
    Pie,
    /// Origin's 3D Color Pie: the pie seen from above at a slant, an ellipse about half as high as
    /// wide with a side band, slices in the palette colours.
    Pie3D,
    /// Origin's 2D B&W Pie: a flat pie of white slices with black hatch patterns that differ per slice.
    PieBw,
}

/// What a violin plot draws inside (or beside) its silhouette.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ViolinInner {
    /// A marker at the median.
    Median,
    /// A narrow box from the first to the third quartile with whiskers and the median.
    Box,
    /// Lines at the quartiles and the median.
    Quartile,
    /// Every observation as a point.
    Point,
    /// Every observation as a line across the silhouette.
    Stick,
}

impl PlotKind {
    /// True for the six violin kinds.
    pub fn is_violin(&self) -> bool {
        self.violin_inner().is_some()
    }

    /// True for the kinds drawn as a distribution per plot at 1, 2, 3, ...: box chart and violins.
    pub fn is_distribution(&self) -> bool {
        *self == PlotKind::Box || self.is_violin() || self.is_ridgeline()
    }

    /// True for [`PlotKind::Parallel`] and [`PlotKind::ParallelIndex`].
    pub fn is_parallel(&self) -> bool {
        matches!(self, PlotKind::Parallel | PlotKind::ParallelIndex)
    }

    /// True for [`PlotKind::Hexbin`].
    pub fn is_hexbin(&self) -> bool {
        *self == PlotKind::Hexbin
    }

    /// True for [`PlotKind::Ridgeline`].
    pub fn is_ridgeline(&self) -> bool {
        *self == PlotKind::Ridgeline
    }

    /// What a violin kind draws inside its silhouette; `None` for other kinds.
    pub fn violin_inner(&self) -> Option<ViolinInner> {
        match self {
            PlotKind::Violin => Some(ViolinInner::Median),
            PlotKind::ViolinBox | PlotKind::HalfViolin => Some(ViolinInner::Box),
            PlotKind::ViolinPoint => Some(ViolinInner::Point),
            PlotKind::ViolinQuartile => Some(ViolinInner::Quartile),
            PlotKind::ViolinStick => Some(ViolinInner::Stick),
            _ => None,
        }
    }

    /// True for [`PlotKind::HalfViolin`]: only the right half of the silhouette is drawn.
    pub fn is_half_violin(&self) -> bool {
        *self == PlotKind::HalfViolin
    }

    /// Kinds whose Drop Lines tab offers Skip Points. Area and bar plots keep every point.
    pub fn supports_skip_points(&self) -> bool {
        matches!(self, PlotKind::Line | PlotKind::Scatter | PlotKind::LineSymbol)
    }

    pub fn has_line(&self) -> bool {
        matches!(self, PlotKind::Line | PlotKind::LineSymbol) || self.is_area()
    }

    /// True for the kinds drawn as a filled area under a curve.
    pub fn is_area(&self) -> bool {
        matches!(self, PlotKind::Area | PlotKind::StackedArea | PlotKind::Stacked100Area)
    }

    /// True for every kind drawn as rectangles standing on a base: the column kinds and bars.
    pub fn is_bar(&self) -> bool {
        matches!(
            self,
            PlotKind::Column
                | PlotKind::StackedColumn
                | PlotKind::Stacked100Column
                | PlotKind::Bar
                | PlotKind::FloatingColumn
                | PlotKind::StackedBar
                | PlotKind::Stacked100Bar
                | PlotKind::FloatingBar
                | PlotKind::Histogram
        )
    }

    /// True for the kinds that pile on each other instead of standing side by side.
    pub fn is_stacked(&self) -> bool {
        self.is_stacked_column() || self.is_stacked_area()
    }

    /// True for the piled column and piled bar kinds.
    pub fn is_stacked_column(&self) -> bool {
        matches!(
            self,
            PlotKind::StackedColumn | PlotKind::Stacked100Column | PlotKind::StackedBar | PlotKind::Stacked100Bar
        )
    }

    /// True for the floating column and floating bar kinds: every plot is a bar between two columns,
    /// and all the plots of a layer stand in the same slot, on top of each other (measured, spec 14.12).
    pub fn is_floating(&self) -> bool {
        matches!(self, PlotKind::FloatingColumn | PlotKind::FloatingBar)
    }

    /// True for the piled area kinds.
    pub fn is_stacked_area(&self) -> bool {
        matches!(self, PlotKind::StackedArea | PlotKind::Stacked100Area)
    }

    /// True when every pile is scaled to a total of 100.
    pub fn is_percent_stacked(&self) -> bool {
        matches!(self, PlotKind::Stacked100Column | PlotKind::Stacked100Bar | PlotKind::Stacked100Area)
    }

    /// True when the bars lie down: values along the horizontal axis.
    pub fn is_horizontal(&self) -> bool {
        matches!(self, PlotKind::Bar | PlotKind::StackedBar | PlotKind::Stacked100Bar | PlotKind::FloatingBar)
    }

    pub fn has_symbols(&self) -> bool {
        matches!(self, PlotKind::Scatter | PlotKind::LineSymbol)
    }
}

/// How the values of a [`PlotKind::Histogram`] plot are binned. `None` takes Origin's automatic
/// choice (see `sheaf_calc::freq`).
#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct HistogramStyle {
    pub bin_size: Option<f64>,
    /// First bin edge.
    pub begin: Option<f64>,
    /// End of the last bin.
    pub end: Option<f64>,
}

impl HistogramStyle {
    pub fn is_default(&self) -> bool {
        *self == HistogramStyle::default()
    }
}

/// One curve: a Y column drawn against an X column.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Plot {
    /// The X column; `None` plots against the row number (1, 2, 3, ...).
    pub x: Option<ColumnId>,
    pub y: ColumnId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y_error: Option<ColumnId>,
    /// Half-open row range `[start, end)` to plot; `None` plots all rows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rows: Option<(usize, usize)>,
    pub kind: PlotKind,
    pub line: LineStyle,
    pub symbol: SymbolStyle,
    /// Legend text. `None` means "use the Y column's comments, long name or short name".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default = "yes")]
    pub visible: bool,
    /// True while the plot's color comes from the layer's palette. Choosing a color by hand
    /// clears it, so that adding plots later re-spreads only the automatic colors.
    #[serde(default = "yes")]
    pub auto_color: bool,
    /// The plot group this plot belongs to (Origin's plot group made by one plot command). Plots
    /// of a layer with the same number form one group; `None` is a plot outside any group. A group
    /// gives its members the colours of Origin's colour list in order (colour increment "by one",
    /// first member first), whatever colour the members were given; the importer sets those
    /// colours. Column plots without a group number still stand side by side
    /// ([`Layer::bars`]), and are written to `.opj` as one group.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<u32>,
    /// Error values in X (the third column type "X error"); drawn as horizontal error bars.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x_error: Option<ColumnId>,
    /// The column that holds the lower end of each bar of a [`PlotKind::FloatingColumn`] or [`PlotKind::FloatingBar`] plot, or
    /// the lower edge of the fill of an [`PlotKind::Area`] plot (Origin's Fill Area between two
    /// curves).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y_base: Option<ColumnId>,
    /// Direction and cap width of the error bars.
    #[serde(default, skip_serializing_if = "ErrorStyle::is_default")]
    pub error: ErrorStyle,
    /// The bins of a histogram plot.
    #[serde(default, skip_serializing_if = "HistogramStyle::is_default")]
    pub hist: HistogramStyle,
    /// The settings of a ridgeline plot (graphing spec 14.23).
    #[serde(default, skip_serializing_if = "RidgelineStyle::is_default")]
    pub ridge: RidgelineStyle,
    /// The bin settings of a hexbin plot (graphing spec 14.24).
    #[serde(default, skip_serializing_if = "HexbinStyle::is_default")]
    pub hexbin: HexbinStyle,
    /// The settings of a Parallel Plot or Parallel Index (graphing spec 14.25).
    #[serde(default, skip_serializing_if = "ParallelStyle::is_default")]
    pub parallel: ParallelStyle,
    /// The settings of a treemap plot (graphing spec 14.26).
    #[serde(default, skip_serializing_if = "TreemapStyle::is_default")]
    pub treemap: TreemapStyle,
    /// The settings of a sunburst plot (graphing spec 14.27).
    #[serde(default, skip_serializing_if = "SunburstStyle::is_default")]
    pub sunburst: SunburstStyle,
    /// Indexed data of a column or box plot: the rows of `y` split by group columns (T75).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub indexed: Option<Indexed>,
    /// A bubble plot: the symbol of each point is as wide as this column's value says (Origin's
    /// Z column of a Bubble plot).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_from: Option<ColumnId>,
    /// A colour mapped plot: the symbol of each point takes its colour from this column's value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_from: Option<ColumnId>,
    /// Drop lines, the fill under the curve and data labels (the Drop Lines, Line and Label tabs
    /// of Origin's Plot Details; graphing spec section 18).
    #[serde(default, skip_serializing_if = "PlotDecor::is_default")]
    pub decor: PlotDecor,
}

/// What a plot draws besides its line, symbols, bars and error bars.
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct PlotDecor {
    /// The Line tab's Symbol/Line interface and Drop Lines' skipped-point gap control.
    #[serde(default, skip_serializing_if = "SymbolLineInterface::is_default")]
    pub symbol_line: SymbolLineInterface,
    /// Lines from each point to the Y axis (Origin's "Horizontal") and to the X axis ("Vertical").
    #[serde(default, skip_serializing_if = "DropLines::is_default")]
    pub drop: DropLines,
    /// Enables Fill Area Under Curve with this colour for filled modes; sidelines use the line
    /// style instead. `None` disables both fills and sidelines.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_under: Option<Color>,
    /// The selected fill/outline mode is retained while Fill Area Under Curve is disabled.
    #[serde(default, skip_serializing_if = "FillUnderMode::is_normal")]
    pub fill_under_mode: FillUnderMode,
    /// Transparency, hatch and gradient of the fill under a curve, of a bar or of an area (the
    /// Pattern tab). Plain by default: the flat colour only.
    #[serde(default, skip_serializing_if = "FillStyle::is_default")]
    pub fill: FillStyle,
    /// The text written next to each point.
    #[serde(default, skip_serializing_if = "DataLabels::is_default")]
    pub labels: DataLabels,
    /// Fitted curves and moving averages drawn over the plot (graphing spec 18.11), in the order
    /// added; each is recomputed from the plotted data when drawn.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trendlines: Vec<Trendline>,
}

pub use sheaf_calc::trend::TrendKind;

/// One on-graph trendline of a plot (Origin 2025b's Add Trend Line, graphing spec 18.11).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Trendline {
    pub kind: TrendKind,
    /// Write the fitted equation (not for a moving average).
    #[serde(default = "yes")]
    pub show_equation: bool,
    /// Write R-squared (not for a moving average).
    #[serde(default = "yes")]
    pub show_r2: bool,
    /// `None` is Auto: the plot's colour.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
    #[serde(default = "Trendline::default_width")]
    pub width_pt: f64,
    #[serde(default)]
    pub dash: Dash,
    /// Moves the label from its default place (points; +x right, +y down).
    #[serde(default, skip_serializing_if = "is_zero_pair")]
    pub label_offset_pt: (f64, f64),
}

/// A slope marked between two rows of one plot of a layer (Origin 2026b's Slope Annotation tool;
/// graphing spec 17.6). It is tied to the plot by its Y column (stable under File > Append) and to
/// the data by the two row numbers, so it is recomputed whenever the data or the axes change.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SlopeAnnotation {
    /// The Y column of the plot the two points belong to.
    pub plot: ColumnId,
    /// The X column of that plot, which tells apart plots of one layer that share a Y column. `None`
    /// (annotations saved without it, or a plot without an X column) matches any X. When several plots
    /// still match, the first one is used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<ColumnId>,
    /// The two rows, counted from 0 (the worksheet shows them from 1).
    pub row_a: usize,
    pub row_b: usize,
    /// Draw the horizontal run and the vertical rise as dashed legs.
    #[serde(default = "yes")]
    pub show_legs: bool,
    /// Moves the label from its default place (points; +x right, +y down).
    #[serde(default, skip_serializing_if = "is_zero_pair")]
    pub label_offset_pt: (f64, f64),
    /// `None` is the plot's colour.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
    #[serde(default = "SlopeAnnotation::default_width")]
    pub width_pt: f64,
}

impl SlopeAnnotation {
    fn default_width() -> f64 {
        1.0
    }

    /// A slope between two rows (from 0) of the plot with Y column `plot`: legs shown, plot colour, 1 pt.
    pub fn new(plot: ColumnId, row_a: usize, row_b: usize) -> Self {
        SlopeAnnotation {
            plot,
            x: None,
            row_a,
            row_b,
            show_legs: true,
            label_offset_pt: (0.0, 0.0),
            color: None,
            width_pt: 1.0,
        }
    }

    /// A slope between two rows of `plot`, tied to both its Y and its X column.
    pub fn for_plot(plot: &Plot, row_a: usize, row_b: usize) -> Self {
        SlopeAnnotation { x: plot.x, ..SlopeAnnotation::new(plot.y, row_a, row_b) }
    }

    /// Whether this annotation belongs to `plot` (same Y column, and the same X column when one is stored).
    pub fn matches(&self, plot: &Plot) -> bool {
        self.plot == plot.y && self.x.is_none_or(|x| plot.x == Some(x))
    }
}

fn is_zero_pair(v: &(f64, f64)) -> bool {
    *v == (0.0, 0.0)
}

impl Trendline {
    fn default_width() -> f64 {
        1.5
    }

    pub fn new(kind: TrendKind) -> Self {
        Trendline {
            kind,
            show_equation: true,
            show_r2: true,
            color: None,
            width_pt: Self::default_width(),
            dash: Dash::Solid,
            label_offset_pt: (0.0, 0.0),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FillUnderMode {
    #[default]
    Normal,
    /// Close each finite run directly from its last point to its first, without an axis baseline.
    InclusiveBrokenByMissingValues,
    SidelinesOnly,
    SidelinesAndBase,
}

impl FillUnderMode {
    pub const ALL: [Self; 4] =
        [Self::Normal, Self::InclusiveBrokenByMissingValues, Self::SidelinesOnly, Self::SidelinesAndBase];

    pub fn is_normal(&self) -> bool {
        *self == Self::Normal
    }

    pub fn is_sidelines(&self) -> bool {
        matches!(self, Self::SidelinesOnly | Self::SidelinesAndBase)
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::InclusiveBrokenByMissingValues => "Inclusive broken by missing values",
            Self::SidelinesOnly => "Sidelines only",
            Self::SidelinesAndBase => "Sidelines and Base",
        }
    }
}

/// How a Line + Symbol connecting line meets its point symbols. Other plot kinds ignore these
/// saved settings. Gaps start unchecked and the line starts behind symbols, as in Origin 2016.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SymbolLineInterface {
    /// The line is painted over symbols when gaps are off; the saved choice survives enabling gaps.
    #[serde(default, skip_serializing_if = "is_false")]
    pub draw_in_front: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub gap_to_symbol: bool,
    /// Keep the connecting line intact at skipped markers while retaining gaps at shown markers.
    #[serde(default, skip_serializing_if = "is_false")]
    pub no_break_skipped: bool,
}

impl SymbolLineInterface {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

impl PlotDecor {
    pub fn is_default(&self) -> bool {
        *self == PlotDecor::default()
    }
}

/// A drop line; drawn when `on`. Origin's defaults: off, solid, 0.5 pt, colour Auto (the plot's).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DropLine {
    pub on: bool,
    #[serde(default)]
    pub dash: Dash,
    pub width_pt: f64,
    /// `None` is Origin's "Auto": the colour of the plot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
    /// Where the line goes (Origin's "Drop To").
    #[serde(default, skip_serializing_if = "DropTarget::is_default")]
    pub target: DropTarget,
    /// How the line is coloured when `color` is `None`.
    #[serde(default, skip_serializing_if = "DropColorMode::is_default")]
    pub color_mode: DropColorMode,
}

impl Default for DropLine {
    fn default() -> Self {
        DropLine {
            on: false,
            dash: Dash::Solid,
            width_pt: 0.5,
            color: None,
            target: DropTarget::Axis,
            color_mode: DropColorMode::Auto,
        }
    }
}

/// Origin's "Drop To" of a drop line.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum DropTarget {
    /// To the axis (the left or bottom edge of the frame): the behaviour before lollipop plots.
    #[default]
    Axis,
    /// To the point of the next data plot of the layer at the same row; the last plot connects
    /// back to the first (Origin's lollipop plot).
    NextDataPlot,
}

impl DropTarget {
    pub fn is_default(&self) -> bool {
        *self == DropTarget::Axis
    }
    pub const ALL: [DropTarget; 2] = [DropTarget::Axis, DropTarget::NextDataPlot];
    pub fn label(&self) -> &'static str {
        match self {
            DropTarget::Axis => "Axis",
            DropTarget::NextDataPlot => "Next Data Plot",
        }
    }
}

/// The colour choice of a drop line besides a fixed colour (Origin's "Auto" and "Y Value:
/// Plus-Minus").
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum DropColorMode {
    /// The colour of the plot.
    #[default]
    Auto,
    /// By the sign of the Y value: [`PLUS_MINUS_POSITIVE`] for values of zero and above and
    /// [`PLUS_MINUS_NEGATIVE`] for negative values (Sheaf's own pair, not measured).
    PlusMinus,
}

impl DropColorMode {
    pub fn is_default(&self) -> bool {
        *self == DropColorMode::Auto
    }
}

/// The drop line colour of values of zero and above in the "Y Value: Plus-Minus" mode (Sheaf's own
/// choice, not measured in Origin): a blue.
pub const PLUS_MINUS_POSITIVE: Color = Color { r: 0, g: 114, b: 178, a: 255 };
/// The drop line colour of negative values in the "Y Value: Plus-Minus" mode (Sheaf's own choice,
/// not measured in Origin): a vermilion red.
pub const PLUS_MINUS_NEGATIVE: Color = Color { r: 213, g: 94, b: 0, a: 255 };

/// The two drop lines of a plot.
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct DropLines {
    /// From the point across to the Y axis.
    #[serde(default)]
    pub horizontal: DropLine,
    /// From the point down to the X axis.
    #[serde(default)]
    pub vertical: DropLine,
}

impl DropLines {
    pub fn is_default(&self) -> bool {
        *self == DropLines::default()
    }
}

/// What a data label says (Origin's "Label Form"; Y by default).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LabelForm {
    X,
    #[default]
    Y,
    /// Origin's "Row Indices": the row number counting from 1.
    Row,
    /// "(X,Y)".
    XY,
    /// Origin's "a column": the cell of the same row in this column, as the worksheet shows it.
    Column {
        column: ColumnId,
    },
}

/// Where a data label stands against its point (Origin's "Position"; Right by default).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LabelPosition {
    Center,
    Left,
    #[default]
    Right,
    Above,
    Below,
}

/// Frame around a plot's data labels (Origin's Label tab).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LabelFrame {
    #[default]
    None,
    Box,
    Shadow,
}

impl LabelFrame {
    pub fn is_none(&self) -> bool {
        *self == Self::None
    }
}

fn label_frame_black() -> Color {
    Color::BLACK
}

fn is_label_frame_black(color: &Color) -> bool {
    *color == Color::BLACK
}

/// The data labels of a plot (Origin's Label tab: off, Arial 22 black, form Y, position Right).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DataLabels {
    pub enable: bool,
    /// Origin's White Out: an opaque white background behind each displayed label.
    #[serde(default, skip_serializing_if = "is_false")]
    pub white_out: bool,
    #[serde(default, skip_serializing_if = "LabelFrame::is_none")]
    pub frame: LabelFrame,
    /// Frame padding on all sides, as a percentage of the font height.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub margin_percent: f64,
    /// Origin 2016 initializes the frame's border and fill to black.
    #[serde(default = "label_frame_black", skip_serializing_if = "is_label_frame_black")]
    pub frame_border: Color,
    #[serde(default = "label_frame_black", skip_serializing_if = "is_label_frame_black")]
    pub frame_fill: Color,
    #[serde(default)]
    pub form: LabelForm,
    #[serde(default)]
    pub position: LabelPosition,
    pub font: Font,
    /// Counter-clockwise rotation in degrees.
    #[serde(default)]
    pub rotate_deg: f64,
    /// Origin's Horizontal and Vertical Offset: a shift to the right and down, in points, added to
    /// the position.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub dx_pt: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub dy_pt: f64,
    /// Decimal places of a number (Origin's Numeric Display Format, only "Decimal Digits" here);
    /// `None` writes the number as the worksheet does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decimals: Option<u8>,
    /// Which rows are labelled, as a list of row numbers and ranges counted from 1 ("1,3,5-8");
    /// empty labels every row.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub only_rows: String,
}

impl Default for DataLabels {
    fn default() -> Self {
        DataLabels {
            enable: false,
            white_out: false,
            frame: LabelFrame::None,
            margin_percent: 0.0,
            frame_border: Color::BLACK,
            frame_fill: Color::BLACK,
            form: LabelForm::Y,
            position: LabelPosition::Right,
            font: Font::new("Arial", 22.0),
            rotate_deg: 0.0,
            dx_pt: 0.0,
            dy_pt: 0.0,
            decimals: None,
            only_rows: String::new(),
        }
    }
}

impl DataLabels {
    pub fn is_default(&self) -> bool {
        *self == DataLabels::default()
    }

    /// True when row `row` (counted from 0) is labelled: every row without a list, otherwise the
    /// rows the list names. An entry that is not a number or a range is ignored.
    pub fn labels_row(&self, row: usize) -> bool {
        parse_row_list(&self.only_rows).is_none_or(|ranges| ranges.iter().any(|&(a, b)| (a..=b).contains(&(row + 1))))
    }
}

/// The ranges (counted from 1) a list like `1,3,5-8` names; `None` for an empty list or one with no
/// valid entry (all rows then count).
pub fn parse_row_list(text: &str) -> Option<Vec<(usize, usize)>> {
    let mut out = Vec::new();
    for part in text.split([',', ';', ' ']).map(str::trim).filter(|p| !p.is_empty()) {
        let range = match part.split_once('-') {
            Some((a, b)) => a.trim().parse::<usize>().ok().zip(b.trim().parse::<usize>().ok()),
            None => part.parse::<usize>().ok().map(|n| (n, n)),
        };
        if let Some((a, b)) = range
            && a >= 1
            && b >= a
        {
            out.push((a, b));
        }
    }
    (!out.is_empty()).then_some(out)
}

/// Which side of the point an error bar reaches.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorDirection {
    #[default]
    Both,
    Plus,
    Minus,
}

/// Origin's default cap width of an error bar, in points, whatever the symbol size (measured:
/// 8.5 pt over all, 9 pt symbols or 18 pt or 3 pt).
pub const DEFAULT_ERROR_CAP_PT: f64 = 8.0;

/// How the error bars of a plot are drawn.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ErrorStyle {
    #[serde(default)]
    pub direction: ErrorDirection,
    /// Total width of the cap in points; 0 draws no cap.
    pub cap_width_pt: f64,
}

impl Default for ErrorStyle {
    fn default() -> Self {
        ErrorStyle { direction: ErrorDirection::Both, cap_width_pt: DEFAULT_ERROR_CAP_PT }
    }
}

impl ErrorStyle {
    pub fn is_default(&self) -> bool {
        *self == ErrorStyle::default()
    }
}

fn yes() -> bool {
    true
}

/// Where the legend sits inside the layer frame.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum LegendPosition {
    /// Stock Line/Scatter/Line+Symbol/Column upper-left anchor (graphing spec 2.4).
    Automatic,
    TopRight,
    TopLeft,
    BottomRight,
    BottomLeft,
    /// Legend centre at these fractions of the layer frame (0,0 = top-left).
    Manual {
        x: f64,
        y: f64,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Legend {
    pub visible: bool,
    pub position: LegendPosition,
    pub font: Font,
    /// Draw a box around the legend.
    pub frame: bool,
    pub background: Option<Color>,
    #[serde(default, skip_serializing_if = "LegendScope::is_layer")]
    pub scope: LegendScope,
    /// The number of columns the entries are arranged in, filled column by column; 1 is a single
    /// list.
    #[serde(default = "one_column", skip_serializing_if = "is_one_column")]
    pub columns: u32,
    /// The legend text in Origin's notation, one entry per line: `\l(n)` draws the symbol of
    /// plot n, `%(n)` is its name, any other text is shown as it is (with the label markup). `n`
    /// counts the plots the legend lists (`l.p`, layer and plot, when it lists several layers).
    /// `None` is the automatic text, `\l(1) %(1)` and so on for every plot, which follows the
    /// plots as they come and go.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

fn one_column() -> u32 {
    1
}

fn is_one_column(c: &u32) -> bool {
    *c == 1
}

impl Legend {
    /// Upper-left fractions of the layer frame, measured in Origin 2016. The rectangle grows
    /// right/down as its text, rows or columns change; it is not a centre or right-edge anchor.
    pub const AUTOMATIC_ANCHOR: (f64, f64) = (0.8223, -0.1044);

    /// The automatic legend text for plots with these ids (see [`Legend::text`]): a symbol and a
    /// name per line.
    pub fn automatic_text<S: AsRef<str>>(ids: &[S]) -> String {
        ids.iter().map(|id| format!("\\l({0}) %({0})", id.as_ref())).collect::<Vec<_>>().join("\n")
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TextLabel {
    pub text: String,
    /// Anchor point as fractions of the layer frame (0,0 = top-left).
    pub x: f64,
    pub y: f64,
    pub font: Font,
    /// Counter-clockwise rotation in degrees.
    #[serde(default)]
    pub rotation_deg: f64,
}

impl TextLabel {
    /// A label with no rotation.
    pub fn new(text: impl Into<String>, x: f64, y: f64, font: Font) -> Self {
        TextLabel { text: text.into(), x, y, font, rotation_deg: 0.0 }
    }
}

impl Layer {
    /// The font a new free text label of this layer starts with: the layer's legend font (the
    /// text size the figure already uses), black and upright.
    pub fn new_label_font(&self) -> Font {
        Font { bold: false, italic: false, color: Color::BLACK, ..self.legend.font.clone() }
    }
}

/// The colors a layer hands out to its plots.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Palette {
    pub colors: Vec<Color>,
    pub mode: PaletteMode,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaletteMode {
    /// Plot `i` takes color `i`, wrapping around at the end of the list.
    Cycle,
    /// Colors run as a gradient through the list: the first plot takes the
    /// first color and the last plot the last color, whatever the number of
    /// plots. Good for a sequence of scans that should read as one family.
    Spread,
    /// Every plot takes the first color (Origin's Line Color increment "None").
    Same,
}

/// How a property of the plots of a group changes from one member to the next (the Increment
/// column of Origin's Group tab).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Increment {
    /// Every member keeps the same value.
    #[default]
    None,
    /// Member k takes the k-th next entry of the list, from the first member's value.
    ByOne,
}

/// The Group tab of a layer whose plots form one group (Origin's defaults for a group made by
/// one plot command: Dependent, Line Color by one (the palette), the rest None; graphing spec 18).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupStyle {
    /// Dependent: a change to one member's line, symbol or decorations is made to every member,
    /// except for the incremented properties. Independent: each plot is edited alone.
    pub dependent: bool,
    #[serde(default)]
    pub line_style: Increment,
    #[serde(default)]
    pub symbol_type: Increment,
    #[serde(default)]
    pub symbol_interior: Increment,
    /// Origin's Subgrouping (By Size here; the Group tab's "By Column Label" is not offered).
    #[serde(default)]
    pub subgrouping: Subgrouping,
    /// The property that steps from one subgroup to the next while the others step inside it.
    #[serde(default)]
    pub subgroup_on: SubgroupOn,
}

impl Default for GroupStyle {
    fn default() -> Self {
        GroupStyle {
            dependent: true,
            line_style: Increment::None,
            symbol_type: Increment::None,
            symbol_interior: Increment::None,
            subgrouping: Subgrouping::None,
            subgroup_on: SubgroupOn::Color,
        }
    }
}

impl GroupStyle {
    pub fn is_default(&self) -> bool {
        *self == GroupStyle::default()
    }
}

impl Palette {
    /// The color of plot number `index` (0-based) out of `count` plots.
    pub fn color(&self, index: usize, count: usize) -> Color {
        if self.colors.is_empty() {
            return Color::BLACK;
        }
        match self.mode {
            PaletteMode::Cycle => self.colors[index % self.colors.len()],
            PaletteMode::Same => self.colors[0],
            PaletteMode::Spread => {
                let k = self.colors.len();
                if k == 1 || count <= 1 {
                    return self.colors[0];
                }
                // Position along the list of colors, 0..k-1.
                let pos = index.min(count - 1) as f64 * (k - 1) as f64 / (count - 1) as f64;
                let lo = pos.floor() as usize;
                let hi = (lo + 1).min(k - 1);
                self.colors[lo].lerp(&self.colors[hi], pos - lo as f64)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slope_annotations_round_trip_and_old_layers_load_without_them() {
        let mut s = SlopeAnnotation::new(ColumnId(7), 1, 4);
        s.show_legs = false;
        s.color = Some(Color::rgb(1, 2, 3));
        s.width_pt = 2.5;
        s.label_offset_pt = (3.0, -4.0);
        let back: SlopeAnnotation = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(back, s);
        // Only the two rows and the column are required; the rest has defaults.
        let min: SlopeAnnotation = serde_json::from_str(r#"{"plot":3,"row_a":0,"row_b":2}"#).unwrap();
        assert_eq!(min, SlopeAnnotation::new(ColumnId(3), 0, 2));
        assert!(min.show_legs && min.width_pt == 1.0 && min.color.is_none());
        // A layer without slopes writes no key, and a layer written without the key loads.
        let mut p = crate::Project::new();
        let g = p.add_graph(crate::template::Template::Scatter);
        let mut layer = p.graph(g).unwrap().layers[0].clone();
        let json = serde_json::to_string(&layer).unwrap();
        assert!(!json.contains("slopes"));
        assert!(serde_json::from_str::<Layer>(&json).unwrap().slopes.is_empty());
        layer.slopes.push(s);
        let back: Layer = serde_json::from_str(&serde_json::to_string(&layer).unwrap()).unwrap();
        assert_eq!(back.slopes, layer.slopes);
    }

    fn pal(mode: PaletteMode) -> Palette {
        Palette { colors: vec![Color::rgb(0, 0, 0), Color::rgb(100, 100, 100), Color::rgb(200, 200, 200)], mode }
    }

    #[test]
    fn cycle_palette_wraps() {
        let p = pal(PaletteMode::Cycle);
        assert_eq!(p.color(0, 5), Color::rgb(0, 0, 0));
        assert_eq!(p.color(3, 5), Color::rgb(0, 0, 0));
        assert_eq!(p.color(4, 5), Color::rgb(100, 100, 100));
    }

    #[test]
    fn spread_palette_uses_the_whole_range() {
        let p = pal(PaletteMode::Spread);
        // Two plots: first and last color, not the first two.
        assert_eq!(p.color(0, 2), Color::rgb(0, 0, 0));
        assert_eq!(p.color(1, 2), Color::rgb(200, 200, 200));
        // Three plots: exactly the list.
        assert_eq!(p.color(1, 3), Color::rgb(100, 100, 100));
        // Five plots: interpolated in between.
        assert_eq!(p.color(1, 5), Color::rgb(50, 50, 50));
        assert_eq!(p.color(4, 5), Color::rgb(200, 200, 200));
        assert_eq!(p.color(0, 1), Color::rgb(0, 0, 0));
    }

    /// A layer as files written before linked layers existed store it: no `link`, `side`,
    /// `color_follows_plot`, `title_color_follows_plot` or `scope` keys.
    const OLD_LAYER: &str = r##"{
        "frame": {"left": 0.16, "top": 0.07, "width": 0.78, "height": 0.75},
        "x_axis": {"scale": "linear", "min": null, "max": 2.0, "auto_range": "nice", "major_step": null,
            "minor_ticks": 1, "title": null, "show_title": true,
            "title_font": {"family": "Arial", "size_pt": 14.0, "color": "#000000"},
            "show_tick_labels": true,
            "tick_font": {"family": "Arial", "size_pt": 12.0, "color": "#000000"},
            "tick_format": {"kind": "auto"}, "tick_label_divisor": 1.0, "major_tick": "out",
            "minor_tick": "out", "major_tick_len_pt": 5.0, "minor_tick_len_pt": 2.5, "line_width_pt": 1.0,
            "color": "#000000", "show_opposite": false, "opposite_ticks": false,
            "major_grid": null, "minor_grid": null},
        "y_axis": {"scale": "linear", "min": null, "max": null, "auto_range": "symmetric_zero",
            "major_step": null, "minor_ticks": 0, "title": "j", "show_title": true,
            "title_font": {"family": "Arial", "size_pt": 14.0, "color": "#000000"},
            "show_tick_labels": true,
            "tick_font": {"family": "Arial", "size_pt": 12.0, "color": "#000000"},
            "tick_label_divisor": 1.0, "major_tick": "in", "minor_tick": "none", "major_tick_len_pt": 5.0,
            "minor_tick_len_pt": 2.5, "line_width_pt": 1.0, "color": "#000000", "show_opposite": true,
            "opposite_ticks": false, "major_grid": null, "minor_grid": null},
        "plots": [],
        "legend": {"visible": true, "position": {"kind": "top_right"},
            "font": {"family": "Arial", "size_pt": 11.0, "color": "#000000"},
            "frame": false, "background": null},
        "palette": {"colors": ["#000000"], "mode": "cycle"},
        "default_line": {"color": "#000000", "width_pt": 1.5},
        "default_symbol": {"shape": "circle", "size_pt": 5.0, "color": "#000000", "filled": true,
            "edge_width_pt": 0.75},
        "default_kind": "line"
    }"##;

    #[test]
    fn a_layer_saved_before_linked_layers_loads_with_the_old_meaning() {
        let layer: Layer = serde_json::from_str(OLD_LAYER).expect("an old layer loads");
        assert_eq!(layer.link, None);
        for axis in [&layer.x_axis, &layer.y_axis] {
            assert_eq!(axis.side, AxisSide::Normal);
            assert!(!axis.color_follows_plot && !axis.title_color_follows_plot);
        }
        assert_eq!(layer.legend.scope, LegendScope::Layer);
        assert_eq!((layer.legend.columns, layer.legend.text.as_deref()), (1, None));
        assert_eq!(layer.x_axis.max, Some(2.0));
        // Saving it again writes none of the new keys, so the file looks as before.
        let json = serde_json::to_string(&layer).unwrap();
        for key in ["\"link\"", "\"side\"", "color_follows_plot", "\"scope\"", "\"columns\"", "\"text\""] {
            assert!(!json.contains(key), "{key} in {json}");
        }
        let back: Layer = serde_json::from_str(&json).unwrap();
        assert_eq!(back, layer);
    }

    #[test]
    fn new_fields_are_written_when_set_and_read_back() {
        let mut layer: Layer = serde_json::from_str(OLD_LAYER).unwrap();
        layer.link = Some(LayerLink { parent: 0, x: ScaleLink::Straight, y: ScaleLink::None });
        layer.y_axis.side = AxisSide::Opposite;
        layer.x_axis.side = AxisSide::Hidden;
        layer.y_axis.color_follows_plot = true;
        layer.y_axis.title_color_follows_plot = true;
        layer.legend.scope = LegendScope::Graph;
        layer.legend.columns = 2;
        layer.legend.text = Some("\\l(1) a\n%(2)".into());
        let json = serde_json::to_string(&layer).unwrap();
        assert!(json.contains(r#""link":{"parent":0,"x":"straight","y":"none"}"#), "{json}");
        assert!(json.contains(r#""side":"opposite""#) && json.contains(r#""side":"hidden""#), "{json}");
        assert!(json.contains(r#""scope":"graph""#), "{json}");
        let back: Layer = serde_json::from_str(&json).unwrap();
        assert_eq!(back, layer);
    }

    #[test]
    fn a_layer_saved_before_stacking_has_no_offsets_and_is_written_as_before() {
        let layer: Layer = serde_json::from_str(OLD_LAYER).expect("an old layer loads");
        assert_eq!(layer.stack, Stack::None);
        assert!(layer.plot_offsets.is_empty());
        let json = serde_json::to_string(&layer).unwrap();
        assert!(!json.contains("\"stack\"") && !json.contains("plot_offsets"), "{json}");
    }

    #[test]
    fn waterfall_colour_mapping_is_saved_with_the_layer_and_defaults_to_none() {
        let mut layer: Layer = serde_json::from_str(OLD_LAYER).unwrap();
        assert_eq!(layer.waterfall_color, WaterfallColor::None);
        assert!(!serde_json::to_string(&layer).unwrap().contains("waterfall_color"));
        layer.waterfall_color = WaterfallColor::Z;
        let json = serde_json::to_string(&layer).unwrap();
        assert!(json.contains(r#""waterfall_color":"z""#), "{json}");
        assert_eq!(serde_json::from_str::<Layer>(&json).unwrap().waterfall_color, WaterfallColor::Z);
    }

    #[test]
    fn stack_modes_are_written_with_a_kind_and_read_back() {
        let cases = [
            (Stack::None, r#"{"kind":"none"}"#),
            (Stack::Auto { gap_percent: 10.0 }, r#"{"kind":"auto","gap_percent":10.0}"#),
            (Stack::Constant { step: -500.0 }, r#"{"kind":"constant","step":-500.0}"#),
            (Stack::Individual, r#"{"kind":"individual"}"#),
            (Stack::Cumulative, r#"{"kind":"cumulative"}"#),
            (
                Stack::Waterfall { dx_percent: 5.0, dy_percent: 10.0 },
                r#"{"kind":"waterfall","dx_percent":5.0,"dy_percent":10.0}"#,
            ),
        ];
        for (stack, text) in cases {
            assert_eq!(serde_json::to_string(&stack).unwrap(), text);
            assert_eq!(serde_json::from_str::<Stack>(text).unwrap(), stack);
        }
        let mut layer: Layer = serde_json::from_str(OLD_LAYER).unwrap();
        layer.stack = Stack::Auto { gap_percent: 10.0 };
        layer.plot_offset_mut(ColumnId(7)).y = 100.0;
        let json = serde_json::to_string(&layer).unwrap();
        assert!(json.contains(r#""stack":{"kind":"auto","gap_percent":10.0}"#), "{json}");
        assert!(json.contains(r#""plot_offsets":[{"column":7,"y":100.0}]"#), "{json}");
        assert_eq!(serde_json::from_str::<Layer>(&json).unwrap(), layer);
    }

    #[test]
    fn plot_offsets_are_found_by_column_and_added_once() {
        let mut layer: Layer = serde_json::from_str(OLD_LAYER).unwrap();
        let entry: PlotOffset = serde_json::from_str(r#"{"column":3}"#).unwrap();
        assert_eq!(entry, PlotOffset::new(ColumnId(3)));
        assert_eq!(layer.plot_offset(ColumnId(3)), None);
        layer.plot_offset_mut(ColumnId(3)).x = 1.5;
        layer.plot_offset_mut(ColumnId(3)).follows = Some(ColumnId(2));
        assert_eq!(layer.plot_offsets.len(), 1);
        assert_eq!(layer.plot_offset(ColumnId(3)).map(|o| o.x), Some(1.5));
        assert_eq!(layer.follows(ColumnId(3)), Some(ColumnId(2)));
        // A curve cannot follow itself.
        layer.plot_offset_mut(ColumnId(4)).follows = Some(ColumnId(4));
        assert_eq!(layer.follows(ColumnId(4)), None);
        assert_eq!(layer.follows(ColumnId(9)), None);
    }

    #[test]
    fn a_layer_saved_before_text_tick_labels_and_bars_loads_with_the_defaults() {
        let layer: Layer = serde_json::from_str(OLD_LAYER).expect("an old layer loads");
        for axis in [&layer.x_axis, &layer.y_axis] {
            assert_eq!(axis.tick_label_source, TickLabelSource::Auto);
            assert_eq!(axis.tick_label_angle_deg, 0.0);
        }
        assert_eq!(layer.bars, BarSpacing { gap_pct: 20.0, overlap_pct: 0.0, width_pct: None });
        // Saved again, none of the new keys is written, and it reads back equal.
        let json = serde_json::to_string(&layer).unwrap();
        for key in ["tick_label_source", "tick_label_angle_deg", "\"bars\""] {
            assert!(!json.contains(key), "{key} in {json}");
        }
        assert_eq!(serde_json::from_str::<Layer>(&json).unwrap(), layer);
    }

    #[test]
    fn tick_label_sources_and_bar_spacing_are_written_when_set_and_read_back() {
        let mut layer: Layer = serde_json::from_str(OLD_LAYER).unwrap();
        layer.x_axis.tick_label_source = TickLabelSource::Column { column: ColumnId(5) };
        layer.x_axis.tick_label_angle_deg = 45.0;
        layer.y_axis.tick_label_source = TickLabelSource::Numbers;
        layer.bars.gap_pct = 50.0;
        let json = serde_json::to_string(&layer).unwrap();
        assert!(json.contains(r#""tick_label_source":{"kind":"column","column":5}"#), "{json}");
        assert!(json.contains(r#""tick_label_source":{"kind":"numbers"}"#), "{json}");
        assert!(json.contains(r#""tick_label_angle_deg":45.0"#), "{json}");
        assert!(json.contains(r#""bars":{"gap_pct":50.0,"overlap_pct":0.0}"#), "{json}");
        assert_eq!(serde_json::from_str::<Layer>(&json).unwrap(), layer);
        // A bar entry with only one of its numbers takes the default for the other.
        let bars: BarSpacing = serde_json::from_str(r#"{"overlap_pct":-50.0}"#).unwrap();
        assert_eq!(bars, BarSpacing { gap_pct: 20.0, overlap_pct: -50.0, width_pct: None });
        assert_eq!(serde_json::from_str::<TickLabelSource>(r#"{"kind":"auto"}"#).unwrap(), TickLabelSource::Auto);
    }

    #[test]
    fn a_layer_saved_before_panels_shows_every_end_label_and_is_written_as_before() {
        let layer: Layer = serde_json::from_str(OLD_LAYER).expect("an old layer loads");
        assert!(!layer.x_axis.hide_end_label && !layer.y_axis.hide_end_label);
        let json = serde_json::to_string(&layer).unwrap();
        assert!(!json.contains("hide_end_label"), "{json}");
        let mut layer = layer;
        layer.y_axis.hide_end_label = true;
        let json = serde_json::to_string(&layer).unwrap();
        assert_eq!(json.matches(r#""hide_end_label":true"#).count(), 1, "{json}");
        assert_eq!(serde_json::from_str::<Layer>(&json).unwrap(), layer);
    }

    #[test]
    fn layers_drawn_in_one_frame_form_one_panel() {
        // Layer 1 covers layer 0 (a right Y axis), layer 2 sits above it (a second panel), layer 3
        // covers layer 2, and layer 4 is a graph of its own.
        let mut g = graph_of(&[None, Some(0), Some(0), Some(0), None]);
        g.layers[1].frame = Frame::WHOLE;
        g.layers[2].frame = Frame { left: 0.0, top: -1.0, width: 1.0, height: 1.0 };
        g.layers[3].frame = g.layers[2].frame;
        g.layers[3].y_axis.side = AxisSide::Opposite;
        g.layers[1].y_axis.side = AxisSide::Opposite;
        assert_eq!(g.place(0), Frame::WHOLE);
        assert_eq!(g.place(2), g.layers[2].frame);
        assert_eq!(g.place(4), Frame::WHOLE, "a root's place is its whole frame");
        assert_eq!(g.group(0), vec![0, 1, 2, 3]);
        assert_eq!(g.same_frame(0), vec![0, 1]);
        assert_eq!(g.same_frame(3), vec![2, 3]);
        assert_eq!(g.same_frame(4), vec![4]);
        assert_eq!(g.same_frame(9), Vec::<usize>::new());
        assert_eq!(g.panels(), vec![0, 2, 4]);
        assert_eq!(g.right_y_member(0), Some(1));
        assert_eq!(g.right_y_member(2), Some(3));
        assert_eq!(g.right_y_member(4), None);
    }

    #[test]
    fn an_automatic_color_fills_a_column_plot_and_leaves_its_outline() {
        let mut layer: Layer = serde_json::from_str(OLD_LAYER).unwrap();
        layer.palette = Palette { colors: vec![Color::BLACK, Color::RED], mode: PaletteMode::Cycle };
        let plot = |kind: PlotKind| Plot {
            x: None,
            y: ColumnId(1),
            y_error: None,
            rows: None,
            kind,
            line: LineStyle::solid(Color::BLACK, 0.5),
            symbol: layer.default_symbol.clone(),
            label: None,
            visible: true,
            auto_color: true,
            group: None,
            x_error: None,
            y_base: None,
            error: ErrorStyle::default(),
            hist: Default::default(),
            ridge: Default::default(),
            hexbin: Default::default(),
            parallel: Default::default(),
            treemap: Default::default(),
            indexed: None,
            sunburst: Default::default(),
            size_from: None,
            color_from: None,
            decor: Default::default(),
        };
        layer.plots = vec![plot(PlotKind::Column), plot(PlotKind::Column), plot(PlotKind::Line)];
        layer.recolor_automatic();
        assert_eq!(layer.plots[1].symbol.color, Color::RED);
        assert_eq!(layer.plots[1].line.color, Color::BLACK, "the outline keeps its color");
        // Other kinds take the color for line and symbol, as before.
        assert_eq!((layer.plots[2].line.color, layer.plots[2].symbol.color), (Color::BLACK, Color::BLACK));
        layer.plots.swap(0, 2);
        layer.recolor_automatic();
        assert_eq!((layer.plots[2].line.color, layer.plots[2].symbol.color), (Color::BLACK, Color::BLACK));
        assert_eq!((layer.plots[1].line.color, layer.plots[1].symbol.color), (Color::BLACK, Color::RED));
        layer.plots[0].kind = PlotKind::Line;
        layer.plots[0].line.color = Color::BLUE;
        layer.recolor_automatic();
        assert_eq!(layer.plots[0].line.color, Color::BLACK);
    }

    #[test]
    fn group_increments_run_from_the_first_plot_and_dependent_edits_reach_every_plot() {
        let mut layer: Layer = serde_json::from_str(OLD_LAYER).unwrap();
        layer.palette = Palette { colors: vec![Color::BLACK, Color::RED, Color::BLUE], mode: PaletteMode::Cycle };
        let plot = Plot {
            x: None,
            y: ColumnId(1),
            y_error: None,
            rows: None,
            kind: PlotKind::LineSymbol,
            line: LineStyle::solid(Color::BLACK, 0.5),
            symbol: layer.default_symbol.clone(),
            label: None,
            visible: true,
            auto_color: true,
            group: None,
            x_error: None,
            y_base: None,
            error: ErrorStyle::default(),
            hist: Default::default(),
            ridge: Default::default(),
            hexbin: Default::default(),
            parallel: Default::default(),
            treemap: Default::default(),
            indexed: None,
            sunburst: Default::default(),
            size_from: None,
            color_from: None,
            decor: Default::default(),
        };
        layer.plots = vec![plot.clone(), plot.clone(), plot];
        assert!(layer.group_style.dependent, "Origin's default edit mode");
        layer.group_style.line_style = Increment::ByOne;
        layer.group_style.symbol_type = Increment::ByOne;
        layer.apply_group_increments();
        let dashes: Vec<Dash> = layer.plots.iter().map(|q| q.line.dash).collect();
        assert_eq!(dashes, Dash::ALL[..3]);
        // The shapes run on from the first plot's (a circle here).
        let shapes: Vec<SymbolShape> = layer.plots.iter().map(|q| q.symbol.shape).collect();
        assert_eq!(shapes, SymbolShape::ALL[1..4]);
        let colors: Vec<Color> = layer.plots.iter().map(|q| q.line.color).collect();
        assert_eq!(colors, [Color::BLACK, Color::RED, Color::BLUE]);
        // Dependent: plot 2's new width and drop line go to all; the increments stay.
        layer.plots[1].line.width_pt = 3.0;
        layer.plots[1].decor.drop.vertical.on = true;
        layer.follow_leader(1);
        assert!(layer.plots.iter().all(|q| q.line.width_pt == 3.0 && q.decor.drop.vertical.on));
        assert_eq!(layer.plots.iter().map(|q| q.line.dash).collect::<Vec<_>>(), Dash::ALL[..3]);
        // Colour increment None: every plot takes the first colour.
        layer.palette.mode = PaletteMode::Same;
        layer.apply_group_increments();
        assert!(layer.plots.iter().all(|q| q.line.color == Color::BLACK));
    }

    #[test]
    fn a_label_column_is_a_referenced_column() {
        let mut g = graph_of(&[None]);
        g.layers[0].x_axis.tick_label_source = TickLabelSource::Column { column: ColumnId(42) };
        assert_eq!(g.referenced_columns(), vec![ColumnId(42)]);
    }

    fn graph_of(links: &[Option<usize>]) -> Graph {
        let base: Layer = serde_json::from_str(OLD_LAYER).unwrap();
        let layers = links
            .iter()
            .map(|p| Layer {
                link: p.map(|parent| LayerLink { parent, x: ScaleLink::Straight, y: ScaleLink::None }),
                ..base.clone()
            })
            .collect();
        Graph { id: WindowId(1), name: "G".into(), long_name: String::new(), page: Page::inches(6.4, 4.8), layers }
    }

    #[test]
    fn set_link_keeps_the_layer_in_place_and_starts_with_unlinked_scales() {
        let mut g = graph_of(&[None, None, None]);
        g.layers[0].frame = Frame { left: 0.2, top: 0.1, width: 0.5, height: 0.8 };
        g.layers[1].frame = Frame { left: 0.3, top: 0.3, width: 0.25, height: 0.4 };
        let on_page = g.layers[1].frame;
        assert!(g.set_link(1, Some(0)));
        assert_eq!(g.layers[1].link, Some(LayerLink { parent: 0, x: ScaleLink::None, y: ScaleLink::None }));
        let f = g.layers[1].frame;
        assert!((f.left - 0.2).abs() < 1e-12 && (f.width - 0.5).abs() < 1e-12, "{f:?}");
        assert!((f.top - 0.25).abs() < 1e-12 && (f.height - 0.5).abs() < 1e-12, "{f:?}");
        g.layers[1].link.as_mut().unwrap().x = ScaleLink::Straight;
        // Unlinking a layer that is not linked changes nothing; layer 1 cannot follow a later layer,
        // and nothing can follow layer 1 while it is linked itself.
        assert!(g.set_link(0, None) && g.layers[0].link.is_none());
        assert!(!g.set_link(1, Some(2)));
        assert!(!g.set_link(2, Some(1)), "layer 1 is linked itself");
        assert!(!g.set_link(0, Some(0)));
        // Unlinked again, it is where it was on the page.
        assert!(g.set_link(1, None));
        assert_eq!(g.layers[1].link, None);
        let f = g.layers[1].frame;
        for (a, b) in
            [(f.left, on_page.left), (f.top, on_page.top), (f.width, on_page.width), (f.height, on_page.height)]
        {
            assert!((a - b).abs() < 1e-12, "{f:?}");
        }
    }

    #[test]
    fn only_links_to_an_earlier_unlinked_layer_are_valid() {
        let g = graph_of(&[None, Some(0), Some(1), Some(3), Some(9), Some(0)]);
        assert_eq!(g.link_parent(0), None);
        assert_eq!(g.link_parent(1), Some(0));
        assert_eq!(g.link_parent(2), None, "a chain 2 -> 1 -> 0 is not followed");
        assert_eq!(g.link_parent(3), None, "a link to itself");
        assert_eq!(g.link_parent(4), None, "a link to a missing layer");
        assert_eq!(g.link_parent(5), Some(0));
        assert_eq!(g.group(0), vec![0, 1, 5]);
        assert_eq!(g.group(5), vec![0, 1, 5]);
        assert_eq!(g.group(2), vec![2]);
        assert_eq!(g.group(17), Vec::<usize>::new());
        assert_eq!(g.root_of(4), 4);
        assert_eq!(g.right_y_member(0), None, "no member has its Y axis on the right");
        let mut g = g;
        g.layers[5].y_axis.side = AxisSide::Opposite;
        g.layers[2].y_axis.side = AxisSide::Opposite;
        assert_eq!(g.right_y_member(1), Some(5));
        assert_eq!(g.right_y_member(2), None, "a layer that is not linked is not a right-Y member");
    }

    #[test]
    fn a_plot_group_is_stored_only_when_set() {
        let mut layer: Layer = serde_json::from_str(OLD_LAYER).unwrap();
        let mut plot = Plot {
            x: None,
            y: ColumnId(1),
            y_error: None,
            rows: None,
            kind: PlotKind::Line,
            line: LineStyle::solid(Color::BLACK, 0.5),
            symbol: layer.default_symbol.clone(),
            label: None,
            visible: true,
            auto_color: true,
            group: None,
            x_error: None,
            y_base: None,
            error: ErrorStyle::default(),
            hist: Default::default(),
            ridge: Default::default(),
            hexbin: Default::default(),
            parallel: Default::default(),
            treemap: Default::default(),
            indexed: None,
            sunburst: Default::default(),
            size_from: None,
            color_from: None,
            decor: Default::default(),
        };
        assert!(!serde_json::to_string(&plot).unwrap().contains("group"));
        plot.group = Some(2);
        let json = serde_json::to_string(&plot).unwrap();
        assert!(json.contains(r#""group":2"#), "{json}");
        let back: Plot = serde_json::from_str(&json).unwrap();
        assert_eq!(back.group, Some(2));
        layer.plots.push(back);
    }
}

#[cfg(test)]
mod polar_tests {
    use super::*;
    use crate::template::Template;

    #[test]
    fn a_layer_without_coords_is_rectangular_and_polar_settings_round_trip() {
        let g = Template::Line.new_graph(WindowId(1), "G");
        let json = serde_json::to_string(&g.layers[0]).unwrap();
        assert!(!json.contains("coords"), "rectangular layers are written as before");
        let back: Layer = serde_json::from_str(&json).unwrap();
        assert_eq!(back.coords, Coords::Cartesian);
        let mut polar = Template::PolarRTheta.new_graph(WindowId(1), "P").layers.remove(0);
        if let Coords::Polar(c) = &mut polar.coords {
            c.zero_deg = 90.0;
            c.clockwise = true;
        }
        let back: Layer = serde_json::from_str(&serde_json::to_string(&polar).unwrap()).unwrap();
        assert_eq!(back, polar);
        let c = back.coords.polar().unwrap();
        assert_eq!((c.radius, c.zero_deg, c.clockwise), (RadiusColumn::X, 90.0, true));
    }

    #[test]
    fn the_radar_template_is_a_polar_layer_with_the_radar_flag_and_round_trips() {
        assert_eq!(Template::from_name("radar"), Some(Template::Radar));
        let l = Template::Radar.new_graph(WindowId(1), "R").layers.remove(0);
        assert!(l.coords.polar().is_some_and(|c| c.radar));
        assert_eq!(l.default_kind, crate::graph::PlotKind::LineSymbol);
        let back: Layer = serde_json::from_str(&serde_json::to_string(&l).unwrap()).unwrap();
        assert_eq!(back, l);
        let plain = serde_json::to_string(&Template::Polar.new_graph(WindowId(1), "P").layers[0]).unwrap();
        assert!(!plain.contains("radar"), "polar layers are written as before");
    }

    #[test]
    fn the_polar_templates_have_names_for_the_command_line() {
        assert_eq!(Template::from_name("polar"), Some(Template::Polar));
        assert_eq!(Template::from_name("polar-r-theta"), Some(Template::PolarRTheta));
        let l = &Template::Polar.new_graph(WindowId(1), "P").layers[0];
        assert_eq!(l.coords.polar().map(|c| c.radius), Some(RadiusColumn::Y));
        assert!(l.x_axis.major_grid.is_some() && l.y_axis.major_grid.is_some(), "rings and spokes");
    }
}

#[cfg(test)]
mod xy_ratio_tests {
    use super::*;
    use crate::template::Template;

    /// Origin 2016 (graphing 18.7a): page 10.72 x 8.205 in, layer 17.91/11.58/68.16/71.83 % of the page,
    /// X -2.5..12.5 and Y -2..22. Ticking the link (ratio 1) left the width at 34.36 %; then ratio 2 left the
    /// height at 35.91 %, or instead ratio 0.5 left the width at 17.18 %. Left and top did not move.
    #[test]
    fn ticking_the_link_shrinks_the_frame_as_origin_did() {
        let page = (10.72 * 72.0, 8.205 * 72.0);
        let mut layer = Template::Line.new_graph(WindowId(1), "G").layers[0].clone();
        assert_eq!(layer.xy_ratio, None, "off by default");
        layer.frame = Frame { left: 0.1791, top: 0.1158, width: 0.6816, height: 0.7183 };
        let pct = |v: f64| (v * 10000.0).round() / 100.0;
        layer.fit_xy_ratio(1.0, page, 15.0, 24.0);
        assert_eq!((pct(layer.frame.width), pct(layer.frame.height)), (34.36, 71.83));
        assert_eq!(layer.xy_ratio, Some(1.0));
        let linked = layer.clone();
        layer.fit_xy_ratio(2.0, page, 15.0, 24.0);
        // Origin showed 35.91 (it works from its own rounded width); Sheaf gets 35.915.
        assert_eq!(pct(layer.frame.width), 34.36);
        assert!((pct(layer.frame.height) - 35.91).abs() <= 0.011, "{:?}", layer.frame);
        let mut layer = linked;
        layer.fit_xy_ratio(0.5, page, 15.0, 24.0);
        assert_eq!((pct(layer.frame.width), pct(layer.frame.height)), (17.18, 71.83));
        assert_eq!((layer.frame.left, layer.frame.top), (0.1791, 0.1158));
        // Unusable spans leave the frame alone.
        let before = layer.frame;
        layer.fit_xy_ratio(1.0, page, 0.0, 24.0);
        assert_eq!(layer.frame, before);
        assert_eq!(xy_ratio_height(100.0, 10.0, 20.0, 2.0), Some(100.0));
        assert_eq!(xy_ratio_height(100.0, f64::NAN, 20.0, 2.0), None);
        // The setting is saved only when on.
        let json = serde_json::to_string(&layer).unwrap();
        assert!(json.contains("\"xy_ratio\":1.0"));
        assert_eq!(serde_json::from_str::<Layer>(&json).unwrap().xy_ratio, Some(1.0));
    }
}

#[cfg(test)]
mod break_tests {
    use super::*;
    use crate::template::Template;

    #[test]
    fn an_old_single_break_reads_as_a_list_and_keeps_its_place() {
        let axis = Template::Line.new_graph(WindowId(1), "G").layers[0].y_axis.clone();
        let mut json = serde_json::to_value(&axis).unwrap();
        json["axis_break"] =
            serde_json::json!({"from": 30.0, "to": 70.0, "position_pct": 60.0, "gap_pct": 2.0, "step_after": null});
        let old: Axis = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(old.axis_breaks.len(), 1);
        let b = old.axis_breaks[0];
        assert_eq!((b.from, b.to, b.position_pct, b.gap_pct), (30.0, 70.0, 60.0, 2.0));
        assert!(!b.auto_position, "an old break stays where it was");
        json["axis_break"] = serde_json::Value::Null;
        assert!(serde_json::from_value::<Axis>(json).unwrap().axis_breaks.is_empty());
        // Several breaks are written as a list and read back unchanged; none are not written at all.
        let mut many = axis.clone();
        many.axis_breaks =
            vec![AxisBreak::new(1.0, 2.0), AxisBreak::next_after(&[AxisBreak::new(1.0, 2.0)], 0.0, 10.0)];
        let text = serde_json::to_string(&many).unwrap();
        assert!(text.contains("\"axis_breaks\":["));
        assert_eq!(serde_json::from_str::<Axis>(&text).unwrap(), many);
        assert!(!serde_json::to_string(&axis).unwrap().contains("axis_break"));
    }

    #[test]
    fn a_new_break_takes_the_middle_half_of_what_is_left() {
        // Origin 2016's rows on the axis -25..125, adding one break at a time (graphing spec 3.6).
        let mut breaks = Vec::new();
        for (from, to) in [(12.5, 87.5), (96.875, 115.625), (117.96875, 122.65625)] {
            let b = AxisBreak::next_after(&breaks, -25.0, 125.0);
            assert_eq!((b.from, b.to), (from, to));
            assert!(b.auto_position && b.gap_pct == BREAK_GAP_PCT);
            breaks.push(b);
        }
        // A reversed range gives the same rows; the gap of the last break is kept.
        breaks[2].gap_pct = 3.0;
        let b = AxisBreak::next_after(&breaks, 125.0, -25.0);
        assert_eq!((b.from, b.gap_pct), (122.65625 + (125.0 - 122.65625) * 0.25, 3.0));
    }
}

#[cfg(test)]
mod ternary_tests {
    use super::*;
    use crate::template::Template;

    #[test]
    fn ternary_settings_round_trip_and_old_layers_are_unchanged() {
        let g = Template::Ternary.new_graph(WindowId(1), "T");
        let layer = &g.layers[0];
        assert_eq!(layer.coords.ternary().map(|t| t.scale), Some(TernaryScale::Auto));
        let back: Layer = serde_json::from_str(&serde_json::to_string(layer).unwrap()).unwrap();
        assert_eq!(&back, layer);
        assert!(back.coords.polar().is_none());
        assert_eq!(Template::from_name("ternary"), Some(Template::Ternary));
    }
}

#[cfg(test)]
mod style_tests {
    use super::*;
    use crate::template::Template;

    fn plot(y: u64) -> Plot {
        Plot {
            x: None,
            y: ColumnId(y),
            y_error: None,
            rows: None,
            kind: PlotKind::LineSymbol,
            line: LineStyle::solid(Color::BLACK, 1.0),
            symbol: SymbolStyle {
                shape: SymbolShape::Square,
                size_pt: 5.0,
                color: Color::BLACK,
                filled: true,
                edge_width_pt: 0.5,
                every: 1,
            },
            label: None,
            visible: true,
            auto_color: true,
            group: None,
            x_error: None,
            y_base: None,
            error: ErrorStyle::default(),
            hist: Default::default(),
            ridge: Default::default(),
            hexbin: Default::default(),
            parallel: Default::default(),
            treemap: Default::default(),
            indexed: None,
            sunburst: Default::default(),
            size_from: None,
            color_from: None,
            decor: Default::default(),
        }
    }

    fn layer_of(n: u64) -> Layer {
        let mut layer = Template::Line.new_graph(WindowId(1), "G").layers.remove(0);
        layer.palette =
            Palette { colors: vec![Color::BLACK, Color::RED, Color::BLUE, Color::WHITE], mode: PaletteMode::Cycle };
        layer.plots = (1..=n).map(plot).collect();
        layer
    }

    #[test]
    fn a_graph_without_the_new_settings_is_written_as_before() {
        let g = Template::Line.new_graph(WindowId(1), "G");
        let json = serde_json::to_string(&g).unwrap();
        for key in ["width_pct", "background_style", "\"style\"", "subgrouping", "show_x_axes", "\"fill\""] {
            assert!(!json.contains(key), "{key} in {json}");
        }
        assert_eq!(serde_json::from_str::<Graph>(&json).unwrap(), g);
        // And a page written without a style loads with the defaults.
        let page: Page =
            serde_json::from_str(r##"{"width_pt": 450.0, "height_pt": 300.0, "background": "#FFFFFF"}"##).unwrap();
        assert_eq!(page.style, PageStyle::default());
        assert!(page.style.layer_icons);
    }

    #[test]
    fn the_new_settings_are_written_when_set_and_read_back() {
        let mut g = Template::Line.new_graph(WindowId(1), "G");
        let l = &mut g.layers[0];
        l.plots.push(plot(1));
        l.bars.width_pct = Some(70.0);
        l.plots[0].decor.fill.transparency_pct = 30;
        l.plots[0].decor.labels.form = LabelForm::Column { column: ColumnId(77) };
        l.group_style.subgrouping = Subgrouping::BySize { size: 3 };
        l.display.show_x_axes = false;
        l.background_style.border = Some(BorderStyle::solid(Color::BLACK, 1.0));
        g.page.style.comments = "notes".into();
        let json = serde_json::to_string(&g).unwrap();
        assert_eq!(serde_json::from_str::<Graph>(&json).unwrap(), g);
        assert!(json.contains(r#""width_pct":70.0"#) && json.contains(r#""transparency_pct":30"#), "{json}");
        assert_eq!(g.referenced_columns(), vec![ColumnId(1), ColumnId(77)], "a label column is read by the graph");
    }

    #[test]
    fn the_title_follows_the_display_choice() {
        let mut g = Template::Line.new_graph(WindowId(1), "Graph1");
        assert_eq!(g.title(), "Graph1");
        g.long_name = "Scan".into();
        assert_eq!(g.title(), "Graph1 - Scan");
        g.page.style.title_display = TitleDisplay::LongName;
        assert_eq!(g.title(), "Scan");
        g.page.style.title_display = TitleDisplay::Name;
        assert_eq!(g.title(), "Graph1");
    }

    #[test]
    fn subgroups_step_one_property_across_them_and_the_others_inside() {
        let mut l = layer_of(4);
        l.group_style.line_style = Increment::ByOne;
        l.group_style.subgrouping = Subgrouping::BySize { size: 2 };
        l.group_style.subgroup_on = SubgroupOn::LineStyle;
        l.apply_palette();
        l.apply_group_increments();
        // Colours restart in each subgroup; the line style steps from one subgroup to the next.
        let colors: Vec<Color> = l.plots.iter().map(|p| p.line.color).collect();
        assert_eq!(colors, [Color::BLACK, Color::RED, Color::BLACK, Color::RED]);
        let dashes: Vec<Dash> = l.plots.iter().map(|p| p.line.dash).collect();
        assert_eq!(dashes, [Dash::Solid, Dash::Solid, Dash::ALL[1], Dash::ALL[1]]);
        // Subgroups stepping on the colour: the colour changes from one subgroup to the next.
        l.group_style.subgroup_on = SubgroupOn::Color;
        l.apply_palette();
        l.apply_group_increments();
        let colors: Vec<Color> = l.plots.iter().map(|p| p.line.color).collect();
        assert_eq!(colors, [Color::BLACK, Color::BLACK, Color::RED, Color::RED]);
        let dashes: Vec<Dash> = l.plots.iter().map(|p| p.line.dash).collect();
        assert_eq!(dashes, [Dash::Solid, Dash::ALL[1], Dash::Solid, Dash::ALL[1]]);
    }

    #[test]
    fn resizing_the_page_can_keep_the_layers_the_same_size() {
        let mut g = Template::Line.new_graph(WindowId(1), "G");
        let (w, h) = (g.page.width_pt, g.page.height_pt);
        let f = g.layers[0].frame;
        let mut scaled = g.clone();
        scaled.resize_page(w * 2.0, h, false);
        assert_eq!(scaled.layers[0].frame, f, "fractions stay: the layer grows with the page");
        g.resize_page(w * 2.0, h, true);
        let kept = g.layers[0].frame;
        assert!((kept.width * g.page.width_pt - f.width * w).abs() < 1e-9, "the layer keeps its width in points");
        assert!((kept.left * g.page.width_pt - f.left * w).abs() < 1e-9);
        assert_eq!((kept.top, kept.height), (f.top, f.height));
    }

    #[test]
    fn a_layer_that_follows_another_is_left_alone_when_the_page_is_resized() {
        let mut g = Template::Line.new_graph(WindowId(1), "G");
        let l = g.layers[0].clone();
        g.layers.push(l);
        g.layers[1].frame = Frame { left: 0.1, top: 0.2, width: 0.5, height: 0.5 };
        assert!(g.set_link(1, Some(0)));
        let linked = g.layers[1].frame;
        let (w, h) = (g.page.width_pt, g.page.height_pt);
        let before = g.layers[0].frame;
        g.resize_page(w * 1.5, h * 2.0, true);
        assert_eq!(g.layers[1].frame, linked, "a fraction of its parent's frame, not of the page");
        // The parent kept its size in points, so the linked layer did too.
        let parent = g.layers[0].frame;
        assert!((parent.width * g.page.width_pt - before.width * w).abs() < 1e-9);
        assert!((parent.height * g.page.height_pt - before.height * h).abs() < 1e-9);
        // A page with no size to give is refused quietly: nothing is scaled.
        g.resize_page(0.0, 100.0, true);
        assert_eq!(g.layers[0].frame, parent);
    }

    #[test]
    fn data_labels_name_rows_by_a_list_of_numbers_and_ranges() {
        let l = DataLabels { only_rows: "1, 3, 5-7".into(), ..Default::default() };
        let hit: Vec<usize> = (0..10).filter(|&r| l.labels_row(r)).collect();
        assert_eq!(hit, vec![0, 2, 4, 5, 6]);
        // Nothing valid, or nothing: every row.
        for text in ["", "x", "0", "5-3"] {
            let l = DataLabels { only_rows: text.into(), ..Default::default() };
            assert!((0..4).all(|r| l.labels_row(r)), "{text:?}");
        }
    }

    #[test]
    fn drop_lines_of_older_files_load_as_axis_and_auto_and_are_written_back_the_same() {
        let old = r#"{"horizontal":{"on":false,"dash":"solid","width_pt":0.5},"vertical":{"on":true,"width_pt":1.0}}"#;
        let d: DropLines = serde_json::from_str(old).unwrap();
        for l in [&d.horizontal, &d.vertical] {
            assert_eq!((l.target, l.color_mode), (DropTarget::Axis, DropColorMode::Auto));
        }
        assert!(d.vertical.on);
        let json = serde_json::to_string(&d).unwrap();
        assert!(!json.contains("target") && !json.contains("color_mode"), "{json}");
        // The new settings round-trip.
        let mut d = d;
        d.vertical.target = DropTarget::NextDataPlot;
        d.vertical.color_mode = DropColorMode::PlusMinus;
        let back: DropLines = serde_json::from_str(&serde_json::to_string(&d).unwrap()).unwrap();
        assert_eq!(back, d);
    }
}

#[cfg(test)]
mod sort_x_tests {
    use super::*;

    #[test]
    fn sort_x_defaults_off_is_not_written_when_off_and_round_trips() {
        let mut line = LineStyle::solid(Color::BLACK, 1.0);
        // A file written before the flag existed has no such key.
        let old: LineStyle = serde_json::from_str(&serde_json::to_string(&line).unwrap()).unwrap();
        assert!(!old.sort_x);
        assert!(!serde_json::to_string(&line).unwrap().contains("sort_x"));
        line.sort_x = true;
        let text = serde_json::to_string(&line).unwrap();
        assert!(text.contains("\"sort_x\":true"));
        assert!(serde_json::from_str::<LineStyle>(&text).unwrap().sort_x);
    }
}

#[cfg(test)]
mod trendline_tests {
    use super::*;

    #[test]
    fn trendlines_are_skipped_when_empty_and_round_trip() {
        let mut decor = PlotDecor::default();
        assert!(!serde_json::to_string(&decor).unwrap().contains("trendlines"));
        assert!(decor.is_default());
        let mut t = Trendline::new(TrendKind::Polynomial { order: 3 });
        t.color = Some(Color::BLACK);
        t.label_offset_pt = (4.0, -2.0);
        decor.trendlines.push(t.clone());
        decor.trendlines.push(Trendline::new(TrendKind::MovingAverage { window: 5 }));
        assert!(!decor.is_default());
        let text = serde_json::to_string(&decor).unwrap();
        let back: PlotDecor = serde_json::from_str(&text).unwrap();
        assert_eq!(back.trendlines, decor.trendlines);
        // A minimal record reads with the defaults.
        let old: Trendline = serde_json::from_str(r#"{"kind":"linear"}"#).unwrap();
        assert_eq!(old, Trendline::new(TrendKind::Linear));
    }
}
