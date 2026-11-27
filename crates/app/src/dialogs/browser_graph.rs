//! Graph > Browser Graph... (T410): make a plot of the active layer step through the Y columns of its sheet or
//! the same column of every sheet (`Project::set_graph_browser`); the bar above the graph then steps it.
//! Remove Browser leaves the plot on the dataset it shows. Rules in `sheaf_core::browser`.

use eframe::egui;
use sheaf_core::browser::BrowseBy;
use sheaf_core::{Project, WindowId};

pub enum BrowserOutcome {
    Open,
    Closed,
    Apply { graph: WindowId, layer: usize, plot: usize, by: BrowseBy, all: bool },
    Remove { graph: WindowId },
}

pub struct BrowserGraphDialog {
    pub graph: WindowId,
    pub layer: usize,
    /// The plots of the layer: their names.
    pub plots: Vec<String>,
    pub plot: usize,
    pub by: BrowseBy,
    /// The graph has a browser already (Remove Browser is offered).
    pub has_browser: bool,
    /// Every plot of the layer steps with it (T489, by sheet or book).
    pub all: bool,
}

impl BrowserGraphDialog {
    /// The window for layer `layer` of `graph`, on the graph's browser when it has one. `None` when the layer
    /// has no plot.
    pub fn new(project: &Project, graph: WindowId, layer: usize) -> Option<Self> {
        let g = project.graph(graph)?;
        let current = g.browser.as_ref();
        let layer = current.map_or(layer, |b| b.layer);
        let l = g.layers.get(layer)?;
        if l.plots.is_empty() {
            return None;
        }
        let plots = l
            .plots
            .iter()
            .map(|p| project.column(p.y).map_or_else(String::new, |c| c.display_name().to_string()))
            .collect();
        Some(BrowserGraphDialog {
            graph,
            layer,
            plots,
            plot: current.map_or(0, |b| b.plot),
            by: current.map_or(BrowseBy::Columns, |b| b.by),
            has_browser: current.is_some(),
            all: current.is_some_and(|b| !b.others.is_empty()),
        })
    }

    pub fn show(&mut self, ctx: &egui::Context) -> BrowserOutcome {
        let mut open = true;
        let mut outcome = BrowserOutcome::Open;
        egui::Window::new("Browser Graph")
            .id(egui::Id::new("browser_graph"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Plot");
                    let shown = self.plots.get(self.plot).cloned().unwrap_or_default();
                    egui::ComboBox::from_id_salt("browser_graph_plot").selected_text(shown).show_ui(ui, |ui| {
                        for (k, name) in self.plots.iter().enumerate() {
                            ui.selectable_value(&mut self.plot, k, name);
                        }
                    });
                });
                ui.label("Browse");
                ui.radio_value(&mut self.by, BrowseBy::Columns, "The Y columns of the sheet");
                ui.radio_value(&mut self.by, BrowseBy::Sheets, "The same column in every sheet of the book");
                ui.radio_value(&mut self.by, BrowseBy::Books, "The same column of the same sheet in every book");
                // T489: every plot of the layer steps with it to its own column of the next sheet or book
                let several = self.plots.len() > 1 && self.by != BrowseBy::Columns;
                ui.add_enabled(several, egui::Checkbox::new(&mut self.all, "Browse all plots of the layer"));
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        outcome = BrowserOutcome::Apply {
                            graph: self.graph,
                            layer: self.layer,
                            plot: self.plot,
                            by: self.by,
                            all: self.all && self.by != BrowseBy::Columns,
                        };
                    }
                    if ui.add_enabled(self.has_browser, egui::Button::new("Remove Browser")).clicked() {
                        outcome = BrowserOutcome::Remove { graph: self.graph };
                    }
                    if ui.button("Cancel").clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        outcome = BrowserOutcome::Closed;
                    }
                });
            });
        if !open { BrowserOutcome::Closed } else { outcome }
    }
}
