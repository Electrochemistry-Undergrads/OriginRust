//! The options of Statistics > Descriptive Statistics > 2D Frequency Counts/Binning, laid out as Origin's
//! `twoDBinning` dialog: a group per axis (Specify Binning Range by, Minimum Bin Beginning and Maximum Bin
//! End with Auto, Step by Bin Size or Number of Bins, the outlier bins of Border Options), then Quantity to
//! Compute. Origin's Periodical, Output Binning Order, Subtotal Count and Output Matrix are not offered
//! (statistics.md 13).

use eframe::egui::{self, ComboBox, DragValue, Ui};
use sheaf_core::analysis::{AxisBins, Bin2dOptions, Bin2dStat, BinSize};

/// One axis group.
#[derive(Debug, Clone, PartialEq)]
pub struct AxisState {
    pub centers: bool,
    pub min: f64,
    pub min_auto: bool,
    pub max: f64,
    pub max_auto: bool,
    /// Step by Number of Bins (else Bin Size).
    pub by_count: bool,
    pub size: f64,
    pub size_auto: bool,
    pub count: usize,
    pub below: bool,
    pub above: bool,
}

impl Default for AxisState {
    fn default() -> Self {
        AxisState {
            centers: false,
            min: 0.0,
            min_auto: true,
            max: 10.0,
            max_auto: true,
            by_count: false,
            size: 1.0,
            size_auto: true,
            count: 10,
            below: false,
            above: false,
        }
    }
}

impl AxisState {
    pub fn bins(&self) -> AxisBins {
        AxisBins {
            min: (!self.min_auto).then_some(self.min),
            max: (!self.max_auto).then_some(self.max),
            size: if self.by_count {
                BinSize::Count(self.count)
            } else if self.size_auto {
                BinSize::Auto
            } else {
                BinSize::Width(self.size)
            },
            centers: self.centers,
            below: self.below,
            above: self.above,
        }
    }

    fn ui(&mut self, ui: &mut Ui, id: &str) {
        egui::Grid::new(id).num_columns(3).spacing([12.0, 4.0]).show(ui, |ui| {
            ui.label("Specify Binning Range by");
            ComboBox::from_id_salt((id, "by"))
                .selected_text(if self.centers { "Bin Centers" } else { "Bin Ends" })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.centers, true, "Bin Centers");
                    ui.selectable_value(&mut self.centers, false, "Bin Ends");
                });
            ui.end_row();
            let (lo, hi) = if self.centers {
                ("Minimum Bin Center", "Maximum Bin Center")
            } else {
                ("Minimum Bin Beginning", "Maximum Bin End")
            };
            ui.label(lo);
            ui.add_enabled(!self.min_auto, DragValue::new(&mut self.min).speed(0.01));
            ui.checkbox(&mut self.min_auto, "Auto");
            ui.end_row();
            ui.label(hi);
            ui.add_enabled(!self.max_auto, DragValue::new(&mut self.max).speed(0.01));
            ui.checkbox(&mut self.max_auto, "Auto");
            ui.end_row();
            ui.label("Step by");
            ui.horizontal(|ui| {
                ui.radio_value(&mut self.by_count, false, "Bin Size");
                ui.radio_value(&mut self.by_count, true, "Number of Bins");
            });
            ui.end_row();
            ui.label("Bin Size");
            ui.add_enabled(
                !self.by_count && !self.size_auto,
                DragValue::new(&mut self.size).speed(0.01).range(1e-12..=1e12),
            );
            ui.add_enabled(!self.by_count, egui::Checkbox::new(&mut self.size_auto, "Auto"));
            ui.end_row();
            ui.label("Number of Bins");
            ui.add_enabled(self.by_count, DragValue::new(&mut self.count).range(1..=10_000));
            ui.end_row();
        });
        ui.collapsing("Border Options", |ui| {
            ui.checkbox(&mut self.below, "Include Outliers < Minimum");
            ui.checkbox(&mut self.above, "Include Outliers >= Maximum");
        });
    }
}

/// The whole option block.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Bin2dState {
    pub x: AxisState,
    pub y: AxisState,
    pub stat: Bin2dStat,
}

/// The order of Origin's Quantity to Compute list.
const QUANTITIES: [Bin2dStat; 7] = [
    Bin2dStat::Min,
    Bin2dStat::Max,
    Bin2dStat::Mean,
    Bin2dStat::Median,
    Bin2dStat::Sum,
    Bin2dStat::Count,
    Bin2dStat::PercentFrequency,
];

impl Bin2dState {
    pub fn options(&self) -> Bin2dOptions {
        Bin2dOptions { x: self.x.bins(), y: self.y.bins(), stat: self.stat }
    }

    /// `x_name` and `y_name` head the two axis groups ("A(X)", "B(Y)").
    pub fn ui(&mut self, ui: &mut Ui, x_name: &str, y_name: &str) {
        ui.weak(
            "Choose a Y column (it is binned against its X column), or an X and a Y column. The quantities are \
             computed from the Y values in each bin.",
        );
        egui::CollapsingHeader::new(x_name)
            .id_salt("bin2d-x")
            .default_open(true)
            .show(ui, |ui| self.x.ui(ui, "bin2d-x"));
        egui::CollapsingHeader::new(y_name)
            .id_salt("bin2d-y")
            .default_open(true)
            .show(ui, |ui| self.y.ui(ui, "bin2d-y"));
        ui.horizontal(|ui| {
            ui.label("Quantity to Compute");
            ComboBox::from_id_salt("bin2d-stat").selected_text(self.stat.label()).show_ui(ui, |ui| {
                for q in QUANTITIES {
                    ui.selectable_value(&mut self.stat, q, q.label());
                }
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origin_s_defaults_are_automatic_bin_ends_and_count() {
        let o = Bin2dState::default().options();
        assert_eq!(o.stat, Bin2dStat::Count);
        assert_eq!(o.x, AxisBins::default());
        assert_eq!(o.y, AxisBins::default());
        assert_eq!(QUANTITIES.len(), Bin2dStat::ALL.len());
    }

    #[test]
    fn given_values_reach_the_options() {
        let mut s = Bin2dState::default();
        s.x.min_auto = false;
        s.x.min = 2.0;
        s.x.size_auto = false;
        s.x.size = 0.5;
        s.y.by_count = true;
        s.y.count = 4;
        s.y.centers = true;
        s.y.above = true;
        let o = s.options();
        assert_eq!(o.x.min, Some(2.0));
        assert_eq!(o.x.max, None);
        assert_eq!(o.x.size, BinSize::Width(0.5));
        assert_eq!(o.y.size, BinSize::Count(4));
        assert!(o.y.centers && o.y.above && !o.y.below);
    }
}
