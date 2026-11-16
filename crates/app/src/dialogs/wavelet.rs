//! Options of Signal Processing > Wavelet > Decompose, Reconstruction, Denoise and Smooth, for the
//! shared analysis window in `analysis.rs`. The fields follow the X-Function windows of Origin
//! `dwt`, `idwt`, `wtdenoise` and `wtsmooth` (spec `docs/spec/analysis-tools.md` 19.7): Wavelet Type,
//! Extension Mode ("Boundary" in Reconstruction), Threshold Type, Thresholding Level, Threshold of
//! every Level (%) and Cutoff (%).

use eframe::egui::{self, DragValue};
use sheaf_core::analysis::{
    Tool, Wavelet, WaveletDecomposeSpec, WaveletDenoiseSpec, WaveletExtension, WaveletReconstructSpec,
    WaveletSmoothSpec, WaveletThresholdKind,
};
use sheaf_core::ids::ColumnId;

/// The most levels the Thresholding Level box offers (Origin: 1 to 7).
const MAX_LEVEL: usize = 7;

/// The options of the four tools; the window uses the one of its kind.
#[derive(Debug, Clone, Copy)]
pub struct WaveletOptions {
    pub decompose: WaveletDecomposeSpec,
    /// Wavelet and boundary of Reconstruction; the detail column comes from the window.
    pub reconstruct: (Wavelet, WaveletExtension),
    pub denoise: WaveletDenoiseSpec,
    pub smooth: WaveletSmoothSpec,
}

impl Default for WaveletOptions {
    fn default() -> Self {
        WaveletOptions {
            decompose: WaveletDecomposeSpec::default(),
            reconstruct: (Wavelet::Haar, WaveletExtension::Periodic),
            denoise: WaveletDenoiseSpec::default(),
            smooth: WaveletSmoothSpec::default(),
        }
    }
}

impl WaveletOptions {
    pub fn decompose_tool(&self) -> Tool {
        Tool::WaveletDecompose(self.decompose)
    }

    pub fn reconstruct_tool(&self, detail: ColumnId) -> Tool {
        Tool::WaveletReconstruct(WaveletReconstructSpec {
            detail,
            wavelet: self.reconstruct.0,
            extension: self.reconstruct.1,
        })
    }

    pub fn denoise_tool(&self) -> Tool {
        Tool::WaveletDenoise(self.denoise)
    }

    pub fn smooth_tool(&self) -> Tool {
        Tool::WaveletSmooth(self.smooth)
    }
}

fn wavelet_combo(ui: &mut egui::Ui, id: &str, label: &str, w: &mut Wavelet) {
    ui.horizontal(|ui| {
        ui.label(label);
        egui::ComboBox::from_id_salt(id).selected_text(w.name()).show_ui(ui, |ui| {
            for c in Wavelet::all() {
                ui.selectable_value(w, c, c.name());
            }
        });
    });
}

fn extension_combo(ui: &mut egui::Ui, id: &str, label: &str, e: &mut WaveletExtension) {
    ui.horizontal(|ui| {
        ui.label(label);
        egui::ComboBox::from_id_salt(id).selected_text(e.label()).show_ui(ui, |ui| {
            for c in [WaveletExtension::Periodic, WaveletExtension::ZeroPadded] {
                ui.selectable_value(e, c, c.label());
            }
        });
    });
}

fn percent_row(ui: &mut egui::Ui, label: &str, v: &mut f64) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(egui::Slider::new(v, 0.0..=100.0).integer());
    });
}

pub fn decompose_options(ui: &mut egui::Ui, spec: &mut WaveletDecomposeSpec) {
    wavelet_combo(ui, "wt_decompose_type", "Wavelet Type && Order", &mut spec.wavelet);
    extension_combo(ui, "wt_decompose_ext", "Extension Mode", &mut spec.extension);
    ui.weak(
        "One level. A periodic transform needs an even number of points; zero-padded gives (n + L - 1) / 2 values.",
    );
}

pub fn reconstruct_options(ui: &mut egui::Ui, (wavelet, extension): &mut (Wavelet, WaveletExtension)) {
    wavelet_combo(ui, "wt_reconstruct_type", "Wavelet Type", wavelet);
    extension_combo(ui, "wt_reconstruct_ext", "Boundary", extension);
}

pub fn denoise_options(ui: &mut egui::Ui, spec: &mut WaveletDenoiseSpec) {
    wavelet_combo(ui, "wt_denoise_type", "Wavelet Type", &mut spec.wavelet);
    extension_combo(ui, "wt_denoise_ext", "Extension Mode", &mut spec.extension);
    ui.horizontal(|ui| {
        ui.label("Threshold Type");
        egui::ComboBox::from_id_salt("wt_denoise_thrt")
            .selected_text(match spec.threshold {
                WaveletThresholdKind::Custom => "custom",
                WaveletThresholdKind::SqtwoLog => "sqtwolog",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut spec.threshold, WaveletThresholdKind::Custom, "custom");
                ui.selectable_value(&mut spec.threshold, WaveletThresholdKind::SqtwoLog, "sqtwolog");
            });
    });
    ui.horizontal(|ui| {
        ui.label("Thresholding Level");
        ui.add(DragValue::new(&mut spec.level).range(1..=MAX_LEVEL));
    });
    ui.add_enabled_ui(spec.threshold == WaveletThresholdKind::Custom, |ui| {
        percent_row(ui, "Threshold of every Level (%)", &mut spec.percent);
    });
    ui.weak(
        "Hard threshold of the detail coefficients. A level needs a signal of 2^(level+1) points periodic, 2^level \
         zero-padded. sqtwolog is fitted to Origin's behaviour, whose rule is not known: it ignores the percentage. \
         The first and last samples can differ from Origin's in some cases.",
    );
}

pub fn smooth_options(ui: &mut egui::Ui, spec: &mut WaveletSmoothSpec) {
    wavelet_combo(ui, "wt_smooth_type", "Wavelet Type", &mut spec.wavelet);
    extension_combo(ui, "wt_smooth_ext", "Extension Mode", &mut spec.extension);
    percent_row(ui, "Cutoff (%)", &mut spec.cutoff);
    ui.weak(
        "Decomposes to the deepest level and zeroes the last cutoff % of the detail coefficients (finest level \
         first). Exact against Origin for periodic power-of-two lengths; other lengths and the zero-padded mode \
         can differ.",
    );
}
