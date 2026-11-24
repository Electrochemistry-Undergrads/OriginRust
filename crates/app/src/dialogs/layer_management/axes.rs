//! Axis settings that can be shared across the selected preview layers.
use eframe::egui;
use sheaf_core::graph::{Axis, AxisSide, Coords, Graph, OppositeAxisStyle, ScaleKind, ScaleLink, TickDirection};

const TICK_DIRECTIONS: [(TickDirection, &str); 4] = [
    (TickDirection::Both, "In & Out"),
    (TickDirection::In, "In"),
    (TickDirection::Out, "Out"),
    (TickDirection::None, "None"),
];

#[derive(Clone, Default, PartialEq)]
pub(super) struct AxisSettings {
    scale: Option<ScaleKind>,
    side: Option<AxisSide>,
    ticks: Option<TickDirection>,
    title: Option<bool>,
    labels: Option<bool>,
    formula: Option<String>,
    opposite_line: Option<bool>,
    opposite_style: OppositeAxisStyle,
}

#[derive(Default)]
pub(super) struct AxesDraft {
    selection: Option<Vec<usize>>,
    original: [AxisSettings; 2],
    draft: [AxisSettings; 2],
    scale_editable: [bool; 2],
    has_layers: bool,
}

impl AxesDraft {
    pub(super) fn invalidate(&mut self) {
        self.selection = None;
    }

    pub(super) fn show(&mut self, ui: &mut egui::Ui, graph: &Graph, selection: &[usize]) -> Option<[AxisSettings; 2]> {
        if self.selection.as_deref() != Some(selection) {
            self.selection = Some(selection.to_vec());
            let layers: Vec<_> =
                selection.iter().copied().filter(|&i| graph.layers[i].coords == Coords::Cartesian).collect();
            self.has_layers = !layers.is_empty();
            for (i, vertical) in [false, true].into_iter().enumerate() {
                let axes = layers.iter().map(|&layer| {
                    let (x, y) = (&graph.layers[layer].x_axis, &graph.layers[layer].y_axis);
                    (if vertical { y } else { x }, scale_linked(graph, layer, vertical))
                });
                self.scale_editable[i] = axes.clone().any(|(_, linked)| !linked);
                self.original[i] = AxisSettings::read(axes);
            }
            self.draft = self.original.clone();
        }

        ui.strong("Modify Axes");
        if self.has_layers {
            axis_controls(ui, "X", ["Bottom", "Top"], &mut self.draft[0], self.scale_editable[0]);
            ui.separator();
            axis_controls(ui, "Y", ["Left", "Right"], &mut self.draft[1], self.scale_editable[1]);
        } else {
            ui.label("Select a layer with Cartesian axes.");
        }
        let changes = std::array::from_fn(|i| self.draft[i].changes_from(&self.original[i]));
        let changed = changes != <[AxisSettings; 2]>::default();
        ui.add_enabled(changed, egui::Button::new("Apply")).clicked().then_some(changes)
    }
}

impl AxisSettings {
    fn read<'a>(axes: impl Iterator<Item = (&'a Axis, bool)> + Clone) -> Self {
        Self {
            scale: common(axes.clone().filter_map(|(axis, linked)| (!linked).then_some(axis.scale))),
            side: common(axes.clone().map(|(axis, _)| axis.side)),
            ticks: common(axes.clone().flat_map(|(axis, _)| [axis.major_tick, axis.minor_tick])),
            title: common(axes.clone().map(|(axis, _)| axis.show_title)),
            labels: common(axes.clone().map(|(axis, _)| axis.show_tick_labels)),
            formula: common(axes.clone().map(|(axis, _)| &axis.tick_label_formula)).cloned(),
            opposite_line: common(axes.clone().map(|(axis, _)| axis.show_opposite)),
            opposite_style: OppositeAxisStyle {
                tick_direction: common(axes.clone().flat_map(|(axis, _)| axis.opposite_tick_directions())),
                show_title: common(axes.clone().map(|(axis, _)| axis.opposite_title_visible())),
                show_tick_labels: common(axes.clone().map(|(axis, _)| axis.opposite_labels_visible())),
                tick_label_formula: common(axes.map(|(axis, _)| axis.opposite_formula())).map(str::to_owned),
            },
        }
    }

    fn changes_from(&self, original: &Self) -> Self {
        Self {
            scale: changed(&self.scale, &original.scale),
            side: changed(&self.side, &original.side),
            ticks: changed(&self.ticks, &original.ticks),
            title: changed(&self.title, &original.title),
            labels: changed(&self.labels, &original.labels),
            formula: changed(&self.formula, &original.formula),
            opposite_line: changed(&self.opposite_line, &original.opposite_line),
            opposite_style: OppositeAxisStyle {
                tick_direction: changed(&self.opposite_style.tick_direction, &original.opposite_style.tick_direction),
                show_title: changed(&self.opposite_style.show_title, &original.opposite_style.show_title),
                show_tick_labels: changed(
                    &self.opposite_style.show_tick_labels,
                    &original.opposite_style.show_tick_labels,
                ),
                tick_label_formula: changed(
                    &self.opposite_style.tick_label_formula,
                    &original.opposite_style.tick_label_formula,
                ),
            },
        }
    }

    fn write(&self, axis: &mut Axis, linked: bool) {
        if let Some(scale) = self.scale
            && !linked
        {
            axis.scale = scale;
        }
        if let Some(side) = self.side {
            axis.side = side;
        }
        if let Some(ticks) = self.ticks {
            axis.major_tick = ticks;
            axis.minor_tick = ticks;
        }
        if let Some(title) = self.title {
            axis.show_title = title;
        }
        if let Some(labels) = self.labels {
            axis.show_tick_labels = labels;
        }
        if let Some(formula) = &self.formula {
            axis.tick_label_formula.clone_from(formula);
        }
        if let Some(line) = self.opposite_line {
            axis.show_opposite = line;
        }
        let style = &self.opposite_style;
        if let Some(direction) = style.tick_direction {
            axis.opposite_ticks = direction != TickDirection::None;
            axis.opposite_style.tick_direction = Some(direction);
        }
        if style.show_title.is_some() {
            axis.opposite_style.show_title = style.show_title;
        }
        if style.show_tick_labels.is_some() {
            axis.opposite_style.show_tick_labels = style.show_tick_labels;
        }
        if style.tick_label_formula.is_some() {
            axis.opposite_style.tick_label_formula.clone_from(&style.tick_label_formula);
        }
    }
}

fn common<T: PartialEq>(mut values: impl Iterator<Item = T>) -> Option<T> {
    let first = values.next()?;
    values.all(|value| value == first).then_some(first)
}

fn changed<T: PartialEq + Clone>(value: &Option<T>, original: &Option<T>) -> Option<T> {
    if value == original { None } else { value.clone() }
}

fn scale_linked(graph: &Graph, layer: usize, vertical: bool) -> bool {
    graph.valid_link(layer).is_some_and(|link| *(if vertical { &link.y } else { &link.x }) == ScaleLink::Straight)
}

pub(super) fn apply(graph: &mut Graph, selection: &[usize], changes: &[AxisSettings; 2]) {
    for &i in selection {
        if graph.layers[i].coords == Coords::Cartesian {
            let x_linked = scale_linked(graph, i, false);
            let y_linked = scale_linked(graph, i, true);
            changes[0].write(&mut graph.layers[i].x_axis, x_linked);
            changes[1].write(&mut graph.layers[i].y_axis, y_linked);
        }
    }
}

fn axis_controls(ui: &mut egui::Ui, axis: &str, edges: [&str; 2], draft: &mut AxisSettings, scale_editable: bool) {
    egui::Grid::new(("lm_axis", axis)).num_columns(2).show(ui, |ui| {
        ui.add_enabled_ui(scale_editable, |ui| {
            combo(
                ui,
                &format!("{axis} Scale"),
                &mut draft.scale,
                if scale_editable { "Mixed" } else { "Linked" },
                ScaleKind::ALL.map(|kind| (kind, kind.label())),
            );
        });
        ui.end_row();
        combo(
            ui,
            &format!("{axis} Axis Edge"),
            &mut draft.side,
            "Mixed",
            [(AxisSide::Normal, edges[0]), (AxisSide::Opposite, edges[1]), (AxisSide::Hidden, "Hidden")],
        );
        ui.end_row();
        ui.add_enabled_ui(draft.side != Some(AxisSide::Hidden), |ui| {
            combo(ui, &format!("{axis} Tick"), &mut draft.ticks, "Mixed", TICK_DIRECTIONS);
        });
        ui.end_row();
        formula_control(ui, format!("{axis} Formula"), &mut draft.formula);
    });
    ui.horizontal(|ui| {
        checkbox(ui, format!("{axis} Title"), &mut draft.title);
        checkbox(ui, format!("{axis} Tick Label"), &mut draft.labels);
    });
    ui.horizontal(|ui| {
        checkbox(ui, format!("{axis} Opposite Line"), &mut draft.opposite_line);
        checkbox(ui, format!("{axis} Opposite Title"), &mut draft.opposite_style.show_title);
        checkbox(ui, format!("{axis} Opposite Tick Label"), &mut draft.opposite_style.show_tick_labels);
    });
    egui::Grid::new(("lm_axis_opposite", axis)).num_columns(2).show(ui, |ui| {
        let can_choose_ticks =
            draft.opposite_line != Some(false) || draft.opposite_style.tick_direction != Some(TickDirection::None);
        ui.add_enabled_ui(can_choose_ticks && draft.side != Some(AxisSide::Hidden), |ui| {
            combo(
                ui,
                &format!("{axis} Opposite Ticks"),
                &mut draft.opposite_style.tick_direction,
                "Mixed",
                TICK_DIRECTIONS,
            );
        });
        ui.end_row();
        formula_control(ui, format!("{axis} Opposite Formula"), &mut draft.opposite_style.tick_label_formula);
    });
}

fn formula_control(ui: &mut egui::Ui, name: String, value: &mut Option<String>) {
    let label = ui.label(name);
    ui.horizontal(|ui| {
        let mut formula = value.clone().unwrap_or_default();
        let hint = if value.is_none() { "Mixed" } else { "x" };
        let edit = egui::TextEdit::singleline(&mut formula).hint_text(hint).desired_width(105.0);
        if ui.add(edit).labelled_by(label.id).changed() {
            *value = Some(formula);
        }
        if value.is_none() && ui.small_button("Clear").clicked() {
            *value = Some(String::new());
        }
    });
    ui.end_row();
}

fn combo<'a, T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    name: &str,
    value: &mut Option<T>,
    unset: &str,
    choices: impl IntoIterator<Item = (T, &'a str)> + Clone,
) {
    let label = ui.label(name);
    let selected = choices.clone().into_iter().find(|(choice, _)| Some(*choice) == *value);
    egui::ComboBox::from_id_salt(name)
        .selected_text(selected.map_or(unset, |(_, label)| label))
        .show_ui(ui, |ui| {
            for (choice, label) in choices {
                ui.selectable_value(value, Some(choice), label);
            }
        })
        .response
        .labelled_by(label.id);
}

fn checkbox(ui: &mut egui::Ui, name: String, value: &mut Option<bool>) {
    let mut on = value.unwrap_or(false);
    if ui.add(egui::Checkbox::new(&mut on, name).indeterminate(value.is_none())).changed() {
        *value = Some(on);
    }
}
