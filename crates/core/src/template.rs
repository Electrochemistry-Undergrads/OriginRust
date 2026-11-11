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
    /// Scatter plot with the same axes as [`Template::Line`].
    Scatter,
    /// Line plot with a symbol on every point.
    LineSymbol,
    /// Cyclic voltammetry overlay: smooth lines without markers, no grid,
    /// ticks pointing inwards, a closed black box around the plot, and a
    /// current axis that is symmetric about zero.
    CyclicVoltammetry,
}

impl Template {
    pub const ALL: [Template; 4] =
        [Template::Line, Template::Scatter, Template::LineSymbol, Template::CyclicVoltammetry];

    /// The name used on the command line and in menus.
    pub fn name(&self) -> &'static str {
        match self {
            Template::Line => "line",
            Template::Scatter => "scatter",
            Template::LineSymbol => "line-symbol",
            Template::CyclicVoltammetry => "cv",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Template::Line => "Line",
            Template::Scatter => "Scatter",
            Template::LineSymbol => "Line + Symbol",
            Template::CyclicVoltammetry => "Cyclic voltammetry overlay",
        }
    }

    pub fn from_name(name: &str) -> Option<Template> {
        Template::ALL.into_iter().find(|t| t.name().eq_ignore_ascii_case(name))
    }

    /// Builds an empty graph (one layer, no plots) in this template's style.
    pub fn new_graph(&self, id: WindowId, name: &str) -> Graph {
        match self {
            Template::Line => general(id, name, PlotKind::Line),
            Template::Scatter => general(id, name, PlotKind::Scatter),
            Template::LineSymbol => general(id, name, PlotKind::LineSymbol),
            Template::CyclicVoltammetry => cyclic_voltammetry(id, name),
        }
    }
}

/// Default font family. Arial ships with Windows; the renderer falls back
/// to a metric-compatible family where it is missing.
pub const DEFAULT_FONT: &str = "Arial";

fn base_axis(title_pt: f64, tick_pt: f64) -> Axis {
    Axis {
        scale: ScaleKind::Linear,
        min: None,
        max: None,
        auto_range: AutoRange::Nice,
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
        major_tick_len_pt: 5.0,
        minor_tick_len_pt: 2.5,
        line_width_pt: 1.0,
        color: Color::BLACK,
        show_opposite: false,
        opposite_ticks: false,
        major_grid: None,
        minor_grid: None,
    }
}

fn general(id: WindowId, name: &str, kind: PlotKind) -> Graph {
    let axis = base_axis(14.0, 12.0);
    let palette = Palette {
        colors: vec![
            Color::rgb(0, 0, 0),
            Color::rgb(220, 30, 30),
            Color::rgb(30, 80, 220),
            Color::rgb(0, 150, 70),
            Color::rgb(190, 50, 190),
            Color::rgb(235, 140, 0),
            Color::rgb(0, 160, 190),
            Color::rgb(130, 80, 40),
            Color::rgb(110, 110, 110),
        ],
        mode: PaletteMode::Cycle,
    };
    Graph {
        id,
        name: name.to_string(),
        long_name: String::new(),
        page: Page::inches(6.4, 4.8),
        layers: vec![Layer {
            frame: Frame { left: 0.16, top: 0.07, width: 0.78, height: 0.75 },
            x_axis: axis.clone(),
            y_axis: axis,
            plots: Vec::new(),
            legend: Legend {
                visible: true,
                position: LegendPosition::TopRight,
                font: Font::new(DEFAULT_FONT, 11.0),
                frame: false,
                background: None,
            },
            labels: Vec::new(),
            palette,
            default_line: LineStyle::solid(Color::BLACK, 1.5),
            default_symbol: SymbolStyle {
                shape: SymbolShape::Circle,
                size_pt: 5.0,
                color: Color::BLACK,
                filled: true,
                edge_width_pt: 0.75,
                every: 1,
            },
            default_kind: kind,
        }],
    }
}

fn cyclic_voltammetry(id: WindowId, name: &str) -> Graph {
    let mut x = base_axis(12.0, 10.0);
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
    layer.frame = Frame { left: 0.17, top: 0.06, width: 0.78, height: 0.74 };
    layer.x_axis = x;
    layer.y_axis = y;
    layer.palette = Palette {
        colors: vec![Color::rgb(0x46, 0xB1, 0xE1), Color::rgb(0x4E, 0x95, 0xD9), Color::rgb(0x10, 0x48, 0x62)],
        mode: PaletteMode::Spread,
    };
    layer.default_line = LineStyle { color: Color::BLACK, width_pt: 1.75, dash: Dash::Solid, connect: Connect::Spline };
    layer.legend = Legend {
        visible: true,
        position: LegendPosition::BottomRight,
        font: Font::new(DEFAULT_FONT, 10.0),
        frame: false,
        background: None,
    };
    g
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn template_names_round_trip() {
        for t in Template::ALL {
            assert_eq!(Template::from_name(t.name()), Some(t));
        }
        assert_eq!(Template::from_name("CV"), Some(Template::CyclicVoltammetry));
        assert_eq!(Template::from_name("nope"), None);
    }
}
