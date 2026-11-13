//! The body of the axis dialog: a list at the left to choose the horizontal or the vertical axis of
//! the layer, and the tabs Scale, Tick Labels, Title, Grids, Line and Ticks, Special Ticks and
//! Breaks (the last two are shown grey: Sheaf has no special ticks and no axis breaks yet).

use eframe::egui::{self, DragValue};
use sheaf_core::graph::*;
use sheaf_core::{Color, ColumnId};
use sheaf_plot::ticks::ResolvedAxis;

use super::graph_props::{
    color_button, dash_ui, font_ui, grid, group_box, optional_number, tab_row, tick_direction_ui,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AxisTab {
    Scale,
    TickLabels,
    Title,
    Grids,
    LineAndTicks,
    SpecialTicks,
    Breaks,
}

impl AxisTab {
    /// Origin's order, with the ones Sheaf cannot do yet marked as not available.
    const ALL: [(AxisTab, &'static str, bool); 7] = [
        (AxisTab::Scale, "Scale", true),
        (AxisTab::TickLabels, "Tick Labels", true),
        (AxisTab::Title, "Title", true),
        (AxisTab::Grids, "Grids", true),
        (AxisTab::LineAndTicks, "Line and Ticks", true),
        (AxisTab::SpecialTicks, "Special Ticks", false),
        (AxisTab::Breaks, "Breaks", false),
    ];
}

/// The list at the left, the tab row and the page of the chosen tab.
pub fn dialog_body(
    ui: &mut egui::Ui,
    vertical: &mut bool,
    tab: &mut AxisTab,
    layer: &mut Layer,
    resolved: Option<&ResolvedAxis>,
    columns: &[(ColumnId, String)],
) {
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.set_width(92.0);
            for (is_vertical, name) in [(false, "Horizontal"), (true, "Vertical")] {
                let button = egui::Button::selectable(*vertical == is_vertical, name);
                if ui.add_sized([88.0, 30.0], button).clicked() {
                    *vertical = is_vertical;
                }
            }
        });
        ui.separator();
        ui.vertical(|ui| {
            tab_row(ui, tab, &AxisTab::ALL);
            ui.separator();
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                let (axis, id) = if *vertical { (&mut layer.y_axis, "y") } else { (&mut layer.x_axis, "x") };
                match *tab {
                    AxisTab::Scale => scale_tab(ui, axis, resolved, id),
                    AxisTab::TickLabels => tick_labels_tab(ui, axis, *vertical, columns, id),
                    AxisTab::Title => title_tab(ui, axis, id),
                    AxisTab::Grids => grids_tab(ui, axis, id),
                    AxisTab::LineAndTicks => line_and_ticks_tab(ui, axis, *vertical, id),
                    AxisTab::SpecialTicks | AxisTab::Breaks => {
                        ui.weak("Not available in this version.");
                    }
                }
            });
        });
    });
}

fn scale_tab(ui: &mut egui::Ui, axis: &mut Axis, resolved: Option<&ResolvedAxis>, id: &str) {
    let span = resolved.map(|r| (r.max - r.min).abs()).unwrap_or(1.0);
    let speed = (span / 200.0).max(1e-12);
    grid(ui, &format!("{id}_scale"), |ui| {
        ui.label("From");
        optional_number(ui, &mut axis.min, resolved.map(|r| r.min), speed);
        ui.end_row();
        ui.label("To");
        optional_number(ui, &mut axis.max, resolved.map(|r| r.max), speed);
        ui.end_row();
        ui.label("Type");
        egui::ComboBox::from_id_salt(format!("{id}_type")).selected_text(axis.scale.label()).show_ui(ui, |ui| {
            for kind in ScaleKind::ALL {
                ui.selectable_value(&mut axis.scale, kind, kind.label());
            }
        });
        ui.end_row();
        ui.label("Rescale");
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
        ui.label("Rescale Margin(%)");
        // Used by "Round out to ticks" (Origin's Normal rescale); the other two modes ignore it.
        ui.add_enabled(
            axis.auto_range == AutoRange::Nice,
            DragValue::new(&mut axis.rescale_margin_pct).range(0.0..=100.0).speed(0.5).max_decimals(2),
        );
        ui.end_row();
        ui.label("Reverse");
        ui.checkbox(&mut axis.reverse, "");
        ui.end_row();
    });
    group_box(ui, "Major Ticks", |ui| {
        grid(ui, &format!("{id}_major"), |ui| {
            ui.label("Type");
            ui.label("By Increment");
            ui.end_row();
            ui.label("Value");
            optional_number(ui, &mut axis.major_step, resolved.map(|r| r.step), speed);
            ui.end_row();
            ui.label("First Tick");
            ui.add_enabled_ui(axis.scale == ScaleKind::Linear, |ui| {
                optional_number(
                    ui,
                    &mut axis.first_tick,
                    resolved.map(|r| r.major.first().copied().unwrap_or(r.min)),
                    speed,
                );
            });
            ui.end_row();
        });
    });
    group_box(ui, "Minor Ticks", |ui| {
        grid(ui, &format!("{id}_minor"), |ui| {
            ui.label("Type");
            ui.label("By Counts");
            ui.end_row();
            ui.label("Count");
            ui.add(DragValue::new(&mut axis.minor_ticks).range(0..=9));
            ui.end_row();
        });
    });
}

fn tick_labels_tab(ui: &mut egui::Ui, axis: &mut Axis, vertical: bool, columns: &[(ColumnId, String)], id: &str) {
    ui.checkbox(&mut axis.show_tick_labels, "Show");
    ui.add_enabled_ui(axis.show_tick_labels, |ui| {
        group_box(ui, "Display", |ui| {
            grid(ui, &format!("{id}_labels"), |ui| {
                ui.label("Type");
                ui.horizontal(|ui| {
                    let name = match axis.tick_label_source {
                        TickLabelSource::Auto => "Automatic",
                        TickLabelSource::Numbers => "Numeric",
                        TickLabelSource::Column { .. } => "Text from dataset",
                    };
                    egui::ComboBox::from_id_salt(format!("{id}_source")).selected_text(name).show_ui(ui, |ui| {
                        // The automatic mode only exists for X: a Y axis always shows numbers.
                        if !vertical {
                            ui.selectable_value(&mut axis.tick_label_source, TickLabelSource::Auto, "Automatic");
                        }
                        ui.selectable_value(&mut axis.tick_label_source, TickLabelSource::Numbers, "Numeric");
                        let is_column = matches!(axis.tick_label_source, TickLabelSource::Column { .. });
                        let first = columns.first().map(|(c, _)| *c);
                        if ui.selectable_label(is_column, "Text from dataset").clicked()
                            && !is_column
                            && let Some(column) = first
                        {
                            axis.tick_label_source = TickLabelSource::Column { column };
                        }
                    });
                    if let TickLabelSource::Column { column } = &mut axis.tick_label_source {
                        let shown = columns.iter().find(|(c, _)| c == column).map(|(_, n)| n.as_str()).unwrap_or("?");
                        egui::ComboBox::from_id_salt(format!("{id}_source_col")).selected_text(shown).show_ui(
                            ui,
                            |ui| {
                                for (c, n) in columns {
                                    ui.selectable_value(column, *c, n);
                                }
                            },
                        );
                    }
                });
                ui.end_row();
                ui.label("Display");
                ui.horizontal(|ui| {
                    let mut kind = match axis.tick_format {
                        TickFormat::Auto => 0,
                        TickFormat::Decimal(_) => 1,
                        TickFormat::Scientific(_) => 2,
                        // Set automatically for a date column; the menu shows it as Automatic.
                        TickFormat::DateTime(_) => 0,
                    };
                    let digits = match axis.tick_format {
                        TickFormat::Decimal(d) | TickFormat::Scientific(d) => d,
                        TickFormat::Auto | TickFormat::DateTime(_) => 2,
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
                ui.label("Divide by Factor");
                ui.add(DragValue::new(&mut axis.tick_label_divisor).speed(0.1).max_decimals(9))
                    .on_hover_text("Labels show the value divided by this factor, for example 0.001 to show A as mA");
                if axis.tick_label_divisor == 0.0 {
                    axis.tick_label_divisor = 1.0;
                }
                ui.end_row();
                ui.label("Rotate (deg.)");
                ui.add_enabled(
                    !vertical,
                    DragValue::new(&mut axis.tick_label_angle_deg).range(-90.0..=90.0).speed(1.0),
                )
                .on_hover_text("Turns the labels of a horizontal axis counter-clockwise");
                ui.end_row();
                ui.label("Hide Last Label");
                ui.checkbox(&mut axis.hide_end_label, "")
                    .on_hover_text("Leaves out the label at the far end, for panels that touch");
                ui.end_row();
                ui.label("Prefix");
                ui.add(egui::TextEdit::singleline(&mut axis.tick_prefix).desired_width(120.0));
                ui.end_row();
                ui.label("Suffix");
                ui.add(egui::TextEdit::singleline(&mut axis.tick_suffix).desired_width(120.0));
                ui.end_row();
            });
        });
        group_box(ui, "Font", |ui| {
            font_ui(ui, &mut axis.tick_font);
        });
    });
}

fn title_tab(ui: &mut egui::Ui, axis: &mut Axis, id: &str) {
    ui.checkbox(&mut axis.show_title, "Show");
    ui.add_enabled_ui(axis.show_title, |ui| {
        grid(ui, &format!("{id}_title"), |ui| {
            ui.label("Text");
            ui.horizontal(|ui| {
                let mut auto = axis.title.is_none();
                if ui
                    .checkbox(&mut auto, "From column")
                    .on_hover_text("Long name and units of the plotted column")
                    .changed()
                {
                    axis.title = if auto { None } else { Some(String::new()) };
                }
                if let Some(t) = &mut axis.title {
                    ui.add(egui::TextEdit::singleline(t).desired_width(220.0));
                }
            });
            ui.end_row();
            ui.label("Color");
            ui.horizontal(|ui| {
                ui.checkbox(&mut axis.title_color_follows_plot, "Follow plot");
                if !axis.title_color_follows_plot {
                    color_button(ui, &mut axis.title_font.color);
                }
            });
            ui.end_row();
            ui.label("Rotate (deg.)");
            ui.add_enabled(false, DragValue::new(&mut 0.0));
            ui.end_row();
        });
        ui.weak("Markup: \\+(superscript)  \\-(subscript)  \\g(greek)  \\b(bold)  \\i(italic)");
        group_box(ui, "Font", |ui| {
            font_ui(ui, &mut axis.title_font);
        });
    });
}

fn grids_tab(ui: &mut egui::Ui, axis: &mut Axis, id: &str) {
    grid_group(
        ui,
        "Major Grid Lines",
        &format!("{id}_major_grid"),
        &mut axis.major_grid,
        Color::rgb(128, 128, 128),
        0.5,
        Dash::Solid,
    );
    grid_group(
        ui,
        "Minor Grid Lines",
        &format!("{id}_minor_grid"),
        &mut axis.minor_grid,
        Color::rgb(200, 200, 200),
        0.3,
        Dash::Dash,
    );
    group_box(ui, "Additional Lines", |ui| {
        ui.add_enabled(false, egui::Checkbox::new(&mut false, "Opposite"));
    });
}

/// One group of the Grids tab: Show, Color, Style and Thickness; the boxes are grey until Show is ticked.
fn grid_group(
    ui: &mut egui::Ui,
    title: &str,
    id: &str,
    line: &mut Option<LineStyle>,
    color: Color,
    width: f64,
    dash: Dash,
) {
    group_box(ui, title, |ui| {
        let mut on = line.is_some();
        if ui.checkbox(&mut on, "Show").changed() {
            *line = on.then_some(LineStyle { color, width_pt: width, dash, connect: Connect::Straight });
        }
        ui.add_enabled_ui(line.is_some(), |ui| {
            let mut fallback = LineStyle { color, width_pt: width, dash, connect: Connect::Straight };
            let l = line.as_mut().unwrap_or(&mut fallback);
            grid(ui, id, |ui| {
                ui.label("Color");
                color_button(ui, &mut l.color);
                ui.end_row();
                ui.label("Style");
                dash_ui(ui, &format!("{id}_dash"), &mut l.dash);
                ui.end_row();
                ui.label("Thickness");
                ui.add(DragValue::new(&mut l.width_pt).range(0.1..=5.0).speed(0.05).suffix(" pt"));
                ui.end_row();
            });
        });
    });
}

fn line_and_ticks_tab(ui: &mut egui::Ui, axis: &mut Axis, vertical: bool, id: &str) {
    let mut shown = axis.side != AxisSide::Hidden;
    if ui.checkbox(&mut shown, "Show Line and Ticks").changed() {
        axis.side = if shown { AxisSide::Normal } else { AxisSide::Hidden };
    }
    ui.add_enabled_ui(shown, |ui| {
        group_box(ui, "Line", |ui| {
            grid(ui, &format!("{id}_line"), |ui| {
                ui.label("Color");
                ui.horizontal(|ui| {
                    ui.checkbox(&mut axis.color_follows_plot, "Follow plot");
                    if !axis.color_follows_plot {
                        color_button(ui, &mut axis.color);
                    }
                });
                ui.end_row();
                ui.label("Thickness");
                ui.add(DragValue::new(&mut axis.line_width_pt).range(0.0..=10.0).speed(0.05).suffix(" pt"));
                ui.end_row();
                ui.label("Axis Position");
                let (normal, opposite) = if vertical { ("Left", "Right") } else { ("Bottom", "Top") };
                egui::ComboBox::from_id_salt(format!("{id}_side"))
                    .selected_text(if axis.side == AxisSide::Opposite { opposite } else { normal })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut axis.side, AxisSide::Normal, normal);
                        ui.selectable_value(&mut axis.side, AxisSide::Opposite, opposite);
                    });
                ui.end_row();
                ui.label("Opposite side");
                ui.horizontal(|ui| {
                    ui.checkbox(&mut axis.show_opposite, "Line")
                        .on_hover_text("With both axes: a closed box around the plot");
                    ui.checkbox(&mut axis.opposite_ticks, "Ticks");
                });
                ui.end_row();
            });
        });
        group_box(ui, "Major Ticks", |ui| {
            grid(ui, &format!("{id}_major_ticks"), |ui| {
                ui.label("Style");
                tick_direction_ui(ui, &format!("{id}_major_dir"), &mut axis.major_tick);
                ui.end_row();
                ui.label("Length");
                ui.add(DragValue::new(&mut axis.major_tick_len_pt).range(0.0..=30.0).speed(0.1).suffix(" pt"));
                ui.end_row();
            });
        });
        group_box(ui, "Minor Ticks", |ui| {
            grid(ui, &format!("{id}_minor_ticks"), |ui| {
                ui.label("Style");
                tick_direction_ui(ui, &format!("{id}_minor_dir"), &mut axis.minor_tick);
                ui.end_row();
                ui.label("Length");
                ui.add(DragValue::new(&mut axis.minor_tick_len_pt).range(0.0..=30.0).speed(0.1).suffix(" pt"));
                ui.end_row();
            });
        });
    });
}
