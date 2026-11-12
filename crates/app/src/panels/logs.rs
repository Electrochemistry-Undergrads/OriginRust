//! The Messages Log and the Results Log: what Sheaf has said in the status bar and in message
//! boxes, and the report text the analysis and fit tools produce. Both keep the order in which
//! things happened and can be cleared.

use super::PanelId;
use crate::actions::Action;
use eframe::egui::{self, Color32, RichText};
use std::time::{Duration, Instant};

/// Older entries than this many are dropped.
const KEEP: usize = 1000;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Level {
    Info,
    Warning,
}

#[derive(Clone, Debug)]
pub struct Message {
    /// Time since Sheaf started (the program has no clock library; a wall clock would need a
    /// time zone).
    pub at: Duration,
    pub level: Level,
    pub text: String,
}

/// The report of one run of a tool: a heading and plain lines.
#[derive(Clone, Debug)]
pub struct ResultBlock {
    pub at: Duration,
    pub title: String,
    pub lines: Vec<String>,
}

pub struct Logs {
    start: Instant,
    pub messages: Vec<Message>,
    pub results: Vec<ResultBlock>,
}

impl Default for Logs {
    fn default() -> Self {
        Logs { start: Instant::now(), messages: Vec::new(), results: Vec::new() }
    }
}

impl Logs {
    pub fn info(&mut self, text: impl Into<String>) {
        self.push(Level::Info, text.into());
    }

    pub fn warn(&mut self, text: impl Into<String>) {
        self.push(Level::Warning, text.into());
    }

    fn push(&mut self, level: Level, text: String) {
        let at = self.start.elapsed();
        self.messages.push(Message { at, level, text });
        if self.messages.len() > KEEP {
            self.messages.remove(0);
        }
    }

    pub fn result(&mut self, title: impl Into<String>, lines: Vec<String>) {
        let at = self.start.elapsed();
        self.results.push(ResultBlock { at, title: title.into(), lines });
        if self.results.len() > KEEP {
            self.results.remove(0);
        }
    }
}

/// `+01:05` for 65 seconds after the start.
fn clock(at: Duration) -> String {
    let s = at.as_secs();
    format!("+{:02}:{:02}", s / 60, s % 60)
}

const WARNING: Color32 = Color32::from_rgb(170, 70, 0);
const DIM: Color32 = Color32::from_rgb(110, 110, 118);

fn clear_row(ui: &mut egui::Ui, label: &str, empty: bool) -> bool {
    ui.horizontal(|ui| ui.add_enabled(!empty, egui::Button::new(label)).clicked()).inner
}

pub fn show_messages(ui: &mut egui::Ui, logs: &Logs, actions: &mut Vec<Action>) {
    ui.add_space(2.0);
    if clear_row(ui, "Clear Messages", logs.messages.is_empty()) {
        actions.push(Action::ClearLog(PanelId::Messages));
    }
    ui.separator();
    egui::ScrollArea::vertical().auto_shrink([false, false]).stick_to_bottom(true).show(ui, |ui| {
        if logs.messages.is_empty() {
            ui.label(RichText::new("No messages yet").color(DIM));
        }
        for m in &logs.messages {
            let color = if m.level == Level::Warning { WARNING } else { Color32::BLACK };
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(clock(m.at)).color(DIM).monospace());
                ui.label(RichText::new(&m.text).color(color));
            });
        }
    });
}

pub fn show_results(ui: &mut egui::Ui, logs: &Logs, actions: &mut Vec<Action>) {
    ui.add_space(2.0);
    if clear_row(ui, "Clear Results", logs.results.is_empty()) {
        actions.push(Action::ClearLog(PanelId::Results));
    }
    ui.separator();
    egui::ScrollArea::vertical().auto_shrink([false, false]).stick_to_bottom(true).show(ui, |ui| {
        if logs.results.is_empty() {
            ui.label(RichText::new("No results yet: run an analysis or a fit").color(DIM));
        }
        for block in &logs.results {
            ui.horizontal(|ui| {
                ui.label(RichText::new(clock(block.at)).color(DIM).monospace());
                ui.label(RichText::new(&block.title).strong());
            });
            for line in &block.lines {
                ui.label(RichText::new(line).monospace());
            }
            ui.add_space(6.0);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logs_keep_the_order_and_forget_the_oldest_beyond_the_limit() {
        let mut l = Logs::default();
        l.info("first");
        l.warn("second");
        assert_eq!(l.messages.len(), 2);
        assert_eq!(l.messages[1].level, Level::Warning);
        for k in 0..KEEP + 5 {
            l.info(format!("m{k}"));
        }
        assert_eq!(l.messages.len(), KEEP);
        assert_eq!(l.messages.last().unwrap().text, format!("m{}", KEEP + 4));
    }

    #[test]
    fn the_clock_reads_minutes_and_seconds() {
        assert_eq!(clock(Duration::from_secs(65)), "+01:05");
        assert_eq!(clock(Duration::from_millis(900)), "+00:00");
    }
}
