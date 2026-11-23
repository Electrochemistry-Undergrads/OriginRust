//! Layer Management > Display settings held until Apply.

use eframe::egui;
use sheaf_core::Color;
use sheaf_core::graph::{Graph, LayerBorder, LayerDrawingOrder, LayerElementScale};

use crate::object_format::PALETTE;

const DIMENSION_LABELS: [&str; 4] = ["Left Border", "Top Border", "Right Border", "Bottom Border"];
const DRAWING_ORDERS: [(LayerDrawingOrder, &str); 2] =
    [(LayerDrawingOrder::Composite, "Draw Composite"), (LayerDrawingOrder::LayerByLayer, "Draw Layer by Layer")];

#[derive(Clone, Copy, Default, PartialEq)]
enum Tab {
    #[default]
    Color,
    Dimensions,
    Scale,
}

#[derive(Default)]
pub(super) struct DisplayDraft {
    selection: Option<Vec<usize>>,
    tab: Tab,
    colors: [ColorDraft; 3],
    dimensions: [DimensionDraft; 4],
    drawing_order: Option<LayerDrawingOrder>,
    scale: ScaleDraft,
}

#[derive(Default)]
pub(super) struct DisplayChanges {
    colors: [Option<Option<Color>>; 3],
    dimensions: [Option<f64>; 4],
    drawing_order: Option<LayerDrawingOrder>,
    element_scale: Option<LayerElementScale>,
}

impl DisplayChanges {
    fn has_change(&self) -> bool {
        self.drawing_order.is_some()
            || self.element_scale.is_some()
            || self.colors.iter().any(Option::is_some)
            || self.dimensions.iter().any(Option::is_some)
    }

    pub(super) fn write(self, graph: &mut Graph, selection: &[usize]) {
        if let Some(order) = self.drawing_order {
            graph.page.style.layer_drawing_order = order;
        }
        for &i in selection {
            let layer = &mut graph.layers[i];
            if let Some(scale) = self.element_scale {
                layer.element_scale = scale;
            }
            for (dst, change) in
                [&mut layer.background, &mut layer.border.fill, &mut layer.border.outline].into_iter().zip(self.colors)
            {
                if let Some(color) = change {
                    *dst = color;
                }
            }
            for (dst, change) in layer.border.dimensions_pct.iter_mut().zip(self.dimensions) {
                if let Some(dimension) = change {
                    *dst = dimension;
                }
            }
        }
    }
}

impl DisplayDraft {
    pub(super) fn invalidate(&mut self) {
        self.selection = None;
        self.drawing_order = None;
    }

    pub(super) fn show(&mut self, ui: &mut egui::Ui, graph: &Graph, selection: &[usize]) -> Option<DisplayChanges> {
        if self.selection.as_deref() != Some(selection) {
            self.selection = Some(selection.to_vec());
            let layers = selection.iter().map(|&i| &graph.layers[i]);
            self.colors = [
                ColorDraft::new(common(layers.clone().map(|layer| layer.background))),
                ColorDraft::new(common(layers.clone().map(|layer| layer.border.fill))),
                ColorDraft::new(common(layers.clone().map(|layer| layer.border.outline))),
            ];
            self.dimensions = std::array::from_fn(|i| {
                DimensionDraft::new(common(layers.clone().map(|l| l.border.dimensions_pct[i])))
            });
            self.scale = ScaleDraft::new(graph, selection);
        }
        let original = graph.page.style.layer_drawing_order;
        let mut order = self.drawing_order.unwrap_or(original);
        ui.horizontal(|ui| {
            let label = ui.label("Drawing Order");
            egui::ComboBox::from_id_salt("lm_drawing_order")
                .selected_text(match order {
                    LayerDrawingOrder::Composite => "Draw Composite",
                    LayerDrawingOrder::LayerByLayer => "Draw Layer by Layer",
                })
                .show_ui(ui, |ui| {
                    for (value, name) in DRAWING_ORDERS {
                        ui.selectable_value(&mut order, value, name);
                    }
                })
                .response
                .labelled_by(label.id);
        });
        self.drawing_order = (order != original).then_some(order);
        ui.group(|ui| {
            ui.strong("Option");
            ui.radio_value(&mut self.tab, Tab::Color, "Color");
            ui.radio_value(&mut self.tab, Tab::Dimensions, "Border Dimensions");
            ui.radio_value(&mut self.tab, Tab::Scale, "Scale Elements");
        });
        let changes = ui
            .add_enabled_ui(!selection.is_empty(), |ui| match self.tab {
                Tab::Color => Some(self.show_colors(ui)),
                Tab::Dimensions => self.show_dimensions(ui),
                Tab::Scale => self.scale.show(ui),
            })
            .inner
            .map(|mut changes| {
                changes.drawing_order = self.drawing_order;
                changes
            })
            .filter(DisplayChanges::has_change);
        if ui.add_enabled(changes.is_some(), egui::Button::new("Apply")).clicked() { changes } else { None }
    }

    fn show_colors(&mut self, ui: &mut egui::Ui) -> DisplayChanges {
        ui.group(|ui| {
            ui.strong("Color");
            for (draft, (label, channels)) in self.colors.iter_mut().zip([
                ("Background Color", "Background"),
                ("Border Fill Color", "Border Fill"),
                ("Border Color", "Border"),
            ]) {
                draft.show(ui, label, channels);
            }
        });
        let colors = self.colors.each_ref().map(ColorDraft::change);
        DisplayChanges { colors, ..DisplayChanges::default() }
    }

    fn show_dimensions(&mut self, ui: &mut egui::Ui) -> Option<DisplayChanges> {
        ui.group(|ui| {
            ui.strong("Border Dimensions (% of Layer)");
            egui::Grid::new("lm_border_dimensions").num_columns(2).show(ui, |ui| {
                for (draft, label) in self.dimensions.iter_mut().zip(DIMENSION_LABELS) {
                    let label = ui.label(label);
                    let hint = if draft.original.is_none() { "Mixed" } else { "" };
                    draft.edited |= ui
                        .add(egui::TextEdit::singleline(&mut draft.text).hint_text(hint).desired_width(90.0))
                        .labelled_by(label.id)
                        .changed();
                    ui.end_row();
                }
            });
        });
        let mut dimensions = [None; 4];
        for (change, draft) in dimensions.iter_mut().zip(&self.dimensions).filter(|(_, draft)| draft.edited) {
            let limit = LayerBorder::MAX_OUTSET_PERCENT;
            let Some(value) = draft.text.trim().parse::<f64>().ok().filter(|value| (0.0..=limit).contains(value))
            else {
                ui.colored_label(ui.visuals().error_fg_color, format!("Enter a border percentage from 0 to {limit}."));
                return None;
            };
            if draft.original != Some(value) {
                *change = Some(value);
            }
        }
        Some(DisplayChanges { dimensions, ..DisplayChanges::default() })
    }
}

fn common<T: PartialEq>(mut values: impl Iterator<Item = T>) -> Option<T> {
    let first = values.next()?;
    values.all(|value| value == first).then_some(first)
}

#[derive(Clone, Copy, PartialEq)]
enum ScaleMode {
    WithFrame,
    Fixed,
}

impl From<LayerElementScale> for ScaleMode {
    fn from(scale: LayerElementScale) -> Self {
        match scale {
            LayerElementScale::WithLayerFrame => Self::WithFrame,
            LayerElementScale::Fixed(_) => Self::Fixed,
        }
    }
}

#[derive(Default)]
struct ScaleDraft {
    original: Option<LayerElementScale>,
    original_mode: Option<ScaleMode>,
    mode: Option<ScaleMode>,
    fixed_factor: String,
    edited_factor: bool,
    frame_factor: Option<f64>,
}

impl ScaleDraft {
    fn new(graph: &Graph, selection: &[usize]) -> Self {
        let scales = selection.iter().map(|&i| graph.layers[i].element_scale);
        let original = common(scales.clone());
        let original_mode = common(scales.map(ScaleMode::from));
        let applied_factor =
            common(selection.iter().map(|&i| graph.layers[i].element_scale.factor(graph.page_frame(i))));
        let frame_factor =
            common(selection.iter().map(|&i| LayerElementScale::WithLayerFrame.factor(graph.page_frame(i))));
        let fixed_factor = applied_factor.map_or_else(String::new, |value| value.to_string());
        Self { original, original_mode, mode: original_mode, fixed_factor, edited_factor: false, frame_factor }
    }

    fn show(&mut self, ui: &mut egui::Ui) -> Option<DisplayChanges> {
        ui.group(|ui| {
            ui.label("Scale Mode");
            ui.radio_value(&mut self.mode, Some(ScaleMode::WithFrame), "Scale with Layer Frame");
            ui.radio_value(&mut self.mode, Some(ScaleMode::Fixed), "Fixed Factor");
            ui.horizontal(|ui| {
                let label = ui.label("Factor");
                let mut frame_factor = self.frame_factor.map_or_else(String::new, |value| value.to_string());
                let fixed = self.mode == Some(ScaleMode::Fixed);
                let text =
                    if self.mode == Some(ScaleMode::WithFrame) { &mut frame_factor } else { &mut self.fixed_factor };
                let hint = if text.is_empty() && !self.edited_factor { "Mixed" } else { "" };
                self.edited_factor |= ui
                    .add_enabled(fixed, egui::TextEdit::singleline(text).hint_text(hint).desired_width(90.0))
                    .labelled_by(label.id)
                    .changed();
            });
        });
        let element_scale = match self.mode {
            Some(ScaleMode::WithFrame) => Some(LayerElementScale::WithLayerFrame),
            Some(ScaleMode::Fixed) if self.edited_factor || self.mode != self.original_mode => {
                let range = LayerElementScale::FIXED_RANGE;
                let Some(factor) = self.fixed_factor.trim().parse::<f64>().ok().filter(|value| range.contains(value))
                else {
                    ui.colored_label(
                        ui.visuals().error_fg_color,
                        format!("Enter a factor from {} to {}.", range.start(), range.end()),
                    );
                    return None;
                };
                Some(LayerElementScale::Fixed(factor))
            }
            _ => None,
        };
        let element_scale = element_scale.filter(|scale| Some(*scale) != self.original);
        Some(DisplayChanges { element_scale, ..DisplayChanges::default() })
    }
}

#[derive(Default)]
struct DimensionDraft {
    original: Option<f64>,
    text: String,
    edited: bool,
}

impl DimensionDraft {
    fn new(original: Option<f64>) -> Self {
        Self { original, text: original.map_or_else(String::new, |value| value.to_string()), edited: false }
    }
}

#[derive(Default)]
struct ColorDraft {
    original: Option<Option<Color>>,
    value: Option<Option<Color>>,
}

impl ColorDraft {
    fn new(original: Option<Option<Color>>) -> Self {
        Self { original, value: original }
    }

    fn change(&self) -> Option<Option<Color>> {
        if self.original == self.value { None } else { self.value }
    }

    fn show(&mut self, ui: &mut egui::Ui, name: &str, prefix: &str) {
        egui::Grid::new(("lm_color", name)).num_columns(2).show(ui, |ui| {
            let label = ui.label(name);
            let text = match self.value {
                None => "Mixed".into(),
                Some(None) => "None".into(),
                Some(Some(c)) => PALETTE
                    .iter()
                    .find(|(_, color)| *color == c)
                    .map_or_else(|| c.to_string(), |(name, _)| (*name).into()),
            };
            egui::ComboBox::from_id_salt(name)
                .selected_text(text)
                .show_ui(ui, |ui| {
                    for (label, color) in std::iter::once(("None", None)).chain(PALETTE.map(|(n, c)| (n, Some(c)))) {
                        if ui.selectable_label(self.value == Some(color), label).clicked() {
                            self.value = Some(color);
                            ui.close();
                        }
                    }
                })
                .response
                .labelled_by(label.id);
            ui.end_row();
            if let Some(Some(color)) = &mut self.value {
                for (name, channel) in [("Red", &mut color.r), ("Green", &mut color.g), ("Blue", &mut color.b)] {
                    let label = ui.label(format!("{prefix} {name}"));
                    if ui.add(egui::DragValue::new(channel).range(0..=255)).labelled_by(label.id).changed() {
                        color.a = 255;
                    }
                    ui.end_row();
                }
                ui.label(format!("{prefix} Preview"));
                let (rect, _) = ui.allocate_exact_size(egui::vec2(90.0, 18.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 0.0, rgba(*color));
                ui.end_row();
            }
        });
    }
}

pub(super) fn rgba(color: Color) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(color.r, color.g, color.b, color.a)
}
