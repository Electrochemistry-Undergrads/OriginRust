//! Built-in figure templates: starting styles for new graphs.
//!
//! A template only sets style. Plots are added afterwards with
//! [`crate::project::Project::add_plot`].

use crate::color::Color;
use crate::graph::*;
use crate::ids::WindowId;
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Template {
    /// General line plot: axes on the bottom and left, ticks pointing out.
    #[default]
    Line,
    /// Plot > Line > Horizontal Step, Vertical Step and Spline Connected (T63 group B): the Line
    /// template with each segment drawn as a step (horizontal or vertical first) or a spline.
    StepHorizontal,
    StepVertical,
    SplineConnected,
    /// Plot > 3D Symbol/Bar/Vector > 3D Scatter and 3D Trajectory (T155): an XYZ layer
    /// ([`Coords::Space3d`]) of symbols, or of a line with symbols.
    Scatter3d,
    Trajectory3d,
    /// 3D Scatter + Error Bar and 3D Bars (T156): the Z error column after the Z column draws vertical
    /// error bars; the bars stand from the floor of the box to Z.
    Scatter3dError,
    Bars3d,
    /// 3D Vector XYZ XYZ and 3D Vector XYZ dXdYdZ (T157): an arrow per row from (X, Y, Z).
    Vector3dXyz,
    Vector3dDelta,
    /// Plot > 3D XYY > XYY 3D Bars, 3D Walls and 3D Waterfall (T158): each Y column at its own depth.
    Xyy3dBars,
    Walls3d,
    Waterfall3d,
    /// Plot > 3D XYY > 3D Ribbons (T161): each Y column a ribbon at its own depth.
    Ribbons3d,
    /// Plot > 3D Surface > Color Map Surface, Color Fill Surface, Wire Frame and Wire Surface (T166):
    /// XYZ points joined into triangles over X and Y.
    ColorMapSurface3d,
    ColorFillSurface3d,
    WireFrame3d,
    WireSurface3d,
    /// Plot > 3D Surface > Colormap Surface with Projection (T168).
    ColorMapProjection3d,
    /// Plot > 3D Surface > X Constant with Base and Y Constant with Base (T170).
    XConstant3d,
    YConstant3d,
    /// Plot > 3D Surface > 3D Ternary Colormap Surface (T173).
    TernarySurface3d,
    /// Scatter plot with the same axes as [`Template::Line`].
    Scatter,
    /// Scatter plot whose axes cross in the middle of the layer (Origin's "Scatter Central", 2016): both
    /// axes at -50 %, the frame drawn as a closed box.
    ScatterCentral,
    /// Origin's Plot > Symbol > Column Scatter (2016): every Y column as filled diamonds (size 8) standing at
    /// 1, 2, 3, ... with a small jitter, the X axis named by the columns, the Y title "Range" (measured, T79).
    ColumnScatter,
    /// Plot > Specialized > Vector XYAM and Vector XYXY (T107).
    VectorXyam,
    VectorXyxy,
    /// Plot > Stock > High-Low-Close, Japanese Candlestick and OHLC Bar Chart (T108).
    HighLowClose,
    Candlestick,
    OhlcBar,
    /// Plot > Stock > OHLC-Volume (T109): candlesticks over a linked layer of volume columns.
    OhlcVolume,
    /// Plot > Stock > Line (T109): the line template with the theme colour list.
    StockLine,
    /// Plot > Statistics > Marginal Histograms and Marginal Box Charts (T110).
    MarginalHistograms,
    MarginalBoxCharts,
    /// Plot > Statistics > QC (X bar R) Chart (T111): the range chart and, linked above it, the mean chart.
    QcXbarR,
    /// Plot > Statistics > Scatter Matrix (T114): every pair of the selected columns in an n x n grid of layers.
    ScatterMatrix,
    /// Plot > Specialized > Zoom (T112): the whole curve with a region rectangle, and the region enlarged below.
    Zoom,
    /// Plot > Specialized > Wind Rose - Binned Data and Wind Rose - Raw Data (T113).
    WindRoseBinned,
    WindRoseRaw,
    /// Plot > Specialized > Smith Chart (T115).
    SmithChart,
    /// Plot > Statistics > 2D Kernel Density (T116): the density colour map with the points on top.
    Kde2d,
    /// Plot > Specialized > Piper (T117).
    Piper,
    /// Line plot with a symbol on every point.
    LineSymbol,
    /// Cyclic voltammetry overlay: smooth lines without markers, no grid,
    /// ticks pointing inwards, a closed black box around the plot, and a
    /// current axis that is symmetric about zero.
    CyclicVoltammetry,
    /// Two Y axes over one X axis: the line template with a second, linked layer whose Y axis is
    /// on the right (Origin's "Double-Y"). Selected columns alternate between the left and the
    /// right axis, and each Y axis is drawn in the color of its first curve.
    DoubleY,
    /// Origin's "3Ys Y-YY": three layers, a left axis and two right ones (graphing spec 21.5).
    Multi3YYy,
    /// Origin's "3Ys Y-Y-Y": a left axis, a left axis in the middle of the plot and a right axis.
    Multi3YYY,
    /// Origin's "4Ys Y-YYY": a left axis and three right ones.
    Multi4YYyy,
    /// Origin's "4Ys YY-YY": two left axes and two right ones.
    Multi4YyYy,
    /// Curves stacked above each other with a gap (Origin's "Stacked Lines by Y Offsets"): the
    /// line template with automatic offsets and a 10 % gap, no Y tick labels and no Y ticks, and
    /// a closed box, as in the owner's XRD figures.
    StackedLines,
    /// A simple two-dimensional waterfall: each later curve is drawn moved right and up.
    Waterfall,
    /// Column chart (Origin's "Column"): one bar per row from zero, several Y columns side by side,
    /// filled from the palette with a thin black outline, in a closed box with ticks pointing in.
    /// With a text X column (sample names) the names are written under the bars.
    Column,
    /// Area plot (Origin's "Area"): the line template with each curve filled down to Y = 0.
    Area,
    /// Stacked column (Origin's "Stacked Column"): the column template with the plots piled up.
    StackedColumn,
    /// 100 % stacked column: every pile scaled to 100, Y axis from 0 to 100.
    Stacked100Column,
    /// Horizontal bars (Origin's "Bar").
    Bar,
    /// Floating column (Origin's "Floating Column"): each bar runs from one Y column to the next.
    FloatingColumn,
    /// Stacked bar: the horizontal form of the stacked column.
    StackedBar,
    /// 100 % stacked bar: every pile scaled to 100, value axis from 0 to 100.
    Stacked100Bar,
    /// Floating bar: the horizontal form of the floating column.
    FloatingBar,
    /// Stacked area (Origin's "Stacked Area"): the area template with the areas piled up.
    StackedArea,
    /// 100 % stacked area: every pile scaled to 100, Y axis from 0 to 100.
    Stacked100Area,
    /// Fill area (Origin's "Fill Area"): the area between two curves, a pair of Y columns each.
    FillArea,
    /// Histogram (Origin's "Histogram"): the bin counts of each Y column as touching bars.
    Histogram,
    /// Plot > Statistics > Histogram + Probabilities (T85, measured in Origin 2016): the histogram in
    /// layer 1 and the cumulative percent of its bins as a step line in a linked right-Y layer of 0 - 100.
    HistogramProbabilities,
    /// Stacked Histograms (Origin's Plot > Statistics > Stacked Histograms): one histogram panel per Y column,
    /// stacked vertically from the top in selection order, all on one X scale and one set of bins (graphing 13.16).
    /// An empty graph of this template is one histogram layer; [`crate::Project::plot_columns`] builds the panels.
    StackedHistograms,
    /// Waterfall Y:Color Mapping: a waterfall whose curves are coloured along their points by Y, with a colour scale.
    WaterfallY,
    /// Waterfall Z:Color Mapping: a waterfall whose curves are coloured one by one by a Z value, with a colour scale.
    WaterfallZ,
    /// Box chart (Origin's "Box Chart"): one box per Y column.
    BoxChart,
    /// Bubble plot (Origin's "Bubble"): open circles whose size follows the Z column.
    Bubble,
    /// Colour mapped scatter plot: the colour of each symbol follows the Z column.
    ColorMapped,
    /// Both: bubbles whose size and colour follow the Z column.
    BubbleColorMapped,
    /// Pie chart (Origin's pie): the Y column's rows as slices, in a layer without axes.
    Pie,
    /// 3D pie seen from above at a slant (Origin's "3D Color Pie").
    Pie3D,
    /// Flat black and white pie with hatch patterns (Origin's "2D B&W Pie").
    PieBw,
    /// Polar plot, angle in degrees from the X column and radius from Y (Origin's "Polar theta(X) r(Y)").
    Polar,
    /// Polar plot with the radius from the X column and the angle from Y ("Polar r(X) theta(Y)").
    PolarRTheta,
    /// Radar chart: every Y column a closed polygon over spokes named by the X column (Origin's "Radar").
    Radar,
    /// Ternary plot of the X, Y and Z columns (Origin's "Ternary").
    Ternary,
    /// Lollipop plot (Origin's "Lollipop Plot", Origin 2021): the scatter template with X and Y
    /// exchanged and vertical drop lines that go from each plot to the next data plot. Needs two
    /// or more Y columns ([`Project::plot_columns`] refuses fewer).
    Lollipop,
    /// Origin's Plot > Symbol > Vertical Drop Line (template `DROPLINE`): the scatter template with a
    /// vertical drop line from every point down to the bottom axis, in the plot's colour, 0.5 pt (measured,
    /// graphing 18.12).
    VerticalDropLine,
    /// Origin's Plot > Line + Symbol > 2 Point Segment: line and symbols, points joined in pairs
    /// ([`crate::graph::Connect::Segment2`]; measured 2026-11-15).
    Segment2,
    /// Origin's Plot > Line + Symbol > 3 Point Segment: points joined in threes.
    Segment3,
    /// Origin's Plot > Column/Bar/Pie > Column + Label (template `ColumnLabel`): the column template with
    /// each column's value written above it in Arial 22 (measured, graphing 18.12).
    ColumnLabel,
    /// Scatter plot with a rug on the bottom and left axes (Origin's "Scatter + Rug", 2019b): one short
    /// line per point beside each axis, all plots overlapping in one strip (graphing spec 18.10).
    ScatterRug,
    /// Violin plot (Origin's "Violin Plot"): every Y column a kernel density silhouette, with the
    /// colours of the box chart template.
    Violin,
    /// Origin's "Violin with Box".
    ViolinBox,
    /// Origin's "Violin with Point".
    ViolinPoint,
    /// Origin's "Violin with Quartile".
    ViolinQuartile,
    /// Origin's "Violin with Stick".
    ViolinStick,
    /// Origin's "Half Violin".
    HalfViolin,
    /// Ridgeline chart (Origin's "Ridgeline Chart", 2022): every Y column a density curve on a
    /// baseline at its column index, with the colours of the box chart template.
    Ridgeline,
    /// Hexbin plot (Origin's "Hexbin", 2027): the X and Y columns binned into hexagons coloured by their
    /// count. The legend is hidden: the colour scale beside the layer names the colours.
    Hexbin,
    /// Parallel Plot (Origin's "Parallel Plot", 2019): one vertical axis per Y column, one line per row, coloured by
    /// a numeric column. No legend: the colour scale beside the layer names the colours.
    Parallel,
    /// Parallel Index (Origin's "Parallel Index"): like [`Template::Parallel`], with the lines coloured by the
    /// group of their row and a legend of the groups.
    ParallelIndex,
    /// Treemap (Origin's "Treemap", 2025b): nested rectangles, one level per category column, the areas
    /// proportional to a value column. No axes and no legend.
    Treemap,
    /// Sunburst plot (Origin's "Sunburst Plot", 2022): the hierarchy of the category columns as concentric rings,
    /// the angles proportional to a value column. No axes and no legend; a colour scale beside the layer.
    Sunburst,
    /// Sankey diagram (Origin 2020, T379): flows of a value column between the nodes of a source and a target column.
    Sankey,
    /// Alluvial diagram (Origin 2020, T381): category columns as stages, rows flowing through their cells.
    Alluvial,
    /// Network plot (Origin 2021b, T383): nodes and edges of a source and a target column.
    Network,
    /// Cause-and-effect (fishbone) diagram (Origin 2026, T375): the chosen columns are the categories, their
    /// cells the causes; drawn as line and text objects in one layer without axes.
    Fishbone,
    /// Origin's Plot > Column/Bar/Pie > Grouped Columns (indexed data) (X-Function plot_gindexed, T76): the
    /// column template; one data column whose rows are split by the values of one or two group columns.
    GroupedColumns,
    /// Origin's Plot > Statistics > Grouped Box Charts (X-Function plot_gboxindexed, T76): the box chart
    /// template; the rows of one data column are split by one or two group columns.
    GroupedBox,
    /// Origin's Plot > Statistics > Interval Plot (BoxInterval template, T83): one point per Y column at its
    /// mean, with an error bar (SD, SE or a confidence interval).
    IntervalPlot,
    /// Origin's Plot > Statistics > Pareto (plot_paretoraw, ParetoRaw template, T83): the counts of the distinct
    /// values of a column as columns in descending order, with the cumulative percent on a right axis of 0 to 100.
    Pareto,
}

impl Template {
    pub const ALL: [Template; 105] = [
        Template::Line,
        Template::Scatter3d,
        Template::Trajectory3d,
        Template::Scatter3dError,
        Template::Bars3d,
        Template::Vector3dXyz,
        Template::Vector3dDelta,
        Template::Xyy3dBars,
        Template::Walls3d,
        Template::Waterfall3d,
        Template::Ribbons3d,
        Template::ColorMapSurface3d,
        Template::ColorFillSurface3d,
        Template::WireFrame3d,
        Template::WireSurface3d,
        Template::ColorMapProjection3d,
        Template::XConstant3d,
        Template::YConstant3d,
        Template::TernarySurface3d,
        Template::StepHorizontal,
        Template::StepVertical,
        Template::SplineConnected,
        Template::Scatter,
        Template::LineSymbol,
        Template::Column,
        Template::StackedColumn,
        Template::Stacked100Column,
        Template::Bar,
        Template::FloatingColumn,
        Template::StackedBar,
        Template::Stacked100Bar,
        Template::FloatingBar,
        Template::Area,
        Template::StackedArea,
        Template::Stacked100Area,
        Template::FillArea,
        Template::Histogram,
        Template::HistogramProbabilities,
        Template::StackedHistograms,
        Template::WaterfallY,
        Template::WaterfallZ,
        Template::BoxChart,
        Template::GroupedBox,
        Template::IntervalPlot,
        Template::Pareto,
        Template::Violin,
        Template::ViolinBox,
        Template::ViolinPoint,
        Template::ViolinQuartile,
        Template::ViolinStick,
        Template::HalfViolin,
        Template::Ridgeline,
        Template::Hexbin,
        Template::Parallel,
        Template::ParallelIndex,
        Template::Treemap,
        Template::Sunburst,
        Template::Sankey,
        Template::Alluvial,
        Template::Network,
        Template::Fishbone,
        Template::Bubble,
        Template::ColorMapped,
        Template::BubbleColorMapped,
        Template::Pie,
        Template::Pie3D,
        Template::PieBw,
        Template::Polar,
        Template::PolarRTheta,
        Template::Radar,
        Template::Ternary,
        Template::CyclicVoltammetry,
        Template::DoubleY,
        Template::Multi3YYy,
        Template::Multi3YYY,
        Template::Multi4YYyy,
        Template::Multi4YyYy,
        Template::StackedLines,
        Template::Waterfall,
        Template::Lollipop,
        Template::VerticalDropLine,
        Template::ColumnLabel,
        Template::GroupedColumns,
        Template::Segment2,
        Template::Segment3,
        Template::ScatterRug,
        Template::ScatterCentral,
        Template::ColumnScatter,
        Template::VectorXyam,
        Template::VectorXyxy,
        Template::HighLowClose,
        Template::Candlestick,
        Template::OhlcBar,
        Template::OhlcVolume,
        Template::StockLine,
        Template::MarginalHistograms,
        Template::MarginalBoxCharts,
        Template::QcXbarR,
        Template::ScatterMatrix,
        Template::Zoom,
        Template::WindRoseBinned,
        Template::WindRoseRaw,
        Template::SmithChart,
        Template::Kde2d,
        Template::Piper,
    ];

    /// The name used on the command line and in menus.
    pub fn name(&self) -> &'static str {
        match self {
            Template::Line => "line",
            Template::Scatter3d => "3d-scatter",
            Template::Trajectory3d => "3d-trajectory",
            Template::Scatter3dError => "3d-scatter-error",
            Template::Bars3d => "3d-bars",
            Template::Vector3dXyz => "3d-vector-xyz-xyz",
            Template::Vector3dDelta => "3d-vector-xyz-dxdydz",
            Template::Xyy3dBars => "xyy-3d-bars",
            Template::Walls3d => "3d-walls",
            Template::Waterfall3d => "3d-waterfall",
            Template::Ribbons3d => "3d-ribbons",
            Template::ColorMapSurface3d => "3d-colormap-surface",
            Template::ColorFillSurface3d => "3d-color-fill-surface",
            Template::WireFrame3d => "3d-wire-frame",
            Template::WireSurface3d => "3d-wire-surface",
            Template::ColorMapProjection3d => "3d-colormap-surface-projection",
            Template::XConstant3d => "3d-x-constant-base",
            Template::YConstant3d => "3d-y-constant-base",
            Template::TernarySurface3d => "3d-ternary-colormap-surface",
            Template::StepHorizontal => "step-horizontal",
            Template::StepVertical => "step-vertical",
            Template::SplineConnected => "spline-connected",
            Template::Scatter => "scatter",
            Template::ScatterCentral => "scatter-central",
            Template::ColumnScatter => "column-scatter",
            Template::VectorXyam => "vector-xyam",
            Template::VectorXyxy => "vector-xyxy",
            Template::HighLowClose => "high-low-close",
            Template::Candlestick => "candlestick",
            Template::OhlcBar => "ohlc-bar",
            Template::OhlcVolume => "ohlc-volume",
            Template::StockLine => "stock-line",
            Template::MarginalHistograms => "marginal-histograms",
            Template::MarginalBoxCharts => "marginal-box-charts",
            Template::QcXbarR => "qc-xbar-r",
            Template::ScatterMatrix => "scatter-matrix",
            Template::Zoom => "zoom",
            Template::WindRoseBinned => "wind-rose-binned",
            Template::WindRoseRaw => "wind-rose-raw",
            Template::SmithChart => "smith-chart",
            Template::Kde2d => "kde-2d",
            Template::Piper => "piper",
            Template::LineSymbol => "line-symbol",
            Template::CyclicVoltammetry => "cv",
            Template::DoubleY => "double-y",
            Template::Multi3YYy => "3ys-y-yy",
            Template::Multi3YYY => "3ys-y-y-y",
            Template::Multi4YYyy => "4ys-y-yyy",
            Template::Multi4YyYy => "4ys-yy-yy",
            Template::StackedLines => "stacked",
            Template::Waterfall => "waterfall",
            Template::Column => "column",
            Template::Area => "area",
            Template::StackedColumn => "stacked-column",
            Template::Stacked100Column => "stacked-column-100",
            Template::Bar => "bar",
            Template::FloatingColumn => "floating-column",
            Template::StackedBar => "stacked-bar",
            Template::Stacked100Bar => "stacked-bar-100",
            Template::FloatingBar => "floating-bar",
            Template::StackedArea => "stacked-area",
            Template::Stacked100Area => "stacked-area-100",
            Template::FillArea => "fill-area",
            Template::Histogram => "histogram",
            Template::HistogramProbabilities => "histogram-probabilities",
            Template::StackedHistograms => "stacked-histograms",
            Template::WaterfallY => "waterfall-y",
            Template::WaterfallZ => "waterfall-z",
            Template::BoxChart => "box",
            Template::Bubble => "bubble",
            Template::ColorMapped => "color-mapped",
            Template::BubbleColorMapped => "bubble-color-mapped",
            Template::Pie => "pie",
            Template::Pie3D => "pie-3d",
            Template::PieBw => "pie-bw",
            Template::Polar => "polar",
            Template::PolarRTheta => "polar-r-theta",
            Template::Radar => "radar",
            Template::Ternary => "ternary",
            Template::Lollipop => "lollipop",
            Template::VerticalDropLine => "vertical-drop-line",
            Template::ColumnLabel => "column-label",
            Template::GroupedColumns => "grouped-columns",
            Template::GroupedBox => "grouped-box",
            Template::IntervalPlot => "interval-plot",
            Template::Pareto => "pareto",
            Template::Segment2 => "segment2",
            Template::Segment3 => "segment3",
            Template::ScatterRug => "scatter-rug",
            Template::Violin => "violin",
            Template::ViolinBox => "violin-box",
            Template::ViolinPoint => "violin-point",
            Template::ViolinQuartile => "violin-quartile",
            Template::ViolinStick => "violin-stick",
            Template::HalfViolin => "half-violin",
            Template::Ridgeline => "ridgeline",
            Template::Hexbin => "hexbin",
            Template::Parallel => "parallel",
            Template::ParallelIndex => "parallel-index",
            Template::Treemap => "treemap",
            Template::Sunburst => "sunburst",
            Template::Sankey => "sankey",
            Template::Alluvial => "alluvial",
            Template::Network => "network",
            Template::Fishbone => "fishbone",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Template::Line => "Line",
            Template::Scatter3d => "3D Scatter",
            Template::Trajectory3d => "3D Trajectory",
            Template::Scatter3dError => "3D Scatter + Error Bar",
            Template::Bars3d => "3D Bars",
            Template::Vector3dXyz => "3D Vector XYZ XYZ",
            Template::Vector3dDelta => "3D Vector XYZ dXdYdZ",
            Template::Xyy3dBars => "XYY 3D Bars",
            Template::Walls3d => "3D Walls",
            Template::Waterfall3d => "3D Waterfall",
            Template::Ribbons3d => "3D Ribbons",
            Template::ColorMapSurface3d => "Color Map Surface",
            Template::ColorFillSurface3d => "Color Fill Surface",
            Template::WireFrame3d => "Wire Frame",
            Template::WireSurface3d => "Wire Surface",
            Template::ColorMapProjection3d => "Colormap Surface with Projection",
            Template::XConstant3d => "X Constant with Base",
            Template::YConstant3d => "Y Constant with Base",
            Template::TernarySurface3d => "3D Ternary Colormap Surface",
            Template::StepHorizontal => "Horizontal Step",
            Template::StepVertical => "Vertical Step",
            Template::SplineConnected => "Spline Connected",
            Template::Scatter => "Scatter",
            Template::ScatterCentral => "Scatter Central",
            Template::ColumnScatter => "Column Scatter",
            Template::VectorXyam => "Vector XYAM",
            Template::VectorXyxy => "Vector XYXY",
            Template::HighLowClose => "High-Low-Close",
            Template::Candlestick => "Japanese Candlestick",
            Template::OhlcBar => "OHLC Bar Chart",
            Template::OhlcVolume => "OHLC-Volume",
            Template::StockLine => "Stock Line",
            Template::MarginalHistograms => "Marginal Histograms",
            Template::MarginalBoxCharts => "Marginal Box Charts",
            Template::QcXbarR => "QC (X bar R) Chart",
            Template::ScatterMatrix => "Scatter Matrix",
            Template::Zoom => "Zoom",
            Template::WindRoseBinned => "Wind Rose - Binned Data",
            Template::WindRoseRaw => "Wind Rose - Raw Data",
            Template::SmithChart => "Smith Chart",
            Template::Kde2d => "2D Kernel Density",
            Template::Piper => "Piper",
            Template::LineSymbol => "Line + Symbol",
            Template::CyclicVoltammetry => "Cyclic voltammetry overlay",
            Template::DoubleY => "Double Y",
            Template::Multi3YYy => "3Ys Y-YY",
            Template::Multi3YYY => "3Ys Y-Y-Y",
            Template::Multi4YYyy => "4Ys Y-YYY",
            Template::Multi4YyYy => "4Ys YY-YY",
            Template::StackedLines => "Stacked lines (Y offsets)",
            Template::Waterfall => "Waterfall",
            Template::Column => "Column",
            Template::Area => "Area",
            Template::StackedColumn => "Stacked Column",
            Template::Stacked100Column => "100% Stacked Column",
            Template::Bar => "Bar",
            Template::FloatingColumn => "Floating Column",
            Template::StackedBar => "Stacked Bar",
            Template::Stacked100Bar => "100% Stacked Bar",
            Template::FloatingBar => "Floating Bar",
            Template::StackedArea => "Stacked Area",
            Template::Stacked100Area => "100% Stacked Area",
            Template::FillArea => "Fill Area",
            Template::Histogram => "Histogram",
            Template::HistogramProbabilities => "Histogram + Probabilities",
            Template::StackedHistograms => "Stacked Histograms",
            Template::WaterfallY => "Waterfall Y:Color Mapping",
            Template::WaterfallZ => "Waterfall Z:Color Mapping",
            Template::BoxChart => "Box Chart",
            Template::Bubble => "Bubble",
            Template::ColorMapped => "Color Mapped",
            Template::BubbleColorMapped => "Bubble + Color Mapped",
            Template::Pie => "Pie",
            Template::Pie3D => "3D Color Pie",
            Template::PieBw => "2D B&W Pie",
            Template::Polar => "Polar theta(X) r(Y)",
            Template::PolarRTheta => "Polar r(X) theta(Y)",
            Template::Radar => "Radar",
            Template::Ternary => "Ternary",
            Template::Lollipop => "Lollipop Plot",
            Template::VerticalDropLine => "Vertical Drop Line",
            Template::ColumnLabel => "Column + Label",
            Template::GroupedColumns => "Grouped Columns (indexed data)",
            Template::GroupedBox => "Grouped Box Charts",
            Template::IntervalPlot => "Interval Plot",
            Template::Pareto => "Pareto",
            Template::Segment2 => "2 Point Segment",
            Template::Segment3 => "3 Point Segment",
            Template::ScatterRug => "Scatter + Rug",
            Template::Violin => "Violin Plot",
            Template::ViolinBox => "Violin with Box",
            Template::ViolinPoint => "Violin with Point",
            Template::ViolinQuartile => "Violin with Quartile",
            Template::ViolinStick => "Violin with Stick",
            Template::HalfViolin => "Half Violin",
            Template::Ridgeline => "Ridgeline Chart",
            Template::Hexbin => "Hexbin",
            Template::Parallel => "Parallel Plot",
            Template::ParallelIndex => "Parallel Index",
            Template::Treemap => "Treemap",
            Template::Sunburst => "Sunburst Plot",
            Template::Sankey => "Sankey Diagram",
            Template::Alluvial => "Alluvial Diagram",
            Template::Network => "Network Plot",
            Template::Fishbone => "Cause-and-Effect Diagram",
        }
    }

    /// The template of Plot > Multi-Y with more than two Y axes, if this is one.
    pub fn multi_y(&self) -> Option<crate::multi_y::MultiYKind> {
        use crate::multi_y::MultiYKind;
        match self {
            Template::Multi3YYy => Some(MultiYKind::ThreeYYy),
            Template::Multi3YYY => Some(MultiYKind::ThreeYYY),
            Template::Multi4YYyy => Some(MultiYKind::FourYYyy),
            Template::Multi4YyYy => Some(MultiYKind::FourYyYy),
            _ => None,
        }
    }

    /// True for [`Template::Parallel`] and [`Template::ParallelIndex`].
    pub fn is_parallel(&self) -> bool {
        matches!(self, Template::Parallel | Template::ParallelIndex)
    }

    /// True for the six violin templates.
    pub fn is_violin(&self) -> bool {
        matches!(
            self,
            Template::Violin
                | Template::ViolinBox
                | Template::ViolinPoint
                | Template::ViolinQuartile
                | Template::ViolinStick
                | Template::HalfViolin
        )
    }

    /// True for the 3D Surface templates (T166), which read X, Y and Z columns.
    pub fn is_surface3d(&self) -> bool {
        matches!(
            self,
            Template::ColorMapSurface3d
                | Template::ColorFillSurface3d
                | Template::WireFrame3d
                | Template::WireSurface3d
                | Template::ColorMapProjection3d
                | Template::XConstant3d
                | Template::YConstant3d
                | Template::TernarySurface3d
        )
    }

    pub fn from_name(name: &str) -> Option<Template> {
        Template::ALL.into_iter().find(|t| t.name().eq_ignore_ascii_case(name))
    }

    /// Builds an empty graph (no plots) in this template's style: one layer, or two for
    /// [`Template::DoubleY`].
    pub fn new_graph(&self, id: WindowId, name: &str) -> Graph {
        let mut graph = match self {
            Template::Line => general(id, name, PlotKind::Line),
            Template::Scatter3d => space3d(id, name, PlotKind::Scatter),
            Template::Trajectory3d => space3d(id, name, PlotKind::LineSymbol),
            Template::Scatter3dError => space3d(id, name, PlotKind::Scatter),
            Template::Vector3dXyz | Template::Vector3dDelta => space3d(id, name, PlotKind::VectorXyxy),
            Template::Xyy3dBars => xyy(bars3d(id, name)),
            Template::Walls3d => walls3d(id, name),
            Template::Waterfall3d => xyy(space3d(id, name, PlotKind::Line)),
            Template::ColorMapSurface3d => surface3d(id, name, Surface3d::ColorMap),
            Template::ColorFillSurface3d => surface3d(id, name, Surface3d::ColorFill),
            Template::WireFrame3d => surface3d(id, name, Surface3d::WireFrame),
            Template::WireSurface3d => surface3d(id, name, Surface3d::WireSurface),
            Template::ColorMapProjection3d => surface3d(id, name, Surface3d::ColorMapProjection),
            Template::XConstant3d => surface3d(id, name, Surface3d::XConstant),
            Template::YConstant3d => surface3d(id, name, Surface3d::YConstant),
            Template::TernarySurface3d => {
                let mut g = surface3d(id, name, Surface3d::ColorMap);
                if let Coords::Space3d(c) = &mut g.layers[0].coords {
                    c.ternary = true;
                }
                g
            }
            Template::Ribbons3d => {
                let mut g = walls3d(id, name);
                if let Coords::Space3d(c) = &mut g.layers[0].coords {
                    c.ribbons = true;
                }
                g
            }
            Template::Bars3d => bars3d(id, name),
            Template::StepHorizontal => connected(id, name, Connect::StepHorizontal),
            Template::StepVertical => connected(id, name, Connect::StepVertical),
            Template::SplineConnected => connected(id, name, Connect::Spline),
            Template::Scatter => general(id, name, PlotKind::Scatter),
            Template::ScatterCentral => scatter_central(id, name),
            Template::ColumnScatter => {
                let mut g = general(id, name, PlotKind::ColumnScatter);
                let s = &mut g.layers[0].default_symbol;
                s.shape = SymbolShape::Diamond;
                s.size_pt = 8.0;
                g
            }
            Template::LineSymbol => general(id, name, PlotKind::LineSymbol),
            Template::VectorXyam => general(id, name, PlotKind::VectorXyam),
            Template::VectorXyxy => general(id, name, PlotKind::VectorXyxy),
            Template::HighLowClose => general(id, name, PlotKind::HighLowClose),
            Template::Candlestick => general(id, name, PlotKind::Candlestick),
            Template::OhlcBar => general(id, name, PlotKind::OhlcBar),
            Template::OhlcVolume => ohlc_volume(id, name),
            Template::StockLine => themed(general(id, name, PlotKind::Line)),
            Template::MarginalHistograms => marginal(id, name, PlotKind::MarginalHistogram),
            Template::MarginalBoxCharts => marginal(id, name, PlotKind::MarginalBox),
            Template::QcXbarR => qc_xbar_r(id, name),
            Template::ScatterMatrix => {
                let mut g = general(id, name, PlotKind::Scatter);
                crate::scatter_matrix::template_layer(&mut g);
                g
            }
            Template::Zoom => zoom(id, name),
            Template::WindRoseBinned | Template::WindRoseRaw => wind_rose(id, name),
            Template::SmithChart => smith(id, name),
            Template::Piper => piper(id, name),
            Template::Kde2d => {
                let mut g = general(id, name, PlotKind::Kde2d);
                g.layers[0].legend.visible = false;
                g
            }
            Template::CyclicVoltammetry => cyclic_voltammetry(id, name),
            Template::DoubleY => double_y(id, name),
            Template::Multi3YYy | Template::Multi3YYY | Template::Multi4YYyy | Template::Multi4YyYy => {
                let mut g = self.multi_y().expect("multi-Y template").plan().graph(Template::Line, id, name);
                self.multi_y().expect("multi-Y template").apply_legend_defaults(&mut g);
                g
            }
            Template::StackedLines => stacked_lines(id, name),
            Template::Waterfall => waterfall(id, name),
            Template::WaterfallY | Template::WaterfallZ => {
                // The colour scale beside the layer names the colours: no legend.
                let mut g = waterfall(id, name);
                for l in &mut g.layers {
                    l.waterfall_color =
                        if *self == Template::WaterfallY { WaterfallColor::Y } else { WaterfallColor::Z };
                    l.legend.visible = false;
                }
                g
            }
            Template::Column => from_red(column(id, name, PlotKind::Column)),
            Template::StackedColumn => themed(column(id, name, PlotKind::StackedColumn)),
            Template::Stacked100Column => themed(column(id, name, PlotKind::Stacked100Column)),
            Template::Bar => column(id, name, PlotKind::Bar),
            Template::FloatingColumn => from_blue(column(id, name, PlotKind::FloatingColumn)),
            Template::StackedBar => themed(column(id, name, PlotKind::StackedBar)),
            Template::Stacked100Bar => themed(column(id, name, PlotKind::Stacked100Bar)),
            Template::FloatingBar => from_blue(column(id, name, PlotKind::FloatingBar)),
            Template::Area => themed(general(id, name, PlotKind::Area)),
            Template::StackedArea => themed(general(id, name, PlotKind::StackedArea)),
            Template::Stacked100Area => themed(general(id, name, PlotKind::Stacked100Area)),
            Template::FillArea => themed(general(id, name, PlotKind::Area)),
            Template::Histogram | Template::StackedHistograms | Template::HistogramProbabilities => {
                from_red(general(id, name, PlotKind::Histogram))
            }
            Template::BoxChart | Template::GroupedBox => from_red(general(id, name, PlotKind::Box)),
            Template::Bubble => bubble(id, name, false, true),
            Template::ColorMapped => bubble(id, name, true, false),
            Template::BubbleColorMapped => bubble(id, name, true, true),
            Template::Pie => pie(id, name, PlotKind::Pie),
            Template::Pie3D => pie(id, name, PlotKind::Pie3D),
            Template::PieBw => pie(id, name, PlotKind::PieBw),
            Template::Polar => polar(id, name, RadiusColumn::Y),
            Template::PolarRTheta => polar(id, name, RadiusColumn::X),
            Template::Radar => radar(id, name),
            Template::Ternary => ternary(id, name),
            Template::Lollipop => general(id, name, PlotKind::Scatter),
            Template::VerticalDropLine => general(id, name, PlotKind::Scatter),
            Template::IntervalPlot => {
                // The mean is a solid circle (documented: the box chart template with the box hidden).
                let mut g = from_red(general(id, name, PlotKind::Scatter));
                for l in &mut g.layers {
                    l.default_symbol.shape = SymbolShape::Circle;
                    l.default_symbol.filled = true;
                }
                g
            }
            Template::ColumnLabel | Template::GroupedColumns | Template::Pareto => {
                from_red(column(id, name, PlotKind::Column))
            }
            Template::Segment2 | Template::Segment3 => {
                let mut g = general(id, name, PlotKind::LineSymbol);
                let connect = if *self == Template::Segment2 {
                    crate::graph::Connect::Segment2
                } else {
                    crate::graph::Connect::Segment3
                };
                for l in &mut g.layers {
                    l.default_line.connect = connect;
                }
                g
            }
            Template::ScatterRug => scatter_rug(id, name),
            Template::Violin => from_red(general(id, name, PlotKind::Violin)),
            Template::ViolinBox => from_red(general(id, name, PlotKind::ViolinBox)),
            Template::ViolinPoint => from_red(general(id, name, PlotKind::ViolinPoint)),
            Template::ViolinQuartile => from_red(general(id, name, PlotKind::ViolinQuartile)),
            Template::ViolinStick => from_red(general(id, name, PlotKind::ViolinStick)),
            Template::HalfViolin => from_red(general(id, name, PlotKind::HalfViolin)),
            Template::Ridgeline => from_red(general(id, name, PlotKind::Ridgeline)),
            Template::Hexbin => {
                let mut g = from_red(general(id, name, PlotKind::Hexbin));
                for l in &mut g.layers {
                    l.legend.visible = false;
                }
                g
            }
            Template::Parallel => {
                let mut g = from_red(general(id, name, PlotKind::Parallel));
                for l in &mut g.layers {
                    l.legend.visible = false;
                }
                g
            }
            Template::Treemap => {
                // The rectangles fill the layer's frame: no axes, no legend (the names are inside).
                let mut g = general(id, name, PlotKind::Treemap);
                for l in &mut g.layers {
                    l.x_axis.side = AxisSide::Hidden;
                    l.y_axis.side = AxisSide::Hidden;
                    l.legend.visible = false;
                }
                g
            }
            Template::Fishbone => {
                // Only drawn objects and labels: no axes, no legend, the frame over most of the page.
                let mut g = general(id, name, PlotKind::Line);
                for l in &mut g.layers {
                    l.x_axis.side = AxisSide::Hidden;
                    l.y_axis.side = AxisSide::Hidden;
                    l.x_axis.show_opposite = false;
                    l.y_axis.show_opposite = false;
                    l.legend.visible = false;
                    l.frame = Frame { left: 0.04, top: 0.06, width: 0.9, height: 0.88 };
                }
                g
            }
            Template::Network => {
                // Nodes and edges fill the layer's frame: no axes, no legend (the names are beside the nodes).
                let mut g = general(id, name, PlotKind::Network);
                for l in &mut g.layers {
                    l.x_axis.side = AxisSide::Hidden;
                    l.y_axis.side = AxisSide::Hidden;
                    l.x_axis.show_opposite = false;
                    l.y_axis.show_opposite = false;
                    l.legend.visible = false;
                }
                g
            }
            Template::Sankey | Template::Alluvial => {
                // The nodes and bands fill the layer's frame: no axes, no legend (the names are beside the nodes).
                let mut g = general(id, name, PlotKind::Sankey);
                for l in &mut g.layers {
                    l.x_axis.side = AxisSide::Hidden;
                    l.y_axis.side = AxisSide::Hidden;
                    l.x_axis.show_opposite = false;
                    l.y_axis.show_opposite = false;
                    l.legend.visible = false;
                }
                g
            }
            Template::Sunburst => {
                // The rings fill the layer's frame: no axes, no legend (the names are in the rings).
                let mut g = general(id, name, PlotKind::Sunburst);
                for l in &mut g.layers {
                    l.x_axis.side = AxisSide::Hidden;
                    l.y_axis.side = AxisSide::Hidden;
                    l.legend.visible = false;
                }
                g
            }
            Template::ParallelIndex => {
                // The legend of the groups goes right of the frame (`Automatic`, see graphing spec 14.25).
                let mut g = from_red(general(id, name, PlotKind::ParallelIndex));
                for l in &mut g.layers {
                    l.legend.position = LegendPosition::Automatic;
                }
                g
            }
        };
        // These four stock templates share the measured anchor. Other templates keep their
        // own placement (including CV, polar/radar and the separately measured Double-Y).
        if matches!(
            self,
            Template::Line | Template::Scatter | Template::ScatterCentral | Template::LineSymbol | Template::Column
        ) {
            graph.layers[0].legend.position = LegendPosition::Automatic;
            // The stock white legend hides the frame/curves under it (Origin exports, spec 2.4).
            graph.layers[0].legend.background = Some(Color::WHITE);
        }
        graph
    }
}

/// Default font family. Arial ships with Windows; the renderer falls back
/// to a metric-compatible family where it is missing.
pub const DEFAULT_FONT: &str = "Arial";

/// Origin's colour list for a new group of plots: its base palette in order (black, red, green,
/// blue, cyan, magenta, yellow, dark yellow, navy, purple, wine, olive, dark cyan, royal, orange,
/// violet, pink, white, light gray, gray, light yellow, light cyan, light magenta, dark gray), then
/// round again (graphing spec section 1.5).
pub const ORIGIN_COLOR_LIST: [(u8, u8, u8); 24] = [
    (0, 0, 0),
    (255, 0, 0),
    (0, 255, 0),
    (0, 0, 255),
    (0, 255, 255),
    (255, 0, 255),
    (255, 255, 0),
    (128, 128, 0),
    (0, 0, 128),
    (128, 0, 128),
    (128, 0, 0),
    (0, 128, 0),
    (0, 128, 128),
    (0, 0, 160),
    (255, 128, 0),
    (128, 0, 255),
    (255, 0, 128),
    (255, 255, 255),
    (192, 192, 192),
    (128, 128, 128),
    (255, 255, 128),
    (128, 255, 255),
    (255, 128, 255),
    (64, 64, 64),
];

/// Origin 2016's named colour lists (`Themes\Color\*.oth` of the installation, decoded as in graphing 1.5: an
/// entry is a base-palette index or red, green, blue), by file name: the colours a Line Color increment hands
/// out when the list is chosen in the Group tab's Details (T332).
pub const ORIGIN_COLOR_LISTS: [(&str, &[(u8, u8, u8)]); 18] = [
    (
        "D01 Pumpkin Patch",
        &[
            (97, 58, 0),
            (188, 94, 0),
            (255, 146, 18),
            (255, 218, 91),
            (216, 231, 108),
            (144, 186, 72),
            (72, 141, 36),
            (0, 96, 0),
        ],
    ),
    (
        "D02 Maple",
        &[
            (21, 87, 44),
            (106, 137, 43),
            (183, 184, 42),
            (234, 196, 41),
            (239, 174, 40),
            (244, 120, 39),
            (250, 86, 73),
            (255, 0, 37),
        ],
    ),
    (
        "D03 Sea and Land",
        &[
            (0, 64, 128),
            (0, 0, 255),
            (0, 128, 192),
            (128, 255, 255),
            (255, 192, 130),
            (192, 97, 10),
            (151, 107, 53),
            (88, 62, 33),
        ],
    ),
    (
        "D04 Watermelon",
        &[
            (0, 129, 0),
            (60, 189, 45),
            (120, 249, 90),
            (180, 255, 135),
            (255, 220, 193),
            (255, 130, 97),
            (255, 70, 52),
            (255, 0, 0),
        ],
    ),
    ("D05 Blue-Greens", &[(16, 0, 129), (0, 0, 255), (1, 225, 255), (16, 255, 1), (0, 128, 0), (0, 128, 128)]),
    ("Q01 Black and RGB", &[(0, 0, 0), (255, 0, 0), (0, 255, 0), (0, 0, 255)]),
    (
        "Q02 Basic",
        &[
            (0, 0, 0),
            (255, 0, 0),
            (0, 0, 255),
            (255, 0, 255),
            (0, 128, 0),
            (0, 0, 128),
            (128, 0, 255),
            (128, 0, 128),
            (128, 0, 0),
            (128, 128, 0),
            (43, 99, 162),
            (30, 150, 150),
            (155, 100, 26),
            (16, 199, 62),
            (185, 36, 122),
            (45, 197, 204),
            (63, 65, 152),
            (147, 172, 43),
            (128, 128, 128),
            (150, 100, 100),
            (100, 150, 100),
            (43, 163, 202),
            (50, 100, 150),
        ],
    ),
    (
        "Q03 Rainbow7",
        &[(255, 0, 0), (255, 102, 0), (255, 238, 0), (0, 255, 0), (0, 125, 255), (68, 0, 255), (153, 0, 255)],
    ),
    (
        "Q04 Paired Color",
        &[
            (151, 215, 242),
            (7, 174, 227),
            (234, 165, 194),
            (220, 73, 89),
            (179, 212, 157),
            (53, 178, 87),
            (237, 193, 148),
            (240, 145, 55),
            (189, 190, 220),
            (98, 111, 179),
            (246, 237, 170),
            (249, 231, 83),
        ],
    ),
    (
        "Q05 Season Spring",
        &[
            (119, 182, 97),
            (59, 131, 197),
            (243, 109, 33),
            (255, 213, 4),
            (139, 141, 197),
            (229, 102, 100),
            (158, 206, 180),
            (172, 203, 233),
            (248, 153, 83),
            (254, 232, 118),
            (186, 182, 218),
            (245, 145, 145),
        ],
    ),
    (
        "Q06 Season Summer",
        &[
            (21, 178, 209),
            (254, 74, 85),
            (117, 184, 85),
            (249, 214, 46),
            (119, 221, 233),
            (253, 254, 6),
            (200, 230, 158),
            (244, 151, 159),
            (34, 110, 148),
            (247, 172, 22),
            (21, 114, 65),
            (199, 20, 0),
        ],
    ),
    (
        "Q07 Season Fall",
        &[
            (233, 84, 0),
            (238, 179, 0),
            (154, 150, 40),
            (242, 159, 97),
            (255, 219, 130),
            (213, 218, 112),
            (149, 43, 0),
            (129, 86, 0),
            (67, 63, 27),
            (251, 154, 9),
            (253, 210, 10),
            (216, 235, 44),
        ],
    ),
    (
        "Q08 Season Winter",
        &[
            (93, 130, 179),
            (204, 172, 156),
            (148, 149, 130),
            (158, 207, 231),
            (38, 68, 123),
            (209, 210, 209),
            (237, 224, 182),
            (99, 99, 100),
            (178, 134, 106),
            (138, 158, 192),
            (170, 171, 171),
            (226, 208, 194),
        ],
    ),
    ("S01 Gray Scale", &[(0, 0, 0), (64, 64, 64), (128, 128, 128), (192, 192, 192), (255, 255, 255)]),
    (
        "S02 Cyan",
        &[(0, 80, 87), (0, 126, 143), (0, 167, 201), (2, 204, 255), (62, 205, 255), (117, 213, 255), (176, 227, 255)],
    ),
    (
        "S03 Green",
        &[(21, 87, 0), (42, 143, 0), (63, 187, 0), (90, 227, 0), (146, 255, 62), (184, 255, 117), (217, 255, 176)],
    ),
    (
        "S04 Red",
        &[(87, 13, 0), (143, 15, 0), (201, 11, 0), (255, 2, 2), (255, 64, 70), (255, 117, 129), (255, 176, 186)],
    ),
    (
        "System Color List",
        &[
            (240, 0, 130),
            (250, 60, 60),
            (240, 130, 40),
            (230, 175, 45),
            (230, 220, 50),
            (160, 230, 50),
            (0, 220, 0),
            (0, 210, 140),
            (0, 200, 200),
            (0, 160, 255),
            (30, 60, 255),
            (110, 0, 220),
            (160, 0, 200),
        ],
    ),
];

/// The colours of a named list: one of [`ORIGIN_COLOR_LISTS`] by its full name or its code (`Q03`, `s01`),
/// without regard to case, or `Default` for [`ORIGIN_COLOR_LIST`]. Returns the list's name as well.
pub fn color_list(name: &str) -> Option<(&'static str, Vec<Color>)> {
    let name = name.trim();
    let rgb = |l: &[(u8, u8, u8)]| l.iter().map(|&(r, g, b)| Color::rgb(r, g, b)).collect();
    if name.eq_ignore_ascii_case("default") {
        return Some(("Default", rgb(&ORIGIN_COLOR_LIST)));
    }
    ORIGIN_COLOR_LISTS
        .iter()
        .find(|(n, _)| {
            n.eq_ignore_ascii_case(name) || n.split(' ').next().is_some_and(|c| c.eq_ignore_ascii_case(name))
        })
        .map(|(n, l)| (*n, rgb(l)))
}

/// The colour list of the owner's graph theme, which Origin's area, stacked area and stacked column
/// templates use (black, red, blue, magenta, green, navy, violet, purple, wine, dark yellow, then
/// custom colours; graphing spec section 1.5), measured on the legends of plots made from those
/// templates.
pub const THEME_COLOR_LIST: [(u8, u8, u8); 23] = [
    (0, 0, 0),
    (255, 0, 0),
    (0, 0, 255),
    (255, 0, 255),
    (0, 128, 0),
    (0, 0, 128),
    (128, 0, 255),
    (128, 0, 128),
    (128, 0, 0),
    (128, 128, 0),
    (43, 99, 162),
    (30, 150, 150),
    (155, 100, 26),
    (16, 199, 62),
    (185, 36, 122),
    (45, 197, 204),
    (63, 65, 152),
    (147, 172, 43),
    (128, 128, 128),
    (150, 100, 100),
    (100, 150, 100),
    (43, 163, 202),
    (50, 100, 150),
];

/// The graph with its colour list replaced by the theme list ([`THEME_COLOR_LIST`]).
fn themed(mut g: Graph) -> Graph {
    for l in &mut g.layers {
        l.palette.colors = THEME_COLOR_LIST.iter().map(|&(r, g, b)| Color::rgb(r, g, b)).collect();
    }
    g
}

/// The graph with its colour list started at the second entry of the base palette (red, green, blue,
/// cyan, ...), as Origin's Column and Box templates do (measured: the first column of a plain
/// Column graph is red, not black).
fn from_red(g: Graph) -> Graph {
    from_entry(g, 1)
}

/// The graph with its colour list started at blue, the fourth entry of the base palette, as Origin's
/// Floating Column template does (measured: its first plot is filled blue, graphing.md 14.12; the
/// colours of further plots were not measured and follow the base palette from there).
fn from_blue(g: Graph) -> Graph {
    from_entry(g, 3)
}

fn from_entry(mut g: Graph, first: usize) -> Graph {
    for l in &mut g.layers {
        l.palette.colors = (first..first + ORIGIN_COLOR_LIST.len())
            .map(|i| ORIGIN_COLOR_LIST[i % ORIGIN_COLOR_LIST.len()])
            .map(|(r, g, b)| Color::rgb(r, g, b))
            .collect();
    }
    g
}

/// Origin's default page, in inches (6432 by 4923 units of 1/600 inch). Its font sizes (18 pt tick
/// labels, 22 pt titles) are meant for this size.
pub const ORIGIN_PAGE_IN: (f64, f64) = (10.72, 8.205);

fn base_axis(title_pt: f64, tick_pt: f64) -> Axis {
    Axis {
        scale: ScaleKind::Linear,
        min: None,
        max: None,
        auto_range: AutoRange::Nice,
        rescale_margin_pct: crate::autorange::DEFAULT_RESCALE_MARGIN_PCT,
        major_step: None,
        minor_ticks: 1,
        title: None,
        show_title: true,
        title_font: Font::new(DEFAULT_FONT, title_pt),
        show_tick_labels: true,
        tick_font: Font::new(DEFAULT_FONT, tick_pt),
        tick_format: TickFormat::Auto,
        tick_label_divisor: 1.0,
        major_tick: TickDirection::Out,
        minor_tick: TickDirection::Out,
        major_tick_len_pt: 9.0,
        minor_tick_len_pt: 4.5,
        line_width_pt: 1.5,
        color: Color::BLACK,
        show_opposite: false,
        opposite_ticks: false,
        opposite_offset_pct: None,
        major_grid: None,
        minor_grid: None,
        side: AxisSide::Normal,
        offset_pct: 0.0,
        color_follows_plot: false,
        title_color_follows_plot: false,
        title_on_top: false,
        tick_label_source: TickLabelSource::Auto,
        tick_label_angle_deg: 0.0,
        hide_end_label: false,
        reverse: false,
        first_tick: None,
        axis_breaks: Vec::new(),
        additional_line: None,
        at_position: None,
        arrow_begin: false,
        arrow_end: false,
        title_rotation_deg: None,
        title_offset_pt: (0.0, 0.0),
        special_ticks: Vec::new(),
        tick_label_formula: String::new(),
        minor_tick_labels: false,
        tick_color: None,
        tick_width_pt: None,
        tick_prefix: String::new(),
        tick_suffix: String::new(),
        rug: Default::default(),
        rug_opposite: Default::default(),
    }
}

/// The scatter template with a default rug on the bottom and left axes.
fn scatter_rug(id: WindowId, name: &str) -> Graph {
    let mut g = general(id, name, PlotKind::Scatter);
    for l in &mut g.layers {
        l.x_axis.rug = Rug::shown();
        l.y_axis.rug = Rug::shown();
    }
    g
}

/// The Line template with another connection between the points.
fn connected(id: WindowId, name: &str, connect: Connect) -> Graph {
    let mut g = general(id, name, PlotKind::Line);
    g.layers[0].default_line.connect = connect;
    g
}

fn general(id: WindowId, name: &str, kind: PlotKind) -> Graph {
    let axis = base_axis(22.0, 18.0);
    let palette = Palette {
        colors: ORIGIN_COLOR_LIST.iter().map(|&(r, g, b)| Color::rgb(r, g, b)).collect(),
        mode: PaletteMode::Cycle,
        list: None,
    };
    Graph {
        id,
        name: name.to_string(),
        long_name: String::new(),
        page: Page::inches(ORIGIN_PAGE_IN.0, ORIGIN_PAGE_IN.1),
        layers: vec![Layer {
            frame: Frame { left: 0.1791, top: 0.1158, width: 0.6816, height: 0.7183 },
            x_axis: axis.clone(),
            y_axis: axis,
            plots: Vec::new(),
            legend: Legend {
                behind_data: false,
                on_top: false,
                visible: true,
                position: LegendPosition::TopRight,
                font: Font::new(DEFAULT_FONT, 22.0),
                frame: true,
                background: None,
                scope: LegendScope::Layer,
                columns: 1,
                text: None,
            },
            labels: Vec::new(),
            shapes: Vec::new(),
            slopes: Vec::new(),
            straight_lines: Vec::new(),
            scales: Vec::new(),
            brackets: Vec::new(),
            comparison: None,
            palette,
            default_line: LineStyle::solid(Color::BLACK, 0.5),
            default_symbol: SymbolStyle {
                shape: SymbolShape::Square,
                size_pt: 9.0,
                color: Color::BLACK,
                filled: true,
                interior: None,
                fill_color: None,
                edge_width_pt: 0.5,
                every: 1,
            },
            default_kind: kind,
            link: None,
            zoom: None,
            markers: Default::default(),
            stack: Stack::None,
            waterfall_color: WaterfallColor::None,
            plot_offsets: Vec::new(),
            bars: BarSpacing::default(),
            coords: Coords::Cartesian,
            background: None,
            background_style: BackgroundStyle::default(),
            border: Default::default(),
            element_scale: LayerElementScale::default(),
            group_style: GroupStyle::default(),
            display: LayerDisplay::default(),
            xy_ratio: None,
            field: None,
        }],
    }
}

/// The scatter template for bubble and colour mapped plots. Bubbles are open circles in the first
/// colour of the list (measured: black rings), colour mapped symbols the usual filled squares, and
/// bubbles that are also colour mapped are filled circles.
fn bubble(id: WindowId, name: &str, mapped: bool, sized: bool) -> Graph {
    let mut g = general(id, name, PlotKind::Scatter);
    let s = &mut g.layers[0].default_symbol;
    if sized {
        s.shape = SymbolShape::Circle;
        s.filled = mapped;
    }
    g
}

/// The pie template: no axes (the plot draws its own frame, slices, labels and legend), the colour
/// list of the line template (measured: black, red, green, blue, cyan, ...).
fn pie(id: WindowId, name: &str, kind: PlotKind) -> Graph {
    let mut g = general(id, name, kind);
    let l = &mut g.layers[0];
    // Origin's pie templates have a larger layer than the line template (measured on the 3D and
    // the B&W pie: 7 % from the left, 86.5 % wide, 8 % from the top, 83 % high).
    l.frame = Frame { left: 0.0695, top: 0.079, width: 0.865, height: 0.83 };
    l.x_axis.side = AxisSide::Hidden;
    l.y_axis.side = AxisSide::Hidden;
    l.legend.visible = false;
    g
}

/// The polar template: the line template in a layer of polar coordinates. The frame is a square
/// on the page (the circle is as wide as the frame is high), the grid rings and spokes are grey
/// and solid (Sheaf's usual grid style, as in Origin's polar template), and the axis titles are
/// not drawn.
fn polar(id: WindowId, name: &str, radius: RadiusColumn) -> Graph {
    polar_of(id, name, radius, PlotKind::Line)
}

fn polar_of(id: WindowId, name: &str, radius: RadiusColumn, kind: PlotKind) -> Graph {
    let mut g = general(id, name, kind);
    // Origin's polar page (measured, spec 14.18): 8.04 by 6.15 in with a square layer of 4.55 in, its
    // left edge at 23.3 % of the page, and the legend above the layer's top right corner.
    g.page = Page::inches(8.04, 6.15);
    let l = &mut g.layers[0];
    l.frame = Frame { left: 0.2326, top: 0.1174, width: 0.5665, height: 0.7402 };
    l.legend.position = LegendPosition::Manual { x: 0.84, y: -0.108 };
    l.coords = Coords::Polar(PolarCoords { radius, ..PolarCoords::default() });
    let grid = LineStyle::solid(Color::rgb(204, 204, 204), 0.5);
    for a in [&mut l.x_axis, &mut l.y_axis] {
        a.major_grid = Some(grid.clone());
        a.show_title = false;
        a.major_tick = TickDirection::Out;
        a.major_tick_len_pt = 6.0;
        a.minor_ticks = 0;
    }
    g
}

/// The radar template: the line + symbol template in a layer of polar coordinates whose spokes are
/// the rows (graphing spec section 14.19). The grid is made of polygons at the radius ticks and
/// straight spokes; the spoke names stand outside, the outer polygon has no ticks.
fn radar(id: WindowId, name: &str) -> Graph {
    let mut g = polar_of(id, name, RadiusColumn::Y, PlotKind::LineSymbol);
    let l = &mut g.layers[0];
    l.coords = Coords::Polar(PolarCoords { zero_deg: 90.0, clockwise: true, radar: true, ..PolarCoords::default() });
    // Origin's radar page (measured, spec 14.19): 8 by 6.125 in, the layer 61 % wide and 75 % high from
    // 11.7 % and 10.9 %, the legend at the top right corner of the layer.
    g.page = Page::inches(8.0, 6.125);
    let l = &mut g.layers[0];
    l.frame = Frame { left: 0.1173, top: 0.1092, width: 0.612, height: 0.7535 };
    l.legend.position = LegendPosition::Manual { x: 1.0116, y: -0.0208 };
    l.x_axis.major_tick = TickDirection::None;
    l.y_axis.major_tick_len_pt = 5.0;
    g
}

/// The ternary template: a scatter plot in a layer of triangle coordinates. The frame is a square
/// on the page (the triangle is as wide as it fits), the grid lines parallel to the sides are on
/// at the major ticks, minor ticks are off. The three axis titles are the column names.
fn ternary(id: WindowId, name: &str) -> Graph {
    // Origin's ternary graph keeps the usual page and layer frame (its picture is 10.72 by 8.205 in,
    // measured); the triangle is the largest that fits in the frame.
    let mut g = general(id, name, PlotKind::Scatter);
    let l = &mut g.layers[0];
    l.coords = Coords::Ternary(TernaryCoords::default());
    // Origin's default ternary graph (measured, graphing spec 14.20): a legend (the plot is named
    // after its Z column), major grid lines in Gray, minor ones dashed in LT Gray, one minor tick.
    l.legend.visible = true;
    let grid = LineStyle::solid(Color::rgb(128, 128, 128), 0.5);
    let minor = LineStyle { dash: Dash::Dash, ..LineStyle::solid(Color::rgb(192, 192, 192), 0.5) };
    for a in [&mut l.x_axis, &mut l.y_axis] {
        a.major_grid = Some(grid.clone());
        a.minor_grid = Some(minor.clone());
        a.major_tick = TickDirection::Out;
        a.minor_tick = TickDirection::Out;
        a.major_tick_len_pt = 6.0;
        a.minor_tick_len_pt = 3.0;
        a.minor_ticks = 1;
    }
    g
}

fn cyclic_voltammetry(id: WindowId, name: &str) -> Graph {
    let mut x = base_axis(12.0, 10.0);
    // The owner's figure style keeps its own line and tick sizes.
    x.line_width_pt = 1.0;
    x.major_tick_len_pt = 5.0;
    x.minor_tick_len_pt = 2.5;
    x.auto_range = AutoRange::Tight;
    x.minor_ticks = 0;
    x.major_tick = TickDirection::In;
    x.minor_tick = TickDirection::None;
    x.show_opposite = true;
    let mut y = x.clone();
    y.auto_range = AutoRange::SymmetricZero;

    let mut g = general(id, name, PlotKind::Line);
    g.page = Page::inches(5.0, 3.6);
    let layer = &mut g.layers[0];
    layer.default_symbol = SymbolStyle {
        shape: SymbolShape::Circle,
        size_pt: 5.0,
        color: Color::BLACK,
        filled: true,
        interior: None,
        fill_color: None,
        edge_width_pt: 0.75,
        every: 1,
    };
    layer.frame = Frame { left: 0.17, top: 0.06, width: 0.78, height: 0.74 };
    layer.x_axis = x;
    layer.y_axis = y;
    layer.palette = Palette {
        colors: vec![Color::rgb(0x46, 0xB1, 0xE1), Color::rgb(0x4E, 0x95, 0xD9), Color::rgb(0x10, 0x48, 0x62)],
        mode: PaletteMode::Spread,
        list: None,
    };
    layer.default_line =
        LineStyle { color: Color::BLACK, width_pt: 1.75, dash: Dash::Solid, connect: Connect::Spline, sort_x: false };
    layer.legend = Legend {
        behind_data: false,
        on_top: false,
        visible: true,
        position: LegendPosition::BottomRight,
        font: Font::new(DEFAULT_FONT, 10.0),
        frame: false,
        background: None,
        scope: LegendScope::Layer,
        columns: 1,
        text: None,
    };
    g
}

/// Origin's OHLC-Volume (T109, measured): a price layer at 8.66 %, 4.69 %, 88.23 % x 65.92 % of the page
/// with its X tick labels on top, and below it a layer linked to it (left 0, top 100 %, width 100 %,
/// height 34.98 % of the price layer) for the volume columns. No legends.
fn ohlc_volume(id: WindowId, name: &str) -> Graph {
    let mut g = general(id, name, PlotKind::Candlestick);
    let price = &mut g.layers[0];
    price.frame = Frame { left: 0.0866, top: 0.04692, width: 0.8823, height: 0.6592 };
    price.legend.visible = false;
    price.x_axis.side = AxisSide::Opposite;
    price.x_axis.show_opposite = true;
    price.x_axis.opposite_ticks = true;
    let mut volume = right_y_layer(price, 0);
    volume.frame = Frame { left: 0.0, top: 1.0, width: 1.0, height: 0.3498 };
    volume.x_axis.side = AxisSide::Normal;
    volume.y_axis.side = AxisSide::Normal;
    volume.default_kind = PlotKind::Column;
    g.layers.push(volume);
    g
}

/// Origin's Marginal Histograms and Marginal Box Charts (T110, measured): the scatter at 15 %, 25.01 %,
/// 60 % x 60 % of the page and two layers linked to it, a third of its size, above it (sharing X) and right
/// of it (sharing Y). The margins draw no tick labels or titles; no legends.
fn marginal(id: WindowId, name: &str, kind: PlotKind) -> Graph {
    let hist = kind == PlotKind::MarginalHistogram;
    let mut g = general(id, name, PlotKind::Scatter);
    let scatter = &mut g.layers[0];
    scatter.frame = Frame { left: 0.15, top: 0.2501, width: 0.6, height: 0.6 };
    scatter.legend.visible = false;
    // Small dots, not the scatter template's squares (measured: about 4 pt circles).
    scatter.default_symbol.shape = SymbolShape::Circle;
    scatter.default_symbol.size_pt = 4.0;
    // The histogram graph's scatter has its bottom and left axes only (the margins' base lines close the
    // box); the box chart graph's scatter is a closed box.
    for axis in [&mut scatter.x_axis, &mut scatter.y_axis] {
        axis.show_opposite = !hist;
        axis.opposite_ticks = false;
    }
    let base = right_y_layer(scatter, 0);
    let margin = |mut l: Layer, frame: Frame, x_shared: bool| {
        l.frame = frame;
        l.default_kind = kind;
        l.link = Some(LayerLink {
            parent: 0,
            x: if x_shared { ScaleLink::Straight } else { ScaleLink::None },
            y: if x_shared { ScaleLink::None } else { ScaleLink::Straight },
        });
        for axis in [&mut l.x_axis, &mut l.y_axis] {
            axis.side = AxisSide::Hidden;
            axis.show_opposite = false;
            axis.show_title = false;
            axis.show_tick_labels = false;
            axis.major_tick = TickDirection::None;
            axis.minor_tick = TickDirection::None;
            axis.minor_ticks = 0;
        }
        if hist {
            // The base line of the bars: the X axis line under the top bars, the Y axis line left of the right ones.
            let shared = if x_shared { &mut l.x_axis } else { &mut l.y_axis };
            shared.side = AxisSide::Normal;
        }
        // The counts start at 0 (measured); a box stands at 1 in a 0.5 to 1.5 span (measured).
        let own = if x_shared { &mut l.y_axis } else { &mut l.x_axis };
        if hist {
            own.min = Some(0.0);
        } else {
            own.min = Some(0.5);
            own.max = Some(1.5);
        }
        l
    };
    let top = margin(base.clone(), Frame { left: 0.0, top: -0.3301, width: 1.0, height: 0.3301 }, true);
    let right = margin(base, Frame { left: 1.0, top: 0.0, width: 0.3301, height: 1.0 }, false);
    g.layers.push(top);
    g.layers.push(right);
    g
}

/// Origin's X bar R chart (T111, measured): the range chart at 13.99 %, 47.95 %, 65.01 % x 37 % of the
/// page, and the mean chart linked above it (top -107 %, the same size) sharing its X scale. No legends.
fn qc_xbar_r(id: WindowId, name: &str) -> Graph {
    let mut g = general(id, name, PlotKind::QcRanges);
    let range = &mut g.layers[0];
    range.frame = Frame { left: 0.1399, top: 0.4795, width: 0.6501, height: 0.37 };
    range.legend.visible = false;
    range.x_axis.title = Some("Subgroup Number".into());
    range.y_axis.title = Some("Range".into());
    for axis in [&mut range.x_axis, &mut range.y_axis] {
        axis.show_opposite = true;
        axis.opposite_ticks = false;
    }
    let mut mean = right_y_layer(range, 0);
    mean.frame = Frame { left: 0.0, top: -1.07, width: 1.0, height: 1.0 };
    mean.default_kind = PlotKind::QcMeans;
    mean.x_axis.side = AxisSide::Normal;
    mean.x_axis.show_tick_labels = false;
    mean.x_axis.show_title = false;
    mean.x_axis.show_opposite = true;
    mean.y_axis.side = AxisSide::Normal;
    mean.y_axis.show_opposite = true;
    mean.y_axis.title = Some("Subgroup Mean (XBAR)".into());
    g.layers.push(mean);
    g
}

/// The fill of a Zoom graph's region rectangle and of its zoomed layer (measured cyan; the rectangle's edge blue).
pub const ZOOM_FILL: Color = Color::rgb(0, 255, 255);

/// Origin's Zoom graph (T112, measured): the whole curve at 16 %, 8 %, 75 % x 30 % of the page in a closed
/// box with grey major grid lines, and below it a layer linked to it (left -5.979 %, top 127.1 %, width
/// 106 %, height 152 % of the first) with a cyan background whose ranges are the region rectangle's
/// (`Layer::zoom`). No axis titles, no legends. The rectangle is placed when the data are plotted
/// (`Project::plot_zoom`).
fn zoom(id: WindowId, name: &str) -> Graph {
    let mut g = general(id, name, PlotKind::Line);
    let whole = &mut g.layers[0];
    whole.frame = Frame { left: 0.16, top: 0.08, width: 0.75, height: 0.30 };
    whole.legend.visible = false;
    for axis in [&mut whole.x_axis, &mut whole.y_axis] {
        axis.show_opposite = true;
        axis.opposite_ticks = false;
        axis.show_title = false;
        axis.major_grid = Some(LineStyle::solid(Color::rgb(128, 128, 128), 0.5));
    }
    let mut zoomed = right_y_layer(whole, 0);
    zoomed.link = Some(LayerLink { parent: 0, x: ScaleLink::None, y: ScaleLink::None });
    zoomed.frame = Frame { left: -0.05979, top: 1.271, width: 1.06, height: 1.52 };
    zoomed.background = Some(ZOOM_FILL);
    zoomed.zoom = Some(0);
    zoomed.palette = whole.palette.clone();
    for axis in [&mut zoomed.x_axis, &mut zoomed.y_axis] {
        axis.side = AxisSide::Normal;
        axis.show_opposite = false;
        axis.show_title = false;
        axis.major_grid = None;
        axis.show_tick_labels = true;
    }
    g.layers.push(zoomed);
    g
}

/// A wind rose (T113, from Origin's documentation, not measured): the polar page with north up and the
/// angles running clockwise in degrees, ticks every 45 degrees, the legend listing the speed bands.
fn wind_rose(id: WindowId, name: &str) -> Graph {
    let mut g = polar_of(id, name, RadiusColumn::Y, PlotKind::WindRose);
    let l = &mut g.layers[0];
    l.coords = Coords::Polar(PolarCoords { zero_deg: 90.0, clockwise: true, ..PolarCoords::default() });
    l.x_axis.min = Some(0.0);
    l.x_axis.max = Some(360.0);
    l.x_axis.major_step = Some(45.0);
    g
}

/// A 3D XYZ graph (T155, Sheaf's own look; Origin's 3D graphs are OpenGL and not measured): a square
/// layer whose X and Y axes run over the view's extent and are not drawn; the layer draws the XYZ box.
fn space3d(id: WindowId, name: &str, kind: PlotKind) -> Graph {
    let mut g = general(id, name, kind);
    let (pw, ph) = (g.page.width_pt, g.page.height_pt);
    let l = &mut g.layers[0];
    l.coords = Coords::Space3d(Space3dCoords::default());
    // A square frame, so that the cube is not stretched.
    let side = 0.78 * ph;
    l.frame = Frame { left: (pw - side) / 2.0 / pw, top: 0.1, width: side / pw, height: side / ph };
    for a in [&mut l.x_axis, &mut l.y_axis] {
        a.min = Some(-SPACE3D_EXTENT);
        a.max = Some(SPACE3D_EXTENT);
        a.side = AxisSide::Hidden;
        a.show_opposite = false;
        a.major_grid = None;
        a.minor_grid = None;
    }
    g
}

/// 3D bars (T156): filled bars with a black edge, so that nearer bars stand out from farther ones.
fn bars3d(id: WindowId, name: &str) -> Graph {
    let mut g = from_red(space3d(id, name, PlotKind::Column));
    g.layers[0].default_line = LineStyle::solid(Color::BLACK, 0.5);
    g.layers[0].default_symbol.filled = true;
    g
}

/// 3D Walls (T158): XYY Area plots with a black edge, each filled down to the floor; 3D Ribbons
/// (T161) start from them.
fn walls3d(id: WindowId, name: &str) -> Graph {
    let mut g = xyy(space3d(id, name, PlotKind::Area));
    g.layers[0].default_line = LineStyle::solid(Color::BLACK, 0.5);
    g
}

/// A 3D surface layer (T166): red surfaces (Color Fill) with black mesh lines, 0.5 pt.
fn surface3d(id: WindowId, name: &str, kind: Surface3d) -> Graph {
    let mut g = from_red(space3d(id, name, PlotKind::Line));
    g.layers[0].default_line = LineStyle::solid(Color::BLACK, 0.5);
    if let Coords::Space3d(c) = &mut g.layers[0].coords {
        c.surface = Some(kind);
    }
    g
}

/// A 3D layer read as XYY (T158): each plot at its own depth.
fn xyy(mut g: Graph) -> Graph {
    if let Coords::Space3d(c) = &mut g.layers[0].coords {
        c.xyy = true;
    }
    g
}

/// Half the width of the hidden axes of a 3D layer: room for the turned cube (at most sqrt(3)).
pub const SPACE3D_EXTENT: f64 = 1.8;

/// A Smith chart (T115, from Origin's documentation, not measured): line + symbol plots in a square layer
/// of the polar page whose axes run from -1 to 1 and are not drawn; the layer draws the Smith grid.
fn smith(id: WindowId, name: &str) -> Graph {
    // Origin 2016 draws the data as a line plot (measured, T385).
    let mut g = polar_of(id, name, RadiusColumn::Y, PlotKind::Line);
    let l = &mut g.layers[0];
    l.coords = Coords::Smith(SmithCoords::default());
    for a in [&mut l.x_axis, &mut l.y_axis] {
        a.min = Some(-1.0);
        a.max = Some(1.0);
        a.side = AxisSide::Hidden;
        a.show_opposite = false;
        a.major_grid = None;
        a.minor_grid = None;
    }
    g
}

/// A Piper diagram (T117, from the standard construction, not measured): one layer with hidden axes over the
/// diagram's extent, one X unit as long as one Y unit, the corner labels, no legend.
fn piper(id: WindowId, name: &str) -> Graph {
    use crate::piper::{X_RANGE, Y_RANGE};
    let mut g = general(id, name, PlotKind::Piper);
    let page = (g.page.width_pt, g.page.height_pt);
    let l = &mut g.layers[0];
    l.legend.visible = false;
    for (a, (lo, hi)) in [(&mut l.x_axis, X_RANGE), (&mut l.y_axis, Y_RANGE)] {
        a.min = Some(lo);
        a.max = Some(hi);
        a.side = AxisSide::Hidden;
        a.show_opposite = false;
        a.major_grid = None;
        a.minor_grid = None;
    }
    l.fit_xy_ratio(1.0, page, X_RANGE.1 - X_RANGE.0, Y_RANGE.1 - Y_RANGE.0);
    // Origin 2016 titles the sides with the column names (drawn with the plot, T389); no corner labels.
    g
}

fn double_y(id: WindowId, name: &str) -> Graph {
    let mut g = general(id, name, PlotKind::Line);
    let layer = &mut g.layers[0];
    // Origin's frame is 68 % wide, which leaves room for the right tick labels and title as well
    // as the left ones.
    // The top line closes the box; the right edge belongs to the right Y axis.
    layer.x_axis.show_opposite = true;
    layer.x_axis.opposite_ticks = true;
    layer.y_axis.color_follows_plot = true;
    layer.legend.scope = LegendScope::Graph;
    layer.legend.position = LegendPosition::TopRight;
    let right = right_y_layer(layer, 0);
    g.layers.push(right);
    g
}

/// Offsets that stack the curves with a gap of 10 % of the tallest curve, as Origin's
/// "Stacked Lines by Y Offsets" does.
pub const STACK_AUTO_DEFAULT: Stack = Stack::Auto { gap_percent: 10.0 };

/// The shift of the waterfall template: 5 % of the front frame to the right and 10 % up per
/// curve. These are Sheaf's own values.
pub const WATERFALL_DEFAULT: Stack = Stack::Waterfall { dx_percent: 5.0, dy_percent: 10.0 };

fn stacked_lines(id: WindowId, name: &str) -> Graph {
    let mut g = general(id, name, PlotKind::Line);
    let layer = &mut g.layers[0];
    layer.stack = STACK_AUTO_DEFAULT;
    // The Y values on the axis are true only for the first curve, so the Y axis shows neither
    // tick labels nor ticks; a closed box frames the curves.
    let y = &mut layer.y_axis;
    y.show_tick_labels = false;
    y.major_tick = TickDirection::None;
    y.minor_tick = TickDirection::None;
    for axis in [&mut layer.x_axis, &mut layer.y_axis] {
        axis.show_opposite = true;
        axis.opposite_ticks = false;
    }
    g
}

/// The scatter template with both axes through the middle of the layer (measured in Origin 2016: axis
/// position -50 % on X and Y) and the frame drawn as a box round the layer.
fn scatter_central(id: WindowId, name: &str) -> Graph {
    let mut g = general(id, name, PlotKind::Scatter);
    let layer = &mut g.layers[0];
    for axis in [&mut layer.x_axis, &mut layer.y_axis] {
        axis.offset_pct = -50.0;
        axis.show_opposite = true;
        axis.opposite_ticks = false;
    }
    g
}

fn waterfall(id: WindowId, name: &str) -> Graph {
    let mut g = general(id, name, PlotKind::Line);
    g.layers[0].stack = WATERFALL_DEFAULT;
    g
}

/// The column template: the line template's page, fonts, legend and colors, with bars outlined in
/// black 0.5 pt (Origin's default outline) and the owner's house style for the axes: Y ticks
/// pointing in and a closed box. The X axis has no ticks, as in the owner's column figures: a tick
/// pointing in would cross the middle bar of each group.
fn column(id: WindowId, name: &str, kind: PlotKind) -> Graph {
    let mut g = general(id, name, kind);
    let layer = &mut g.layers[0];
    layer.default_line = LineStyle::solid(Color::BLACK, 0.5);
    layer.default_symbol.color = Color::BLACK;
    layer.default_symbol.filled = true;
    for axis in [&mut layer.x_axis, &mut layer.y_axis] {
        axis.show_opposite = true;
        axis.opposite_ticks = false;
    }
    let x = &mut layer.x_axis;
    x.major_tick = TickDirection::None;
    x.minor_tick = TickDirection::None;
    x.minor_ticks = 0;
    let y = &mut layer.y_axis;
    y.major_tick = TickDirection::In;
    y.minor_tick = TickDirection::In;
    y.minor_ticks = 1;
    if kind.is_horizontal() {
        // Lying bars: the value axis is the horizontal one.
        std::mem::swap(&mut layer.x_axis.major_tick, &mut layer.y_axis.major_tick);
        std::mem::swap(&mut layer.x_axis.minor_tick, &mut layer.y_axis.minor_tick);
        std::mem::swap(&mut layer.x_axis.minor_ticks, &mut layer.y_axis.minor_ticks);
    }
    g
}

/// Builds the layer that gives layer `parent_index` (whose content is `parent`) a right Y axis:
/// a layer linked to it with the same X scale, its own Y scale, and the parent's style.
///
/// The new layer is a copy of the parent without plots and free labels. Its frame covers the
/// parent's frame, its X axis is not drawn, and its Y axis is drawn on the right with automatic
/// limits and step and a title built from its own first plot. Its legend is hidden: the parent's
/// legend lists the plots of both layers (see [`crate::project::Project::add_right_y_layer`]).
/// Its palette starts at the parent's third color (for a gradient: runs the other way), so the
/// first right curve differs from the first left one.
pub fn right_y_layer(parent: &Layer, parent_index: usize) -> Layer {
    let mut l = parent.clone();
    l.display.hidden = false;
    l.plots.clear();
    l.labels.clear();
    l.shapes.clear();
    l.slopes.clear();
    l.straight_lines.clear();
    l.scales.clear();
    l.brackets.clear();
    l.comparison = None;
    l.plot_offsets.clear();
    // The right axis has curves of its own: they are not stacked, and its values are shown even
    // when the stacked parent hides its own (offset) Y values.
    if l.stack != Stack::None {
        l.stack = Stack::None;
        l.y_axis.show_tick_labels = true;
    }
    l.link = Some(LayerLink { parent: parent_index, x: ScaleLink::Straight, y: ScaleLink::None });
    // The parent's background and border already cover the frame; copying them would hide its plots.
    l.background = None;
    l.background_style = BackgroundStyle::default();
    l.border = Default::default();
    l.frame = Frame { left: 0.0, top: 0.0, width: 1.0, height: 1.0 };
    l.x_axis.side = AxisSide::Hidden;
    l.x_axis.opposite_offset_pct = None;
    l.x_axis.major_grid = None;
    l.x_axis.minor_grid = None;
    let y = &mut l.y_axis;
    y.side = AxisSide::Opposite;
    y.min = None;
    y.max = None;
    y.major_step = None;
    y.title = None;
    y.tick_label_divisor = 1.0;
    y.show_opposite = false;
    y.opposite_ticks = false;
    y.opposite_offset_pct = None;
    y.major_grid = None;
    y.minor_grid = None;
    l.legend.visible = false;
    l.legend.scope = LegendScope::Layer;
    let colors = &mut l.palette.colors;
    match l.palette.mode {
        PaletteMode::Cycle => {
            if !colors.is_empty() {
                let start = if colors.len() >= 3 { 2 } else { colors.len() - 1 };
                colors.rotate_left(start);
            }
        }
        PaletteMode::Spread => colors.reverse(),
        PaletteMode::Same => {}
    }
    l
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_and_spline_templates_are_line_plots_with_their_connection() {
        for (t, connect) in [
            (Template::StepHorizontal, Connect::StepHorizontal),
            (Template::StepVertical, Connect::StepVertical),
            (Template::SplineConnected, Connect::Spline),
        ] {
            let g = t.new_graph(WindowId(1), "Graph1");
            let l = &g.layers[0];
            assert_eq!((l.default_kind, l.default_line.connect), (PlotKind::Line, connect), "{t:?}");
            assert_eq!(Template::from_name(t.name()), Some(t));
        }
        let line = Template::Line.new_graph(WindowId(1), "Graph1");
        assert_eq!(line.layers[0].default_line.connect, Connect::Straight);
    }

    #[test]
    fn cv_template_matches_the_requested_style() {
        let g = Template::CyclicVoltammetry.new_graph(WindowId(1), "Graph1");
        let l = &g.layers[0];
        assert_eq!(l.default_kind, PlotKind::Line, "lines without markers");
        assert_eq!(l.default_line.connect, Connect::Spline, "smooth lines");
        assert!(l.x_axis.major_grid.is_none() && l.y_axis.major_grid.is_none(), "no gridlines");
        assert_eq!(l.x_axis.major_tick, TickDirection::In);
        assert_eq!(l.y_axis.major_tick, TickDirection::In);
        assert!(l.x_axis.show_opposite && l.y_axis.show_opposite, "closed box");
        assert_eq!(l.x_axis.color, Color::BLACK);
        assert_eq!(l.y_axis.auto_range, AutoRange::SymmetricZero);
    }

    /// Origin 2016's Column Scatter template (T79).
    #[test]
    fn column_scatter_template_makes_filled_diamonds_without_a_line() {
        use crate::{ColumnData, Designation, Project};
        let mut p = Project::new();
        let book = p.add_workbook("T");
        let y1 = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![3.0, 5.0, 2.0])).unwrap();
        let y2 = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![2.0, 1.0, 4.0])).unwrap();
        let g = p.plot_columns(Template::ColumnScatter, &[y1, y2]).unwrap();
        let layer = &p.graph(g).unwrap().layers[0];
        assert!(layer.plots.iter().all(|q| q.kind == PlotKind::ColumnScatter));
        assert!(!PlotKind::ColumnScatter.has_line() && PlotKind::ColumnScatter.has_symbols());
        let s = &layer.plots[0].symbol;
        assert_eq!((s.shape, s.size_pt, s.filled, s.color), (SymbolShape::Diamond, 8.0, true, Color::BLACK));
        assert_ne!(layer.plots[0].symbol.color, layer.plots[1].symbol.color, "the palette steps");
        assert_eq!(Template::ColumnScatter.label(), "Column Scatter");
    }

    #[test]
    fn template_names_round_trip() {
        for t in Template::ALL {
            assert_eq!(Template::from_name(t.name()), Some(t));
        }
        assert_eq!(Template::from_name("CV"), Some(Template::CyclicVoltammetry));
        assert_eq!(Template::from_name("double-y"), Some(Template::DoubleY));
        assert_eq!(Template::from_name("stacked"), Some(Template::StackedLines));
        assert_eq!(Template::from_name("Waterfall"), Some(Template::Waterfall));
        assert_eq!(Template::from_name("column"), Some(Template::Column));
        assert_eq!(Template::from_name("nope"), None);
        assert_eq!(Template::from_name("area"), Some(Template::Area));
        assert_eq!(Template::from_name("stacked-column-100"), Some(Template::Stacked100Column));
        assert_eq!(Template::from_name("bar"), Some(Template::Bar));
        assert_eq!(Template::from_name("floating-column"), Some(Template::FloatingColumn));
        assert_eq!(Template::from_name("pie-3d"), Some(Template::Pie3D));
        assert_eq!(Template::from_name("pie-bw"), Some(Template::PieBw));
        assert_eq!(Template::ALL.len(), 105);
        assert_eq!(Template::from_name("network"), Some(Template::Network));
        assert_eq!(Template::from_name("alluvial"), Some(Template::Alluvial));
        assert_eq!(Template::from_name("sankey"), Some(Template::Sankey));
        assert_eq!(Template::from_name("fishbone"), Some(Template::Fishbone));
        assert_eq!(Template::from_name("piper"), Some(Template::Piper));
        assert_eq!(Template::from_name("kde-2d"), Some(Template::Kde2d));
        assert_eq!(Template::from_name("smith-chart"), Some(Template::SmithChart));
        assert_eq!(Template::from_name("wind-rose-raw"), Some(Template::WindRoseRaw));
        assert_eq!(Template::from_name("zoom"), Some(Template::Zoom));
        assert_eq!(Template::from_name("qc-xbar-r"), Some(Template::QcXbarR));
        assert_eq!(Template::from_name("scatter-matrix"), Some(Template::ScatterMatrix));
        assert_eq!(Template::from_name("marginal-histograms"), Some(Template::MarginalHistograms));
        assert_eq!(Template::from_name("marginal-box-charts"), Some(Template::MarginalBoxCharts));
        assert_eq!(Template::from_name("ohlc-volume"), Some(Template::OhlcVolume));
        assert_eq!(Template::from_name("stock-line"), Some(Template::StockLine));
        assert_eq!(Template::from_name("candlestick"), Some(Template::Candlestick));
        assert_eq!(Template::from_name("vector-xyam"), Some(Template::VectorXyam));
        assert_eq!(Template::from_name("vector-xyxy"), Some(Template::VectorXyxy));
        assert_eq!(Template::from_name("column-scatter"), Some(Template::ColumnScatter));
        assert_eq!(Template::from_name("scatter-central"), Some(Template::ScatterCentral));
        assert_eq!(Template::from_name("lollipop"), Some(Template::Lollipop));
        assert_eq!(Template::from_name("violin"), Some(Template::Violin));
        assert_eq!(Template::from_name("violin-box"), Some(Template::ViolinBox));
        assert_eq!(Template::from_name("half-violin"), Some(Template::HalfViolin));
    }

    #[test]
    fn column_template_draws_outlined_bars_in_a_closed_box_with_y_ticks_in() {
        let g = Template::Column.new_graph(WindowId(1), "Graph1");
        assert_eq!(g.layers.len(), 1);
        let l = &g.layers[0];
        assert_eq!(l.default_kind, PlotKind::Column);
        assert_eq!(l.default_line, LineStyle::solid(Color::BLACK, 0.5), "a thin black outline");
        assert!(l.default_symbol.filled);
        assert_eq!(l.default_symbol.color, Color::BLACK);
        for axis in [&l.x_axis, &l.y_axis] {
            assert!(axis.show_opposite && !axis.opposite_ticks, "closed box");
            assert!(axis.major_grid.is_none() && axis.minor_grid.is_none(), "no grid");
            assert_eq!(axis.tick_label_source, TickLabelSource::Auto);
            assert!(axis.show_tick_labels);
        }
        assert_eq!((l.y_axis.major_tick, l.y_axis.minor_tick), (TickDirection::In, TickDirection::In));
        assert_eq!((l.x_axis.major_tick, l.x_axis.minor_tick), (TickDirection::None, TickDirection::None));
        assert_eq!((l.x_axis.minor_ticks, l.y_axis.minor_ticks), (0, 1));
        assert_eq!(l.bars, BarSpacing { gap_pct: 20.0, overlap_pct: 0.0, width_pct: None });
        // Page, fonts and legend are those of the line template; the colours start at red (measured).
        let line = Template::Line.new_graph(WindowId(1), "Graph1");
        assert_eq!(g.page, line.page);
        assert_eq!(l.legend, line.layers[0].legend);
        assert_eq!(l.palette.colors[0], Color::rgb(255, 0, 0));
        assert_eq!(l.palette.colors.len(), line.layers[0].palette.colors.len());
        assert_eq!(l.x_axis.tick_font, line.layers[0].x_axis.tick_font);
        // No template writes the new keys, so their files look as before.
        for t in Template::ALL {
            let json = serde_json::to_string(&t.new_graph(WindowId(1), "G")).unwrap();
            for key in ["tick_label_source", "tick_label_angle_deg", "\"bars\""] {
                assert!(!json.contains(key), "{t:?}: {key}");
            }
        }
    }

    #[test]
    fn stacked_template_stacks_with_a_10_percent_gap_in_a_closed_box() {
        let g = Template::StackedLines.new_graph(WindowId(1), "Graph1");
        assert_eq!(g.layers.len(), 1);
        let l = &g.layers[0];
        assert_eq!(l.stack, Stack::Auto { gap_percent: 10.0 });
        assert!(!l.y_axis.show_tick_labels);
        assert_eq!((l.y_axis.major_tick, l.y_axis.minor_tick), (TickDirection::None, TickDirection::None));
        assert!(l.x_axis.show_opposite && l.y_axis.show_opposite, "closed box");
        assert!(!l.x_axis.opposite_ticks && !l.y_axis.opposite_ticks);
        // The X axis and legend styling follow Line; this unmeasured template keeps its
        // existing corner placement rather than adopting the four stock templates' anchor.
        let mut line = Template::Line.new_graph(WindowId(1), "Graph1");
        line.layers[0].legend.position = LegendPosition::TopRight;
        line.layers[0].legend.background = None;
        assert_eq!(l.x_axis.major_tick, line.layers[0].x_axis.major_tick);
        assert!(l.x_axis.show_tick_labels);
        assert_eq!(l.legend, line.layers[0].legend);
        assert_eq!(l.default_kind, PlotKind::Line);
    }

    #[test]
    fn waterfall_template_is_the_line_template_with_a_waterfall() {
        let g = Template::Waterfall.new_graph(WindowId(1), "Graph1");
        let mut line = Template::Line.new_graph(WindowId(1), "Graph1");
        assert_eq!(g.layers[0].stack, Stack::Waterfall { dx_percent: 5.0, dy_percent: 10.0 });
        line.layers[0].stack = g.layers[0].stack;
        // Waterfall's separate placement has not been measured.
        line.layers[0].legend.position = LegendPosition::TopRight;
        line.layers[0].legend.background = None;
        assert_eq!(g, line);
        // Every other template has no offsets.
        for t in
            [Template::Line, Template::Scatter, Template::LineSymbol, Template::CyclicVoltammetry, Template::DoubleY]
        {
            let g = t.new_graph(WindowId(1), "G");
            assert!(g.layers.iter().all(|l| l.stack.is_none() && l.plot_offsets.is_empty()));
            // ... and their files are written as before.
            let json = serde_json::to_string(&g).unwrap();
            assert!(!json.contains("\"stack\"") && !json.contains("plot_offsets"), "{json}");
        }
    }

    #[test]
    fn double_y_template_has_a_right_layer_linked_to_the_first() {
        let g = Template::DoubleY.new_graph(WindowId(1), "Graph1");
        assert_eq!(g.layers.len(), 2);
        let (left, right) = (&g.layers[0], &g.layers[1]);
        assert_eq!(right.link, Some(LayerLink { parent: 0, x: ScaleLink::Straight, y: ScaleLink::None }));
        assert_eq!(g.link_parent(1), Some(0));
        assert_eq!(right.frame, Frame { left: 0.0, top: 0.0, width: 1.0, height: 1.0 });
        assert_eq!(right.x_axis.side, AxisSide::Hidden);
        assert_eq!(right.y_axis.side, AxisSide::Opposite);
        assert!(!right.legend.visible);
        assert_eq!(left.legend.scope, LegendScope::Graph);
        assert!(!left.y_axis.show_opposite && left.x_axis.show_opposite && left.x_axis.opposite_ticks);
        for y in [&left.y_axis, &right.y_axis] {
            assert!(y.color_follows_plot && !y.title_color_follows_plot);
        }
        assert!(!left.x_axis.color_follows_plot);
        // The first left curve is black and the first right curve takes the third colour of the list
        // (green), so that the two differ.
        assert_eq!(left.palette.colors[0], Color::rgb(0, 0, 0));
        assert_eq!(right.palette.colors[0], Color::rgb(0, 255, 0));
        assert_eq!(left.x_axis.major_tick, TickDirection::Out);
    }

    #[test]
    fn right_y_layer_has_no_slope_annotations() {
        let mut g = Template::Scatter.new_graph(WindowId(1), "g");
        g.layers[0].slopes.push(crate::graph::SlopeAnnotation::new(crate::ColumnId(1), 0, 1));
        assert!(right_y_layer(&g.layers[0], 0).slopes.is_empty());
    }

    #[test]
    fn right_y_layer_copies_the_parent_style_but_not_its_limits() {
        let mut g = Template::CyclicVoltammetry.new_graph(WindowId(1), "Graph1");
        let parent = &mut g.layers[0];
        parent.y_axis.min = Some(-3.0);
        parent.y_axis.title = Some("j".into());
        parent.labels.push(TextLabel {
            text: "note".into(),
            x: 0.1,
            y: 0.1,
            font: Font::new(DEFAULT_FONT, 9.0),
            rotation_deg: 0.0,
            table: false,
            group: None,
        });
        g.layers[0].plot_offset_mut(crate::ids::ColumnId(4)).y = 3.0;
        let r = right_y_layer(&g.layers[0], 0);
        assert!(r.plot_offsets.is_empty(), "offsets belong to the parent's curves");
        assert_eq!(r.y_axis.major_tick, TickDirection::In);
        assert_eq!(r.y_axis.auto_range, AutoRange::SymmetricZero);
        assert_eq!(r.default_line, g.layers[0].default_line);
        assert_eq!((r.y_axis.min, r.y_axis.title.clone()), (None, None));
        assert!(r.labels.is_empty() && r.plots.is_empty());
        assert!(!r.y_axis.show_opposite && !r.y_axis.opposite_ticks);
        // A gradient runs the other way on the right.
        let mut reversed = g.layers[0].palette.colors.clone();
        reversed.reverse();
        assert_eq!(r.palette.colors, reversed);
        // Short color lists start at their last color.
        let mut two = g.layers[0].clone();
        two.palette = Palette { colors: vec![Color::BLACK, Color::RED], mode: PaletteMode::Cycle, list: None };
        assert_eq!(right_y_layer(&two, 0).palette.colors, vec![Color::RED, Color::BLACK]);
        two.palette.colors.clear();
        assert!(right_y_layer(&two, 0).palette.colors.is_empty());
        // The right layer of a stacked graph is not stacked and shows its values.
        let mut stacked = g.layers[0].clone();
        stacked.stack = Stack::Constant { step: 2.0 };
        stacked.y_axis.show_tick_labels = false;
        let r = right_y_layer(&stacked, 0);
        assert_eq!(r.stack, Stack::None);
        assert!(r.y_axis.show_tick_labels);
        // The parent's background covers the frame; the right layer draws none of its own.
        let mut coloured = g.layers[0].clone();
        coloured.background = Some(Color::rgb(240, 240, 255));
        coloured.background_style.border = Some(BorderStyle::solid(Color::BLACK, 1.0));
        let r = right_y_layer(&coloured, 0);
        assert_eq!((r.background, r.background_style), (None, BackgroundStyle::default()));
    }

    #[test]
    fn floating_column_starts_its_colours_at_blue_as_origin() {
        let g = Template::FloatingColumn.new_graph(WindowId(0), "");
        assert_eq!(g.layers[0].palette.colors[0], Color::rgb(0, 0, 255));
        let c = Template::Column.new_graph(WindowId(0), "");
        assert_eq!(c.layers[0].palette.colors[0], Color::rgb(255, 0, 0), "plain columns still start at red");
    }

    /// Measured on the legends of 12 and 13 plots in Origin (spec 14.12).
    #[test]
    fn horizontal_stacked_and_floating_bars_use_the_colour_lists_of_their_vertical_forms() {
        let colors = |t: Template| t.new_graph(WindowId(0), "").layers[0].palette.colors.clone();
        let floating: Vec<Color> = [
            (0, 0, 255),
            (0, 255, 255),
            (255, 0, 255),
            (255, 255, 0),
            (128, 128, 0),
            (0, 0, 128),
            (128, 0, 128),
            (128, 0, 0),
            (0, 128, 0),
            (0, 128, 128),
            (0, 0, 160),
            (255, 128, 0),
        ]
        .map(|(r, g, b)| Color::rgb(r, g, b))
        .to_vec();
        for t in [Template::FloatingColumn, Template::FloatingBar] {
            assert_eq!(colors(t)[..12], floating[..], "{t:?}");
        }
        for t in [Template::StackedBar, Template::Stacked100Bar] {
            assert_eq!(colors(t), colors(Template::StackedColumn), "{t:?}");
            assert_eq!(colors(t)[1], Color::rgb(255, 0, 0));
            assert_eq!(colors(t)[2], Color::rgb(0, 0, 255), "black, red, blue, magenta, green, ...");
        }
    }

    #[test]
    fn lollipop_needs_two_y_columns_and_exchanges_x_and_y() {
        use crate::graph::DropTarget;
        use crate::{ColumnData, Designation, Project};
        let mut p = Project::new();
        let b = p.add_workbook("d");
        let x = p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
        let y1 = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![3.0, 4.0])).unwrap();
        let y2 = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![5.0, 6.0])).unwrap();
        let err = p.plot_columns(Template::Lollipop, &[y1]).unwrap_err();
        assert!(err.to_string().contains("two or more Y columns"), "{err}");
        assert!(p.plot_columns(Template::Lollipop, &[]).is_err());
        assert_eq!(p.graphs().count(), 0, "a refused plot leaves no graph behind");
        let g = p.plot_columns(Template::Lollipop, &[y1, y2]).unwrap();
        let l = &p.graph(g).unwrap().layers[0];
        assert_eq!(l.default_kind, PlotKind::Scatter);
        assert_eq!(l.plots.len(), 2);
        for (plot, y) in l.plots.iter().zip([y1, y2]) {
            // Exchanged: the Y column runs along X and the X column up the Y axis.
            assert_eq!((plot.x, plot.y), (Some(y), x));
            assert!(plot.decor.drop.vertical.on && !plot.decor.drop.horizontal.on);
            assert_eq!(plot.decor.drop.vertical.target, DropTarget::NextDataPlot);
        }
        assert_eq!(p.plot_label(&l.plots[0]), p.column(y1).unwrap().display_name());
        assert_eq!(Template::Lollipop.label(), "Lollipop Plot");
    }

    /// Origin 2016's DROPLINE and ColumnLabel templates (graphing 18.12).
    #[test]
    fn vertical_drop_line_and_column_label_templates() {
        use crate::{ColumnData, Designation, Project};
        let mut p = Project::new();
        let book = p.add_workbook("T");
        p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0, 3.0])).unwrap();
        let y1 = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![3.0, 5.0, 2.0])).unwrap();
        let y2 = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![2.0, 1.0, 4.0])).unwrap();
        let g = p.plot_columns(Template::VerticalDropLine, &[y1, y2]).unwrap();
        let layer = &p.graph(g).unwrap().layers[0];
        assert!(layer.plots.iter().all(|q| q.kind == PlotKind::Scatter && q.decor.drop.vertical.on));
        let drop = &layer.plots[0].decor.drop.vertical;
        assert_eq!(
            (drop.width_pt, drop.color, drop.target),
            (0.5, None, DropTarget::Axis),
            "0.5 pt in the plot's colour"
        );
        assert!(!layer.plots[0].decor.drop.horizontal.on);
        assert_ne!(layer.plots[0].symbol.shape, layer.plots[1].symbol.shape, "square, then circle");
        let g = p.plot_columns(Template::ColumnLabel, &[y1, y2]).unwrap();
        let layer = &p.graph(g).unwrap().layers[0];
        assert!(layer.plots.iter().all(|q| q.kind == PlotKind::Column && q.decor.labels.enable));
        assert_eq!(layer.plots[0].decor.labels.position, LabelPosition::Above);
        assert_eq!(layer.plots[0].decor.labels.font.size_pt, 22.0);
        assert_eq!(Template::from_name("vertical-drop-line"), Some(Template::VerticalDropLine));
        assert_eq!(Template::from_name("column-label"), Some(Template::ColumnLabel));
        assert_eq!(Template::ColumnLabel.label(), "Column + Label");
    }

    #[test]
    fn the_violin_templates_are_box_chart_templates_with_their_own_kinds() {
        use crate::graph::ViolinInner;
        let cases = [
            (Template::Violin, "violin", "Violin Plot", PlotKind::Violin, ViolinInner::Median),
            (Template::ViolinBox, "violin-box", "Violin with Box", PlotKind::ViolinBox, ViolinInner::Box),
            (Template::ViolinPoint, "violin-point", "Violin with Point", PlotKind::ViolinPoint, ViolinInner::Point),
            (
                Template::ViolinQuartile,
                "violin-quartile",
                "Violin with Quartile",
                PlotKind::ViolinQuartile,
                ViolinInner::Quartile,
            ),
            (Template::ViolinStick, "violin-stick", "Violin with Stick", PlotKind::ViolinStick, ViolinInner::Stick),
            (Template::HalfViolin, "half-violin", "Half Violin", PlotKind::HalfViolin, ViolinInner::Box),
        ];
        let boxed = Template::BoxChart.new_graph(WindowId(1), "G");
        for (t, name, label, kind, inner) in cases {
            assert_eq!((t.name(), t.label()), (name, label));
            assert_eq!(Template::from_name(name), Some(t));
            assert!(t.is_violin());
            let g = t.new_graph(WindowId(1), "G");
            let l = &g.layers[0];
            assert_eq!(l.default_kind, kind);
            assert_eq!(kind.violin_inner(), Some(inner));
            assert!(kind.is_violin() && kind.is_distribution());
            assert!(!kind.is_bar() && !kind.has_line() && !kind.has_symbols() && !kind.is_area());
            // Same page, axes and colours (starting at red) as the box chart.
            assert_eq!(l.palette, boxed.layers[0].palette);
            assert_eq!(
                (&l.x_axis, &l.y_axis, &g.page),
                (&boxed.layers[0].x_axis, &boxed.layers[0].y_axis, &boxed.page)
            );
        }
        assert!(!Template::BoxChart.is_violin() && PlotKind::Box.is_distribution() && !PlotKind::Box.is_violin());
        assert!(PlotKind::HalfViolin.is_half_violin() && !PlotKind::ViolinBox.is_half_violin());
    }

    #[test]
    fn violin_kinds_save_by_name_and_old_kinds_still_load() {
        for (kind, name) in [
            (PlotKind::Violin, "violin"),
            (PlotKind::ViolinBox, "violin_box"),
            (PlotKind::ViolinPoint, "violin_point"),
            (PlotKind::ViolinQuartile, "violin_quartile"),
            (PlotKind::ViolinStick, "violin_stick"),
            (PlotKind::HalfViolin, "half_violin"),
        ] {
            let json = serde_json::to_string(&kind).unwrap();
            assert_eq!(json, format!("\"{name}\""));
            assert_eq!(serde_json::from_str::<PlotKind>(&json).unwrap(), kind);
        }
        // A graph saved before the violin kinds existed loads unchanged, and a box chart stays a box chart.
        let g = Template::BoxChart.new_graph(WindowId(1), "G");
        let json = serde_json::to_string(&g).unwrap();
        assert!(json.contains("\"default_kind\":\"box\""));
        let back: crate::graph::Graph = serde_json::from_str(&json).unwrap();
        assert_eq!(back, g);
    }

    #[test]
    fn the_ridgeline_template_is_a_box_chart_template_with_its_own_kind() {
        use crate::graph::{RidgeBandwidth, RidgeDisplay, RidgelineStyle};
        assert_eq!(Template::from_name("ridgeline"), Some(Template::Ridgeline));
        assert_eq!(Template::Ridgeline.label(), "Ridgeline Chart");
        let g = Template::Ridgeline.new_graph(WindowId(1), "G");
        assert_eq!(g.layers[0].default_kind, PlotKind::Ridgeline);
        assert!(PlotKind::Ridgeline.is_ridgeline() && PlotKind::Ridgeline.is_distribution());
        assert!(!PlotKind::Ridgeline.is_violin() && !Template::Ridgeline.is_violin());
        assert_eq!(serde_json::to_string(&PlotKind::Ridgeline).unwrap(), "\"ridgeline\"");
        let back: crate::graph::Graph = serde_json::from_str(&serde_json::to_string(&g).unwrap()).unwrap();
        assert_eq!(back, g);
        // Defaults: Scott, curve, no quantile lines; a plot saved without settings loads with them.
        let d = RidgelineStyle::default();
        assert_eq!(
            (d.display, d.bandwidth, d.quantile_lines),
            (RidgeDisplay::DistributionCurve, RidgeBandwidth::Scott, false)
        );
        assert_eq!((d.height, d.fill_transparency_pct, d.quantiles.clone()), (1.5, 30.0, vec![25.0, 50.0, 75.0]));
        let partial: RidgelineStyle =
            serde_json::from_str(r#"{"display":"with_rug","bandwidth":{"k_sd":0.5}}"#).unwrap();
        assert_eq!(partial.display, RidgeDisplay::WithRug);
        assert_eq!(partial.bandwidth, RidgeBandwidth::KSd(0.5));
        assert_eq!(partial.height, 1.5);
        let all: RidgelineStyle = serde_json::from_str("{}").unwrap();
        assert!(all.is_default());
    }

    #[test]
    fn the_hexbin_template_has_its_own_kind_and_a_hidden_legend() {
        use crate::graph::{HexbinBy, HexbinStyle};
        assert_eq!(Template::from_name("hexbin"), Some(Template::Hexbin));
        assert_eq!(Template::Hexbin.label(), "Hexbin");
        let g = Template::Hexbin.new_graph(WindowId(1), "G");
        assert_eq!(g.layers[0].default_kind, PlotKind::Hexbin);
        assert!(!g.layers[0].legend.visible);
        assert!(PlotKind::Hexbin.is_hexbin() && !PlotKind::Hexbin.is_distribution() && !PlotKind::Hexbin.is_bar());
        assert_eq!(serde_json::to_string(&PlotKind::Hexbin).unwrap(), "\"hexbin\"");
        let back: crate::graph::Graph = serde_json::from_str(&serde_json::to_string(&g).unwrap()).unwrap();
        assert_eq!(back, g);
        // Defaults: 20 bins along each axis, data range, regular hexagons, Y derived from X.
        let d = HexbinStyle::default();
        assert_eq!((d.x.by, d.y.by), (HexbinBy::Bins(20), HexbinBy::Bins(20)));
        assert_eq!((d.x.min_center, d.x.max_center, d.prefer_regular, d.derive_y_from_x), (None, None, true, true));
        // A partial style (older file, or one that sets a single field) fills in the rest.
        let partial: HexbinStyle =
            serde_json::from_str(r#"{"x":{"by":{"spacing":2.5},"min_center":0.0},"derive_y_from_x":false}"#).unwrap();
        assert_eq!(partial.x.by, HexbinBy::Spacing(2.5));
        assert_eq!((partial.x.min_center, partial.x.max_center), (Some(0.0), None));
        assert!(!partial.derive_y_from_x && partial.prefer_regular);
        assert_eq!(partial.y, d.y);
        assert!(serde_json::from_str::<HexbinStyle>("{}").unwrap().is_default());
    }

    #[test]
    fn the_parallel_templates_have_their_own_kinds_and_refuse_too_few_columns() {
        use crate::graph::{LegendPosition, ParallelStyle};
        use crate::{ColumnData, Designation, Project};
        assert_eq!(Template::from_name("parallel"), Some(Template::Parallel));
        assert_eq!(Template::from_name("parallel-index"), Some(Template::ParallelIndex));
        assert_eq!((Template::Parallel.label(), Template::ParallelIndex.label()), ("Parallel Plot", "Parallel Index"));
        assert!(
            Template::Parallel.is_parallel()
                && Template::ParallelIndex.is_parallel()
                && !Template::Hexbin.is_parallel()
        );
        assert!(
            PlotKind::Parallel.is_parallel() && PlotKind::ParallelIndex.is_parallel() && !PlotKind::Line.is_parallel()
        );
        assert_eq!(serde_json::to_string(&PlotKind::ParallelIndex).unwrap(), "\"parallel_index\"");
        let plain = Template::Parallel.new_graph(WindowId(1), "G");
        assert_eq!(plain.layers[0].default_kind, PlotKind::Parallel);
        assert!(!plain.layers[0].legend.visible, "the colour scale names the colours");
        let grouped = Template::ParallelIndex.new_graph(WindowId(1), "G");
        assert!(grouped.layers[0].legend.visible && grouped.layers[0].legend.position == LegendPosition::Automatic);
        for g in [plain, grouped] {
            let back: Graph = serde_json::from_str(&serde_json::to_string(&g).unwrap()).unwrap();
            assert_eq!(back, g);
        }
        // The defaults: rescale each axis, 0.5 pt, opaque, no colour column; a partial style fills in the rest.
        let d = ParallelStyle::default();
        assert_eq!((d.rescale_each_axis, d.width_pt, d.transparency_pct, d.color), (true, 0.5, 0.0, None));
        let partial: ParallelStyle = serde_json::from_str(r#"{"rescale_each_axis":false}"#).unwrap();
        assert!(!partial.rescale_each_axis && partial.width_pt == 0.5);
        assert!(serde_json::from_str::<ParallelStyle>("{}").unwrap().is_default());

        let mut p = Project::new();
        let b = p.add_workbook("d");
        let ys: Vec<_> = (0..4)
            .map(|i| p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![i as f64, 2.0])).unwrap())
            .collect();
        let species = p.add_column(b, 0, Designation::Y, ColumnData::text(vec!["a".into(), "b".into()])).unwrap();
        let err = p.plot_columns(Template::Parallel, &ys[..1]).unwrap_err();
        assert!(err.to_string().contains("two or more numeric columns"), "{err}");
        let err = p.plot_columns(Template::ParallelIndex, &[ys[0], ys[1], species]).unwrap_err();
        assert!(err.to_string().contains("three or more numeric columns"), "{err}");
        let err = p.plot_parallel(Template::ParallelIndex, &ys[..3], None).unwrap_err();
        assert!(err.to_string().contains("group column"), "{err}");
        assert_eq!(p.graphs().count(), 0, "a refused plot leaves no graph behind");
        let err = p.plot_columns_double_y(Template::Parallel, &ys[..2], &ys[2..]).unwrap_err();
        assert!(err.to_string().contains("right Y axis"), "{err}");
        // The text column among the columns is the group; one axis per numeric column.
        let g = p.plot_columns(Template::ParallelIndex, &[ys[0], ys[1], ys[2], species]).unwrap();
        let l = &p.graph(g).unwrap().layers[0];
        assert_eq!(l.plots.len(), 3);
        assert!(l.plots.iter().all(|pl| pl.kind == PlotKind::ParallelIndex && pl.parallel.color == Some(species)));
        // Without a named group the first text column of the sheet groups the rows.
        assert_eq!(p.parallel_group_default(&ys), Some(species));
        let g2 = p.plot_columns(Template::ParallelIndex, &ys[..3]).unwrap();
        assert_eq!(p.graph(g2).unwrap().layers[0].plots[0].parallel.color, Some(species));
        // A parallel plot colours by its last column by default.
        let g3 = p.plot_columns(Template::Parallel, &ys).unwrap();
        assert_eq!(p.graph(g3).unwrap().layers[0].plots[2].parallel.color, Some(ys[3]));
    }

    #[test]
    fn the_treemap_template_has_its_own_kind_settings_and_refusals() {
        use crate::graph::{TreemapLayout, TreemapLevel, TreemapOrder, TreemapStyle};
        use crate::{ColumnData, Designation, Project};
        assert_eq!(Template::from_name("treemap"), Some(Template::Treemap));
        assert_eq!(Template::Treemap.label(), "Treemap");
        assert_eq!(serde_json::to_string(&PlotKind::Treemap).unwrap(), "\"treemap\"");
        let g = Template::Treemap.new_graph(WindowId(1), "G");
        assert_eq!(g.layers[0].default_kind, PlotKind::Treemap);
        assert!(!g.layers[0].legend.visible);
        assert_eq!(g.layers[0].x_axis.side, crate::graph::AxisSide::Hidden);
        let back: Graph = serde_json::from_str(&serde_json::to_string(&g).unwrap()).unwrap();
        assert_eq!(back, g);
        // Defaults and partial styles.
        let d = TreemapStyle::default();
        assert_eq!(
            (d.layout, d.order, d.depth, d.gap_pt, d.per_level_on),
            (TreemapLayout::Squarified, TreemapOrder::Categorical, None, 1.5, false)
        );
        assert!(serde_json::from_str::<TreemapStyle>("{}").unwrap().is_default());
        let partial: TreemapStyle = serde_json::from_str(r#"{"layout":"pivot_split_size","depth":2}"#).unwrap();
        assert_eq!((partial.layout, partial.depth, partial.gap_pt), (TreemapLayout::PivotSplitSize, Some(2), 1.5));
        // Per-level settings apply to the levels below the first, only while the box is checked.
        let mut s = TreemapStyle {
            per_level: vec![TreemapLevel { layout: TreemapLayout::Split, order: TreemapOrder::BySize }],
            ..d
        };
        assert_eq!(s.level_setting(1).layout, TreemapLayout::Squarified);
        s.per_level_on = true;
        assert_eq!(s.level_setting(0).layout, TreemapLayout::Squarified);
        assert_eq!((s.level_setting(1).layout, s.level_setting(1).order), (TreemapLayout::Split, TreemapOrder::BySize));
        assert_eq!(s.level_setting(2).layout, TreemapLayout::Squarified, "no entry: the general setting");

        let mut p = Project::new();
        let b = p.add_workbook("d");
        let region =
            p.add_column(b, 0, Designation::Y, ColumnData::text(vec!["a".into(), "a".into(), "b".into()])).unwrap();
        let name =
            p.add_column(b, 0, Designation::Y, ColumnData::text(vec!["x".into(), "y".into(), "z".into()])).unwrap();
        let v = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0, 3.0])).unwrap();
        let w = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![4.0, 5.0, 6.0])).unwrap();
        // Text columns only: nested frequencies, every row counts 1 (T392).
        let g = p.plot_columns(Template::Treemap, &[region, name]).unwrap();
        let plot = &p.graph(g).unwrap().layers[0].plots[0];
        assert_eq!((plot.y, plot.treemap.categories.clone()), (region, vec![region, name]));
        let counted_graphs = p.graphs().count();
        let err = p.plot_treemap(&[], v).unwrap_err();
        assert!(err.to_string().contains("category column"), "{err}");
        let err = p.plot_treemap(&[v], region).unwrap_err();
        assert!(err.to_string().contains("numeric value column"), "{err}");
        let err = p.plot_columns_double_y(Template::Treemap, &[v], &[w]).unwrap_err();
        assert!(err.to_string().contains("right Y axis"), "{err}");
        assert_eq!(p.graphs().count(), counted_graphs, "a refused plot leaves no graph behind");
        // Text columns among the selection are the categories in selection order; the last numeric is the value.
        let g = p.plot_columns(Template::Treemap, &[name, region, v, w]).unwrap();
        let l = p.graph(g).unwrap().layers[0].clone();
        assert_eq!(l.plots.len(), 1);
        assert_eq!((l.plots[0].y, l.plots[0].kind), (w, PlotKind::Treemap));
        assert_eq!(l.plots[0].treemap.categories, vec![name, region]);
        // Without text among the columns, every text column of the sheet, in sheet order.
        let g2 = p.plot_columns(Template::Treemap, &[v]).unwrap();
        assert_eq!(p.graph(g2).unwrap().layers[0].plots[0].treemap.categories, vec![region, name]);
        assert_eq!(l.inherited_treemap().categories, vec![name, region]);
    }

    #[test]
    fn indexed_templates_split_one_data_column_by_group_columns() {
        use crate::graph::{Indexed, IndexedError};
        use crate::{ColumnData, Designation, Project};
        assert_eq!(Template::from_name("grouped-columns"), Some(Template::GroupedColumns));
        assert_eq!(Template::from_name("grouped-box"), Some(Template::GroupedBox));
        assert_eq!(Template::GroupedColumns.label(), "Grouped Columns (indexed data)");
        assert_eq!(Template::GroupedBox.label(), "Grouped Box Charts");
        assert_eq!(Template::GroupedColumns.new_graph(WindowId(1), "G").layers[0].default_kind, PlotKind::Column);
        assert_eq!(Template::GroupedBox.new_graph(WindowId(1), "G").layers[0].default_kind, PlotKind::Box);

        let text = |v: &[&str]| ColumnData::text(v.iter().map(|s| s.to_string()).collect());
        let mut p = Project::new();
        let b = p.add_workbook("d");
        let a = p.add_column(b, 0, Designation::X, text(&["a", "a", "b", "b"])).unwrap();
        let s = p.add_column(b, 0, Designation::X, text(&["u", "v", "u", "v"])).unwrap();
        let t = p.add_column(b, 0, Designation::X, text(&["1", "2", "3", "4"])).unwrap();
        let v = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0, 3.0, 4.0])).unwrap();
        let w = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![4.0, 5.0, 6.0, 7.0])).unwrap();

        // Errors: no data, no group, too many groups, two data columns, a text data column, a right axis.
        let err = p.plot_columns(Template::GroupedColumns, &[a, s]).unwrap_err();
        assert!(err.to_string().contains("one numeric data column"), "{err}");
        let err = p.plot_columns(Template::GroupedBox, &[v, w]).unwrap_err();
        assert!(err.to_string().contains("one numeric data column"), "two data columns: {err}");
        let err = p.plot_columns(Template::GroupedColumns, &[v]).unwrap_err();
        assert!(err.to_string().contains("group columns"), "{err}");
        let err = p.plot_indexed(Template::GroupedColumns, &[a, s, t], v).unwrap_err();
        assert!(err.to_string().contains("at most two"), "{err}");
        let err = p.plot_indexed(Template::GroupedColumns, &[a], s).unwrap_err();
        assert!(err.to_string().contains("numeric data column"), "{err}");
        let blank = p.add_column(b, 0, Designation::X, text(&["", " ", "", ""])).unwrap();
        let before = p.graphs().count();
        assert!(p.plot_indexed(Template::GroupedColumns, &[a, blank], v).is_err(), "no levels in the subgroup column");
        assert!(p.plot_indexed(Template::GroupedColumns, &[blank], v).is_err(), "no levels in the group column");
        assert_eq!(p.graphs().count(), before, "a refused plot leaves no graph behind");
        let err = p.plot_columns_double_y(Template::GroupedBox, &[v], &[w]).unwrap_err();
        assert!(err.to_string().contains("right Y axis"), "{err}");
        assert_eq!(p.graphs().count(), 0, "a refused plot leaves no graph behind");

        // One group column: one plot.
        let g = p.plot_columns(Template::GroupedColumns, &[a, v]).unwrap();
        let l = p.graph(g).unwrap().layers[0].clone();
        assert_eq!(l.plots.len(), 1);
        let pl = &l.plots[0];
        assert_eq!((pl.y, pl.x, pl.kind, pl.label.clone()), (v, None, PlotKind::Column, None));
        assert_eq!(pl.indexed, Some(Indexed { group: a, subset: None, error: IndexedError::None }));

        // Two group columns: one plot per level of the second, named after the level, with its colour.
        let g = p.plot_columns(Template::GroupedBox, &[a, s, v]).unwrap();
        let l = p.graph(g).unwrap().layers[0].clone();
        assert_eq!(l.plots.len(), 2);
        let labels: Vec<_> = l.plots.iter().map(|q| q.label.clone().unwrap()).collect();
        assert_eq!(labels, ["u", "v"]);
        assert!(l.plots.iter().all(|q| q.kind == PlotKind::Box && q.y == v));
        assert_eq!(l.plots[1].indexed.as_ref().unwrap().subset, Some((s, "v".to_string())));
        assert_ne!(l.plots[0].symbol.color, l.plots[1].symbol.color);

        // The .sheaf form keeps the grouping; a plot without it writes no key.
        let json = serde_json::to_string(&p).unwrap();
        let back: Project = serde_json::from_str(&json).unwrap();
        assert_eq!(back.graph(g).unwrap().layers[0].plots[1].indexed, l.plots[1].indexed);
        let plain = p.plot_columns(Template::Column, &[v]).unwrap();
        assert!(!serde_json::to_string(p.graph(plain).unwrap()).unwrap().contains("indexed"));

        // A grouped plot is not a template look.
        let err = crate::user_template::UserTemplate::from_graph("t", p.graph(g).unwrap()).unwrap_err();
        assert!(err.to_string().contains("indexed"), "{err}");
    }

    #[test]
    fn interval_and_pareto_templates_build_their_plots_and_refuse_a_wrong_selection() {
        use crate::graph::{IntervalError, StatPlot};
        use crate::{ColumnData, Designation, Project};
        assert_eq!(Template::from_name("interval-plot"), Some(Template::IntervalPlot));
        assert_eq!(Template::from_name("pareto"), Some(Template::Pareto));
        assert_eq!((Template::IntervalPlot.label(), Template::Pareto.label()), ("Interval Plot", "Pareto"));
        assert_eq!(Template::IntervalPlot.new_graph(WindowId(1), "G").layers[0].default_kind, PlotKind::Scatter);
        let sym = Template::IntervalPlot.new_graph(WindowId(1), "G").layers[0].default_symbol.clone();
        assert!((sym.shape, sym.filled) == (SymbolShape::Circle, true));
        assert_eq!(Template::Pareto.new_graph(WindowId(1), "G").layers[0].default_kind, PlotKind::Column);

        let mut p = Project::new();
        let b = p.add_workbook("d");
        let t = p
            .add_column(b, 0, Designation::X, ColumnData::text(["a", "b", "a", "", "a"].map(String::from).to_vec()))
            .unwrap();
        let v = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0, 3.0, 4.0, 5.0])).unwrap();
        let w = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![2.0, 4.0, 6.0, 8.0, 10.0])).unwrap();

        assert!(p.plot_columns(Template::IntervalPlot, &[t]).unwrap_err().to_string().contains("numeric data column"));
        assert!(p.plot_columns(Template::IntervalPlot, &[]).is_err());
        assert!(p.plot_columns(Template::Pareto, &[v, w]).unwrap_err().to_string().contains("exactly one column"));
        assert!(
            p.plot_columns_double_y(Template::IntervalPlot, &[v], &[w]).unwrap_err().to_string().contains("right Y")
        );
        assert!(p.plot_columns_double_y(Template::Pareto, &[v], &[w]).unwrap_err().to_string().contains("right Y"));
        assert_eq!(p.graphs().count(), 0, "a refused plot leaves no graph behind");

        let g = p.plot_columns(Template::IntervalPlot, &[v, w]).unwrap();
        let l = &p.graph(g).unwrap().layers[0];
        assert_eq!(l.plots.len(), 2);
        assert!(l.plots.iter().all(|q| q.kind == PlotKind::Scatter && q.x.is_none()));
        assert_eq!(l.plots[1].stat, Some(StatPlot::Interval { error: IntervalError::Ci, level: 95.0 }));

        // Pareto: left columns, right cumulative line of 0 - 100, on a text column.
        let g = p.plot_columns(Template::Pareto, &[t]).unwrap();
        let gr = p.graph(g).unwrap();
        assert_eq!(gr.layers.len(), 2);
        assert_eq!(gr.layers[0].plots[0].stat, Some(StatPlot::Pareto));
        assert_eq!(gr.layers[1].plots[0].stat, Some(StatPlot::ParetoCumulative));
        assert_eq!(gr.layers[1].plots[0].kind, PlotKind::LineSymbol);
        assert_eq!((gr.layers[1].y_axis.min, gr.layers[1].y_axis.max), (Some(0.0), Some(110.0)));
        let blank = p.add_column(b, 0, Designation::X, ColumnData::text(vec![String::new(); 5])).unwrap();
        assert!(p.plot_pareto(blank).is_err());

        // The .sheaf form round trips; a plain plot writes no "stat" key; no user template.
        let back: Project = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        assert_eq!(back.graph(g).unwrap().layers[1].plots[0].stat, Some(StatPlot::ParetoCumulative));
        let plain = p.plot_columns(Template::Column, &[v]).unwrap();
        assert!(!serde_json::to_string(p.graph(plain).unwrap()).unwrap().contains("\"stat\""));
        assert!(crate::user_template::UserTemplate::from_graph("t", p.graph(g).unwrap()).is_err());
    }

    #[test]
    fn the_sunburst_template_has_its_own_kind_settings_and_refusals() {
        use crate::graph::{SunburstOrder, SunburstStyle};
        use crate::{ColumnData, Designation, Project};
        assert_eq!(Template::from_name("sunburst"), Some(Template::Sunburst));
        assert_eq!(Template::Sunburst.label(), "Sunburst Plot");
        assert_eq!(serde_json::to_string(&PlotKind::Sunburst).unwrap(), "\"sunburst\"");
        let g = Template::Sunburst.new_graph(WindowId(1), "G");
        assert_eq!(g.layers[0].default_kind, PlotKind::Sunburst);
        assert!(!g.layers[0].legend.visible);
        assert_eq!(g.layers[0].x_axis.side, crate::graph::AxisSide::Hidden);
        let back: Graph = serde_json::from_str(&serde_json::to_string(&g).unwrap()).unwrap();
        assert_eq!(back, g);
        let d = SunburstStyle::default();
        assert_eq!(
            (d.start_angle_deg, d.hole, d.ring_gap_pt, d.border_pt, d.show_names, d.show_values, d.order),
            (90.0, 0.3, 0.0, 0.5, true, false, SunburstOrder::Worksheet)
        );
        assert!(serde_json::from_str::<SunburstStyle>("{}").unwrap().is_default());
        let partial: SunburstStyle = serde_json::from_str(r#"{"hole":0.5,"show_values":true}"#).unwrap();
        assert_eq!(
            (partial.hole, partial.start_angle_deg, partial.show_names, partial.show_values),
            (0.5, 90.0, true, true)
        );

        let mut p = Project::new();
        let b = p.add_workbook("d");
        let region =
            p.add_column(b, 0, Designation::Y, ColumnData::text(vec!["a".into(), "a".into(), "b".into()])).unwrap();
        let name =
            p.add_column(b, 0, Designation::Y, ColumnData::text(vec!["x".into(), "y".into(), "z".into()])).unwrap();
        let v = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0, 3.0])).unwrap();
        let w = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![4.0, 5.0, 6.0])).unwrap();
        // Text columns only: nested frequencies, every row counts 1 (T392).
        let g = p.plot_columns(Template::Sunburst, &[region, name]).unwrap();
        let plot = &p.graph(g).unwrap().layers[0].plots[0];
        assert_eq!((plot.y, plot.sunburst.categories.clone()), (region, vec![region, name]));
        let counted_graphs = p.graphs().count();
        let err = p.plot_sunburst(&[], v).unwrap_err();
        assert!(err.to_string().contains("category column"), "{err}");
        let err = p.plot_sunburst(&[v], region).unwrap_err();
        assert!(err.to_string().contains("numeric value column"), "{err}");
        let err = p.plot_columns_double_y(Template::Sunburst, &[v], &[w]).unwrap_err();
        assert!(err.to_string().contains("right Y axis"), "{err}");
        assert_eq!(p.graphs().count(), counted_graphs, "a refused plot leaves no graph behind");
        let g = p.plot_columns(Template::Sunburst, &[name, region, v, w]).unwrap();
        let l = p.graph(g).unwrap().layers[0].clone();
        assert_eq!(l.plots.len(), 1);
        assert_eq!((l.plots[0].y, l.plots[0].kind), (w, PlotKind::Sunburst));
        assert_eq!(l.plots[0].sunburst.categories, vec![name, region]);
        let g2 = p.plot_columns(Template::Sunburst, &[v]).unwrap();
        assert_eq!(p.graph(g2).unwrap().layers[0].plots[0].sunburst.categories, vec![region, name]);
        assert_eq!(l.inherited_sunburst().categories, vec![name, region]);
    }

    #[test]
    fn scatter_rug_template_and_rug_serde() {
        assert_eq!(Template::from_name("scatter-rug"), Some(Template::ScatterRug));
        assert_eq!(Template::ScatterRug.label(), "Scatter + Rug");
        let g = Template::ScatterRug.new_graph(WindowId(1), "g");
        let l = &g.layers[0];
        assert!(l.x_axis.rug.show && l.y_axis.rug.show && l.x_axis.rug.overlap);
        assert!(!l.x_axis.rug_opposite.show && !l.y_axis.rug_opposite.show);
        assert_eq!((l.x_axis.rug.size_pct, l.x_axis.rug.thickness_pt, l.x_axis.rug.margin_pct), (5.0, 0.5, 0.0));
        // Round trip.
        let back: Graph = serde_json::from_str(&serde_json::to_string(&g).unwrap()).unwrap();
        assert_eq!(back, g);
        // Graphs without a rug carry no rug keys; old files read as no rug.
        let plain = Template::Scatter.new_graph(WindowId(1), "g");
        let json = serde_json::to_string(&plain).unwrap();
        assert!(!json.contains("rug"), "{json}");
        let back: Graph = serde_json::from_str(&json).unwrap();
        assert!(back.layers[0].x_axis.rug.is_default());
        // A partly written rug keeps the defaults for the rest.
        let r: Rug = serde_json::from_str(r#"{"show":true,"plot":2}"#).unwrap();
        assert!(r.show && r.plot == Some(crate::ids::ColumnId(2)) && r.overlap && r.size_pct == 5.0);
    }
}

/// The templates of graphs made from a matrix or from XYZ columns. They are kept apart from
/// [`Template`], whose members make graphs of curves from columns.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldTemplate {
    /// Contour plot with colour fill: 8 major steps of 5 bands each, level lines at the major
    /// levels (Origin's "Contour - Color Fill").
    ContourFill,
    /// Contour lines only: 10 major steps, a line and a label at every level in between (Origin's
    /// "Contour - Lines").
    ContourLines,
    /// Heat map: a block per matrix cell, blue to white to red, 5 major steps of 6 bands (Origin's
    /// "Heat Map").
    HeatMap,
    /// Gray Scale Map (T201, measured by T297 on Origin 2016's CONTGRAY template): a block per matrix
    /// cell in 32 grays from black (lowest) to white (highest), 8 major steps of 6 bands, with a colour
    /// scale; X tick labels upright.
    GrayScale,
}

impl FieldTemplate {
    pub const ALL: [FieldTemplate; 4] =
        [FieldTemplate::ContourFill, FieldTemplate::ContourLines, FieldTemplate::HeatMap, FieldTemplate::GrayScale];

    /// The name used on the command line.
    pub fn name(&self) -> &'static str {
        match self {
            FieldTemplate::ContourFill => "contour",
            FieldTemplate::ContourLines => "contour-lines",
            FieldTemplate::HeatMap => "heatmap",
            FieldTemplate::GrayScale => "grayscale",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            FieldTemplate::ContourFill => "Contour - Color Fill",
            FieldTemplate::ContourLines => "Contour - Lines",
            FieldTemplate::HeatMap => "Heat Map",
            FieldTemplate::GrayScale => "Gray Scale Map",
        }
    }

    pub fn from_name(name: &str) -> Option<FieldTemplate> {
        FieldTemplate::ALL.into_iter().find(|t| t.name().eq_ignore_ascii_case(name))
    }

    /// Polar Contour (T225): Origin's polar page and layer with this template's contour of `source`,
    /// whose X is the angle and Y the radius. The colour scale stands to the right of the circle.
    pub fn new_polar_graph(&self, id: WindowId, name: &str, source: FieldSource) -> Graph {
        let mut g = polar(id, name, RadiusColumn::Y);
        let field = self.new_graph(id, name, source).layers.swap_remove(0).field;
        let layer = &mut g.layers[0];
        layer.legend.visible = false;
        layer.field = field;
        g
    }

    /// Ternary Contour (T226): Origin's ternary layer with this template's contour of `source`.
    pub fn new_ternary_graph(&self, id: WindowId, name: &str, source: FieldSource) -> Graph {
        let mut g = ternary(id, name);
        let field = self.new_graph(id, name, source).layers.swap_remove(0).field;
        let layer = &mut g.layers[0];
        layer.legend.visible = false;
        layer.field = field;
        // The field templates' frame, so that the colour scale and its labels fit on the page.
        layer.frame = Frame { left: 0.14, top: 0.113, width: 0.5725, height: 0.733 };
        g
    }

    /// An empty graph in this template's style: one layer whose field is `source`.
    pub fn new_graph(&self, id: WindowId, name: &str, source: FieldSource) -> Graph {
        let mut g = general(id, name, PlotKind::Line);
        let layer = &mut g.layers[0];
        // The frame leaves the right part of the page to the colour scale (Origin's proportions).
        layer.frame = Frame { left: 0.14, top: 0.113, width: 0.5725, height: 0.733 };
        if *self == FieldTemplate::ContourLines {
            // No colour scale: the frame takes the width of the page (Origin: 0.179 to 0.8625).
            layer.frame = Frame { left: 0.179, top: 0.113, width: 0.6835, height: 0.733 };
        }
        layer.legend.visible = false;
        for axis in [&mut layer.x_axis, &mut layer.y_axis] {
            axis.auto_range = AutoRange::Tight;
            axis.show_opposite = true;
            axis.opposite_ticks = false;
            // Origin's matrix graphs have smaller text than its line graphs: more ticks fit.
            axis.tick_font.size_pt = 10.0;
            axis.title_font.size_pt = 12.0;
        }
        let kind = if matches!(self, FieldTemplate::HeatMap | FieldTemplate::GrayScale) {
            FieldKind::Cells
        } else {
            FieldKind::Contour
        };
        let (fill, lines, labels, palette, major, minor) = match self {
            FieldTemplate::ContourFill => (true, true, false, FieldPalette::Rainbow, 8, 5),
            FieldTemplate::ContourLines => (false, true, true, FieldPalette::Rainbow, 10, 1),
            FieldTemplate::HeatMap => (true, false, false, FieldPalette::BlueWhiteRed, 5, 6),
            FieldTemplate::GrayScale => (true, false, false, FieldPalette::Gray, 8, 6),
        };
        if *self == FieldTemplate::HeatMap {
            layer.x_axis.tick_label_angle_deg = 90.0;
        }
        layer.field = Some(FieldPlot {
            source,
            kind,
            fill,
            lines,
            labels,
            palette,
            levels: FieldLevels { min: None, max: None, major, minor },
            line: LineStyle::solid(Color::BLACK, 0.5),
            label_font: Font::new(DEFAULT_FONT, 11.0),
            scale: ColorScale {
                visible: fill,
                title: None,
                font: Font::new(DEFAULT_FONT, 8.0),
                title_font: Font::new(DEFAULT_FONT, 12.0),
            },
        });
        g
    }
}

#[cfg(test)]
mod color_list_tests {
    use super::*;

    #[test]
    fn origins_named_colour_lists_are_found_by_name_or_code() {
        assert_eq!(ORIGIN_COLOR_LISTS.len(), 18);
        assert!(ORIGIN_COLOR_LISTS.iter().all(|(_, l)| (4..=23).contains(&l.len())));
        let (name, q03) = color_list("q03").unwrap();
        assert_eq!(name, "Q03 Rainbow7");
        assert_eq!(&q03[..3], [Color::rgb(255, 0, 0), Color::rgb(255, 102, 0), Color::rgb(255, 238, 0)]);
        assert_eq!(color_list("S01 gray scale").unwrap().1.len(), 5);
        assert_eq!(color_list("Default").unwrap().1.len(), ORIGIN_COLOR_LIST.len());
        assert!(color_list("Q99").is_none() && color_list("").is_none());
        // Q02 Basic is the theme list measured on Origin's area templates.
        let q02 = color_list("Q02").unwrap().1;
        let theme: Vec<Color> = THEME_COLOR_LIST.iter().map(|&(r, g, b)| Color::rgb(r, g, b)).collect();
        assert_eq!(q02, theme);
    }

    #[test]
    fn a_palette_names_its_list_only_when_one_was_chosen() {
        let g = Template::Line.new_graph(WindowId(1), "G");
        let mut palette = g.layers[0].palette.clone();
        assert!(!serde_json::to_string(&palette).unwrap().contains("list"));
        palette.list = Some("Q03 Rainbow7".into());
        let back: Palette = serde_json::from_str(&serde_json::to_string(&palette).unwrap()).unwrap();
        assert_eq!(back, palette);
    }

    #[test]
    fn a_sankey_plot_takes_two_text_columns_and_the_flow() {
        let mut p = crate::Project::new();
        let b = p.add_workbook("");
        let text = |v: &[&str]| crate::ColumnData::text(v.iter().map(|s| s.to_string()).collect());
        let src = p.add_column(b, 0, crate::Designation::Y, text(&["Coal", "Gas", "Power"])).unwrap();
        let tgt = p.add_column(b, 0, crate::Designation::Y, text(&["Power", "Power", "Homes"])).unwrap();
        let v = p.add_column(b, 0, crate::Designation::Y, crate::ColumnData::numeric(vec![3.0, 2.0, 5.0])).unwrap();
        let g = p.plot_columns(super::Template::Sankey, &[src, tgt, v]).unwrap();
        let plot = &p.graph(g).unwrap().layers[0].plots[0];
        assert_eq!(
            (plot.kind, plot.y, plot.sankey.source, plot.sankey.target),
            (crate::graph::PlotKind::Sankey, v, Some(src), Some(tgt))
        );
        // Only the value chosen: the sheet's text columns are the ends.
        let g = p.plot_columns(super::Template::Sankey, &[v]).unwrap();
        assert_eq!(p.graph(g).unwrap().layers[0].plots[0].sankey.source, Some(src));
        assert!(p.plot_columns_double_y(super::Template::Sankey, &[v], &[v]).is_err());
        assert_eq!(serde_json::to_string(&crate::graph::PlotKind::Sankey).unwrap(), "\"sankey\"");
    }
}
