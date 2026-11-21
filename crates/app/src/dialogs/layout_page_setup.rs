//! Layout > Page Setup... (T237 slice 4): the layout page's width and height, in inches,
//! centimetres or points, with paper presets and Portrait/Landscape. OK sets the size as one undo
//! step; the items keep their places as fractions of the page.

use eframe::egui;
use sheaf_core::{Document, WindowId};

pub enum Outcome {
    Open,
    Closed,
    Done,
}

/// The units the sizes are typed in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Unit {
    Inch,
    Cm,
    Point,
}

impl Unit {
    pub const ALL: [Unit; 3] = [Unit::Inch, Unit::Cm, Unit::Point];

    /// Points in one of this unit.
    pub fn points(self) -> f64 {
        match self {
            Unit::Inch => 72.0,
            Unit::Cm => 72.0 / 2.54,
            Unit::Point => 1.0,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Unit::Inch => "Inch",
            Unit::Cm => "cm",
            Unit::Point => "Point",
        }
    }
}

/// Paper presets: name and (width, height) in points, portrait.
pub const PRESETS: [(&str, (f64, f64)); 4] =
    [("Letter", (612.0, 792.0)), ("Legal", (612.0, 1008.0)), ("A4", (595.28, 841.89)), ("A3", (841.89, 1190.55))];

/// The smallest and largest page side, in points (half an inch to 200 inches).
pub const SIDE_PT: (f64, f64) = (36.0, 14_400.0);

/// Sets the page size of `layout` to `width_pt` x `height_pt` as one undo step.
pub fn set_page_size(doc: &mut Document, layout: WindowId, width_pt: f64, height_pt: f64) -> Result<(), String> {
    for v in [width_pt, height_pt] {
        if !(v.is_finite() && (SIDE_PT.0..=SIDE_PT.1).contains(&v)) {
            return Err(format!("a page side must be between {} and {} points", SIDE_PT.0, SIDE_PT.1));
        }
    }
    let page = doc.project().layout(layout).ok_or("the layout page is gone")?;
    if (page.width_pt, page.height_pt) == (width_pt, height_pt) {
        return Ok(());
    }
    doc.edit("Layout page size", |p| {
        let l = p.layout_mut(layout).ok_or("the layout page is gone")?;
        (l.width_pt, l.height_pt) = (width_pt, height_pt);
        Ok::<_, String>(())
    })
}

pub struct LayoutPageSetup {
    pub layout: WindowId,
    pub unit: Unit,
    /// The size being edited, in points.
    pub width_pt: f64,
    pub height_pt: f64,
    pub error: Option<String>,
}

impl LayoutPageSetup {
    pub fn new(doc: &Document, layout: WindowId) -> Option<Self> {
        let page = doc.project().layout(layout)?;
        Some(LayoutPageSetup {
            layout,
            unit: Unit::Inch,
            width_pt: page.width_pt,
            height_pt: page.height_pt,
            error: None,
        })
    }

    /// The preset whose size (either way round) this is, if any.
    pub fn preset(&self) -> Option<&'static str> {
        let near = |a: f64, b: f64| (a - b).abs() < 0.5;
        let (w, h) = (self.width_pt, self.height_pt);
        PRESETS
            .iter()
            .find(|(_, (pw, ph))| (near(w, *pw) && near(h, *ph)) || (near(w, *ph) && near(h, *pw)))
            .map(|(name, _)| *name)
    }

    pub fn landscape(&self) -> bool {
        self.width_pt > self.height_pt
    }

    /// Turns the page to portrait (`false`) or landscape (`true`).
    pub fn set_landscape(&mut self, landscape: bool) {
        if self.landscape() != landscape {
            std::mem::swap(&mut self.width_pt, &mut self.height_pt);
        }
    }

    /// Takes the preset `name`, keeping the orientation.
    pub fn pick_preset(&mut self, name: &str) {
        if let Some((_, (w, h))) = PRESETS.iter().find(|(n, _)| *n == name) {
            let landscape = self.landscape();
            (self.width_pt, self.height_pt) = (*w, *h);
            self.set_landscape(landscape);
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> Outcome {
        if doc.project().layout(self.layout).is_none() || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            return Outcome::Closed;
        }
        let mut open = true;
        let (mut ok, mut cancel) = (false, false);
        egui::Window::new("Page Setup")
            .id(egui::Id::new("layout_page_setup"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                egui::Grid::new("layout_page_setup_grid").num_columns(2).show(ui, |ui| {
                    ui.label("Size");
                    let shown = self.preset().unwrap_or("Custom");
                    let mut picked = None;
                    egui::ComboBox::from_id_salt("layout_page_setup_preset").selected_text(shown).show_ui(ui, |ui| {
                        for (name, _) in PRESETS {
                            if ui.selectable_label(shown == name, name).clicked() {
                                picked = Some(name);
                            }
                        }
                    });
                    if let Some(name) = picked {
                        self.pick_preset(name);
                    }
                    ui.end_row();
                    ui.label("Orientation");
                    ui.horizontal(|ui| {
                        let landscape = self.landscape();
                        if ui.radio(!landscape, "Portrait").clicked() {
                            self.set_landscape(false);
                        }
                        if ui.radio(landscape, "Landscape").clicked() {
                            self.set_landscape(true);
                        }
                    });
                    ui.end_row();
                    ui.label("Units");
                    egui::ComboBox::from_id_salt("layout_page_setup_unit").selected_text(self.unit.label()).show_ui(
                        ui,
                        |ui| {
                            for u in Unit::ALL {
                                ui.selectable_value(&mut self.unit, u, u.label());
                            }
                        },
                    );
                    ui.end_row();
                    let per = self.unit.points();
                    let speed = if self.unit == Unit::Point { 1.0 } else { 0.05 };
                    for (label, v) in [("Width", &mut self.width_pt), ("Height", &mut self.height_pt)] {
                        ui.label(label);
                        let mut shown = *v / per;
                        let range = (SIDE_PT.0 / per)..=(SIDE_PT.1 / per);
                        if ui.add(egui::DragValue::new(&mut shown).speed(speed).range(range).max_decimals(3)).changed()
                        {
                            *v = shown * per;
                        }
                        ui.end_row();
                    }
                });
                if let Some(e) = &self.error {
                    ui.colored_label(egui::Color32::RED, e);
                }
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
            return match set_page_size(doc, self.layout, self.width_pt, self.height_pt) {
                Ok(()) => Outcome::Done,
                Err(e) => {
                    self.error = Some(e);
                    Outcome::Open
                }
            };
        }
        Outcome::Open
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_keep_the_orientation_and_units_convert() {
        let mut p = sheaf_core::Project::new();
        let id = p.add_layout();
        let mut doc = Document::new(p);
        let mut d = LayoutPageSetup::new(&doc, id).unwrap();
        assert_eq!(d.preset(), Some("Letter"), "11 x 8.5 in landscape is Letter");
        assert!(d.landscape());
        d.pick_preset("A4");
        assert_eq!((d.width_pt, d.height_pt), (841.89, 595.28), "A4 turned landscape");
        d.set_landscape(false);
        assert_eq!((d.width_pt, d.height_pt), (595.28, 841.89));
        d.width_pt = 500.0;
        assert_eq!(d.preset(), None, "custom");
        assert!((Unit::Cm.points() * 2.54 - 72.0).abs() < 1e-12);
        assert!(set_page_size(&mut doc, id, 10.0, 600.0).is_err(), "too small");
        set_page_size(&mut doc, id, 595.28, 841.89).unwrap();
        assert_eq!(doc.undo_label(), Some("Layout page size"));
        let l = doc.project().layout(id).unwrap();
        assert_eq!((l.width_pt, l.height_pt), (595.28, 841.89));
    }
}
