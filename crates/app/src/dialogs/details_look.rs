//! Controls of Plot Details for the look settings beyond Origin's measured tabs (brief T13): fills
//! with a pattern, a gradient and a transparency, borders, the extra Label settings, subgroups and
//! the page's Miscellaneous tab. Each control writes a field of the model that the renderer or the
//! graph window uses (`sheaf_core::graph_style`, `Plot::decor`, `Layer::group_style`).

use eframe::egui::{self, DragValue};
use sheaf_core::graph::*;
use sheaf_core::{Color, ColumnId};

use super::graph_props::{color_button, dash_ui, grid, group_box};

fn combo<T: Copy + PartialEq>(ui: &mut egui::Ui, id: &str, value: &mut T, all: &[T], name: impl Fn(T) -> String) {
    egui::ComboBox::from_id_salt(id).selected_text(name(*value)).show_ui(ui, |ui| {
        for &v in all {
            ui.selectable_value(value, v, name(v));
        }
    });
}

/// Pattern, transparency and gradient of a fill (the colour is edited by the caller).
pub fn fill_controls(ui: &mut egui::Ui, id: &str, style: &mut FillStyle) {
    grid(ui, id, |ui| {
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
            style.gradient = on.then(|| Gradient::new(Color::WHITE));
        }
        if let Some(g) = &mut style.gradient {
            gradient_ui(ui, &format!("{id}_gradient"), g);
        }
    });
}

/// The end colour and direction of a gradient.
pub fn gradient_ui(ui: &mut egui::Ui, id: &str, g: &mut Gradient) {
    grid(ui, id, |ui| {
        ui.label("Color at the end");
        color_button(ui, &mut g.color);
        ui.end_row();
        ui.label("Direction");
        combo(ui, &format!("{id}_direction"), &mut g.direction, &GradientDirection::ALL, |d| d.name().to_string());
        ui.end_row();
    });
}

/// A border line that can be switched on: colour, style and width.
pub fn border_ui(ui: &mut egui::Ui, id: &str, border: &mut Option<BorderStyle>) {
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

/// The Label tab's settings beyond the measured ones: a column as the label form (in the form
/// list), the rows that are labelled, the number of decimals and the offsets.
pub fn label_extras(ui: &mut egui::Ui, labels: &mut DataLabels) {
    grid(ui, "label_extras", |ui| {
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
        ui.label("Show at Specified Rows Only");
        ui.add(
            egui::TextEdit::singleline(&mut labels.only_rows).hint_text("all rows, or 1,3,5-8").desired_width(160.0),
        );
        ui.end_row();
    });
}

/// The name of a label form in the form list; `columns` names the columns that can be chosen.
pub fn form_name(form: LabelForm, columns: &[(ColumnId, String)]) -> String {
    match form {
        LabelForm::X => "X".into(),
        LabelForm::Y => "Y".into(),
        LabelForm::Row => "Row Indices".into(),
        LabelForm::XY => "(X,Y)".into(),
        LabelForm::Column { column } => {
            let n = columns.iter().find(|(c, _)| *c == column).map(|(_, n)| n.as_str()).unwrap_or("column");
            format!("Column {n}")
        }
    }
}

/// The Label Form list: Origin's forms and a column of the project.
pub fn label_form_ui(ui: &mut egui::Ui, labels: &mut DataLabels, columns: &[(ColumnId, String)]) {
    egui::ComboBox::from_id_salt("label_form").selected_text(form_name(labels.form, columns)).width(220.0).show_ui(
        ui,
        |ui| {
            for f in [LabelForm::X, LabelForm::Y, LabelForm::Row, LabelForm::XY] {
                ui.selectable_value(&mut labels.form, f, form_name(f, columns));
            }
            for (c, _) in columns {
                let form = LabelForm::Column { column: *c };
                ui.selectable_value(&mut labels.form, form, form_name(form, columns));
            }
        },
    );
}

/// Subgrouping of the plot group: none or by size, and the property that steps between subgroups.
/// Returns true when it changed.
pub fn subgrouping_ui(ui: &mut egui::Ui, style: &mut GroupStyle) -> bool {
    let before = (style.subgrouping, style.subgroup_on);
    let by_size = matches!(style.subgrouping, Subgrouping::BySize { .. });
    let mut choice = by_size;
    ui.horizontal(|ui| {
        ui.label("Subgrouping");
        ui.selectable_value(&mut choice, false, "None");
        ui.selectable_value(&mut choice, true, "By Size");
        if let Subgrouping::BySize { size } = &mut style.subgrouping {
            ui.add(DragValue::new(size).range(1..=1000).suffix(" plots"));
        }
    });
    if choice != by_size {
        style.subgrouping = if choice { Subgrouping::BySize { size: 2 } } else { Subgrouping::None };
    }
    ui.add_enabled_ui(choice, |ui| {
        ui.horizontal(|ui| {
            ui.label("Subgroup on Increment");
            combo(ui, "subgroup_on", &mut style.subgroup_on, &SubgroupOn::ALL, |s| s.name().to_string());
        });
    });
    (style.subgrouping, style.subgroup_on) != before
}

/// The Miscellaneous tab of the page: the layer icons and the notes about the graph.
pub fn misc_tab(ui: &mut egui::Ui, graph: &mut Graph) {
    ui.checkbox(&mut graph.page.style.layer_icons, "Show the layer icons")
        .on_hover_text("The small numbered buttons at the top left of the graph window");
    ui.add_space(6.0);
    ui.label("Comments");
    ui.add(egui::TextEdit::multiline(&mut graph.page.style.comments).desired_rows(6).desired_width(380.0));
    ui.weak("Shown when the pointer rests on the graph's name in the Object Manager.");
}
