//! The graph settings window: axes, plots, legend and page.
//!
//! Every change is applied to the project at once, so the figure behind the
//! window always shows the current settings. All changes made while the
//! window is open form one undo step.

use eframe::egui::{self, Color32, DragValue, RichText};
use sheaf_core::graph::*;
use sheaf_core::{Color, Document, WindowId};
use sheaf_plot::ticks::ResolvedAxis;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PropsTab {
    XAxis,
    YAxis,
    Plots,
    Legend,
    Page,
}

pub struct GraphPropsDialog {
    pub graph: WindowId,
    pub tab: PropsTab,
    /// Groups all edits of this dialog session into one undo step.
    token: u64,
    selected_plot: usize,
}

impl GraphPropsDialog {
    pub fn new(graph: WindowId, tab: PropsTab, token: u64) -> Self {
        GraphPropsDialog { graph, tab, token, selected_plot: 0 }
    }

    /// Draws the window. Returns `false` when it was closed.
    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> bool {
        let Some(original) = doc.project().graph(self.graph) else {
            return false;
        };
        let mut graph = original.clone();
        let resolved = sheaf_plot::layout_geometry(doc.project(), self.graph).ok().and_then(|g| g.into_iter().next());
        let labels: Vec<String> = graph
            .layers
            .first()
            .map(|l| l.plots.iter().map(|p| doc.project().plot_label(p)).collect())
            .unwrap_or_default();

        let mut open = true;
        egui::Window::new(format!("Graph settings: {}", graph.name))
            .id(egui::Id::new("graph_props"))
            .open(&mut open)
            .default_width(440.0)
            .resizable(true)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for (tab, name) in [
                        (PropsTab::XAxis, "X axis"),
                        (PropsTab::YAxis, "Y axis"),
                        (PropsTab::Plots, "Plots"),
                        (PropsTab::Legend, "Legend"),
                        (PropsTab::Page, "Page"),
                    ] {
                        ui.selectable_value(&mut self.tab, tab, name);
                    }
                });
                ui.separator();
                egui::ScrollArea::vertical().max_height(520.0).show(ui, |ui| {
                    let Some(layer) = graph.layers.first_mut() else {
                        ui.label("This graph has no layer.");
                        return;
                    };
                    match self.tab {
                        PropsTab::XAxis => axis_ui(ui, &mut layer.x_axis, resolved.as_ref().map(|g| &g.x), "x"),
                        PropsTab::YAxis => axis_ui(ui, &mut layer.y_axis, resolved.as_ref().map(|g| &g.y), "y"),
                        PropsTab::Plots => plots_ui(ui, layer, &labels, &mut self.selected_plot),
                        PropsTab::Legend => legend_ui(ui, &mut layer.legend),
                        PropsTab::Page => page_ui(ui, &mut graph),
                    }
                });
            });

        if graph != *original {
            let id = self.graph;
            doc.apply_merging("Change graph settings", self.token, move |p| {
                if let Some(g) = p.graph_mut(id) {
                    *g = graph;
                }
            });
        }
        open
    }
}

fn grid<R>(ui: &mut egui::Ui, id: &str, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Grid::new(id).num_columns(2).spacing([14.0, 6.0]).show(ui, add).inner
}

fn section(ui: &mut egui::Ui, title: &str) {
    ui.add_space(8.0);
    ui.label(RichText::new(title).strong());
}

/// A number field that can be switched between "automatic" and a fixed value.
fn optional_number(ui: &mut egui::Ui, value: &mut Option<f64>, auto_value: Option<f64>, speed: f64) {
    ui.horizontal(|ui| {
        let mut auto = value.is_none();
        if ui.checkbox(&mut auto, "Auto").changed() {
            *value = if auto { None } else { Some(auto_value.unwrap_or(0.0)) };
        }
        match value {
            Some(v) => {
                ui.add(DragValue::new(v).speed(speed).max_decimals(8));
            }
            None => {
                let shown = auto_value.map(|v| sheaf_core::format::format_general(v, 6)).unwrap_or_default();
                ui.weak(shown);
            }
        }
    });
}

pub fn color_button(ui: &mut egui::Ui, color: &mut Color) {
    let mut c = Color32::from_rgba_unmultiplied(color.r, color.g, color.b, color.a);
    if ui.color_edit_button_srgba(&mut c).changed() {
        let [r, g, b, a] = c.to_srgba_unmultiplied();
        *color = Color::rgba(r, g, b, a);
    }
}

fn font_ui(ui: &mut egui::Ui, font: &mut Font) {
    ui.horizontal(|ui| {
        ui.add(DragValue::new(&mut font.size_pt).range(4.0..=96.0).speed(0.25).suffix(" pt"));
        ui.checkbox(&mut font.bold, "Bold");
        ui.checkbox(&mut font.italic, "Italic");
        color_button(ui, &mut font.color);
    });
}

fn tick_direction_ui(ui: &mut egui::Ui, id: &str, dir: &mut TickDirection) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(match dir {
            TickDirection::In => "In",
            TickDirection::Out => "Out",
            TickDirection::Both => "In and out",
            TickDirection::None => "None",
        })
        .show_ui(ui, |ui| {
            ui.selectable_value(dir, TickDirection::In, "In");
            ui.selectable_value(dir, TickDirection::Out, "Out");
            ui.selectable_value(dir, TickDirection::Both, "In and out");
            ui.selectable_value(dir, TickDirection::None, "None");
        });
}

fn grid_line_ui(ui: &mut egui::Ui, label: &str, line: &mut Option<LineStyle>) {
    ui.horizontal(|ui| {
        let mut on = line.is_some();
        if ui.checkbox(&mut on, label).changed() {
            *line = on.then(|| LineStyle {
                color: Color::rgb(200, 200, 200),
                width_pt: 0.5,
                dash: Dash::Solid,
                connect: Connect::Straight,
            });
        }
        if let Some(l) = line {
            color_button(ui, &mut l.color);
            ui.add(DragValue::new(&mut l.width_pt).range(0.1..=5.0).speed(0.05).suffix(" pt"));
            dash_ui(ui, &format!("{label}_dash"), &mut l.dash);
        }
    });
}

fn dash_name(d: Dash) -> &'static str {
    match d {
        Dash::Solid => "Solid",
        Dash::Dash => "Dash",
        Dash::Dot => "Dot",
        Dash::DashDot => "Dash dot",
        Dash::DashDotDot => "Dash dot dot",
        Dash::ShortDash => "Short dash",
        Dash::ShortDot => "Short dot",
        Dash::ShortDashDot => "Short dash dot",
    }
}

fn dash_ui(ui: &mut egui::Ui, id: &str, dash: &mut Dash) {
    egui::ComboBox::from_id_salt(id).selected_text(dash_name(*dash)).show_ui(ui, |ui| {
        for d in Dash::ALL {
            ui.selectable_value(dash, d, dash_name(d));
        }
    });
}

fn axis_ui(ui: &mut egui::Ui, axis: &mut Axis, resolved: Option<&ResolvedAxis>, id: &str) {
    section(ui, "Title");
    grid(ui, &format!("{id}_title"), |ui| {
        ui.label("Text");
        ui.horizontal(|ui| {
            let mut auto = axis.title.is_none();
            if ui.checkbox(&mut auto, "From column").on_hover_text("Long name and units of the plotted column").changed()
            {
                axis.title = if auto { None } else { Some(String::new()) };
            }
            if let Some(t) = &mut axis.title {
                ui.add(egui::TextEdit::singleline(t).desired_width(190.0));
            }
        });
        ui.end_row();
        ui.label("Show");
        ui.checkbox(&mut axis.show_title, "");
        ui.end_row();
        ui.label("Font");
        font_ui(ui, &mut axis.title_font);
        ui.end_row();
    });
    ui.weak("Markup: \\+(superscript)  \\-(subscript)  \\g(greek)  \\b(bold)  \\i(italic)");

    section(ui, "Scale");
    grid(ui, &format!("{id}_scale"), |ui| {
        ui.label("Type");
        ui.horizontal(|ui| {
            ui.selectable_value(&mut axis.scale, ScaleKind::Linear, "Linear");
            ui.selectable_value(&mut axis.scale, ScaleKind::Log10, "Log10");
        });
        ui.end_row();
        let span = resolved.map(|r| (r.max - r.min).abs()).unwrap_or(1.0);
        let speed = (span / 200.0).max(1e-12);
        ui.label("From");
        optional_number(ui, &mut axis.min, resolved.map(|r| r.min), speed);
        ui.end_row();
        ui.label("To");
        optional_number(ui, &mut axis.max, resolved.map(|r| r.max), speed);
        ui.end_row();
        ui.label("Automatic range");
        egui::ComboBox::from_id_salt(format!("{id}_auto"))
            .selected_text(match axis.auto_range {
                AutoRange::Nice => "Round out to ticks",
                AutoRange::Tight => "Exact data range",
                AutoRange::SymmetricZero => "Symmetric about zero",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut axis.auto_range, AutoRange::Nice, "Round out to ticks");
                ui.selectable_value(&mut axis.auto_range, AutoRange::Tight, "Exact data range");
                ui.selectable_value(&mut axis.auto_range, AutoRange::SymmetricZero, "Symmetric about zero");
            });
        ui.end_row();
        ui.label("Major tick step");
        optional_number(ui, &mut axis.major_step, resolved.map(|r| r.step), speed);
        ui.end_row();
        ui.label("Minor ticks between");
        ui.add(DragValue::new(&mut axis.minor_ticks).range(0..=9));
        ui.end_row();
    });

    section(ui, "Ticks and labels");
    grid(ui, &format!("{id}_ticks"), |ui| {
        ui.label("Major ticks");
        ui.horizontal(|ui| {
            tick_direction_ui(ui, &format!("{id}_major_dir"), &mut axis.major_tick);
            ui.add(DragValue::new(&mut axis.major_tick_len_pt).range(0.0..=30.0).speed(0.1).suffix(" pt"));
        });
        ui.end_row();
        ui.label("Minor ticks");
        ui.horizontal(|ui| {
            tick_direction_ui(ui, &format!("{id}_minor_dir"), &mut axis.minor_tick);
            ui.add(DragValue::new(&mut axis.minor_tick_len_pt).range(0.0..=30.0).speed(0.1).suffix(" pt"));
        });
        ui.end_row();
        ui.label("Tick labels");
        ui.checkbox(&mut axis.show_tick_labels, "Show");
        ui.end_row();
        ui.label("Label font");
        font_ui(ui, &mut axis.tick_font);
        ui.end_row();
        ui.label("Number format");
        ui.horizontal(|ui| {
            let mut kind = match axis.tick_format {
                TickFormat::Auto => 0,
                TickFormat::Decimal(_) => 1,
                TickFormat::Scientific(_) => 2,
            };
            let digits = match axis.tick_format {
                TickFormat::Decimal(d) | TickFormat::Scientific(d) => d,
                TickFormat::Auto => 2,
            };
            let before = kind;
            egui::ComboBox::from_id_salt(format!("{id}_fmt"))
                .selected_text(["Automatic", "Decimal", "Scientific"][kind])
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut kind, 0, "Automatic");
                    ui.selectable_value(&mut kind, 1, "Decimal");
                    ui.selectable_value(&mut kind, 2, "Scientific");
                });
            let mut d = digits;
            if kind != 0 {
                ui.add(DragValue::new(&mut d).range(0..=10).suffix(" digits"));
            }
            if kind != before || d != digits {
                axis.tick_format = match kind {
                    1 => TickFormat::Decimal(d),
                    2 => TickFormat::Scientific(d),
                    _ => TickFormat::Auto,
                };
            }
        });
        ui.end_row();
        ui.label("Divide labels by");
        ui.add(DragValue::new(&mut axis.tick_label_divisor).speed(0.1).max_decimals(9))
            .on_hover_text("Tick labels show the value divided by this factor, e.g. 0.001 to show A as mA");
        if axis.tick_label_divisor == 0.0 {
            axis.tick_label_divisor = 1.0;
        }
        ui.end_row();
    });

    section(ui, "Lines");
    grid(ui, &format!("{id}_lines"), |ui| {
        ui.label("Axis line");
        ui.horizontal(|ui| {
            ui.add(DragValue::new(&mut axis.line_width_pt).range(0.0..=10.0).speed(0.05).suffix(" pt"));
            color_button(ui, &mut axis.color);
        });
        ui.end_row();
        ui.label("Opposite side");
        ui.horizontal(|ui| {
            ui.checkbox(&mut axis.show_opposite, "Line").on_hover_text("With both axes: a closed box around the plot");
            ui.checkbox(&mut axis.opposite_ticks, "Ticks");
        });
        ui.end_row();
        ui.label("Grid");
        ui.vertical(|ui| {
            grid_line_ui(ui, "Major", &mut axis.major_grid);
            grid_line_ui(ui, "Minor", &mut axis.minor_grid);
        });
        ui.end_row();
    });
}

fn plots_ui(ui: &mut egui::Ui, layer: &mut Layer, labels: &[String], selected: &mut usize) {
    if layer.plots.is_empty() {
        ui.label("This graph has no plots yet. Select Y columns in a workbook and choose Plot.");
        return;
    }
    *selected = (*selected).min(layer.plots.len() - 1);

    ui.horizontal_wrapped(|ui| {
        for (i, plot) in layer.plots.iter().enumerate() {
            let name = labels.get(i).cloned().unwrap_or_else(|| format!("Plot {}", i + 1));
            ui.horizontal(|ui| {
                crate::grid::color_swatch(ui, plot.line.color);
                ui.selectable_value(selected, i, name);
            });
        }
    });
    ui.separator();

    let count = layer.plots.len();
    let mut apply_to_all = None;
    let mut remove = false;
    let mut move_by: i32 = 0;
    {
        let plot = &mut layer.plots[*selected];
        grid(ui, "plot_props", |ui| {
            ui.label("Type");
            ui.horizontal(|ui| {
                ui.selectable_value(&mut plot.kind, PlotKind::Line, "Line");
                ui.selectable_value(&mut plot.kind, PlotKind::Scatter, "Scatter");
                ui.selectable_value(&mut plot.kind, PlotKind::LineSymbol, "Line + symbol");
            });
            ui.end_row();
            ui.label("Visible");
            ui.checkbox(&mut plot.visible, "");
            ui.end_row();
            ui.label("Legend text");
            ui.horizontal(|ui| {
                let mut auto = plot.label.is_none();
                if ui.checkbox(&mut auto, "From column").changed() {
                    plot.label = if auto { None } else { Some(labels.get(*selected).cloned().unwrap_or_default()) };
                }
                if let Some(l) = &mut plot.label {
                    ui.add(egui::TextEdit::singleline(l).desired_width(170.0));
                }
            });
            ui.end_row();
            ui.label("Color");
            ui.horizontal(|ui| {
                let before = plot.line.color;
                color_button(ui, &mut plot.line.color);
                if plot.line.color != before {
                    plot.symbol.color = plot.line.color;
                }
            });
            ui.end_row();
            if plot.kind.has_line() {
                ui.label("Line");
                ui.horizontal(|ui| {
                    ui.add(DragValue::new(&mut plot.line.width_pt).range(0.1..=20.0).speed(0.05).suffix(" pt"));
                    dash_ui(ui, "plot_dash", &mut plot.line.dash);
                });
                ui.end_row();
                ui.label("Connect");
                egui::ComboBox::from_id_salt("plot_connect")
                    .selected_text(connect_name(plot.line.connect))
                    .show_ui(ui, |ui| {
                        for c in [Connect::Straight, Connect::Spline, Connect::StepHorizontal, Connect::StepVertical] {
                            ui.selectable_value(&mut plot.line.connect, c, connect_name(c));
                        }
                    });
                ui.end_row();
            }
            if plot.kind.has_symbols() {
                ui.label("Symbol");
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt("plot_symbol")
                        .selected_text(symbol_name(plot.symbol.shape))
                        .show_ui(ui, |ui| {
                            for s in SymbolShape::ALL {
                                ui.selectable_value(&mut plot.symbol.shape, s, symbol_name(s));
                            }
                        });
                    ui.add(DragValue::new(&mut plot.symbol.size_pt).range(0.5..=40.0).speed(0.1).suffix(" pt"));
                    ui.checkbox(&mut plot.symbol.filled, "Filled");
                });
                ui.end_row();
                ui.label("Show every");
                ui.add(DragValue::new(&mut plot.symbol.every).range(1..=100000).suffix(" th point"));
                ui.end_row();
            }
        });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if count > 1 && ui.button("Use this style for all plots").on_hover_text("Keeps each plot's color").clicked() {
                apply_to_all = Some(plot.clone());
            }
            if ui.add_enabled(*selected > 0, egui::Button::new("Move up")).clicked() {
                move_by = -1;
            }
            if ui.add_enabled(*selected + 1 < count, egui::Button::new("Move down")).clicked() {
                move_by = 1;
            }
            if ui.button("Remove plot").clicked() {
                remove = true;
            }
        });
    }
    if let Some(src) = apply_to_all {
        for p in &mut layer.plots {
            let color = p.line.color;
            p.kind = src.kind;
            p.line = LineStyle { color, ..src.line.clone() };
            p.symbol = SymbolStyle { color, ..src.symbol.clone() };
        }
    }
    if move_by != 0 {
        let j = (*selected as i32 + move_by) as usize;
        layer.plots.swap(*selected, j);
        *selected = j;
    }
    if remove {
        layer.plots.remove(*selected);
        *selected = selected.saturating_sub(1);
    }

    section(ui, "Colors of this graph");
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
    ui.horizontal(|ui| {
        ui.selectable_value(&mut layer.palette.mode, PaletteMode::Cycle, "One color per plot")
            .on_hover_text("Plot 1 takes color 1, plot 2 color 2, and so on");
        ui.selectable_value(&mut layer.palette.mode, PaletteMode::Spread, "Gradient")
            .on_hover_text("Colors run from the first to the last color over all plots");
        if ui.button("Recolor plots").clicked() {
            layer.apply_palette();
        }
    });
}

fn connect_name(c: Connect) -> &'static str {
    match c {
        Connect::Straight => "Straight",
        Connect::Spline => "Smooth (spline)",
        Connect::StepHorizontal => "Step, horizontal first",
        Connect::StepVertical => "Step, vertical first",
    }
}

fn symbol_name(s: SymbolShape) -> &'static str {
    match s {
        SymbolShape::Square => "Square",
        SymbolShape::Circle => "Circle",
        SymbolShape::TriangleUp => "Triangle up",
        SymbolShape::TriangleDown => "Triangle down",
        SymbolShape::Diamond => "Diamond",
        SymbolShape::Plus => "Plus",
        SymbolShape::Cross => "Cross",
        SymbolShape::Star => "Star",
    }
}

fn legend_ui(ui: &mut egui::Ui, legend: &mut Legend) {
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

fn page_ui(ui: &mut egui::Ui, graph: &mut Graph) {
    section(ui, "Page");
    grid(ui, "page_props", |ui| {
        let mut w = graph.page.width_pt / 72.0;
        let mut h = graph.page.height_pt / 72.0;
        ui.label("Width");
        ui.add(DragValue::new(&mut w).range(1.0..=40.0).speed(0.02).suffix(" in"));
        ui.end_row();
        ui.label("Height");
        ui.add(DragValue::new(&mut h).range(1.0..=40.0).speed(0.02).suffix(" in"));
        ui.end_row();
        graph.page.width_pt = w * 72.0;
        graph.page.height_pt = h * 72.0;
        ui.label("Background");
        color_button(ui, &mut graph.page.background);
        ui.end_row();
        ui.label("Title");
        ui.add(egui::TextEdit::singleline(&mut graph.long_name).desired_width(220.0));
        ui.end_row();
    });

    let Some(layer) = graph.layers.first_mut() else {
        return;
    };
    section(ui, "Plot area (percent of the page)");
    grid(ui, "frame_props", |ui| {
        let pct = |ui: &mut egui::Ui, label: &str, v: &mut f64| {
            let mut p = *v * 100.0;
            ui.label(label);
            ui.add(DragValue::new(&mut p).range(0.0..=100.0).speed(0.2).suffix(" %"));
            ui.end_row();
            *v = p / 100.0;
        };
        pct(ui, "Left", &mut layer.frame.left);
        pct(ui, "Top", &mut layer.frame.top);
        pct(ui, "Width", &mut layer.frame.width);
        pct(ui, "Height", &mut layer.frame.height);
    });

    section(ui, "All text");
    ui.horizontal(|ui| {
        ui.label("Font family");
        let mut family = layer.x_axis.title_font.family.clone();
        if ui.add(egui::TextEdit::singleline(&mut family).desired_width(160.0)).changed() {
            for f in [
                &mut layer.x_axis.title_font,
                &mut layer.x_axis.tick_font,
                &mut layer.y_axis.title_font,
                &mut layer.y_axis.tick_font,
                &mut layer.legend.font,
            ] {
                f.family = family.clone();
            }
        }
    });
    ui.horizontal(|ui| {
        ui.label("Scale all sizes");
        for (label, factor) in [("-10 %", 0.9), ("+10 %", 1.1)] {
            if ui.button(label).clicked() {
                for f in [
                    &mut layer.x_axis.title_font,
                    &mut layer.x_axis.tick_font,
                    &mut layer.y_axis.title_font,
                    &mut layer.y_axis.tick_font,
                    &mut layer.legend.font,
                ] {
                    f.size_pt = (f.size_pt * factor * 4.0).round() / 4.0;
                }
            }
        }
    });
}
