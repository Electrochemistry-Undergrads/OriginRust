//! The question at start-up when an earlier run left work behind: Recover, Discard or Later.

use eframe::egui::{self, RichText};
use sheaf_io::recovery::Recovery;
use std::time::{SystemTime, UNIX_EPOCH};

/// What the person chose in a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryChoice {
    Open,
    /// Open the newest recovery as a new, modified project.
    Recover,
    /// Delete the newest recovery.
    Discard,
    /// Keep all of them for the next start.
    Later,
}

/// The recoveries on offer, newest first.
pub struct RecoveryOffer {
    pub orphans: Vec<Recovery>,
}

impl RecoveryOffer {
    pub fn new(orphans: Vec<Recovery>) -> Self {
        RecoveryOffer { orphans }
    }

    /// The window. It names the newest recovery and says how many others there are.
    pub fn show(&self, ctx: &egui::Context, now: SystemTime) -> RecoveryChoice {
        let mut choice = RecoveryChoice::Open;
        let Some(newest) = self.orphans.first() else { return RecoveryChoice::Later };
        egui::Modal::new(egui::Id::new("recovery")).show(ctx, |ui| {
            ui.set_max_width(420.0);
            egui::Frame::new().inner_margin(egui::Margin::same(8)).show(ui, |ui| {
                ui.label(RichText::new("Sheaf found work that was not saved").strong());
                ui.add_space(4.0);
                ui.label("The last run ended without a normal exit. This project was recovered:");
                ui.add_space(4.0);
                ui.label(RichText::new(name(newest)).strong());
                ui.label(saved_text(newest, now));
                if let Some(path) = newest.sidecar.as_ref().and_then(|s| s.original.as_deref()) {
                    ui.weak(path);
                }
                if self.orphans.len() > 1 {
                    ui.add_space(4.0);
                    ui.label(format!("There are {} recoveries; the buttons act on the newest.", self.orphans.len()));
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Recover").clicked() {
                        choice = RecoveryChoice::Recover;
                    }
                    if ui.button("Discard").clicked() {
                        choice = RecoveryChoice::Discard;
                    }
                    if ui.button("Later").clicked() {
                        choice = RecoveryChoice::Later;
                    }
                });
            });
        });
        choice
    }
}

/// The file name of the project the recovery belongs to, or "Untitled".
pub fn name(r: &Recovery) -> String {
    r.sidecar
        .as_ref()
        .and_then(|s| s.original_path())
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "Untitled".to_string())
}

/// "Saved 5 minutes ago".
pub fn saved_text(r: &Recovery, now: SystemTime) -> String {
    let at = r.sidecar.as_ref().map_or(r.modified, |s| UNIX_EPOCH + std::time::Duration::from_secs(s.saved_at));
    let secs = now.duration_since(at).map_or(0, |d| d.as_secs());
    format!("Saved {}", age_text(secs))
}

fn age_text(secs: u64) -> String {
    let plural = |n: u64, unit: &str| format!("{n} {unit}{} ago", if n == 1 { "" } else { "s" });
    match secs {
        0..60 => "less than a minute ago".to_string(),
        60..3600 => plural(secs / 60, "minute"),
        3600..86400 => plural(secs / 3600, "hour"),
        _ => plural(secs / 86400, "day"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ages_are_written_in_the_largest_whole_unit() {
        assert_eq!(age_text(5), "less than a minute ago");
        assert_eq!(age_text(60), "1 minute ago");
        assert_eq!(age_text(125), "2 minutes ago");
        assert_eq!(age_text(3 * 3600 + 5), "3 hours ago");
        assert_eq!(age_text(86400), "1 day ago");
        assert_eq!(age_text(10 * 86400), "10 days ago");
    }
}
