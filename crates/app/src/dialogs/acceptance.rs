//! Statistics > Acceptance Sampling... (T369): an attribute single sampling plan designed from AQL, LTPD and
//! the two risks, or a plan typed in, with an optional lot size. It needs no worksheet data; the result goes
//! to a new workbook with a graph of the OC curve. Rules: `docs/spec/statistics.md`, "Acceptance sampling".

use eframe::egui;
use egui::DragValue;
use sheaf_core::Document;
use sheaf_core::analysis::{AcceptanceSpec, Report};

/// The Acceptance Sampling window.
pub struct AcceptanceDialog {
    /// The spec as edited; `plan` and `lot` are taken from the fields below when OK is pressed.
    pub spec: AcceptanceSpec,
    /// Evaluate the plan typed in instead of designing one.
    pub by_hand: bool,
    pub n: u64,
    pub c: u64,
    pub use_lot: bool,
    pub lot: u64,
    problem: Option<String>,
    undo_token: u64,
}

/// What the window did in a frame.
pub enum AcceptanceOutcome {
    Open,
    Closed,
    Done(Box<Report>),
}

impl Default for AcceptanceDialog {
    fn default() -> Self {
        AcceptanceDialog {
            spec: AcceptanceSpec::default(),
            by_hand: false,
            n: 50,
            c: 1,
            use_lot: false,
            lot: 1000,
            problem: None,
            undo_token: crate::dialogs::analysis::next_undo_token(),
        }
    }
}

impl AcceptanceDialog {
    /// The request as the fields say.
    pub fn request(&self) -> AcceptanceSpec {
        AcceptanceSpec {
            plan: self.by_hand.then_some((self.n, self.c)),
            lot: self.use_lot.then_some(self.lot),
            ..self.spec.clone()
        }
    }

    /// Runs the request as one undo step.
    pub fn run(&mut self, doc: &mut Document) -> Result<Report, String> {
        let spec = self.request();
        doc.apply_merging("Acceptance sampling", self.undo_token, |p| p.acceptance_sampling(&spec))
            .map_err(|e| e.to_string())
    }

    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> AcceptanceOutcome {
        let mut open = true;
        let mut outcome = AcceptanceOutcome::Open;
        egui::Window::new("Acceptance Sampling").open(&mut open).collapsible(false).resizable(false).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.radio_value(&mut self.by_hand, false, "Design a plan");
                ui.radio_value(&mut self.by_hand, true, "Evaluate a plan");
            });
            egui::Grid::new("acceptance_grid").num_columns(2).show(ui, |ui| {
                let s = &mut self.spec;
                ui.label("AQL (fraction defective)");
                ui.add(DragValue::new(&mut s.aql).speed(0.0005).range(0.0001..=0.9999).max_decimals(4));
                ui.end_row();
                ui.label("LTPD (fraction defective)");
                ui.add(DragValue::new(&mut s.ltpd).speed(0.0005).range(0.0001..=0.9999).max_decimals(4));
                ui.end_row();
                ui.add_enabled_ui(!self.by_hand, |ui| ui.label("Producer's risk (alpha)"));
                ui.add_enabled(!self.by_hand, DragValue::new(&mut s.alpha).speed(0.001).range(0.001..=0.5));
                ui.end_row();
                ui.add_enabled_ui(!self.by_hand, |ui| ui.label("Consumer's risk (beta)"));
                ui.add_enabled(!self.by_hand, DragValue::new(&mut s.beta).speed(0.001).range(0.001..=0.5));
                ui.end_row();
                ui.add_enabled_ui(self.by_hand, |ui| ui.label("Sample size n"));
                ui.add_enabled(self.by_hand, DragValue::new(&mut self.n).range(1..=1_000_000));
                ui.end_row();
                ui.add_enabled_ui(self.by_hand, |ui| ui.label("Acceptance number c"));
                ui.add_enabled(self.by_hand, DragValue::new(&mut self.c).range(0..=999_999));
                ui.end_row();
                ui.checkbox(&mut self.use_lot, "Lot size (AOQ, ATI)");
                ui.add_enabled(self.use_lot, DragValue::new(&mut self.lot).range(1..=100_000_000));
                ui.end_row();
            });
            ui.weak("Binomial operating characteristic: a lot is accepted with at most c defectives in the sample.");
            if let Some(p) = &self.problem {
                ui.colored_label(ui.visuals().error_fg_color, p);
            }
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("OK").clicked() {
                    match self.run(doc) {
                        Ok(report) => outcome = AcceptanceOutcome::Done(Box::new(report)),
                        Err(e) => self.problem = Some(e),
                    }
                }
                if ui.button("Cancel").clicked() {
                    outcome = AcceptanceOutcome::Closed;
                }
            });
        });
        if !open && matches!(outcome, AcceptanceOutcome::Open) {
            outcome = AcceptanceOutcome::Closed;
        }
        outcome
    }
}
