fn main() -> eframe::Result {
    eframe::run_native("Sheaf", eframe::NativeOptions::default(), Box::new(|_cc| Ok(Box::new(Probe))))
}

struct Probe;

impl eframe::App for Probe {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("probe");
        });
    }
}
