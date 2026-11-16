//! Tools > Fitting Function Organizer (F9, T105): the built-in fit functions by category, read-only, as
//! Origin 2016's organizer shows its library (measured 2026-11-16: a tree of categories and functions; for
//! a function Function Name, File Name, Brief Description, References, Function Type Built-in, Function
//! Model Explicit, Independent Variables x, Dependent Variables y, Parameter Names, Function Form
//! Expression, Function, Peak Function; Equation, Sample Curve and Hints tabs). Rules in
//! `docs/spec/fitting.md`, "Fitting Function Organizer".

use eframe::egui::{self, RichText};
use sheaf_core::fitting::FunctionInfo;

pub struct FitOrganizer {
    functions: Vec<FunctionInfo>,
    pub selected: Option<String>,
}

impl Default for FitOrganizer {
    fn default() -> Self {
        FitOrganizer { functions: sheaf_core::fitting::builtin_functions(), selected: None }
    }
}

impl FitOrganizer {
    /// The categories in library order, each once.
    pub fn categories(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for f in &self.functions {
            if !out.contains(&f.category) {
                out.push(f.category.clone());
            }
        }
        out
    }

    /// Shows the window; false when it was closed.
    pub fn show(&mut self, ctx: &egui::Context) -> bool {
        let mut open = true;
        let mut close = false;
        egui::Window::new("Fitting Function Organizer")
            .id(egui::Id::new("fit_organizer"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.horizontal_top(|ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("fit_org_tree")
                        .max_height(380.0)
                        .min_scrolled_width(220.0)
                        .show(ui, |ui| {
                            for cat in self.categories() {
                                egui::CollapsingHeader::new(&cat).default_open(false).show(ui, |ui| {
                                    for f in self.functions.iter().filter(|f| f.category == cat) {
                                        let on = self.selected.as_deref() == Some(f.name.as_str());
                                        if ui.selectable_label(on, &f.name).clicked() {
                                            self.selected = Some(f.name.clone());
                                        }
                                    }
                                });
                            }
                        });
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.set_min_width(360.0);
                        match self.selected.as_ref().and_then(|n| self.functions.iter().find(|f| &f.name == n)) {
                            None => {
                                ui.weak("Pick a function to see its definition.");
                            }
                            Some(f) => {
                                egui::Grid::new("fit_org_fields").num_columns(2).show(ui, |ui| {
                                    let mut row = |k: &str, v: String| {
                                        ui.label(RichText::new(k).strong());
                                        ui.label(v);
                                        ui.end_row();
                                    };
                                    row("Function Name", f.name.clone());
                                    row("Category", f.category.clone());
                                    row("Function Type", "Built-in".into());
                                    row("Function Model", "Explicit".into());
                                    row("Independent Variables", "x".into());
                                    row("Dependent Variables", "y".into());
                                    row("Parameter Names", f.params.join(","));
                                    row("Function Form", "Expression".into());
                                    row(
                                        "Function",
                                        f.formula
                                            .as_ref()
                                            .map_or("(computed by Sheaf, no formula)".into(), |e| format!("y = {e}")),
                                    );
                                    row(
                                        "Peak Function",
                                        if f.category == "Peak Functions" { "Yes" } else { "No" }.into(),
                                    );
                                    let derived =
                                        f.derived.iter().map(|(n, e)| format!("{n} = {e}")).collect::<Vec<_>>();
                                    row(
                                        "Derived Parameters",
                                        if derived.is_empty() { "-".into() } else { derived.join("\n") },
                                    );
                                });
                            }
                        }
                    });
                });
                if ui.button("Close").clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    close = true;
                }
            });
        open && !close
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_library_is_listed_by_category_with_gauss_among_the_peaks() {
        let o = FitOrganizer::default();
        let cats = o.categories();
        assert!(cats.iter().any(|c| c == "Peak Functions"), "{cats:?}");
        let gauss = o.functions.iter().find(|f| f.name == "Gauss").expect("Gauss is built in");
        assert_eq!(gauss.category, "Peak Functions");
        assert_eq!(gauss.params, ["y0", "xc", "w", "A"], "Origin's Parameter Names y0,xc,w,A");
    }
}
