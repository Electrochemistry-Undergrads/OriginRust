//! The layer tabs (Background, Size, Display, Link Axes Scales) and the page tabs (Print/Dimensions,
//! Legends/Titles, Miscellaneous, Display) of Plot Details. Each control writes a field of the model
//! (`sheaf_core::graph_style`) that the renderer or the graph window uses.

use eframe::egui::{self, DragValue};
use sheaf_core::Color;
use sheaf_core::graph::*;

use super::details_plot::fill_controls;
use super::graph_props::{color_button, dash_ui, grid, group_box};

fn border_ui(ui: &mut egui::Ui, id: &str, border: &mut Option<BorderStyle>) {
    let mut on = border.is_some();
    if ui.checkbox(&mut on, "Draw a border line").changed() {
        *border = on.then(|| BorderStyle::solid(Color::BLACK, 1.0));
    }
    if let Some(b) = border {
        grid(ui, id, |ui| {
            ui.label("Color");
            color_button(ui, &mut b.color);
            ui.end_row();
            ui.label("Style");
            dash_ui(ui, &format!("{id}_dash"), &mut b.dash);
            ui.end_row();
            ui.label("Width");
            ui.add(DragValue::new(&mut b.width_pt).range(0.0..=20.0).speed(0.05).suffix(" pt"));
            ui.end_row();
        });
    }
}

pub fn background_tab(ui: &mut egui::Ui, layer: &mut Layer) {
    let bg = &mut layer.background;
    group_box(ui, "Fill", |ui| {
        let mut on = bg.color.is_some();
        if ui.checkbox(&mut on, "Fill the area behind the plots").changed() {
            bg.color = on.then_some(Color::WHITE);
        }
        if let Some(color) = &mut bg.color {
            let mut style =
                FillStyle { transparency_pct: bg.transparency_pct, gradient: bg.gradient, ..Default::default() };
            grid(ui, "bg_fill", |ui| {
                ui.label("Color");
                color_button(ui, color);
                ui.end_row();
                ui.label("Transparency");
                ui.add(DragValue::new(&mut style.transparency_pct).range(0..=100).suffix(" %"));
                ui.end_row();
                let mut g = style.gradient.is_some();
                ui.label("Gradient");
                if ui.checkbox(&mut g, "Fill with a gradient").changed() {
                    style.gradient = g.then(|| Gradient::new(Color::WHITE));
                }
                ui.end_row();
                if let Some(grad) = &mut style.gradient {
                    ui.label("Color at the end");
                    color_button(ui, &mut grad.color);
                    ui.end_row();
                    ui.label("Direction");
                    egui::ComboBox::from_id_salt("bg_direction").selected_text(grad.direction.name()).show_ui(
                        ui,
                        |ui| {
                            for d in GradientDirection::ALL {
                                ui.selectable_value(&mut grad.direction, d, d.name());
                            }
                        },
                    );
                    ui.end_row();
                }
            });
            bg.transparency_pct = style.transparency_pct;
            bg.gradient = style.gradient;
        } else {
            bg.transparency_pct = 0;
            bg.gradient = None;
        }
    });
    group_box(ui, "Border", |ui| border_ui(ui, "bg_border", &mut bg.border));
}

pub fn display_tab(ui: &mut egui::Ui, layer: &mut Layer) {
    let d = &mut layer.display;
    ui.checkbox(&mut d.show_layer, "Show the layer")
        .on_hover_text("A hidden layer draws nothing but keeps its place, so layers linked to it do not move");
    ui.add_enabled_ui(d.show_layer, |ui| {
        ui.checkbox(&mut d.show_plots, "Show the plots");
        ui.checkbox(&mut d.show_axes, "Show the axes (lines, ticks, tick labels and titles)");
        ui.checkbox(&mut d.show_labels, "Show text labels and drawn objects");
    });
}

pub fn size_tab(ui: &mut egui::Ui, graph: &mut Graph, l: usize) {
    let units = graph.page.style.units;
    let (pw, ph) = (graph.page.width_pt, graph.page.height_pt);
    let linked = graph.layers[l].link.map(|k| k.parent);
    let layer = &mut graph.layers[l];
    match linked {
        Some(parent) => {
            ui.weak(format!(
                "This layer follows layer {}: the values are fractions of that layer's plot area.",
                parent + 1
            ));
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
        }
        None => {
            ui.weak(format!("Size and place of the plot area on the page, in {}.", units.name()));
            grid(ui, "frame_props", |ui| {
                let row = |ui: &mut egui::Ui, label: &str, frac: &mut f64, whole_pt: f64| {
                    let mut v = units.from_pt(*frac * whole_pt);
                    ui.label(label);
                    let speed = units.from_pt(0.5);
                    if ui
                        .add(DragValue::new(&mut v).range(0.0..=units.from_pt(whole_pt)).speed(speed).max_decimals(3))
                        .changed()
                    {
                        *frac = units.to_pt(v) / whole_pt;
                    }
                    ui.end_row();
                };
                row(ui, "Left", &mut layer.frame.left, pw);
                row(ui, "Top", &mut layer.frame.top, ph);
                row(ui, "Width", &mut layer.frame.width, pw);
                row(ui, "Height", &mut layer.frame.height, ph);
            });
        }
    }
}

fn scale_link_name(s: ScaleLink) -> &'static str {
    match s {
        ScaleLink::None => "Independent (its own scale)",
        ScaleLink::Straight => "Straight (1 to 1)",
    }
}

pub fn link_tab(ui: &mut egui::Ui, graph: &mut Graph, l: usize) {
    let parents = graph.linkable_parents(l);
    let current = graph.layers[l].link.map(|k| k.parent);
    if parents.is_empty() && current.is_none() {
        ui.weak(if l == 0 {
            "The first layer has no layer to follow."
        } else {
            "No layer can be followed: another layer already follows this one."
        });
        return;
    }
    let mut want = current;
    grid(ui, "link_parent", |ui| {
        ui.label("Link to");
        egui::ComboBox::from_id_salt("link_parent_box")
            .selected_text(match want {
                Some(p) => format!("Layer {}", p + 1),
                None => "None".to_string(),
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut want, None, "None");
                for p in &parents {
                    ui.selectable_value(&mut want, Some(*p), format!("Layer {}", p + 1));
                }
            });
        ui.end_row();
    });
    if want != current {
        graph.set_layer_link(l, want);
    }
    if let Some(link) = &mut graph.layers[l].link {
        ui.add_space(6.0);
        grid(ui, "link_scales", |ui| {
            for (label, scale) in [("X Axis Scale", &mut link.x), ("Y Axis Scale", &mut link.y)] {
                ui.label(label);
                egui::ComboBox::from_id_salt(label).selected_text(scale_link_name(*scale)).width(190.0).show_ui(
                    ui,
                    |ui| {
                        for s in [ScaleLink::None, ScaleLink::Straight] {
                            ui.selectable_value(scale, s, scale_link_name(s));
                        }
                    },
                );
                ui.end_row();
            }
        });
        ui.weak("A straight link takes the limits, type, ticks and direction of the parent's axis.");
    }
}

// ----- page -----------------------------------------------------------------------------------

/// Print/Dimensions: page size in the page unit, and whether layers keep their size.
pub fn dimensions_tab(ui: &mut egui::Ui, graph: &mut Graph, keep_layers: &mut bool) {
    let units = &mut graph.page.style.units;
    grid(ui, "page_units", |ui| {
        ui.label("Units");
        egui::ComboBox::from_id_salt("page_unit_box").selected_text(units.name()).show_ui(ui, |ui| {
            for u in LengthUnit::ALL {
                ui.selectable_value(units, u, u.name());
            }
        });
        ui.end_row();
    });
    let unit = graph.page.style.units;
    let (mut w, mut h) = (unit.from_pt(graph.page.width_pt), unit.from_pt(graph.page.height_pt));
    let (w0, h0) = (w, h);
    let (lo, hi) = (unit.from_pt(72.0), unit.from_pt(72.0 * 40.0));
    grid(ui, "page_size", |ui| {
        ui.label("Width");
        ui.add(DragValue::new(&mut w).range(lo..=hi).speed(unit.from_pt(1.0)).max_decimals(3));
        ui.end_row();
        ui.label("Height");
        ui.add(DragValue::new(&mut h).range(lo..=hi).speed(unit.from_pt(1.0)).max_decimals(3));
        ui.end_row();
    });
    ui.checkbox(keep_layers, "Keep the size of the layers when the page is resized")
        .on_hover_text("Off: the layers grow and shrink with the page");
    if (w, h) != (w0, h0) {
        graph.resize_page(unit.to_pt(w), unit.to_pt(h), *keep_layers);
    }
}

/// Legends/Titles: the title of the graph and what its window shows.
pub fn titles_tab(ui: &mut egui::Ui, graph: &mut Graph) {
    grid(ui, "page_titles", |ui| {
        ui.label("Long name");
        ui.add(egui::TextEdit::singleline(&mut graph.long_name).desired_width(220.0));
        ui.end_row();
        ui.label("Window title shows");
        let t = &mut graph.page.style.title_display;
        egui::ComboBox::from_id_salt("title_display").selected_text(t.name()).show_ui(ui, |ui| {
            for d in TitleDisplay::ALL {
                ui.selectable_value(t, d, d.name());
            }
        });
        ui.end_row();
    });
    ui.add_space(6.0);
    let mut shown = graph.layers.iter().all(|l| l.legend.visible);
    if ui.checkbox(&mut shown, "Show the legend of every layer").changed() {
        for l in &mut graph.layers {
            l.legend.visible = shown;
        }
    }
}

pub fn misc_tab(ui: &mut egui::Ui, graph: &mut Graph) {
    ui.label("Comments");
    ui.add(egui::TextEdit::multiline(&mut graph.page.style.comments).desired_rows(6).desired_width(380.0));
    ui.weak("Shown when the pointer rests on the graph's name in the Object Manager.");
}

pub fn page_display_tab(ui: &mut egui::Ui, graph: &mut Graph) {
    grid(ui, "page_display", |ui| {
        ui.label("Background");
        color_button(ui, &mut graph.page.background);
        ui.end_row();
    });
    group_box(ui, "Page border", |ui| border_ui(ui, "page_border", &mut graph.page.style.border));
    ui.checkbox(&mut graph.page.style.layer_icons, "Show the layer icons")
        .on_hover_text("The small numbered buttons at the top left of the graph window");
}

/// Used by the Pattern tab of a plot and the Line tab's fill under the curve.
pub fn plot_fill(ui: &mut egui::Ui, id: &str, plot: &mut Plot) {
    let mut color = plot.symbol.color;
    let mut style = plot.details.fill;
    if fill_controls(ui, id, &mut style, &mut color) {
        plot.symbol.color = color;
        plot.auto_color = false;
    }
    plot.details.fill = style;
}
