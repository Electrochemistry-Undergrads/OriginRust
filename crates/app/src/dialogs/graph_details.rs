//! The body of Plot Details: a tree at the left (the graph, its layers, their plots) with the plot
//! type under it, and at the right the tabs of what is chosen.
//!
//! * graph: Page, Text
//! * layer: Layer Contents (the plots and "Add data to this graph"), Background (the colour behind
//!   the frame), Size, Stack, Legend
//! * plot: Group, Line (with Origin's Transparency and Fill Area Under Curve), Symbol, Drop Lines,
//!   Label, Pattern (columns), Spacing (columns), Error Bar (when the plot has error bars); the
//!   tabs, controls and defaults of Origin's are in graphing spec section 18

use eframe::egui::{self, DragValue};
use sheaf_core::graph::*;
use sheaf_core::{Color, ColumnId};

use super::details_look as look;
use super::graph_props::{
    Source, color_button, connect_name, dash_ui, font_ui, grid, group_box, section, symbol_name, tab_row,
};

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
    Pattern,
    Spacing,
    ErrorBar,
    DropLines,
    Label,
    Offset,
    Display,
    PageDisplay,
    LegendsTitles,
    Misc,
    LayerContents,
    Background,
    Size,
    /// Origin's Link Axes Scales tab, for every layer but the first.
    Link,
    Stack,
    Legend,
    Page,
    Text,
}

fn tabs_for(node: Node, graph: &Graph) -> Vec<(DetailTab, &'static str, bool)> {
    match node {
        // Origin's page tabs (Miscellaneous holds Sheaf's layer icons and comments) and Sheaf's Text tab.
        Node::Graph => vec![
            (DetailTab::Page, "Print/Dimensions", true),
            (DetailTab::Misc, "Miscellaneous", true),
            (DetailTab::PageDisplay, "Display", true),
            (DetailTab::LegendsTitles, "Legends/Titles", true),
            (DetailTab::Text, "Text", true),
        ],
        Node::Layer(l) => {
            let mut tabs = vec![
                (DetailTab::LayerContents, "Layer Contents", true),
                (DetailTab::Background, "Background", true),
                (DetailTab::Size, "Size/Speed", true),
                (DetailTab::Display, "Display", true),
            ];
            // Origin has no Link Axes Scales tab for the first layer: it has nothing to link to.
            if l > 0 {
                tabs.push((DetailTab::Link, "Link Axes Scales", true));
            }
            tabs.push((DetailTab::Stack, "Stack", true));
            tabs.push((DetailTab::Legend, "Legend", true));
            tabs
        }
        Node::Plot(l, p) => {
            let plot = &graph.layers[l].plots[p];
            let mut tabs = vec![(DetailTab::Group, "Group", true)];
            if plot.kind.has_line() {
                tabs.push((DetailTab::Line, "Line", true));
            }
            if plot.kind.has_symbols() {
                tabs.push((DetailTab::Symbol, "Symbol", true));
            }
            let curve = plot.kind.has_line() || plot.kind.has_symbols();
            if curve && !plot.kind.is_bar() && !plot.kind.is_area() {
                tabs.push((DetailTab::DropLines, "Drop Lines", true));
            }
            if curve || plot.kind.is_bar() {
                tabs.push((DetailTab::Label, "Label", true));
            }
            if plot.kind.is_bar() || plot.kind.is_area() {
                tabs.push((DetailTab::Pattern, "Pattern", true));
            }
            if plot.kind.is_bar() {
                tabs.push((DetailTab::Spacing, "Spacing", true));
            }
            if plot.y_error.is_some() || plot.x_error.is_some() {
                tabs.push((DetailTab::ErrorBar, "Error Bar", true));
            }
            // Origin shows Offset for the members of a group (here: a layer of several plots).
            if graph.layers[l].plots.len() > 1 {
                tabs.push((DetailTab::Offset, "Offset", true));
            }
            tabs
        }
    }
}

/// The tab to show when the one that was shown does not exist for the new node.
fn first_tab(tabs: &[(DetailTab, &str, bool)]) -> DetailTab {
    let preferred = [DetailTab::Line, DetailTab::Pattern, DetailTab::Symbol];
    preferred.into_iter().find(|t| tabs.iter().any(|(x, _, on)| x == t && *on)).unwrap_or(tabs[0].0)
}

fn kind_name(k: PlotKind) -> &'static str {
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

/// Whether Origin's ">>" button has hidden the tree (kept in egui's memory while the app runs).
pub fn tree_hidden_id() -> egui::Id {
    egui::Id::new("plot_details_tree_hidden")
}

#[allow(clippy::too_many_arguments)]
pub fn dialog_body(
    ui: &mut egui::Ui,
    node: &mut Node,
    tab: &mut DetailTab,
    graph: &mut Graph,
    labels: &[Vec<String>],
    names: &[Vec<String>],
    sources: &[Source],
    columns: &[(ColumnId, String)],
    add_from: &mut usize,
) {
    // The node may have gone (a plot was removed).
    *node = match *node {
        Node::Layer(l) | Node::Plot(l, _) if l >= graph.layers.len() => Node::Layer(graph.layers.len() - 1),
        Node::Plot(l, p) if p >= graph.layers[l].plots.len() => Node::Layer(l),
        n => n,
    };
    let before = *node;
    let tree_hidden = ui.data(|d| d.get_temp::<bool>(tree_hidden_id()).unwrap_or(false));

    ui.horizontal_top(|ui| {
        // Origin's ">>" leaves only the tabs of the chosen node.
        if !tree_hidden {
            ui.vertical(|ui| {
                ui.set_width(300.0);
                let tree_h = (ui.available_height() - 40.0).max(120.0);
                egui::ScrollArea::vertical()
                    .id_salt("details_tree")
                    .max_height(tree_h)
                    .auto_shrink([false, false])
                    .show(ui, |ui| tree(ui, node, graph, names));
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
                            for k in [
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
                            ] {
                                ui.selectable_value(&mut plot.kind, k, kind_name(k));
                            }
                        });
                    } else {
                        ui.add_enabled(false, egui::Button::new("-"));
                    }
                });
            });
            ui.separator();
        }
        ui.vertical(|ui| {
            let tabs = tabs_for(*node, graph);
            if *node != before || !tabs.iter().any(|(t, _, on)| t == tab && *on) {
                *tab = first_tab(&tabs);
            }
            tab_row(ui, tab, &tabs);
            ui.separator();
            egui::ScrollArea::vertical().id_salt("details_page").auto_shrink([false, false]).show(
                ui,
                |ui| match *node {
                    Node::Graph => match *tab {
                        DetailTab::Text => text_tab(ui, graph),
                        DetailTab::PageDisplay => {
                            grid(ui, "page_display", |ui| {
                                ui.label("Color");
                                color_button(ui, &mut graph.page.background);
                                ui.end_row();
                            });
                            group_box(ui, "Page border", |ui| {
                                look::border_ui(ui, "page_border", &mut graph.page.style.border)
                            });
                        }
                        DetailTab::Misc => look::misc_tab(ui, graph),
                        DetailTab::LegendsTitles => {
                            section(ui, "Auto Legend");
                            let mut all = graph.layers.iter().all(|l| l.legend.scope == LegendScope::Graph);
                            if ui
                                .checkbox(&mut all, "Include data plots from all Layers when constructing legend")
                                .changed()
                            {
                                for l in &mut graph.layers {
                                    l.legend.scope = if all { LegendScope::Graph } else { LegendScope::Layer };
                                }
                            }
                            section(ui, "Window Title");
                            let t = &mut graph.page.style.title_display;
                            egui::ComboBox::from_id_salt("title_display").selected_text(t.name()).show_ui(ui, |ui| {
                                for d in TitleDisplay::ALL {
                                    ui.selectable_value(t, d, d.name());
                                }
                            });
                        }
                        _ => page_tab(ui, graph),
                    },
                    Node::Layer(l) => layer_tab(ui, *tab, graph, l, sources, add_from),
                    Node::Plot(l, p) => plot_tab(ui, *tab, graph, l, p, labels, columns, node),
                },
            );
        });
    });
}

/// `names` are Origin's tree names of the plots ([`sheaf_core::Project::plot_data_name`]).
fn tree(ui: &mut egui::Ui, node: &mut Node, graph: &mut Graph, names: &[Vec<String>]) {
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
                            names.get(l).and_then(|v| v.get(p)).cloned().unwrap_or_else(|| format!("Plot {}", p + 1));
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

#[allow(clippy::too_many_arguments)]
fn plot_tab(
    ui: &mut egui::Ui,
    tab: DetailTab,
    graph: &mut Graph,
    l: usize,
    p: usize,
    labels: &[Vec<String>],
    columns: &[(ColumnId, String)],
    node: &mut Node,
) {
    // In Origin's Dependent edit mode a change to one plot's look is made to the whole group.
    let before = graph.layers[l].plots[p].clone();
    plot_tab_body(ui, tab, graph, l, p, labels, columns, node);
    let follows = matches!(tab, DetailTab::Line | DetailTab::Symbol | DetailTab::DropLines | DetailTab::Label);
    if let Node::Plot(l2, p2) = *node
        && (l2, p2) == (l, p)
        && follows
        && graph.layers[l].group_style.dependent
        && graph.layers[l].plots.get(p).is_some_and(|now| *now != before)
    {
        graph.layers[l].follow_leader(p);
    }
}

#[allow(clippy::too_many_arguments)]
fn plot_tab_body(
    ui: &mut egui::Ui,
    tab: DetailTab,
    graph: &mut Graph,
    l: usize,
    p: usize,
    labels: &[Vec<String>],
    columns: &[(ColumnId, String)],
    node: &mut Node,
) {
    match tab {
        DetailTab::Line => {
            let plot = &mut graph.layers[l].plots[p];
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
                dash_ui(ui, "plot_dash", &mut plot.line.dash);
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
                let mut pct = (100.0 - f64::from(plot.line.color.a) / 2.55).round();
                if ui.add(egui::Slider::new(&mut pct, 0.0..=100.0).suffix(" %")).changed() {
                    plot.line.color.a = ((100.0 - pct) * 2.55).round() as u8;
                    plot.auto_color = false;
                }
                ui.end_row();
            });
            if plot.kind == PlotKind::LineSymbol {
                group_box(ui, "Symbol/Line interface", |ui| {
                    ui.checkbox(&mut plot.decor.symbol_line.gap_to_symbol, "Gap to Symbol");
                });
            }
            if !plot.kind.is_area() {
                let mut fill = plot.decor.fill_under.is_some();
                if ui.checkbox(&mut fill, "Fill Area Under Curve").changed() {
                    // Origin starts the fill in the line's colour.
                    plot.decor.fill_under = fill.then_some(plot.line.color);
                }
                if let Some(color) = plot.decor.fill_under.as_mut() {
                    ui.horizontal(|ui| {
                        ui.label("Fill color");
                        color_button(ui, color);
                    });
                    ui.weak("Normal: the area between the curve and the bottom of the layer.");
                    look::fill_controls(ui, "line_fill", &mut plot.decor.fill);
                }
            }
        }
        DetailTab::Offset => {
            let layer = &mut graph.layers[l];
            let individual = layer.stack == Stack::Individual;
            let column = layer.plots[p].y;
            // Read without adding an entry, so that looking at the tab changes nothing.
            let o = layer.plot_offset(column).copied().unwrap_or_else(|| PlotOffset::new(column));
            let (mut x, mut y, mut mx, mut my) = (o.x, o.y, o.x_multiplier, o.y_multiplier);
            let before = (x, y, mx, my);
            ui.add_enabled_ui(individual, |ui| {
                grid(ui, "plot_offset", |ui| {
                    ui.label("");
                    ui.strong("Multiplier");
                    ui.strong("Offset");
                    ui.end_row();
                    let label = ui.label("X");
                    ui.add(DragValue::new(&mut mx).speed(0.01)).labelled_by(label.id);
                    ui.add(DragValue::new(&mut x).speed(0.01));
                    ui.end_row();
                    let label = ui.label("Y");
                    ui.add(DragValue::new(&mut my).speed(0.01)).labelled_by(label.id);
                    ui.add(DragValue::new(&mut y).speed(0.01));
                    ui.end_row();
                });
            });
            if (x, y, mx, my) != before {
                let o = layer.plot_offset_mut(column);
                (o.x, o.y, o.x_multiplier, o.y_multiplier) = (x, y, mx, my);
            }
            if !individual {
                ui.weak("Offsets of single plots are used when the layer's Stack tab is set to Individual.");
            }
        }
        DetailTab::DropLines => {
            let plot = &mut graph.layers[l].plots[p];
            let drop = &mut plot.decor.drop;
            grid(ui, "plot_drop", |ui| {
                ui.label("");
                ui.checkbox(&mut drop.horizontal.on, "Horizontal");
                ui.checkbox(&mut drop.vertical.on, "Vertical");
                ui.end_row();
                ui.label("Style");
                for (id, d) in [("drop_h_dash", &mut drop.horizontal), ("drop_v_dash", &mut drop.vertical)] {
                    ui.add_enabled_ui(d.on, |ui| dash_ui(ui, id, &mut d.dash));
                }
                ui.end_row();
                ui.label("Width");
                for d in [&mut drop.horizontal, &mut drop.vertical] {
                    ui.add_enabled(d.on, DragValue::new(&mut d.width_pt).range(0.1..=20.0).speed(0.05).suffix(" pt"));
                }
                ui.end_row();
                ui.label("Color");
                for d in [&mut drop.horizontal, &mut drop.vertical] {
                    ui.add_enabled_ui(d.on, |ui| auto_color_ui(ui, &mut d.color));
                }
                ui.end_row();
            });
            ui.group(|ui| {
                ui.label("Data Points Display Control");
                ui.horizontal(|ui| {
                    let mut skipping = plot.symbol.every > 1;
                    if ui.checkbox(&mut skipping, "Skip Points").changed() {
                        plot.symbol.every = if skipping { 2 } else { 1 };
                    }
                    let label = ui.label("Frequency");
                    let mut frequency = plot.symbol.every.max(2);
                    if ui
                        .add_enabled(skipping, DragValue::new(&mut frequency).range(2..=u32::MAX))
                        .labelled_by(label.id)
                        .changed()
                    {
                        plot.symbol.every = frequency;
                    }
                });
                if plot.kind == PlotKind::LineSymbol {
                    ui.add_enabled(
                        plot.decor.symbol_line.gap_to_symbol && plot.symbol.every > 1,
                        egui::Checkbox::new(
                            &mut plot.decor.symbol_line.no_break_skipped,
                            "No Line Break for Skipped Point(s)",
                        ),
                    );
                }
            });
        }
        DetailTab::Label => {
            let labels = &mut graph.layers[l].plots[p].decor.labels;
            ui.checkbox(&mut labels.enable, "Enable");
            ui.add_enabled_ui(labels.enable, |ui| {
                grid(ui, "plot_label", |ui| {
                    ui.label("Font");
                    ui.text_edit_singleline(&mut labels.font.family);
                    ui.end_row();
                    ui.label("Size, style, color");
                    font_ui(ui, &mut labels.font);
                    ui.end_row();
                    ui.label("Background");
                    ui.checkbox(&mut labels.white_out, "White Out");
                    ui.end_row();
                    let label = ui.label("Frame");
                    ui.horizontal(|ui| {
                        egui::ComboBox::from_id_salt("data_label_frame")
                            .selected_text(match labels.frame {
                                LabelFrame::None => "None",
                                LabelFrame::Box => "Box",
                                LabelFrame::Shadow => "Shadow",
                            })
                            .show_ui(ui, |ui| {
                                for (frame, name) in [
                                    (LabelFrame::None, "None"),
                                    (LabelFrame::Box, "Box"),
                                    (LabelFrame::Shadow, "Shadow"),
                                ] {
                                    ui.selectable_value(&mut labels.frame, frame, name);
                                }
                            })
                            .response
                            .labelled_by(label.id);
                        ui.add_enabled_ui(!labels.frame.is_none(), |ui| {
                            ui.label("Border");
                            color_button(ui, &mut labels.frame_border);
                            ui.label("Fill");
                            color_button(ui, &mut labels.frame_fill);
                        });
                    });
                    ui.end_row();
                    let label = ui.label("Margin (% Font Height)");
                    ui.add(DragValue::new(&mut labels.margin_percent).range(0.0..=500.0).speed(0.5))
                        .labelled_by(label.id);
                    ui.end_row();
                    ui.label("Rotate (deg)");
                    ui.add(DragValue::new(&mut labels.rotate_deg).range(-360.0..=360.0).speed(1.0));
                    ui.end_row();
                    ui.label("Position");
                    egui::ComboBox::from_id_salt("label_position")
                        .selected_text(position_name(labels.position))
                        .show_ui(ui, |ui| {
                            for p in [
                                LabelPosition::Center,
                                LabelPosition::Left,
                                LabelPosition::Right,
                                LabelPosition::Above,
                                LabelPosition::Below,
                            ] {
                                ui.selectable_value(&mut labels.position, p, position_name(p));
                            }
                        });
                    ui.end_row();
                    ui.label("Label Form");
                    look::label_form_ui(ui, labels, columns);
                    ui.end_row();
                });
                look::label_extras(ui, labels);
            });
        }
        DetailTab::Symbol => {
            let plot = &mut graph.layers[l].plots[p];
            grid(ui, "plot_symbol", |ui| {
                ui.label("Shape");
                egui::ComboBox::from_id_salt("plot_symbol").selected_text(symbol_name(plot.symbol.shape)).show_ui(
                    ui,
                    |ui| {
                        for s in SymbolShape::ALL {
                            ui.selectable_value(&mut plot.symbol.shape, s, symbol_name(s));
                        }
                    },
                );
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
            });
        }
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
            group_box(ui, "Fill", |ui| {
                grid(ui, "plot_fill", |ui| {
                    ui.label("Color");
                    let before = plot.symbol.color;
                    color_button(ui, &mut plot.symbol.color);
                    if plot.symbol.color != before {
                        plot.auto_color = false;
                    }
                    ui.end_row();
                });
                look::fill_controls(ui, "plot_fill_style", &mut plot.decor.fill);
            });
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
                        // Starts at the width the gap gives.
                        bars.width_pct = set.then_some(100.0 - bars.gap_pct);
                    }
                    if let Some(w) = &mut bars.width_pct {
                        ui.add(DragValue::new(w).range(0.0..=100.0).speed(0.5));
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

/// A colour that can follow the plot's ("Auto", Origin's default for drop lines) or be set.
fn auto_color_ui(ui: &mut egui::Ui, color: &mut Option<Color>) {
    ui.horizontal(|ui| {
        let mut auto = color.is_none();
        if ui.checkbox(&mut auto, "Auto").changed() {
            *color = if auto { None } else { Some(Color::BLACK) };
        }
        if let Some(c) = color.as_mut() {
            color_button(ui, c);
        }
    });
}

/// Origin's names of the label positions.
fn position_name(p: LabelPosition) -> &'static str {
    match p {
        LabelPosition::Center => "Center",
        LabelPosition::Left => "Left",
        LabelPosition::Right => "Right",
        LabelPosition::Above => "Above",
        LabelPosition::Below => "Below",
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
            // The look settings of the other tabs go too; the fill under the curve keeps its own colour.
            q.decor = PlotDecor { fill_under: src.decor.fill_under.map(|_| color), ..src.decor.clone() };
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

    // Origin's Group tab: the plots of the layer form one group (graphing spec 18).
    section(ui, "Edit Mode");
    ui.horizontal(|ui| {
        ui.radio_value(&mut layer.group_style.dependent, false, "Independent");
        ui.radio_value(&mut layer.group_style.dependent, true, "Dependent");
    });
    let before = (layer.palette.mode, layer.group_style);
    let has_symbols = layer.plots.iter().any(|q| q.kind.has_symbols());
    egui::Grid::new("group_increments").num_columns(3).striped(true).spacing([12.0, 4.0]).show(ui, |ui| {
        ui.label("");
        ui.strong("Increment");
        ui.strong("Details");
        ui.end_row();
        ui.label(if has_symbols { "Line Color / Symbol Edge Color" } else { "Line Color" });
        egui::ComboBox::from_id_salt("inc_color")
            .selected_text(match layer.palette.mode {
                PaletteMode::Same => "None",
                PaletteMode::Cycle => "By One",
                PaletteMode::Spread => "Stretch",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut layer.palette.mode, PaletteMode::Same, "None");
                ui.selectable_value(&mut layer.palette.mode, PaletteMode::Cycle, "By One");
                ui.selectable_value(&mut layer.palette.mode, PaletteMode::Spread, "Stretch");
            });
        ui.horizontal(|ui| {
            for c in &mut layer.palette.colors {
                color_button(ui, c);
            }
            if ui.small_button("+").clicked() {
                layer.palette.colors.push(Color::BLACK);
            }
            if layer.palette.colors.len() > 1 && ui.small_button("-").clicked() {
                layer.palette.colors.pop();
            }
        });
        ui.end_row();
        let row = |ui: &mut egui::Ui, name: &str, id: &str, inc: &mut Increment, details: &str| {
            ui.label(name);
            egui::ComboBox::from_id_salt(id)
                .selected_text(if *inc == Increment::ByOne { "By One" } else { "None" })
                .show_ui(ui, |ui| {
                    ui.selectable_value(inc, Increment::None, "None");
                    ui.selectable_value(inc, Increment::ByOne, "By One");
                });
            ui.weak(details);
            ui.end_row();
        };
        row(ui, "Line Style", "inc_dash", &mut layer.group_style.line_style, "Solid, Dash, Dot, Dash Dot, ...");
        if has_symbols {
            row(ui, "Symbol Type", "inc_shape", &mut layer.group_style.symbol_type, "Square, Circle, Up Triangle, ...");
            row(ui, "Symbol Interior", "inc_fill", &mut layer.group_style.symbol_interior, "Solid, Open, ...");
        }
    });
    look::subgrouping_ui(ui, &mut layer.group_style);
    if (layer.palette.mode, layer.group_style) != before || ui.button("Recolor plots").clicked() {
        layer.apply_palette();
        layer.apply_group_increments();
    }
    section(ui, "Group Members");
    for (i, q) in layer.plots.iter().enumerate() {
        ui.horizontal(|ui| {
            crate::grid::color_swatch(ui, q.line.color);
            let name = labels[l].get(i).cloned().unwrap_or_else(|| format!("Plot {}", i + 1));
            if i == p {
                ui.strong(name)
            } else {
                ui.label(name)
            };
        });
    }
}

// ----- layer --------------------------------------------------------------------------------

fn layer_tab(ui: &mut egui::Ui, tab: DetailTab, graph: &mut Graph, l: usize, sources: &[Source], add_from: &mut usize) {
    if tab == DetailTab::Link {
        link_tab(ui, graph, l);
        return;
    }
    let link = graph.layers[l].link.map(|k| k.parent);
    // The ids that `\l(n)` and `%(n)` in the legend text refer to.
    let legend_ids = graph.legend_ids(l, &|j| graph.layers[j].plots.iter().filter(|p| p.visible).count());
    let page = (graph.page.width_pt, graph.page.height_pt);
    let layer = &mut graph.layers[l];
    match tab {
        DetailTab::Size => {
            if let Some(parent) = link {
                ui.weak(format!(
                    "This layer follows layer {}: the values are fractions of that layer's plot area.",
                    parent + 1
                ));
            }
            // Origin's Layer Area units (% of Page by default; pixel is not offered). The choice is
            // only how the numbers are shown: the frame is kept as fractions of the page.
            const UNITS: [(&str, f64); 5] =
                [("% of Page", 0.0), ("inch", 72.0), ("cm", 72.0 / 2.54), ("mm", 72.0 / 25.4), ("point", 1.0)];
            let unit_id = ui.id().with("layer_area_unit");
            let mut unit: usize = ui.data(|d| d.get_temp(unit_id)).unwrap_or(0);
            if link.is_some() {
                unit = 0;
            }
            section(ui, "Layer Area");
            grid(ui, "frame_props", |ui| {
                ui.label("Units");
                ui.add_enabled_ui(link.is_none(), |ui| {
                    egui::ComboBox::from_id_salt("layer_area_units").selected_text(UNITS[unit].0).show_ui(ui, |ui| {
                        for (k, (name, _)) in UNITS.iter().enumerate() {
                            ui.selectable_value(&mut unit, k, *name);
                        }
                    });
                });
                ui.end_row();
                let per_unit = UNITS[unit].1;
                let edit = |ui: &mut egui::Ui, label: &str, v: &mut f64, side_pt: f64| {
                    ui.label(label);
                    if per_unit == 0.0 {
                        let mut p = *v * 100.0;
                        ui.add(DragValue::new(&mut p).range(0.0..=100.0).speed(0.2).suffix(" %"));
                        *v = p / 100.0;
                    } else {
                        let mut a = *v * side_pt / per_unit;
                        let max = side_pt / per_unit;
                        if ui.add(DragValue::new(&mut a).range(0.0..=max).speed(max / 500.0).max_decimals(3)).changed()
                        {
                            *v = a * per_unit / side_pt;
                        }
                    }
                    ui.end_row();
                };
                edit(ui, "Left", &mut layer.frame.left, page.0);
                edit(ui, "Top", &mut layer.frame.top, page.1);
                edit(ui, "Width", &mut layer.frame.width, page.0);
                edit(ui, "Height", &mut layer.frame.height, page.1);
            });
            ui.data_mut(|d| d.insert_temp(unit_id, unit));
        }
        DetailTab::Stack => stack_tab(ui, layer),
        DetailTab::Display => {
            section(ui, "Show Elements");
            ui.horizontal(|ui| {
                ui.checkbox(&mut layer.display.show_x_axes, "X Axes");
                ui.checkbox(&mut layer.display.show_y_axes, "Y Axes");
                // Labels: the tick labels of both axes; Frame: the lines on the opposite edges.
                let mut labels = layer.x_axis.show_tick_labels && layer.y_axis.show_tick_labels;
                if ui.checkbox(&mut labels, "Labels").changed() {
                    layer.x_axis.show_tick_labels = labels;
                    layer.y_axis.show_tick_labels = labels;
                }
                ui.checkbox(&mut layer.display.show_data, "Data");
                let mut frame = layer.x_axis.show_opposite && layer.y_axis.show_opposite;
                if ui.checkbox(&mut frame, "Frame").changed() {
                    layer.x_axis.show_opposite = frame;
                    layer.y_axis.show_opposite = frame;
                }
            });
            section(ui, "Data Drawing Options");
            ui.checkbox(&mut layer.display.clip_data, "Clip Data to Frame");
            ui.checkbox(&mut layer.display.data_on_top, "Data on Top of Axes");
            ui.checkbox(&mut layer.display.grid_on_top, "Grid on Top of Data");
        }
        DetailTab::Background => {
            grid(ui, "layer_background", |ui| {
                ui.label("Color");
                ui.horizontal(|ui| {
                    let mut on = layer.background.is_some();
                    if ui.checkbox(&mut on, "").changed() {
                        layer.background = on.then_some(Color::WHITE);
                    }
                    match layer.background.as_mut() {
                        Some(c) => color_button(ui, c),
                        None => {
                            ui.weak("None");
                        }
                    }
                });
                ui.end_row();
                ui.label("Gradient");
                let mut on = layer.background_style.gradient.is_some();
                ui.add_enabled_ui(layer.background.is_some(), |ui| {
                    if ui.checkbox(&mut on, "Fill with a gradient").changed() {
                        layer.background_style.gradient = on.then(|| Gradient::new(Color::WHITE));
                    }
                });
                ui.end_row();
            });
            if let (Some(g), true) = (layer.background_style.gradient.as_mut(), layer.background.is_some()) {
                look::gradient_ui(ui, "bg_gradient", g);
            }
            group_box(ui, "Border", |ui| look::border_ui(ui, "bg_border", &mut layer.background_style.border));
        }
        DetailTab::Legend => legend_ui(ui, &mut layer.legend, &legend_ids),
        _ => {
            ui.label(format!(
                "{} plot(s) in this layer. Select one in the tree to change its look.",
                layer.plots.len()
            ));
            add_plot_ui(ui, layer, sources, add_from);
        }
    }
}

/// Lets the user overlay columns of any worksheet of the project on this layer.
fn add_plot_ui(ui: &mut egui::Ui, layer: &mut Layer, sources: &[Source], from: &mut usize) {
    section(ui, "Add data to this graph");
    if sources.is_empty() {
        ui.weak("The project has no numeric Y columns.");
        return;
    }
    *from = (*from).min(sources.len() - 1);
    egui::ComboBox::from_id_salt("add_plot_source").selected_text(&sources[*from].title).width(320.0).show_ui(
        ui,
        |ui| {
            for (i, s) in sources.iter().enumerate() {
                ui.selectable_value(from, i, &s.title);
            }
        },
    );
    let mut add: Vec<&super::graph_props::Addable> = Vec::new();
    ui.horizontal_wrapped(|ui| {
        for a in &sources[*from].columns {
            let already = layer.plots.iter().any(|p| p.y == a.y);
            if ui.add_enabled(!already, egui::Button::new(&a.label).small()).clicked() {
                add.push(a);
            }
        }
    });
    if ui.button("Add all columns of this sheet").clicked() {
        add = sources[*from].columns.iter().filter(|a| !layer.plots.iter().any(|p| p.y == a.y)).collect();
    }
    if !add.is_empty() {
        for a in add {
            layer.plots.push(Plot {
                x: a.x,
                y: a.y,
                y_error: a.y_error,
                rows: None,
                kind: layer.default_kind,
                line: layer.default_line.clone(),
                symbol: layer.default_symbol.clone(),
                label: None,
                visible: true,
                auto_color: true,
                group: None,
                x_error: None,
                y_base: None,
                error: ErrorStyle::default(),
                hist: Default::default(),
                size_from: None,
                color_from: None,
                decor: Default::default(),
            });
        }
        layer.recolor_automatic();
    }
}

/// Origin's Link Axes Scales tab (measured, graphing spec 18.4): Link to (None or an earlier
/// layer), then an X Axis Link and a Y Axis Link group with None, Straight (1 to 1), Align and
/// Custom. Sheaf has None and Straight; Align and Custom are shown greyed with their boxes.
fn link_tab(ui: &mut egui::Ui, graph: &mut Graph, l: usize) {
    let current = graph.layers[l].link.map(|k| k.parent);
    let name = |p: Option<usize>| p.map_or_else(|| "None".to_string(), |p| format!("Layer{}", p + 1));
    let children = graph.has_children(l);
    ui.horizontal(|ui| {
        ui.label("Link to");
        let mut chosen = current;
        ui.add_enabled_ui(!children, |ui| {
            egui::ComboBox::from_id_salt("link_to").selected_text(name(current)).width(105.0).show_ui(ui, |ui| {
                ui.selectable_value(&mut chosen, None, "None");
                // Sheaf follows one level of links: the parent must not be linked itself.
                for p in (0..l).filter(|&p| graph.layers[p].link.is_none()) {
                    ui.selectable_value(&mut chosen, Some(p), name(Some(p)));
                }
            });
        });
        if chosen != current {
            graph.set_link(l, chosen);
        }
    });
    if children {
        ui.weak("Other layers are linked to this layer, so it cannot be linked itself.");
    } else if current.is_some() && graph.link_parent(l).is_none() {
        ui.weak("The layer it is linked to is linked itself, so this link is not followed.");
    }
    ui.add_space(6.0);
    ui.label("Link Axes in Child Layer to Parent");
    let linked = graph.layers[l].link.is_some();
    let layer = &mut graph.layers[l];
    ui.columns(2, |cols| {
        for (col, axis) in cols.iter_mut().zip(["X", "Y"]) {
            group_box(col, &format!("{axis} Axis Link"), |ui| {
                ui.add_enabled_ui(linked, |ui| {
                    let mut value = layer.link.map_or(ScaleLink::None, |k| if axis == "X" { k.x } else { k.y });
                    ui.radio_value(&mut value, ScaleLink::None, "None");
                    ui.radio_value(&mut value, ScaleLink::Straight, "Straight (1 to 1)");
                    if let Some(k) = layer.link.as_mut() {
                        if axis == "X" {
                            k.x = value;
                        } else {
                            k.y = value;
                        }
                    }
                    let not_yet = "Not in Sheaf yet: the scale is a formula of the parent's";
                    ui.add_enabled(false, egui::RadioButton::new(false, "Align")).on_disabled_hover_text(not_yet);
                    let boxes = |ui: &mut egui::Ui, labels: &[String]| {
                        for label in labels {
                            ui.horizontal(|ui| {
                                ui.add_space(12.0);
                                ui.add_enabled(false, egui::Label::new(format!("{label} =")));
                                ui.add_enabled(
                                    false,
                                    egui::TextEdit::singleline(&mut String::new()).desired_width(80.0),
                                );
                            });
                        }
                    };
                    boxes(ui, &[axis.to_string()]);
                    ui.add_enabled(false, egui::RadioButton::new(false, "Custom")).on_disabled_hover_text(not_yet);
                    boxes(ui, &[format!("{axis}1"), format!("{axis}2")]);
                });
            });
        }
    });
}

/// Origin's Stack tab (measured, graphing spec 13.12): an Offset group of radio buttons with the
/// Constant step, the Auto gap (8 % when chosen here) and Individual, then the column/bar options.
/// Cumulative, Incremental and the column/bar options are greyed (stacked columns are not in Sheaf
/// yet); Waterfall is Sheaf's own.
fn stack_tab(ui: &mut egui::Ui, layer: &mut Layer) {
    let is = |s: &Stack, other: &Stack| std::mem::discriminant(s) == std::mem::discriminant(other);
    let later = "Stacked columns and areas are not in Sheaf yet";
    group_box(ui, "Offset", |ui| {
        let mut mode = layer.stack;
        let radio = |ui: &mut egui::Ui, mode: &mut Stack, choice: Stack, label: &str| {
            let r = ui.radio(is(mode, &choice), label);
            if r.clicked() && !is(mode, &choice) {
                *mode = choice;
            }
            r
        };
        radio(ui, &mut mode, Stack::None, "None");
        ui.add_enabled(false, egui::RadioButton::new(false, "Cumulative")).on_disabled_hover_text(later);
        ui.add_enabled(false, egui::RadioButton::new(false, "Incremental")).on_disabled_hover_text(later);
        ui.horizontal(|ui| {
            radio(ui, &mut mode, Stack::Constant { step: 1.0 }, "Constant");
            if let Stack::Constant { step } = &mut mode {
                ui.add(DragValue::new(step).speed(0.05).max_decimals(8))
                    .on_hover_text("The step between neighbouring curves, in axis units (Origin also takes a list)");
            }
        });
        ui.horizontal(|ui| {
            // Origin's tab starts the gap at 8 % (the stacked template uses 10 %).
            radio(ui, &mut mode, Stack::Auto { gap_percent: 8.0 }, "Auto");
            ui.add_space(24.0);
            ui.label("Gap (%)");
            match &mut mode {
                Stack::Auto { gap_percent } => {
                    ui.add(DragValue::new(gap_percent).range(0.0..=500.0).speed(0.5));
                }
                _ => {
                    ui.add_enabled(false, DragValue::new(&mut 8.0));
                }
            }
            ui.add_enabled(false, egui::Checkbox::new(&mut false, "Keep Proportion of Plot Range"));
        });
        ui.horizontal(|ui| {
            radio(ui, &mut mode, Stack::Individual, "Individual");
            ui.add_space(12.0);
            let tip = "Sheaf moves each curve by both offsets of its entry";
            ui.add_enabled(false, egui::Checkbox::new(&mut false, "X")).on_disabled_hover_text(tip);
            ui.add_enabled(false, egui::Checkbox::new(&mut true, "Y")).on_disabled_hover_text(tip);
        });
        ui.horizontal(|ui| {
            radio(ui, &mut mode, Stack::Waterfall { dx_percent: 5.0, dy_percent: 5.0 }, "Waterfall")
                .on_hover_text("Sheaf's own: a flat two-dimensional waterfall");
            if let Stack::Waterfall { dx_percent, dy_percent } = &mut mode {
                ui.label("X shift (%)");
                ui.add(DragValue::new(dx_percent).range(0.0..=100.0).speed(0.2));
                ui.label("Y shift (%)");
                ui.add(DragValue::new(dy_percent).range(0.0..=100.0).speed(0.2));
            }
        });
        layer.stack = mode;
    });
    for label in [
        "Display column/bar plot by sorted order",
        "Use Subgrouping ( in Group tab ) for Cumulative/Incremental",
        "Normalize to Percent for Cumulative/Incremental",
        "Show Connect Line on Stacked Column/Bar",
    ] {
        ui.add_enabled(false, egui::Checkbox::new(&mut false, label)).on_disabled_hover_text(later);
    }
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
                        None => layer.plot_offsets.push(PlotOffset { x, y, ..PlotOffset::new(column) }),
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

/// Origin's Print/Dimensions tab: the page size in a chosen unit (inch by default; pixel is not
/// offered), and Sheaf's page title.
fn page_tab(ui: &mut egui::Ui, graph: &mut Graph) {
    const UNITS: [(&str, f64); 4] = [("inch", 72.0), ("cm", 72.0 / 2.54), ("mm", 72.0 / 25.4), ("point", 1.0)];
    let unit_id = ui.id().with("page_unit");
    let mut unit: usize = ui.data(|d| d.get_temp(unit_id)).unwrap_or(0);
    let keep_id = ui.id().with("page_keep_layers");
    let mut keep: bool = ui.data(|d| d.get_temp(keep_id)).unwrap_or(false);
    section(ui, "Dimensions");
    grid(ui, "page_props", |ui| {
        ui.label("Units");
        egui::ComboBox::from_id_salt("page_units").selected_text(UNITS[unit].0).show_ui(ui, |ui| {
            for (k, (name, _)) in UNITS.iter().enumerate() {
                ui.selectable_value(&mut unit, k, *name);
            }
        });
        ui.end_row();
        let per = UNITS[unit].1;
        let (mut w, mut h) = (graph.page.width_pt, graph.page.height_pt);
        for (label, v) in [("Width", &mut w), ("Height", &mut h)] {
            let mut a = *v / per;
            ui.label(label);
            if ui
                .add(DragValue::new(&mut a).range(72.0 / per..=2880.0 / per).speed(1.44 / per).max_decimals(3))
                .changed()
            {
                *v = a * per;
            }
            ui.end_row();
        }
        if (w, h) != (graph.page.width_pt, graph.page.height_pt) {
            graph.resize_page(w, h, keep);
        }
        ui.label("Title");
        ui.add(egui::TextEdit::singleline(&mut graph.long_name).desired_width(220.0));
        ui.end_row();
    });
    ui.checkbox(&mut keep, "Keep the size of the layers when the page is resized")
        .on_hover_text("Off: the layers grow and shrink with the page");
    ui.data_mut(|d| {
        d.insert_temp(unit_id, unit);
        d.insert_temp(keep_id, keep);
    });
}

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
