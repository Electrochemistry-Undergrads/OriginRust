//! Options of Signal Processing > IIR Filter and > FIR Filter, for the shared analysis window in
//! `analysis.rs`. The controls follow Origin's documented dialogs (Response Type, Method, Filter
//! Order with Minimum, Frequency Specification with Unit and Sample Frequency, Magnitude
//! Specification, Forward-Backward Filtering; for the FIR app also Window and Scale Passband);
//! the preview panels of Origin's dialogs are not built. Spec: `docs/spec/analysis-tools.md` 17.

use eframe::egui::{self, DragValue};
use sheaf_core::analysis::{
    FilterResponse, FirMethod, FirSpec, FirWindow, FreqUnit, IirMethod, IirSpec, MagnitudeUnit, Tool,
};

/// The two filters' options; the window uses the one of its kind.
#[derive(Debug, Clone, Copy, Default)]
pub struct FilterOptions {
    pub iir: IirSpec,
    pub fir: FirSpec,
}

impl FilterOptions {
    pub fn iir_tool(&self) -> Tool {
        Tool::IirFilter(self.iir)
    }

    pub fn fir_tool(&self) -> Tool {
        Tool::FirFilter(self.fir)
    }
}

const RESPONSES: [(&str, FilterResponse); 4] = [
    ("Low Pass", FilterResponse::LowPass),
    ("High Pass", FilterResponse::HighPass),
    ("Band Pass", FilterResponse::BandPass),
    ("Band Stop", FilterResponse::BandStop),
];

/// The Nyquist frequency in the unit of the dialog, the base of the start values of the frequencies: from the
/// Fs given, else the one Auto gives for the input, else 1000 Hz.
fn nyquist_start(unit: FreqUnit, fs: Option<f64>, auto_fs: Option<f64>) -> f64 {
    if unit == FreqUnit::Hz { fs.or(auto_fs).unwrap_or(1000.0) / 2.0 } else { 1.0 }
}

/// A frequency that is Sheaf's default until "Auto" is unticked.
fn auto_value(ui: &mut egui::Ui, label: &str, value: &mut Option<f64>, start: f64) {
    ui.horizontal(|ui| {
        ui.label(label);
        let mut auto = value.is_none();
        if ui.checkbox(&mut auto, "Auto").changed() {
            *value = if auto { None } else { Some(start) };
        }
        if let Some(v) = value {
            ui.add(DragValue::new(v).speed(0.01).range(0.0..=f64::MAX));
        }
    });
}

fn response_combo(ui: &mut egui::Ui, id: &str, response: &mut FilterResponse) {
    ui.horizontal(|ui| {
        ui.label("Response Type");
        egui::ComboBox::from_id_salt(id)
            .selected_text(RESPONSES.iter().find(|r| r.1 == *response).map_or("", |r| r.0))
            .show_ui(ui, |ui| {
                for (label, r) in RESPONSES {
                    ui.selectable_value(response, r, label);
                }
            });
    });
}

/// Order: Minimum or an integer.
fn order_row(ui: &mut egui::Ui, order: &mut Option<usize>, minimum_allowed: bool, max: usize) {
    ui.horizontal(|ui| {
        ui.label("Filter Order");
        let mut minimum = order.is_none();
        if ui.add_enabled(minimum_allowed, egui::Checkbox::new(&mut minimum, "Minimum")).changed() {
            *order = if minimum { None } else { Some(10) };
        }
        if let Some(n) = order {
            ui.add(DragValue::new(n).range(1..=max));
        }
    });
}

/// Frequency Specification: Unit, Sample Frequency and the pass, stop and cutoff frequencies.
/// `use_edges`: pass and stop frequencies (minimum order, least squares) instead of cutoffs.
/// `auto_fs` is the sample frequency Auto gives (1 / the X step of the input), so that the start values of the
/// frequencies stay below the Nyquist frequency of the data; `cutoff_note` says what the cutoff is.
#[allow(clippy::too_many_arguments)]
fn frequency_section(
    ui: &mut egui::Ui,
    response: FilterResponse,
    auto_fs: Option<f64>,
    cutoff_note: Option<&str>,
    unit: &mut FreqUnit,
    fs: &mut Option<f64>,
    cutoffs: [&mut Option<f64>; 2],
    pass: [&mut Option<f64>; 2],
    stop: [&mut Option<f64>; 2],
    use_edges: bool,
) {
    ui.separator();
    ui.label("Frequency Specification");
    let before = *unit;
    ui.horizontal(|ui| {
        ui.label("Unit");
        egui::ComboBox::from_id_salt(format!("filter_unit_{}", if use_edges { "e" } else { "c" }))
            .selected_text(if *unit == FreqUnit::Hz { "Hz" } else { "Normalized (0 to 1)" })
            .show_ui(ui, |ui| {
                ui.selectable_value(unit, FreqUnit::Hz, "Hz");
                ui.selectable_value(unit, FreqUnit::Normalized, "Normalized (0 to 1)");
            });
    });
    let [c1, c2] = cutoffs;
    let [p1, p2] = pass;
    let [s1, s2] = stop;
    if *unit != before {
        // the values were given in the other unit
        for v in [&mut *c1, &mut *c2, &mut *p1, &mut *p2, &mut *s1, &mut *s2] {
            *v = None;
        }
    }
    if *unit == FreqUnit::Hz {
        auto_value(ui, "Sample Frequency (Fs)", fs, auto_fs.unwrap_or(1000.0));
    }
    let band = response.is_band();
    let start = nyquist_start(*unit, *fs, auto_fs);
    if use_edges {
        auto_value(ui, if band { "Pass Frequency 1" } else { "Pass Frequency" }, p1, start * 0.4);
        if band {
            auto_value(ui, "Pass Frequency 2", p2, start * 0.6);
        }
        auto_value(ui, if band { "Stop Frequency 1" } else { "Stop Frequency" }, s1, start * 0.5);
        if band {
            auto_value(ui, "Stop Frequency 2", s2, start * 0.7);
        }
    } else {
        auto_value(ui, if band { "Cutoff Frequency 1 (Fc1)" } else { "Cutoff Frequency (Fc)" }, c1, start * 0.25);
        if band {
            auto_value(ui, "Cutoff Frequency 2 (Fc2)", c2, start * 0.75);
        }
    }
    ui.weak("Auto: the sample frequency is 1 / the X step; cutoffs at a quarter (and three quarters) of the Nyquist frequency.");
    if let Some(note) = cutoff_note.filter(|_| !use_edges) {
        ui.weak(note);
    }
}

/// What the cutoff frequency means for each IIR method (scipy's convention; unmeasured in Origin).
fn iir_cutoff_note(method: IirMethod) -> &'static str {
    match method {
        IirMethod::Butterworth => "Fc is the frequency where the gain is down by 3 dB.",
        IirMethod::Chebyshev1 | IirMethod::Elliptic => {
            "Fc is the passband edge: the gain there is down by the passband ripple."
        }
        IirMethod::Chebyshev2 => "Fc is the stopband edge: the gain there is down by the stopband attenuation.",
    }
}

/// The options of Signal Processing > IIR Filter.
pub fn iir_options(ui: &mut egui::Ui, spec: &mut IirSpec, auto_fs: Option<f64>) {
    response_combo(ui, "iir_response", &mut spec.response);
    const METHODS: [(&str, IirMethod); 4] = [
        ("Butterworth", IirMethod::Butterworth),
        ("Chebyshev Type I", IirMethod::Chebyshev1),
        ("Chebyshev Type II", IirMethod::Chebyshev2),
        ("Elliptic", IirMethod::Elliptic),
    ];
    ui.horizontal(|ui| {
        ui.label("Method");
        egui::ComboBox::from_id_salt("iir_method")
            .selected_text(METHODS.iter().find(|m| m.1 == spec.method).map_or("", |m| m.0))
            .show_ui(ui, |ui| {
                for (label, m) in METHODS {
                    ui.selectable_value(&mut spec.method, m, label);
                }
            });
    });
    order_row(ui, &mut spec.order, true, sheaf_core::analysis::IIR_MAX_ORDER);
    let minimum = spec.order.is_none();
    let note = iir_cutoff_note(spec.method);
    let IirSpec { response, unit, sample_frequency, cutoff, cutoff2, pass, pass2, stop, stop2, .. } = spec;
    frequency_section(
        ui,
        *response,
        auto_fs,
        Some(note),
        unit,
        sample_frequency,
        [cutoff, cutoff2],
        [pass, pass2],
        [stop, stop2],
        minimum,
    );
    ui.separator();
    ui.label("Magnitude Specification (dB)");
    let ripple = minimum || matches!(spec.method, IirMethod::Chebyshev1 | IirMethod::Elliptic);
    let attenuation = minimum || matches!(spec.method, IirMethod::Chebyshev2 | IirMethod::Elliptic);
    ui.horizontal(|ui| {
        ui.label("Passband Ripple");
        ui.add_enabled(ripple, DragValue::new(&mut spec.passband_ripple).speed(0.1).range(0.001..=100.0).suffix(" dB"));
    });
    ui.horizontal(|ui| {
        ui.label("Stopband Attenuation");
        ui.add_enabled(
            attenuation,
            DragValue::new(&mut spec.stopband_attenuation).speed(0.5).range(0.01..=300.0).suffix(" dB"),
        );
    });
    ui.separator();
    ui.checkbox(&mut spec.forward_backward, "Forward-Backward Filtering");
}

/// The options of Signal Processing > FIR Filter.
pub fn fir_options(ui: &mut egui::Ui, spec: &mut FirSpec, auto_fs: Option<f64>) {
    response_combo(ui, "fir_response", &mut spec.response);
    ui.horizontal(|ui| {
        ui.label("Method");
        egui::ComboBox::from_id_salt("fir_method")
            .selected_text(if spec.method == FirMethod::Window { "Window" } else { "Least Squares" })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut spec.method, FirMethod::LeastSquares, "Least Squares");
                ui.selectable_value(&mut spec.method, FirMethod::Window, "Window");
            });
    });
    if spec.method == FirMethod::LeastSquares && spec.order.is_none() {
        spec.order = Some(20);
    }
    order_row(ui, &mut spec.order, spec.method == FirMethod::Window, sheaf_core::analysis::FIR_MAX_ORDER);
    let even_needed = matches!(spec.response, FilterResponse::HighPass | FilterResponse::BandStop);
    if even_needed && spec.order.is_some_and(|n| !n.is_multiple_of(2)) {
        ui.colored_label(ui.visuals().warn_fg_color, "High pass and band stop filters need an even order.");
    }
    if spec.method == FirMethod::Window {
        ui.horizontal(|ui| {
            ui.label("Window");
            egui::ComboBox::from_id_salt("fir_window").selected_text(spec.window.label()).show_ui(ui, |ui| {
                for w in FirWindow::ALL {
                    ui.selectable_value(&mut spec.window, w, w.label());
                }
            });
        });
        if let Some(label) = spec.window.param_label() {
            ui.horizontal(|ui| {
                ui.label(label);
                let mut v = spec.window_param.unwrap_or_else(|| spec.window.default_param());
                if ui.add(DragValue::new(&mut v).speed(0.1).range(0.0..=300.0)).changed() {
                    spec.window_param = Some(v);
                }
            });
        }
    }
    ui.checkbox(&mut spec.scale_passband, "Scale Passband");
    let edges = spec.method == FirMethod::LeastSquares || spec.order.is_none();
    let FirSpec { response, unit, sample_frequency, cutoff, cutoff2, pass, pass2, stop, stop2, .. } = spec;
    frequency_section(
        ui,
        *response,
        auto_fs,
        None,
        unit,
        sample_frequency,
        [cutoff, cutoff2],
        [pass, pass2],
        [stop, stop2],
        edges,
    );
    ui.separator();
    ui.label("Magnitude Specification");
    if spec.method == FirMethod::LeastSquares {
        ui.horizontal(|ui| {
            ui.label("Passband Weight");
            ui.add(DragValue::new(&mut spec.pass_weight).speed(0.1).range(0.001..=1e6));
        });
        ui.horizontal(|ui| {
            ui.label("Stopband Weight");
            ui.add(DragValue::new(&mut spec.stop_weight).speed(0.1).range(0.001..=1e6));
        });
    } else if spec.order.is_none() {
        let before = spec.magnitude_unit;
        ui.horizontal(|ui| {
            ui.label("Unit");
            egui::ComboBox::from_id_salt("fir_magnitude_unit")
                .selected_text(if spec.magnitude_unit == MagnitudeUnit::Db { "dB" } else { "Linear" })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut spec.magnitude_unit, MagnitudeUnit::Db, "dB");
                    ui.selectable_value(&mut spec.magnitude_unit, MagnitudeUnit::Linear, "Linear");
                });
        });
        if spec.magnitude_unit != before {
            let (rp, rs) = (spec.passband_ripple, spec.stopband_attenuation);
            (spec.passband_ripple, spec.stopband_attenuation) = if spec.magnitude_unit == MagnitudeUnit::Linear {
                let g = 10f64.powf(rp / 20.0);
                ((g - 1.0) / (g + 1.0), 10f64.powf(-rs / 20.0))
            } else {
                (20.0 * ((1.0 + rp) / (1.0 - rp)).log10(), -20.0 * rs.log10())
            };
        }
        let db = spec.magnitude_unit == MagnitudeUnit::Db;
        ui.horizontal(|ui| {
            ui.label("Passband Ripple");
            let d = if db {
                DragValue::new(&mut spec.passband_ripple).speed(0.1).range(0.001..=100.0).suffix(" dB")
            } else {
                DragValue::new(&mut spec.passband_ripple).speed(0.001).range(1e-6..=0.99)
            };
            ui.add(d);
        });
        ui.horizontal(|ui| {
            ui.label("Stopband Attenuation");
            let d = if db {
                DragValue::new(&mut spec.stopband_attenuation).speed(0.5).range(0.01..=300.0).suffix(" dB")
            } else {
                DragValue::new(&mut spec.stopband_attenuation).speed(0.0001).range(1e-9..=0.99)
            };
            ui.add(d);
        });
    } else {
        ui.weak("Weights and ripple apply to the Least Squares method and the minimum order.");
    }
    ui.separator();
    ui.checkbox(&mut spec.forward_backward, "Forward-Backward Filtering");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_start_values_stay_below_the_nyquist_frequency_of_the_input() {
        // X steps by 0.01 s: Fs 100 Hz, Nyquist 50 Hz (the start was 500 Hz whatever the data)
        assert_eq!(nyquist_start(FreqUnit::Hz, None, Some(100.0)), 50.0);
        assert!(nyquist_start(FreqUnit::Hz, None, Some(100.0)) * 0.75 < 50.0);
        // a given Fs wins over the Auto one; without either the old 1000 Hz stays
        assert_eq!(nyquist_start(FreqUnit::Hz, Some(200.0), Some(100.0)), 100.0);
        assert_eq!(nyquist_start(FreqUnit::Hz, None, None), 500.0);
        assert_eq!(nyquist_start(FreqUnit::Normalized, None, Some(100.0)), 1.0);
    }
}
