//! The body of Plot Details: a tree at the left (the graph, its layers, their plots) with the plot
//! type under it, and at the right the tabs of what is chosen.
//!
//! * graph: Print/Dimensions, Legends/Titles, Miscellaneous, Display, Text (Sheaf's own)
//! * layer: Background, Size, Display, Stack, Link Axes Scales, Legend (Sheaf's own place for the
//!   legend text). Layer Contents and Plot Setup are separate dialogs (`layer_contents.rs`).
//! * plot: Group, Line, Symbol, Drop Lines, Label, Offset (a line or symbol plot); Group, Pattern,
//!   Spacing, Label (columns); Error Bar when the plot has error bars
//!
//! The Workbook button shows the sheet of the chosen plot; ">>" opens a side panel with the rows
//! the plot draws and the row range.

use eframe::egui::{self, DragValue};
use sheaf_core::graph::*;
use sheaf_core::{Color, ColumnId};

use super::details_layer as layer_tabs;
use super::details_plot as plot_tabs;
use super::graph_props::{
    color_button, connect_name, dash_ui, font_ui, grid, group_box, section, symbol_name, tab_row,
};
use sheaf_core::{Project, WindowId};

/// What the tree has chosen.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Node {
    Graph,
    Layer(usize),
    /// Layer index, plot index.
    Plot(usize, usize),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DetailTab {
    Group,
    Line,
    Symbol,
    DropLines,
    Label,
    Offset,
    Pattern,
    Spacing,
    ErrorBar,
    Background,
    Size,
    Display,
    Stack,
    LinkAxes,
    Legend,
    Dimensions,
    Titles,
    Misc,
    PageDisplay,
    Text,
}

fn tabs_for(node: Node, graph: &Graph) -> Vec<(DetailTab, &'static str, bool)> {
    match node {
        Node::Graph => vec![
            (DetailTab::Dimensions, "Print/Dimensions", true),
            (DetailTab::Titles, "Legends/Titles", true),
            (DetailTab::Misc, "Miscellaneous", true),
            (DetailTab::PageDisplay, "Display", true),
            (DetailTab::Text, "Text", true),
        ],
        Node::Layer(_) => vec![
            (DetailTab::Background, "Background", true),
            (DetailTab::Size, "Size", true),
            (DetailTab::Display, "Display", true),
            (DetailTab::Stack, "Stack", true),
            (DetailTab::LinkAxes, "Link Axes Scales", true),
            (DetailTab::Legend, "Legend", true),
        ],
        Node::Plot(l, p) => {
            let plot = &graph.layers[l].plots[p];
            let k = plot.kind;
            let mut tabs = vec![(DetailTab::Group, "Group", true)];
            if k.has_line() {
                tabs.push((DetailTab::Line, "Line", true));
            }
            if k.has_symbols() {
                tabs.push((DetailTab::Symbol, "Symbol", true));
            }
            if k.has_line() || k.has_symbols() {
                tabs.push((DetailTab::DropLines, "Drop Lines", true));
            }
            if k.is_bar() || k.is_area() {
                tabs.push((DetailTab::Pattern, "Pattern", true));
            }
            if k.is_bar() {
                tabs.push((DetailTab::Spacing, "Spacing", true));
            }
            if has_labels(k) {
                tabs.push((DetailTab::Label, "Label", true));
            }
            if k.has_line() || k.has_symbols() {
                tabs.push((DetailTab::Offset, "Offset", true));
            }
            if plot.y_error.is_some() || plot.x_error.is_some() {
                tabs.push((DetailTab::ErrorBar, "Error Bar", true));
            }
            tabs
        }
    }
}

/// The kinds that draw data labels (the renderer skips boxes and pies).
fn has_labels(k: PlotKind) -> bool {
    !matches!(k, PlotKind::Box | PlotKind::Pie | PlotKind::Pie3D | PlotKind::PieBw)
}

/// The tab to show when the one that was shown does not exist for the new node.
fn first_tab(tabs: &[(DetailTab, &str, bool)]) -> DetailTab {
    let preferred = [DetailTab::Line, DetailTab::Pattern, DetailTab::Symbol];
    preferred.into_iter().find(|t| tabs.iter().any(|(x, _, on)| x == t && *on)).unwrap_or(tabs[0].0)
}

pub fn kind_name(k: PlotKind) -> &'static str {
    match k {
        PlotKind::Line => "Line",
        PlotKind::Scatter => "Scatter",
        PlotKind::LineSymbol => "Line + Symbol",
        PlotKind::Column => "Column",
        PlotKind::Area => "Area",
        PlotKind::StackedColumn => "Stacked Column",
        PlotKind::Stacked100Column => "100% Stacked Column",
        PlotKind::Bar => "Bar",
        PlotKind::FloatingColumn => "Floating Column",
        PlotKind::StackedBar => "Stacked Bar",
        PlotKind::Stacked100Bar => "100% Stacked Bar",
        PlotKind::FloatingBar => "Floating Bar",
        PlotKind::StackedArea => "Stacked Area",
        PlotKind::Stacked100Area => "100% Stacked Area",
        PlotKind::Histogram => "Histogram",
        PlotKind::Box => "Box Chart",
        PlotKind::Pie => "Pie",
        PlotKind::Pie3D => "3D Color Pie",
        PlotKind::PieBw => "2D B&W Pie",
    }
}

/// What Plot Details reads about the project (the graph itself is edited).
pub struct Info<'a> {
    pub project: &'a Project,
    /// The short name of every plot, per layer (legend text, group members).
    pub labels: &'a [Vec<String>],
    /// The tree text of every plot, per layer: `[Book1]Sheet1! "Conc"(X), "Signal"(Y) [1*:10*]`.
    pub texts: &'a [Vec<String>],
    pub columns: &'a [(ColumnId, String)],
}

/// What the body keeps between frames besides the chosen node and tab.
#[derive(Default)]
pub struct Local {
    /// The ">>" side panel is open.
    pub side: bool,
    /// Resizing the page keeps the layers' size (Print/Dimensions).
    pub keep_layers: bool,
    /// Set by the Workbook button: the window to show.
    pub open_workbook: Option<WindowId>,
}

/// The kinds of plot a plot of the tree can be changed to in the Plot Type box.
pub const KINDS: [PlotKind; 19] = [
    PlotKind::Line,
    PlotKind::Scatter,
    PlotKind::LineSymbol,
    PlotKind::Column,
    PlotKind::StackedColumn,
    PlotKind::Stacked100Column,
    PlotKind::Bar,
    PlotKind::FloatingColumn,
    PlotKind::StackedBar,
    PlotKind::Stacked100Bar,
    PlotKind::FloatingBar,
    PlotKind::Area,
    PlotKind::StackedArea,
    PlotKind::Stacked100Area,
    PlotKind::Histogram,
    PlotKind::Box,
    PlotKind::Pie,
    PlotKind::Pie3D,
    PlotKind::PieBw,
];

pub fn dialog_body(
    ui: &mut egui::Ui,
    node: &mut Node,
    tab: &mut DetailTab,
    graph: &mut Graph,
    info: &Info,
    local: &mut Local,
) {
    // The node may have gone (a plot was removed).
    *node = match *node {
        Node::Layer(l) | Node::Plot(l, _) if l >= graph.layers.len() => Node::Layer(graph.layers.len() - 1),
        Node::Plot(l, p) if p >= graph.layers[l].plots.len() => Node::Layer(l),
        n => n,
    };
    let before = *node;

    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.set_width(300.0);
            let tree_h = (ui.available_height() - 70.0).max(120.0);
            egui::ScrollArea::both()
                .id_salt("details_tree")
                .max_height(tree_h)
                .auto_shrink([false, false])
                .show(ui, |ui| tree(ui, node, graph, info.texts));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label("Plot Type");
                if let Node::Plot(l, p) = *node {
                    let polar = graph.layers[l].coords.polar().is_some();
                    let radar = graph.layers[l].coords.polar().is_some_and(|p| p.radar);
                    let ternary = graph.layers[l].coords.ternary().is_some();
                    let plot = &mut graph.layers[l].plots[p];
                    // A plot of a polar or ternary layer is an ordinary line or symbol plot.
                    let shown = if radar {
                        format!("Radar {}", kind_name(plot.kind))
                    } else if ternary {
                        format!("Ternary {}", kind_name(plot.kind))
                    } else if polar {
                        format!("Polar {}", kind_name(plot.kind))
                    } else {
                        kind_name(plot.kind).to_string()
                    };
                    egui::ComboBox::from_id_salt("plot_type").selected_text(shown).show_ui(ui, |ui| {
                        for k in KINDS {
                            ui.selectable_value(&mut plot.kind, k, kind_name(k));
                        }
                    });
                } else {
                    ui.add_enabled(false, egui::Button::new("-"));
                }
            });
            ui.horizontal(|ui| {
                if ui.selectable_label(local.side, ">>").on_hover_text("Rows of the chosen plot").clicked() {
                    local.side = !local.side;
                }
                let book = match *node {
                    Node::Plot(l, p) => info.project.plot_book(&graph.layers[l].plots[p]),
                    _ => None,
                };
                if ui
                    .add_enabled(book.is_some(), egui::Button::new("Workbook"))
                    .on_hover_text("Show the worksheet of the chosen plot")
                    .clicked()
                {
                    local.open_workbook = book;
                }
            });
        });
        ui.separator();
        ui.vertical(|ui| {
            let side_w = if local.side { 250.0 } else { 0.0 };
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width((ui.available_width() - side_w - 10.0).max(300.0));
                    let tabs = tabs_for(*node, graph);
                    if *node != before || !tabs.iter().any(|(t, _, on)| t == tab && *on) {
                        *tab = first_tab(&tabs);
                    }
                    tab_row(ui, tab, &tabs);
                    ui.separator();
                    egui::ScrollArea::vertical().id_salt("details_page").auto_shrink([false, false]).show(ui, |ui| {
                        match *node {
                            Node::Graph => match *tab {
                                DetailTab::Text => text_tab(ui, graph),
                                DetailTab::Titles => layer_tabs::titles_tab(ui, graph),
                                DetailTab::Misc => layer_tabs::misc_tab(ui, graph),
                                DetailTab::PageDisplay => layer_tabs::page_display_tab(ui, graph),
                                _ => layer_tabs::dimensions_tab(ui, graph, &mut local.keep_layers),
                            },
                            Node::Layer(l) => layer_tab(ui, *tab, graph, l),
                            Node::Plot(l, p) => plot_tab(ui, *tab, graph, l, p, info, node),
                        }
                    });
                });
                if local.side {
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.set_width(side_w);
                        side_panel(ui, *node, graph, info);
                    });
                }
            });
        });
    });
}

/// The ">>" panel: the rows the chosen plot draws and the row range it uses.
fn side_panel(ui: &mut egui::Ui, node: Node, graph: &mut Graph, info: &Info) {
    let Node::Plot(l, p) = node else {
        ui.weak("Choose a plot to see its rows.");
        return;
    };
    let plot = &mut graph.layers[l].plots[p];
    let Some((book, sheet, y)) = info.project.locate(plot.y) else {
        ui.weak("The column of this plot is gone.");
        return;
    };
    ui.label(egui::RichText::new(format!("{} / {}", book.title(), sheet.name)).strong());
    let len = y.len();
    ui.label(format!("{len} rows in the Y column"));
    let mut limited = plot.rows.is_some();
    if ui.checkbox(&mut limited, "Plot a range of rows only").changed() {
        plot.rows = limited.then_some((0, len));
    }
    if let Some((a, b)) = &mut plot.rows {
        let (mut from, mut to) = (*a + 1, *b);
        ui.horizontal(|ui| {
            ui.label("From");
            ui.add(DragValue::new(&mut from).range(1..=len.max(1)));
            ui.label("to");
            ui.add(DragValue::new(&mut to).range(1..=len.max(1)));
        });
        *a = from.saturating_sub(1).min(len);
        *b = to.max(*a).min(len);
    }
    ui.separator();
    let x = plot.x.and_then(|id| info.project.column(id));
    let range = plot.rows.unwrap_or((0, len));
    egui::ScrollArea::vertical().id_salt("side_rows").auto_shrink([false, false]).show(ui, |ui| {
        egui::Grid::new("side_grid").num_columns(3).striped(true).show(ui, |ui| {
            ui.label(egui::RichText::new("Row").weak());
            ui.label(egui::RichText::new(x.map(|c| c.display_name().to_string()).unwrap_or_else(|| "X".into())).weak());
            ui.label(egui::RichText::new(y.display_name()).weak());
            ui.end_row();
            for r in range.0..range.1.min(range.0 + 200) {
                ui.label(format!("{}", r + 1));
                ui.label(x.map(|c| c.shown_text(r)).unwrap_or_else(|| format!("{}", r + 1)));
                ui.label(y.shown_text(r));
                ui.end_row();
            }
        });
    });
}

fn tree(ui: &mut egui::Ui, node: &mut Node, graph: &mut Graph, texts: &[Vec<String>]) {
    if ui.selectable_label(*node == Node::Graph, &graph.name).clicked() {
        *node = Node::Graph;
    }
    ui.indent("tree_layers", |ui| {
        for l in 0..graph.layers.len() {
            let name = format!("Layer {}", l + 1);
            if ui.selectable_label(*node == Node::Layer(l), name).clicked() {
                *node = Node::Layer(l);
            }
            ui.indent(("tree_plots", l), |ui| {
                for p in 0..graph.layers[l].plots.len() {
                    ui.horizontal(|ui| {
                        let plot = &mut graph.layers[l].plots[p];
                        ui.checkbox(&mut plot.visible, "").on_hover_text("Show this plot");
                        crate::grid::color_swatch(ui, plot.line.color);
                        let text =
                            texts.get(l).and_then(|v| v.get(p)).cloned().unwrap_or_else(|| format!("Plot {}", p + 1));
                        if ui.selectable_label(*node == Node::Plot(l, p), text).clicked() {
                            *node = Node::Plot(l, p);
                        }
                    });
                }
            });
        }
    });
}

// ----- plot ---------------------------------------------------------------------------------

fn plot_tab(ui: &mut egui::Ui, tab: DetailTab, graph: &mut Graph, l: usize, p: usize, info: &Info, node: &mut Node) {
    let (labels, columns) = (info.labels, info.columns);
    match tab {
        DetailTab::Line => {
            let layer = &mut graph.layers[l];
            let plot = &mut layer.plots[p];
            let mut dash = plot.line.dash;
            grid(ui, "plot_line", |ui| {
                ui.label("Connect");
                egui::ComboBox::from_id_salt("plot_connect").selected_text(connect_name(plot.line.connect)).show_ui(
                    ui,
                    |ui| {
                        for c in [
                            Connect::Straight,
                            Connect::Spline,
                            Connect::BSpline,
                            Connect::Bezier,
                            Connect::StepHorizontal,
                            Connect::StepVertical,
                        ] {
                            ui.selectable_value(&mut plot.line.connect, c, connect_name(c));
                        }
                    },
                );
                ui.end_row();
                ui.label("Style");
                dash_ui(ui, "plot_dash", &mut dash);
                ui.end_row();
                ui.label("Width");
                ui.add(DragValue::new(&mut plot.line.width_pt).range(0.1..=20.0).speed(0.05).suffix(" pt"));
                ui.end_row();
                ui.label("Color");
                let before = plot.line.color;
                color_button(ui, &mut plot.line.color);
                if plot.line.color != before {
                    plot.symbol.color = plot.line.color;
                    // Chosen by hand: later palette changes from added plots leave it alone.
                    plot.auto_color = false;
                }
                ui.end_row();
                ui.label("Transparency");
                ui.add(DragValue::new(&mut plot.details.line_transparency_pct).range(0..=100).suffix(" %"));
                ui.end_row();
            });
            let plot = &mut layer.plots[p];
            if plot.kind.is_area() {
                ui.weak("The area under the curve is set on the Pattern tab.");
            } else {
                ui.add_space(4.0);
                ui.checkbox(&mut plot.details.fill_under, "Fill Area Under Curve");
                if plot.details.fill_under {
                    layer_tabs::plot_fill(ui, "line_fill", plot);
                }
            }
            if dash != layer.plots[p].line.dash {
                layer.set_line_dash(p, dash);
            }
        }
        DetailTab::Symbol => {
            let layer = &mut graph.layers[l];
            let plot = &mut layer.plots[p];
            let mut shape = plot.symbol.shape;
            grid(ui, "plot_symbol", |ui| {
                ui.label("Shape");
                egui::ComboBox::from_id_salt("plot_symbol").selected_text(symbol_name(shape)).show_ui(ui, |ui| {
                    for s in SymbolShape::ALL {
                        ui.selectable_value(&mut shape, s, symbol_name(s));
                    }
                });
                ui.end_row();
                ui.label("Size");
                ui.add(DragValue::new(&mut plot.symbol.size_pt).range(0.5..=40.0).speed(0.1).suffix(" pt"));
                ui.end_row();
                ui.label("Interior");
                ui.checkbox(&mut plot.symbol.filled, "Filled");
                ui.end_row();
                ui.label("Edge Thickness");
                ui.add(DragValue::new(&mut plot.symbol.edge_width_pt).range(0.0..=10.0).speed(0.05).suffix(" pt"));
                ui.end_row();
                ui.label("Color");
                let before = plot.symbol.color;
                color_button(ui, &mut plot.symbol.color);
                if plot.symbol.color != before {
                    plot.line.color = plot.symbol.color;
                    plot.auto_color = false;
                }
                ui.end_row();
                ui.label("Show every");
                ui.add(DragValue::new(&mut plot.symbol.every).range(1..=100000).suffix(" th point"));
                ui.end_row();
            });
            if shape != layer.plots[p].symbol.shape {
                layer.set_symbol_shape(p, shape);
            }
        }
        DetailTab::DropLines => plot_tabs::drop_lines_tab(ui, &mut graph.layers[l].plots[p]),
        DetailTab::Label => plot_tabs::label_tab(ui, &mut graph.layers[l].plots[p], columns),
        DetailTab::Offset => plot_tabs::offset_tab(ui, &mut graph.layers[l], p),
        DetailTab::Pattern => {
            let plot = &mut graph.layers[l].plots[p];
            group_box(ui, "Border", |ui| {
                grid(ui, "plot_border", |ui| {
                    ui.label("Color");
                    let before = plot.line.color;
                    color_button(ui, &mut plot.line.color);
                    if plot.line.color != before {
                        plot.auto_color = false;
                    }
                    ui.end_row();
                    ui.label("Style");
                    dash_ui(ui, "plot_border_dash", &mut plot.line.dash);
                    ui.end_row();
                    ui.label("Width");
                    ui.add(DragValue::new(&mut plot.line.width_pt).range(0.0..=20.0).speed(0.05).suffix(" pt"));
                    ui.end_row();
                });
            });
            group_box(ui, "Fill", |ui| layer_tabs::plot_fill(ui, "plot_fill", plot));
        }
        DetailTab::Spacing => {
            let bars = &mut graph.layers[l].bars;
            grid(ui, "plot_spacing", |ui| {
                ui.label("Gap Between Bars (in %)");
                ui.add(DragValue::new(&mut bars.gap_pct).range(0.0..=100.0).speed(0.5));
                ui.end_row();
                ui.label("Overlap (in %)");
                ui.add(DragValue::new(&mut bars.overlap_pct).range(-100.0..=100.0).speed(0.5));
                ui.end_row();
                ui.label("Width (in %)");
                ui.horizontal(|ui| {
                    let mut set = bars.width_pct.is_some();
                    if ui.checkbox(&mut set, "Set").changed() {
                        bars.width_pct = set.then_some(100.0 - bars.gap_pct);
                    }
                    if let Some(w) = &mut bars.width_pct {
                        ui.add(DragValue::new(w).range(1.0..=100.0).speed(0.5));
                    }
                });
                ui.end_row();
            });
            ui.weak("Gap, overlap and width are shared by all column plots of the layer; a width replaces the gap.");
        }
        DetailTab::ErrorBar => {
            let plot = &mut graph.layers[l].plots[p];
            let name = plot
                .y_error
                .and_then(|e| columns.iter().find(|(c, _)| *c == e))
                .map(|(_, n)| n.clone())
                .unwrap_or_default();
            let xname = plot
                .x_error
                .and_then(|e| columns.iter().find(|(c, _)| *c == e))
                .map(|(_, n)| n.clone())
                .unwrap_or_default();
            grid(ui, "plot_error", |ui| {
                ui.label("Y error column");
                ui.label(name);
                ui.end_row();
                ui.label("X error column");
                ui.label(xname);
                ui.end_row();
                ui.label("Direction");
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut plot.error.direction, ErrorDirection::Both, "Both");
                    ui.selectable_value(&mut plot.error.direction, ErrorDirection::Plus, "Positive");
                    ui.selectable_value(&mut plot.error.direction, ErrorDirection::Minus, "Negative");
                });
                ui.end_row();
                ui.label("Cap width");
                ui.add(DragValue::new(&mut plot.error.cap_width_pt).range(0.0..=100.0).speed(0.1).suffix(" pt"));
                ui.end_row();
            });
            if ui.button("Remove error bars").clicked() {
                plot.y_error = None;
                plot.x_error = None;
            }
        }
        _ => group_tab(ui, graph, l, p, labels, node),
    }
}

/// The Y axis of a plot, left or right. Choosing the other side moves the plot to the right Y
/// layer of its panel (made when there is none) or back to the panel's own layer, and the tree
/// follows the plot to its new place.
fn y_axis_choice(ui: &mut egui::Ui, graph: &mut Graph, l: usize, p: usize, node: &mut Node) -> bool {
    let on_right = graph.right_y_member(l) == Some(l);
    let mut want = on_right;
    grid(ui, "plot_y_axis", |ui| {
        ui.label("Y axis");
        ui.horizontal(|ui| {
            ui.selectable_value(&mut want, false, "Left");
            ui.selectable_value(&mut want, true, "Right");
        });
        ui.end_row();
    });
    if want != on_right
        && let Ok((layer, plot)) = graph.set_plot_y_axis(l, p, want)
    {
        *node = Node::Plot(layer, plot);
        return true;
    }
    false
}

/// Origin's Group tab for a plot: where it stands in the layer and the colours of the group.
fn group_tab(ui: &mut egui::Ui, graph: &mut Graph, l: usize, p: usize, labels: &[Vec<String>], node: &mut Node) {
    if y_axis_choice(ui, graph, l, p, node) {
        // The plot is somewhere else now; the tab is drawn for it in the next frame.
        return;
    }
    ui.add_space(6.0);
    let layer = &mut graph.layers[l];
    let count = layer.plots.len();
    grid(ui, "plot_group", |ui| {
        ui.label("Legend text");
        ui.horizontal(|ui| {
            let plot = &mut layer.plots[p];
            let mut auto = plot.label.is_none();
            if ui.checkbox(&mut auto, "From column").changed() {
                plot.label = if auto { None } else { Some(labels[l].get(p).cloned().unwrap_or_default()) };
            }
            if let Some(t) = &mut plot.label {
                ui.add(egui::TextEdit::singleline(t).desired_width(200.0));
            }
        });
        ui.end_row();
    });
    ui.add_space(6.0);
    let mut apply_to_all = None;
    let mut remove = false;
    let mut move_by: i32 = 0;
    ui.horizontal_wrapped(|ui| {
        if count > 1 && ui.button("Use this style for all plots").on_hover_text("Keeps each plot's color").clicked() {
            apply_to_all = Some(layer.plots[p].clone());
        }
        if ui.add_enabled(p > 0, egui::Button::new("Move up")).clicked() {
            move_by = -1;
        }
        if ui.add_enabled(p + 1 < count, egui::Button::new("Move down")).clicked() {
            move_by = 1;
        }
        if ui.button("Remove plot").clicked() {
            remove = true;
        }
    });
    if let Some(src) = apply_to_all {
        for q in &mut layer.plots {
            let color = q.line.color;
            q.kind = src.kind;
            q.line = LineStyle { color, ..src.line.clone() };
            q.symbol = SymbolStyle { color, ..src.symbol.clone() };
        }
    }
    if move_by != 0 {
        let j = (p as i32 + move_by) as usize;
        layer.plots.swap(p, j);
        *node = Node::Plot(l, j);
    }
    if remove {
        layer.plots.remove(p);
        *node = if layer.plots.is_empty() { Node::Layer(l) } else { Node::Plot(l, p.saturating_sub(1)) };
    }

    ui.horizontal(|ui| {
        if ui.button("Recolor plots").on_hover_text("Style every plot of the layer again from the rules").clicked() {
            layer.apply_palette();
        }
    });
    plot_tabs::group_rules(ui, layer, &labels[l]);
}

// ----- layer --------------------------------------------------------------------------------

fn layer_tab(ui: &mut egui::Ui, tab: DetailTab, graph: &mut Graph, l: usize) {
    // The ids that `\l(n)` and `%(n)` in the legend text refer to.
    let legend_ids = graph.legend_ids(l, &|j| graph.layers[j].plots.iter().filter(|p| p.visible).count());
    match tab {
        DetailTab::Size => layer_tabs::size_tab(ui, graph, l),
        DetailTab::Display => layer_tabs::display_tab(ui, &mut graph.layers[l]),
        DetailTab::LinkAxes => layer_tabs::link_tab(ui, graph, l),
        DetailTab::Stack => stack_tab(ui, &mut graph.layers[l]),
        DetailTab::Legend => legend_ui(ui, &mut graph.layers[l].legend, &legend_ids),
        _ => layer_tabs::background_tab(ui, &mut graph.layers[l]),
    }
}

fn stack_tab(ui: &mut egui::Ui, layer: &mut Layer) {
    let name = |s: &Stack| match s {
        Stack::None => "None",
        Stack::Auto { .. } => "Automatic",
        Stack::Constant { .. } => "Constant step",
        Stack::Individual => "Individual offsets",
        Stack::Waterfall { .. } => "Waterfall",
    };
    grid(ui, "stack_mode", |ui| {
        ui.label("Offset Grouped Data");
        let mut mode = layer.stack;
        egui::ComboBox::from_id_salt("stack_mode_box").selected_text(name(&mode)).show_ui(ui, |ui| {
            for s in [
                Stack::None,
                Stack::Auto { gap_percent: 10.0 },
                Stack::Constant { step: 1.0 },
                Stack::Individual,
                Stack::Waterfall { dx_percent: 5.0, dy_percent: 5.0 },
            ] {
                let same = std::mem::discriminant(&s) == std::mem::discriminant(&mode);
                if ui.selectable_label(same, name(&s)).clicked() && !same {
                    mode = s;
                }
            }
        });
        layer.stack = mode;
        ui.end_row();
        match &mut layer.stack {
            Stack::None | Stack::Individual => {}
            Stack::Auto { gap_percent } => {
                ui.label("Gap (% of height)");
                ui.add(DragValue::new(gap_percent).range(0.0..=500.0).speed(0.5));
                ui.end_row();
            }
            Stack::Constant { step } => {
                ui.label("Step (axis units)");
                ui.add(DragValue::new(step).speed(0.05).max_decimals(8));
                ui.end_row();
            }
            Stack::Waterfall { dx_percent, dy_percent } => {
                ui.label("X shift (%)");
                ui.add(DragValue::new(dx_percent).range(0.0..=100.0).speed(0.2));
                ui.end_row();
                ui.label("Y shift (%)");
                ui.add(DragValue::new(dy_percent).range(0.0..=100.0).speed(0.2));
                ui.end_row();
            }
        }
    });
    if layer.stack == Stack::Individual {
        section(ui, "Offset of each plot (axis units)");
        let plots: Vec<(ColumnId, usize)> = layer.plots.iter().enumerate().map(|(i, p)| (p.y, i)).collect();
        grid(ui, "stack_individual", |ui| {
            for (column, i) in plots {
                let found = layer.plot_offsets.iter().position(|o| o.column == column);
                let (mut x, mut y) =
                    found.map(|k| (layer.plot_offsets[k].x, layer.plot_offsets[k].y)).unwrap_or((0.0, 0.0));
                let (x0, y0) = (x, y);
                ui.label(format!("Plot {}", i + 1));
                ui.horizontal(|ui| {
                    ui.add(DragValue::new(&mut x).speed(0.05).max_decimals(8).prefix("X "));
                    ui.add(DragValue::new(&mut y).speed(0.05).max_decimals(8).prefix("Y "));
                });
                ui.end_row();
                if (x, y) != (x0, y0) {
                    match found {
                        Some(k) => {
                            layer.plot_offsets[k].x = x;
                            layer.plot_offsets[k].y = y;
                        }
                        None => layer.plot_offsets.push(PlotOffset { column, x, y, follows: None }),
                    }
                }
            }
        });
    }
}

fn legend_ui(ui: &mut egui::Ui, legend: &mut Legend, ids: &[String]) {
    grid(ui, "legend_props", |ui| {
        ui.label("Show legend");
        ui.checkbox(&mut legend.visible, "");
        ui.end_row();
        ui.label("Position");
        ui.horizontal(|ui| {
            let name = |p: &LegendPosition| match p {
                LegendPosition::TopRight => "Top right",
                LegendPosition::TopLeft => "Top left",
                LegendPosition::BottomRight => "Bottom right",
                LegendPosition::BottomLeft => "Bottom left",
                LegendPosition::Manual { .. } => "Custom",
            };
            egui::ComboBox::from_id_salt("legend_pos").selected_text(name(&legend.position)).show_ui(ui, |ui| {
                for p in [
                    LegendPosition::TopRight,
                    LegendPosition::TopLeft,
                    LegendPosition::BottomRight,
                    LegendPosition::BottomLeft,
                ] {
                    ui.selectable_value(&mut legend.position, p, name(&p));
                }
                let is_manual = matches!(legend.position, LegendPosition::Manual { .. });
                if ui.selectable_label(is_manual, "Custom").clicked() && !is_manual {
                    legend.position = LegendPosition::Manual { x: 0.8, y: 0.5 };
                }
            });
        });
        ui.end_row();
        if let LegendPosition::Manual { x, y } = &mut legend.position {
            ui.label("Centre (fraction of plot area)");
            ui.horizontal(|ui| {
                ui.add(DragValue::new(x).range(0.0..=1.0).speed(0.005).prefix("x "));
                ui.add(DragValue::new(y).range(0.0..=1.0).speed(0.005).prefix("y "));
            });
            ui.end_row();
        }
        ui.label("Lists");
        ui.horizontal(|ui| {
            ui.selectable_value(&mut legend.scope, LegendScope::Layer, "This layer");
            ui.selectable_value(&mut legend.scope, LegendScope::Graph, "Layers in this frame");
        });
        ui.end_row();
        ui.label("Columns");
        ui.add(DragValue::new(&mut legend.columns).range(1..=12).speed(0.1))
            .on_hover_text("The entries stand in this many columns, filled top to bottom");
        ui.end_row();
        ui.label("Text");
        ui.vertical(|ui| {
            let automatic = Legend::automatic_text(ids);
            let mut text = legend.text.clone().unwrap_or_else(|| automatic.clone());
            let shown = ui.add(egui::TextEdit::multiline(&mut text).desired_rows(3).desired_width(260.0));
            if shown.changed() {
                legend.text = (text != automatic).then_some(text);
            }
            ui.horizontal(|ui| {
                if ui.add_enabled(legend.text.is_some(), egui::Button::new("Reconstruct")).clicked() {
                    legend.text = None;
                }
                ui.weak("\\l(n) draws the symbol of plot n, %(n) its name");
            });
        });
        ui.end_row();
        ui.label("Font");
        font_ui(ui, &mut legend.font);
        ui.end_row();
        ui.label("Box");
        ui.checkbox(&mut legend.frame, "Draw a frame");
        ui.end_row();
        ui.label("Background");
        ui.horizontal(|ui| {
            let mut on = legend.background.is_some();
            if ui.checkbox(&mut on, "Fill").changed() {
                legend.background = on.then_some(Color::WHITE);
            }
            if let Some(c) = &mut legend.background {
                color_button(ui, c);
            }
        });
        ui.end_row();
    });
}

// ----- graph --------------------------------------------------------------------------------

/// The text settings of the whole graph: one font family and one size factor for every text.
fn text_tab(ui: &mut egui::Ui, graph: &mut Graph) {
    fn fonts(layer: &mut Layer) -> [&mut Font; 5] {
        [
            &mut layer.x_axis.title_font,
            &mut layer.x_axis.tick_font,
            &mut layer.y_axis.title_font,
            &mut layer.y_axis.tick_font,
            &mut layer.legend.font,
        ]
    }
    ui.horizontal(|ui| {
        ui.label("Font family");
        let mut family = graph.layers.first().map(|l| l.x_axis.title_font.family.clone()).unwrap_or_default();
        if ui.add(egui::TextEdit::singleline(&mut family).desired_width(160.0)).changed() {
            for layer in &mut graph.layers {
                for f in fonts(layer) {
                    f.family = family.clone();
                }
            }
        }
    });
    ui.horizontal(|ui| {
        ui.label("Scale all sizes");
        for (label, factor) in [("-10 %", 0.9), ("+10 %", 1.1)] {
            if ui.button(label).clicked() {
                for layer in &mut graph.layers {
                    for f in fonts(layer) {
                        f.size_pt = (f.size_pt * factor * 4.0).round() / 4.0;
                    }
                }
            }
        }
    });
}
