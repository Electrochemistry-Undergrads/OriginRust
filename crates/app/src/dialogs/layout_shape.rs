//! A double-click on a drawn layout object (T237 slices 11 and 12): its kind (rectangle, ellipse,
//! lines), line colour and width, the fill of a rectangle or ellipse and the arrowhead of a line. OK
//! changes it as one undo step.

use eframe::egui;
use sheaf_core::layout_page::{LayoutContent, LayoutShape};
use sheaf_core::{Color, Document, WindowId};

pub enum Outcome {
    Open,
    Closed,
    Done,
}

pub struct LayoutShapeDialog {
    pub layout: WindowId,
    pub index: usize,
    pub shape: LayoutShape,
    pub color: Color,
    /// The line width in tenths of a point.
    pub width: u16,
    pub fill: Option<Color>,
    pub arrow: bool,
}

/// Sets item `index` of `layout` to the drawn object `shape` as one undo step.
pub fn set_shape(doc: &mut Document, layout: WindowId, index: usize, content: LayoutContent) -> Result<(), String> {
    let content = match content {
        LayoutContent::Shape { shape, color, width, fill, arrow } => LayoutContent::Shape {
            shape,
            color,
            width: width.clamp(1, 200),
            fill: fill.filter(|_| shape.closed()),
            arrow: arrow && !shape.closed(),
        },
        other => other,
    };
    let item = doc.project().layout(layout).and_then(|l| l.items.get(index)).ok_or("the object is gone")?;
    if item.content == content {
        return Ok(());
    }
    doc.edit("Layout object properties", |p| {
        let it = p.layout_mut(layout).and_then(|l| l.items.get_mut(index)).ok_or("the object is gone")?;
        it.content = content;
        Ok::<_, String>(())
    })
}

impl LayoutShapeDialog {
    /// The window for item `index` of `layout`; `None` when it is not a drawn object.
    pub fn new(doc: &Document, layout: WindowId, index: usize) -> Option<Self> {
        match doc.project().layout(layout)?.items.get(index)?.content {
            LayoutContent::Shape { shape, color, width, fill, arrow } => {
                Some(LayoutShapeDialog { layout, index, shape, color, width, fill, arrow })
            }
            _ => None,
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> Outcome {
        if doc.project().layout(self.layout).is_none() || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            return Outcome::Closed;
        }
        let mut open = true;
        let (mut ok, mut cancel) = (false, false);
        egui::Window::new("Object Properties")
            .id(egui::Id::new("layout_shape"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                egui::Grid::new("layout_shape_grid").num_columns(2).show(ui, |ui| {
                    ui.label("Object");
                    egui::ComboBox::from_id_salt("layout_shape_kind").selected_text(self.shape.label()).show_ui(
                        ui,
                        |ui| {
                            for s in LayoutShape::ALL {
                                ui.selectable_value(&mut self.shape, s, s.label());
                            }
                        },
                    );
                    ui.end_row();
                    ui.label("Color");
                    crate::dialogs::graph_props::color_button(ui, &mut self.color);
                    ui.end_row();
                    ui.label("Width (pt)");
                    let mut w = f64::from(self.width) / 10.0;
                    if ui.add(egui::DragValue::new(&mut w).speed(0.1).range(0.1..=20.0).max_decimals(1)).changed() {
                        self.width = (w * 10.0).round() as u16;
                    }
                    ui.end_row();
                    if self.shape.closed() {
                        ui.label("Fill");
                        ui.horizontal(|ui| {
                            let mut on = self.fill.is_some();
                            if ui.checkbox(&mut on, "").changed() {
                                self.fill = on.then_some(self.fill.unwrap_or(Color::WHITE));
                            }
                            if let Some(f) = &mut self.fill {
                                crate::dialogs::graph_props::color_button(ui, f);
                            }
                        });
                    } else {
                        ui.label("Arrow");
                        ui.checkbox(&mut self.arrow, "at the end");
                    }
                    ui.end_row();
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ok = ui.button("OK").clicked();
                    cancel = ui.button("Cancel").clicked();
                });
            });
        if !open || cancel {
            return Outcome::Closed;
        }
        if ok {
            let content = LayoutContent::Shape {
                shape: self.shape,
                color: self.color,
                width: self.width,
                fill: self.fill,
                arrow: self.arrow,
            };
            return match set_shape(doc, self.layout, self.index, content) {
                Ok(()) => Outcome::Done,
                Err(_) => Outcome::Closed,
            };
        }
        Outcome::Open
    }
}
