//! The 3D Axis window (T164): Format > Axes > Z Axis..., and X Axis... or Y Axis... on a 3D layer.
//! It sets From, To and Increment of the box's X, Y and Z axes (`Space3dCoords::scales`), each
//! automatic until set. Origin's 3D axis dialog was not probed; this is Sheaf's smaller window.

use eframe::egui;
use sheaf_core::graph::Space3dScale;
use sheaf_core::{Document, Project, WindowId};

use super::graph_props::optional_number;

pub enum Outcome {
    Open,
    Closed,
    Done,
}

const NAMES: [&str; 3] = ["X", "Y", "Z"];

pub struct Axis3dDialog {
    graph: WindowId,
    layer: usize,
    /// The axis shown: 0 X, 1 Y, 2 Z.
    pub dim: usize,
    pub scales: [Space3dScale; 3],
    /// What the box shows now on each axis (From, To, Increment), offered next to Auto.
    shown: [(f64, f64, f64); 3],
    /// An XYY layer's Y axis is its depth: one tick per plot, not set here.
    xyy: bool,
    problem: Option<String>,
}

/// Why `scales` cannot be used, if they cannot.
pub fn check(scales: &[Space3dScale; 3]) -> Result<(), String> {
    for (s, name) in scales.iter().zip(NAMES) {
        if let (Some(a), Some(b)) = (s.from, s.to)
            && a >= b
        {
            return Err(format!("{name}: From must be less than To"));
        }
        if s.step.is_some_and(|v| v <= 0.0 || !v.is_finite()) {
            return Err(format!("{name}: Increment must be more than 0"));
        }
    }
    Ok(())
}

/// Sets the scales of a 3D layer as one undo step.
pub fn set_scales(doc: &mut Document, graph: WindowId, layer: usize, scales: [Space3dScale; 3]) -> Result<(), String> {
    check(&scales)?;
    doc.edit("3D Axis", |p| {
        let l = p.graph_mut(graph).and_then(|g| g.layers.get_mut(layer)).ok_or("the layer is gone")?;
        match &mut l.coords {
            sheaf_core::graph::Coords::Space3d(c) => {
                c.scales = scales;
                Ok(())
            }
            _ => Err("the layer is not a 3D layer".to_string()),
        }
    })
}

impl Axis3dDialog {
    /// The window on axis `dim` of a 3D layer; `None` when the layer is not 3D.
    pub fn new(project: &Project, graph: WindowId, layer: usize, dim: usize) -> Option<Self> {
        let l = project.graph(graph)?.layers.get(layer)?;
        let c = l.coords.space3d()?;
        let shown = sheaf_plot::space3d_scales(project, l).unwrap_or([(-1.0, 1.0, 0.5); 3]);
        Some(Axis3dDialog { graph, layer, dim: dim.min(2), scales: c.scales, shown, xyy: c.xyy, problem: None })
    }

    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> Outcome {
        let alive = doc.project().graph(self.graph).and_then(|g| g.layers.get(self.layer)).is_some();
        if !alive || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            return Outcome::Closed;
        }
        let mut open = true;
        let mut result = Outcome::Open;
        egui::Window::new("3D Axis")
            .id(egui::Id::new("axis3d_window"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for (k, name) in NAMES.iter().enumerate() {
                        ui.selectable_value(&mut self.dim, k, format!("{name} Axis"));
                    }
                });
                ui.separator();
                let d = self.dim;
                if self.xyy && d == 1 {
                    ui.label("The Y axis of a 3D XYY graph has one tick per plot.");
                } else {
                    let (from, to, step) = self.shown[d];
                    let s = &mut self.scales[d];
                    let speed = ((to - from).abs() / 100.0).max(1e-9);
                    egui::Grid::new("axis3d_grid").num_columns(2).show(ui, |ui| {
                        ui.label("From");
                        optional_number(ui, &mut s.from, Some(from), speed);
                        ui.end_row();
                        ui.label("To");
                        optional_number(ui, &mut s.to, Some(to), speed);
                        ui.end_row();
                        ui.label("Increment");
                        optional_number(ui, &mut s.step, Some(step), speed);
                        ui.end_row();
                    });
                }
                if let Some(e) = &self.problem {
                    ui.colored_label(ui.visuals().error_fg_color, e);
                }
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        match set_scales(doc, self.graph, self.layer, self.scales) {
                            Ok(()) => result = Outcome::Done,
                            Err(e) => self.problem = Some(e),
                        }
                    }
                    if ui.button("Apply").clicked() {
                        match set_scales(doc, self.graph, self.layer, self.scales) {
                            Ok(()) => {
                                self.problem = None;
                                if let Some(l) = doc.project().graph(self.graph).and_then(|g| g.layers.get(self.layer))
                                    && let Some(shown) = sheaf_plot::space3d_scales(doc.project(), l)
                                {
                                    self.shown = shown;
                                }
                            }
                            Err(e) => self.problem = Some(e),
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        result = Outcome::Closed;
                    }
                });
            });
        if !open { Outcome::Closed } else { result }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_must_be_below_to_and_the_increment_positive() {
        let mut s = [Space3dScale::default(); 3];
        assert!(check(&s).is_ok());
        s[2] = Space3dScale { from: Some(5.0), to: Some(1.0), step: None };
        assert_eq!(check(&s).unwrap_err(), "Z: From must be less than To");
        s[2] = Space3dScale { from: None, to: None, step: Some(0.0) };
        assert_eq!(check(&s).unwrap_err(), "Z: Increment must be more than 0");
    }
}
