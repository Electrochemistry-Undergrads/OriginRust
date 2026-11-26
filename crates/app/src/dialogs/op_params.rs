//! The operation tree's Parameters... (T469): the settings of a linked analysis, every number, switch and choice of its
//! tool as a field (from the tool's saved form), and OK runs the operation again with them
//! (`Project::set_operation_tool`). A choice is the name of one of the tool's options; one that is not leaves the
//! window open with the reason.

use eframe::egui;
use serde_json::Value;
use sheaf_core::analysis_link::LinkTool;

pub enum ParamsOutcome {
    Open,
    Closed,
    Apply { index: usize, tool: Box<LinkTool> },
}

pub struct OpParamsDialog {
    pub index: usize,
    pub name: &'static str,
    /// The tool's saved form, and its leaves by path (`npts`, `method.AdjacentAveraging.weighted`).
    form: Value,
    pub fields: Vec<(String, Value)>,
    /// Why the last OK was refused.
    pub error: Option<String>,
}

fn leaves(v: &Value, path: &str, out: &mut Vec<(String, Value)>) {
    let join = |k: &str| if path.is_empty() { k.to_string() } else { format!("{path}.{k}") };
    match v {
        Value::Object(m) => {
            for (k, x) in m {
                leaves(x, &join(k), out);
            }
        }
        Value::Array(a) => {
            for (i, x) in a.iter().enumerate() {
                leaves(x, &join(&i.to_string()), out);
            }
        }
        leaf => out.push((path.to_string(), leaf.clone())),
    }
}

fn set(v: &mut Value, path: &str, new: Value) {
    let mut at = v;
    for k in path.split('.') {
        at = match at {
            Value::Object(m) => m.entry(k.to_string()).or_insert(Value::Null),
            Value::Array(a) => match k.parse::<usize>().ok().and_then(|i| a.get_mut(i)) {
                Some(x) => x,
                None => return,
            },
            _ => return,
        };
    }
    *at = new;
}

impl OpParamsDialog {
    pub fn new(index: usize, name: &'static str, tool: &LinkTool) -> Option<Self> {
        let form = serde_json::to_value(tool).ok()?;
        let mut fields = Vec::new();
        leaves(&form, "", &mut fields);
        Some(OpParamsDialog { index, name, form, fields, error: None })
    }

    /// The tool with the fields as edited, or why not.
    pub fn tool(&self) -> Result<LinkTool, String> {
        let mut form = self.form.clone();
        for (path, v) in &self.fields {
            set(&mut form, path, v.clone());
        }
        serde_json::from_value(form).map_err(|e| e.to_string())
    }

    pub fn show(&mut self, ctx: &egui::Context) -> ParamsOutcome {
        let mut open = true;
        let mut outcome = ParamsOutcome::Open;
        egui::Window::new(format!("{} Parameters", self.name))
            .id(egui::Id::new("op_params"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                if self.fields.is_empty() {
                    ui.label("This operation has no settings.");
                }
                egui::Grid::new("op_params_grid").num_columns(2).show(ui, |ui| {
                    for (path, v) in &mut self.fields {
                        // The last part of the path names the field; a tool's own name in front is left out.
                        let label = path.split('.').skip(1).collect::<Vec<_>>().join(" ");
                        ui.label(if label.is_empty() { path.clone() } else { label });
                        match v {
                            Value::Bool(b) => {
                                ui.checkbox(b, "");
                            }
                            Value::Number(n) => {
                                if let Some(mut i) = n.as_i64() {
                                    if ui.add(egui::DragValue::new(&mut i)).changed() {
                                        *v = Value::from(i);
                                    }
                                } else {
                                    let mut x = n.as_f64().unwrap_or(0.0);
                                    if ui.add(egui::DragValue::new(&mut x).speed(0.01)).changed() {
                                        *v = serde_json::Number::from_f64(x).map_or(Value::Null, Value::Number);
                                    }
                                }
                            }
                            Value::String(s) => {
                                ui.text_edit_singleline(s);
                            }
                            _ => {
                                ui.label("(none)");
                            }
                        }
                        ui.end_row();
                    }
                });
                if let Some(e) = &self.error {
                    ui.colored_label(egui::Color32::from_rgb(180, 0, 0), e);
                }
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        match self.tool() {
                            Ok(tool) => outcome = ParamsOutcome::Apply { index: self.index, tool: Box::new(tool) },
                            Err(e) => self.error = Some(format!("These settings cannot be used: {e}")),
                        }
                    }
                    if ui.button("Cancel").clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        outcome = ParamsOutcome::Closed;
                    }
                });
            });
        if !open { ParamsOutcome::Closed } else { outcome }
    }
}
