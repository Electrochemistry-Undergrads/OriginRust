//! The plot tabs of Plot Details that carry the look settings of brief T13: the fill controls shared
//! by the Line and Pattern tabs, Drop Lines, Label, Offset, and the plot-group rules of the Group
//! tab (edit mode, increments, members, subgrouping).
//!
//! Every control here writes a field of the model that the renderer draws (`sheaf_core::graph_style`).

use eframe::egui::{self, DragValue};
use sheaf_core::ColumnId;
use sheaf_core::graph::*;

use super::graph_props::{color_button, dash_ui, font_ui, grid, group_box};

fn combo<T: Copy + PartialEq>(ui: &mut egui::Ui, id: &str, value: &mut T, all: &[T], name: impl Fn(T) -> String) {
    egui::ComboBox::from_id_salt(id).selected_text(name(*value)).show_ui(ui, |ui| {
        for &v in all {
            ui.selectable_value(value, v, name(v));
        }
    });
}

/// Colour, pattern, transparency and gradient of a fill. `color` is the plot's fill colour (its
/// symbol colour); the returned flag says that it was changed by hand.
pub fn fill_controls(ui: &mut egui::Ui, id: &str, style: &mut FillStyle, color: &mut sheaf_core::Color) -> bool {
    let mut by_hand = false;
    grid(ui, id, |ui| {
        ui.label("Color");
        let before = *color;
        color_button(ui, color);
        by_hand = *color != before;
        ui.end_row();
        ui.label("Pattern");
        combo(ui, &format!("{id}_pattern"), &mut style.pattern, &FillPattern::ALL, |p| p.name().to_string());
        ui.end_row();
        let on = style.pattern != FillPattern::None;
        ui.label("Pattern Color");
        ui.add_enabled_ui(on, |ui| color_button(ui, &mut style.pattern_color));
        ui.end_row();
        ui.label("Pattern Width");
        ui.add_enabled(on, DragValue::new(&mut style.pattern_width_pt).range(0.1..=10.0).speed(0.05).suffix(" pt"));
        ui.end_row();
        ui.label("Pattern Spacing");
        ui.add_enabled(on, DragValue::new(&mut style.pattern_spacing_pt).range(0.5..=50.0).speed(0.1).suffix(" pt"));
        ui.end_row();
        ui.label("Transparency");
        ui.add(DragValue::new(&mut style.transparency_pct).range(0..=100).suffix(" %"));
        ui.end_row();
    });
    group_box(ui, "Gradient Fill", |ui| {
        let mut on = style.gradient.is_some();
        if ui.checkbox(&mut on, "Fill with a gradient").changed() {
            style.gradient = on.then(|| Gradient::new(sheaf_core::Color::WHITE));
        }
        if let Some(g) = &mut style.gradient {
            grid(ui, &format!("{id}_gradient"), |ui| {
                ui.label("Color at the end");
                color_button(ui, &mut g.color);
                ui.end_row();
                ui.label("Direction");
                combo(ui, &format!("{id}_direction"), &mut g.direction, &GradientDirection::ALL, |d| {
                    d.name().to_string()
                });
                ui.end_row();
            });
        }
    });
    by_hand
}

/// One drop line direction: a tick and, when on, its style, width and colour.
fn drop_line(ui: &mut egui::Ui, id: &str, title: &str, line: &mut Option<LineStyle>, plot_color: sheaf_core::Color) {
    let mut on = line.is_some();
    if ui.checkbox(&mut on, title).changed() {
        *line = on.then(|| DropLines::starting_style(plot_color));
    }
    if let Some(style) = line {
        ui.indent(id, |ui| {
            grid(ui, id, |ui| {
                ui.label("Style");
                dash_ui(ui, &format!("{id}_dash"), &mut style.dash);
                ui.end_row();
                ui.label("Width");
                ui.add(DragValue::new(&mut style.width_pt).range(0.0..=20.0).speed(0.05).suffix(" pt"));
                ui.end_row();
                ui.label("Color");
                color_button(ui, &mut style.color);
                ui.end_row();
            });
        });
    }
}

pub fn drop_lines_tab(ui: &mut egui::Ui, plot: &mut Plot) {
    let color = plot.line.color;
    let drops = &mut plot.details.drop_lines;
    drop_line(ui, "drop_h", "Horizontal (to the Y axis)", &mut drops.horizontal, color);
    ui.add_space(6.0);
    drop_line(ui, "drop_v", "Vertical (to the X axis)", &mut drops.vertical, color);
}

pub fn label_tab(ui: &mut egui::Ui, plot: &mut Plot, columns: &[(ColumnId, String)]) {
    let labels = &mut plot.details.labels;
    ui.checkbox(&mut labels.enabled, "Enable");
    ui.add_space(4.0);
    ui.add_enabled_ui(labels.enabled, |ui| {
        grid(ui, "label_grid", |ui| {
            ui.label("Label Form");
            let name = match labels.form {
                LabelForm::Y => "Y value".to_string(),
                LabelForm::X => "X value".to_string(),
                LabelForm::Row => "Row number".to_string(),
                LabelForm::Column { column } => {
                    let n = columns.iter().find(|(c, _)| *c == column).map(|(_, n)| n.as_str()).unwrap_or("column");
                    format!("Column {n}")
                }
            };
            egui::ComboBox::from_id_salt("label_form").selected_text(name).width(260.0).show_ui(ui, |ui| {
                ui.selectable_value(&mut labels.form, LabelForm::Y, "Y value");
                ui.selectable_value(&mut labels.form, LabelForm::X, "X value");
                ui.selectable_value(&mut labels.form, LabelForm::Row, "Row number");
                for (c, n) in columns {
                    let form = LabelForm::Column { column: *c };
                    ui.selectable_value(&mut labels.form, form, format!("Column {n}"));
                }
            });
            ui.end_row();
            ui.label("Position");
            combo(ui, "label_pos", &mut labels.position, &LabelPosition::ALL, |p| p.name().to_string());
            ui.end_row();
            ui.label("Show at Specified Rows Only");
            ui.add(
                egui::TextEdit::singleline(&mut labels.only_rows)
                    .hint_text("all rows, or 1,3,5-8")
                    .desired_width(160.0),
            );
            ui.end_row();
            ui.label("Font");
            ui.add(egui::TextEdit::singleline(&mut labels.font.family).desired_width(120.0));
            ui.end_row();
            ui.label("Size, Style, Color");
            font_ui(ui, &mut labels.font);
            ui.end_row();
            ui.label("Rotate");
            ui.add(DragValue::new(&mut labels.rotation_deg).range(-360.0..=360.0).speed(1.0).suffix(" deg"));
            ui.end_row();
            ui.label("Horizontal Offset");
            ui.add(DragValue::new(&mut labels.dx_pt).range(-200.0..=200.0).speed(0.25).suffix(" pt"));
            ui.end_row();
            ui.label("Vertical Offset");
            ui.add(DragValue::new(&mut labels.dy_pt).range(-200.0..=200.0).speed(0.25).suffix(" pt"));
            ui.end_row();
            ui.label("Decimal Places");
            ui.horizontal(|ui| {
                let mut set = labels.decimals.is_some();
                if ui.checkbox(&mut set, "Set").changed() {
                    labels.decimals = set.then_some(2);
                }
                if let Some(d) = &mut labels.decimals {
                    ui.add(DragValue::new(d).range(0..=12));
                }
            });
            ui.end_row();
        });
    });
}

/// The per-plot offset of Origin's Offset tab. Offsets are used while the layer's Stack tab says
/// "Individual offsets"; the button switches that on.
pub fn offset_tab(ui: &mut egui::Ui, layer: &mut Layer, p: usize) {
    let column = layer.plots[p].y;
    let (mut x, mut y) = layer.plot_offset(column).map(|o| (o.x, o.y)).unwrap_or((0.0, 0.0));
    let before = (x, y);
    grid(ui, "plot_offset", |ui| {
        ui.label("Offset in X");
        ui.add(DragValue::new(&mut x).speed(0.05).max_decimals(8));
        ui.end_row();
        ui.label("Offset in Y");
        ui.add(DragValue::new(&mut y).speed(0.05).max_decimals(8));
        ui.end_row();
    });
    if (x, y) != before {
        let o = layer.plot_offset_mut(column);
        o.x = x;
        o.y = y;
    }
    if layer.stack == Stack::Individual {
        ui.weak("In axis units. Used because this layer stacks by individual offsets.");
    } else {
        ui.add_space(4.0);
        ui.weak("Offsets are used when the layer stacks by individual offsets.");
        if ui.button("Offset each plot by hand").clicked() {
            layer.stack = Stack::Individual;
        }
    }
}

fn increment_name(i: Increment) -> String {
    match i {
        Increment::None => "None".into(),
        Increment::ByOne => "By One".into(),
    }
}

fn palette_name(m: PaletteMode) -> String {
    match m {
        PaletteMode::Fixed => "None".into(),
        PaletteMode::Cycle => "By One".into(),
        PaletteMode::Spread => "Gradient".into(),
    }
}

/// The plot group of a layer: edit mode, the increment rules, the members and the subgroups. The
/// members are the plots that follow the layer's colour list (the tick beside a plot adds it to
/// the group or lets it go). Changes restyle the members at once while the group is dependent.
pub fn group_rules(ui: &mut egui::Ui, layer: &mut Layer, names: &[String]) {
    let before = (layer.groups, layer.palette.clone());
    group_box(ui, "Edit Mode", |ui| {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut layer.groups.edit_mode, EditMode::Independent, "Independent")
                .on_hover_text("Plots keep their style; only Recolor plots styles them again");
            ui.selectable_value(&mut layer.groups.edit_mode, EditMode::Dependent, "Dependent")
                .on_hover_text("The members follow the increment rules");
        });
    });
    ui.add_space(4.0);
    grid(ui, "group_increments", |ui| {
        ui.label("");
        ui.label("Increment");
        ui.label("Details");
        ui.end_row();
        ui.label("Color");
        combo(
            ui,
            "inc_color",
            &mut layer.palette.mode,
            &[PaletteMode::Fixed, PaletteMode::Cycle, PaletteMode::Spread],
            palette_name,
        );
        ui.horizontal(|ui| {
            for c in &mut layer.palette.colors {
                color_button(ui, c);
            }
            if ui.small_button("+").clicked() {
                layer.palette.colors.push(sheaf_core::Color::BLACK);
            }
            if layer.palette.colors.len() > 1 && ui.small_button("-").clicked() {
                layer.palette.colors.pop();
            }
        });
        ui.end_row();
        ui.label("Line Style");
        combo(ui, "inc_line", &mut layer.groups.line_style, &[Increment::None, Increment::ByOne], increment_name);
        ui.label("");
        ui.end_row();
        ui.label("Symbol Shape");
        combo(ui, "inc_shape", &mut layer.groups.symbol_shape, &[Increment::None, Increment::ByOne], increment_name);
        ui.label("");
        ui.end_row();
    });

    group_box(ui, "Group Members", |ui| {
        let mut toggled = false;
        for (i, name) in names.iter().enumerate().take(layer.plots.len()) {
            ui.horizontal(|ui| {
                let p = &mut layer.plots[i];
                toggled |= ui.checkbox(&mut p.auto_color, "").on_hover_text("Member of the group").changed();
                crate::grid::color_swatch(ui, p.line.color);
                ui.label(name);
            });
        }
        if toggled {
            layer.recolor_automatic();
        }
    });

    group_box(ui, "Subgrouping", |ui| {
        let by_size = matches!(layer.groups.subgrouping, Subgrouping::BySize { .. });
        let mut choice = by_size;
        ui.horizontal(|ui| {
            ui.selectable_value(&mut choice, false, "None");
            ui.selectable_value(&mut choice, true, "By Size");
            if let Subgrouping::BySize { size } = &mut layer.groups.subgrouping {
                ui.add(DragValue::new(size).range(1..=1000).suffix(" plots"));
            }
        });
        if choice != by_size {
            layer.groups.subgrouping = if choice { Subgrouping::BySize { size: 2 } } else { Subgrouping::None };
        }
        ui.add_enabled_ui(choice, |ui| {
            ui.horizontal(|ui| {
                ui.label("Subgroup on Increment");
                combo(
                    ui,
                    "subgroup_on",
                    &mut layer.groups.subgroup_on,
                    &[SubgroupOn::Color, SubgroupOn::LineStyle, SubgroupOn::SymbolShape],
                    |s| {
                        match s {
                            SubgroupOn::Color => "Color",
                            SubgroupOn::LineStyle => "Line Style",
                            SubgroupOn::SymbolShape => "Symbol Shape",
                        }
                        .to_string()
                    },
                );
            });
        });
    });
    if (layer.groups, &layer.palette) != (before.0, &before.1) {
        layer.recolor_automatic();
    }
}
