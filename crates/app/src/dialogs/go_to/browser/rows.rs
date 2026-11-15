//! Edits the row subset of one added browser range without applying it to the parent.

use eframe::egui;

pub(super) enum Outcome {
    Open,
    Closed,
    Selected(Option<(usize, usize)>),
}

pub(super) struct RowsDialog {
    entire: bool,
    first: String,
    last: String,
    maximum: usize,
    first_frame: bool,
    error: Option<String>,
}

impl RowsDialog {
    pub fn new(rows: Option<(usize, usize)>, maximum: usize) -> Self {
        let (first, last) = rows.unwrap_or((1, maximum));
        Self {
            entire: rows.is_none(),
            first: first.to_string(),
            last: last.to_string(),
            maximum,
            first_frame: true,
            error: None,
        }
    }

    fn endpoint(&self, text: &str) -> Option<usize> {
        // The native integer edit consumes a decimal prefix: "2.9", "2+1" and "2abc" all yield 2.
        let text = text.trim_start();
        let text = text.strip_prefix('+').unwrap_or(text);
        let end = text.bytes().take_while(u8::is_ascii_digit).count();
        let row = text[..end].parse::<usize>().ok()?;
        (1..=self.maximum).contains(&row).then_some(row)
    }

    fn selection(&self) -> Result<Option<(usize, usize)>, String> {
        if self.entire {
            return Ok(None);
        }
        if let (Some(first), Some(last)) = (self.endpoint(&self.first), self.endpoint(&self.last)) {
            return Ok(Some((first, last)));
        }
        Err(format!("Enter integer row numbers from 1 to {}.", self.maximum))
    }

    pub fn show(&mut self, ctx: &egui::Context) -> Outcome {
        let mut open = true;
        let mut outcome = Outcome::Open;
        let mut accept = false;
        let first_frame = self.first_frame;
        // Enter is the dialog's default action; it must not toggle a focused checkbox first.
        let enter = ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
        egui::Window::new("Range").open(&mut open).collapsible(false).resizable(false).default_width(245.0).show(
            ctx,
            |ui| {
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    egui::Grid::new("goto_browser_row_bounds").show(ui, |ui| {
                        let label = ui.label("Entire Column(s)");
                        let entire = ui.checkbox(&mut self.entire, "").labelled_by(label.id);
                        let _ = ctx.accesskit_node_builder(entire.id, |node| node.set_label("Entire Column(s)"));
                        if self.first_frame {
                            entire.request_focus();
                            self.first_frame = false;
                        }
                        ui.end_row();
                        for (label, text) in [("From", &mut self.first), ("To", &mut self.last)] {
                            let label = ui.label(label);
                            ui.add_enabled(!self.entire, egui::TextEdit::singleline(text).desired_width(100.0))
                                .labelled_by(label.id);
                            ui.end_row();
                        }
                    });
                    ui.colored_label(egui::Color32::BLUE, format!("Rows: 1 to {}", self.maximum));
                });
                if let Some(error) = &self.error {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                }
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let cancel = ui.add_sized([64.0, 24.0], egui::Button::new("Cancel"));
                        if cancel.clicked() || (!first_frame && enter && cancel.has_focus()) {
                            outcome = Outcome::Closed;
                        }
                        accept = ui.add_sized([64.0, 24.0], egui::Button::new("OK")).clicked();
                    });
                });
                accept |= !first_frame && enter && matches!(outcome, Outcome::Open);
            },
        );
        if !open || ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            return Outcome::Closed;
        }
        if accept && matches!(outcome, Outcome::Open) {
            match self.selection() {
                Ok(rows) => outcome = Outcome::Selected(rows),
                Err(error) => self.error = Some(error),
            }
        }
        outcome
    }
}
