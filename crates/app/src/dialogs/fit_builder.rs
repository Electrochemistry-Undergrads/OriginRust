//! Tools > Fitting Function Builder... (F8, T132): a fit function of one's own, y = f(x) in Sheaf's
//! expression language with named parameters and their initial values, saved as an Origin `.FDF` file in
//! the user's fitting-function folder; Analysis > Fitting's "Load .FDF" category reads it. Built from
//! Origin's documented behaviour, not measured; rules in `docs/spec/fitting.md`, "Fitting Function Builder".

use eframe::egui;

pub enum BuilderOutcome {
    Open,
    Closed,
    /// Save `text` as `<name>.FDF`.
    Save {
        name: String,
        text: String,
    },
}

pub struct FitBuilder {
    pub name: String,
    pub description: String,
    /// `a=1, b=0.5`: the parameters in order with their initial values (1 when left out).
    pub params: String,
    pub formula: String,
    /// Why the function cannot be saved, after the last Save.
    pub problem: Option<String>,
}

impl Default for FitBuilder {
    fn default() -> Self {
        FitBuilder {
            name: "MyFunction".into(),
            description: String::new(),
            params: "a=1, b=1".into(),
            formula: "a*x + b".into(),
            problem: None,
        }
    }
}

/// Reads `a=1, b` as `[("a", 1.0), ("b", 1.0)]`.
pub fn parse_params(text: &str) -> Result<Vec<(String, f64)>, String> {
    let mut out: Vec<(String, f64)> = Vec::new();
    for part in text.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        let (name, value) = match part.split_once('=') {
            Some((n, v)) => (n.trim(), v.trim().parse::<f64>().map_err(|_| format!("\"{v}\" is not a number"))?),
            None => (part, 1.0),
        };
        let ok = name.chars().next().is_some_and(|c| c.is_alphabetic() || c == '_')
            && name.chars().all(|c| c.is_alphanumeric() || c == '_');
        if !ok || name == "x" || name == "y" {
            return Err(format!("\"{name}\" cannot be a parameter name"));
        }
        if out.iter().any(|(n, _)| n == name) {
            return Err(format!("the parameter \"{name}\" is listed twice"));
        }
        out.push((name.to_string(), value));
    }
    if out.is_empty() {
        return Err("give at least one parameter".into());
    }
    Ok(out)
}

impl FitBuilder {
    /// The `.FDF` text of the function, checked by reading it back as a fit function.
    pub fn build(&self) -> Result<String, String> {
        let name = self.name.trim();
        let ok = !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_');
        if !ok {
            return Err("the function name takes letters, digits and _ only".into());
        }
        let params = parse_params(&self.params)?;
        let text = sheaf_fit::fdf::write_fdf(name, self.description.trim(), &params, self.formula.trim());
        let f = sheaf_fit::fdf::parse_fdf(&text).map_err(|e| e.to_string())?;
        f.to_fit_function().map_err(|e| e.to_string())?;
        Ok(text)
    }

    pub fn show(&mut self, ctx: &egui::Context) -> BuilderOutcome {
        let mut open = true;
        let mut outcome = BuilderOutcome::Open;
        egui::Window::new("Fitting Function Builder")
            .id(egui::Id::new("fit_builder"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                egui::Grid::new("fit_builder_grid").num_columns(2).show(ui, |ui| {
                    ui.label("Function Name");
                    ui.text_edit_singleline(&mut self.name);
                    ui.end_row();
                    ui.label("Brief Description");
                    ui.text_edit_singleline(&mut self.description);
                    ui.end_row();
                    ui.label("Parameters");
                    ui.text_edit_singleline(&mut self.params);
                    ui.end_row();
                    ui.label("y =");
                    ui.text_edit_singleline(&mut self.formula);
                    ui.end_row();
                });
                ui.weak("Independent variable x; parameters as name=initial value, separated by commas.");
                if let Some(p) = &self.problem {
                    ui.colored_label(egui::Color32::from_rgb(180, 30, 30), p);
                }
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        match self.build() {
                            Ok(text) => {
                                self.problem = None;
                                outcome = BuilderOutcome::Save { name: self.name.trim().to_string(), text };
                            }
                            Err(e) => self.problem = Some(e),
                        }
                    }
                    if ui.button("Close").clicked() {
                        outcome = BuilderOutcome::Closed;
                    }
                });
            });
        if !open { BuilderOutcome::Closed } else { outcome }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parameters_are_named_with_initial_values_and_bad_input_is_explained() {
        assert_eq!(parse_params("a=1, b").unwrap(), [("a".to_string(), 1.0), ("b".to_string(), 1.0)]);
        assert!(parse_params("").is_err());
        assert!(parse_params("a=1, a=2").is_err());
        assert!(parse_params("x=1").is_err());
        assert!(parse_params("a=one").is_err());
        let mut b = FitBuilder::default();
        assert!(b.build().is_ok());
        b.formula = "a*x +".into();
        assert!(b.build().is_err(), "a formula error is reported before saving");
        b.formula = "a*x".into();
        b.name = "my func".into();
        assert!(b.build().is_err());
    }
}
