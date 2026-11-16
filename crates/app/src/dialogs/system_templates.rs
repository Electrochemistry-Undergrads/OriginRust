//! Plot > System Templates... (T104): Origin 2016's "System Templates" window, measured 2026-11-16: a
//! Category tree of the built-in graph templates (Line, Symbol, Line & Symbol, Columns & Bars,
//! Multi-Curve, ...), a Preview Window, Description, Location and Date Modified, and Plot Setup..., Plot
//! (greyed until a template is picked) and Cancel. Sheaf lists its own built-in templates in Origin's
//! categories; the preview is the template drawn with sample data. Rules in `docs/spec/graphing.md` 24.

use eframe::egui::{self, RichText};
use sheaf_core::template::Template;
use sheaf_core::{ColumnData, ColumnId, Designation, Project, WindowId};

/// Origin's category of a built-in template (Sheaf's assignment for templates Origin files elsewhere).
pub fn category(t: Template) -> &'static str {
    use Template::*;
    match t {
        Line | CyclicVoltammetry | VerticalDropLine => "Line",
        Scatter | ScatterRug | ScatterCentral | ColumnScatter | Bubble | ColorMapped | BubbleColorMapped => "Symbol",
        LineSymbol | Segment2 | Segment3 => "Line & Symbol",
        Column | StackedColumn | Stacked100Column | Bar | FloatingColumn | StackedBar | Stacked100Bar | FloatingBar
        | ColumnLabel | GroupedColumns | Lollipop | Pie | Pie3D | PieBw => "Columns & Bars",
        DoubleY | Multi3YYy | Multi3YYY | Multi4YYyy | Multi4YyYy | StackedLines | Waterfall | WaterfallY
        | WaterfallZ => "Multi-Curve",
        Histogram | StackedHistograms | BoxChart | GroupedBox | IntervalPlot | Pareto | Violin | ViolinBox
        | ViolinPoint | ViolinQuartile | ViolinStick | HalfViolin | Ridgeline | ScatterMatrix => "Statistics",
        Area | StackedArea | Stacked100Area | FillArea => "Area",
        _ => "Specialized",
    }
}

/// The categories in Origin's order (those Sheaf has templates for).
pub const CATEGORIES: [&str; 8] =
    ["Line", "Symbol", "Line & Symbol", "Columns & Bars", "Multi-Curve", "Statistics", "Area", "Specialized"];

pub enum SystemTemplatesOutcome {
    Open,
    Closed,
    Plot(Template),
}

pub struct SystemTemplatesDialog {
    pub book: WindowId,
    pub columns: Vec<ColumnId>,
    pub selected: Option<Template>,
    preview: Option<(Template, Option<egui::TextureHandle>)>,
}

/// A graph of `template` with sample data (four Y columns of 1 to 32), drawn small.
fn preview_raster(template: Template) -> Option<sheaf_plot::Raster> {
    let mut p = Project::new();
    let b = p.add_workbook("Sample");
    let x: Vec<f64> = (1..=32).map(f64::from).collect();
    p.add_column(b, 0, Designation::X, ColumnData::numeric(x.clone())).ok()?;
    let ys: Vec<ColumnId> = (1..=4)
        .filter_map(|k| {
            let v = x.iter().map(|v| v * k as f64 + (v * 0.7).sin() * 3.0).collect();
            p.add_column(b, 0, Designation::Y, ColumnData::numeric(v)).ok()
        })
        .collect();
    let g = p.plot_columns(template, &ys).ok()?;
    sheaf_plot::render_raster(&p, g, 0.5).ok()
}

impl SystemTemplatesDialog {
    pub fn new(book: WindowId, columns: Vec<ColumnId>) -> Self {
        SystemTemplatesDialog { book, columns, selected: None, preview: None }
    }

    pub fn show(&mut self, ctx: &egui::Context) -> SystemTemplatesOutcome {
        let mut outcome = SystemTemplatesOutcome::Open;
        let mut open = true;
        if let Some(t) = self.selected
            && self.preview.as_ref().is_none_or(|(p, _)| *p != t)
        {
            let tex = preview_raster(t).map(|r| {
                let img = egui::ColorImage::from_rgba_unmultiplied([r.width as usize, r.height as usize], &r.rgba);
                ctx.load_texture("system_template_preview", img, egui::TextureOptions::LINEAR)
            });
            self.preview = Some((t, tex));
        }
        egui::Window::new("System Templates")
            .id(egui::Id::new("system_templates"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Category").strong());
                        egui::ScrollArea::vertical().max_height(300.0).min_scrolled_width(200.0).show(ui, |ui| {
                            ui.label("Graph Template");
                            for cat in CATEGORIES {
                                egui::CollapsingHeader::new(cat).default_open(false).show(ui, |ui| {
                                    for t in Template::ALL.into_iter().filter(|t| category(*t) == cat) {
                                        if ui.selectable_label(self.selected == Some(t), t.label()).clicked() {
                                            self.selected = Some(t);
                                        }
                                    }
                                });
                            }
                        });
                    });
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Preview Window").strong());
                        let size = egui::vec2(320.0, 240.0);
                        match &self.preview {
                            Some((_, Some(tex))) => {
                                ui.add(egui::Image::new(tex).fit_to_exact_size(size));
                            }
                            Some((_, None)) => {
                                ui.allocate_ui(size, |ui| ui.weak("This template needs data of its own kind to show."));
                            }
                            None => {
                                ui.allocate_space(size);
                            }
                        }
                    });
                });
                ui.label(format!(
                    "Description: {}",
                    self.selected.map_or(String::new(), |t| format!("{} ({})", t.label(), category(t)))
                ));
                ui.label("Location: built into Sheaf");
                ui.horizontal(|ui| {
                    let can = self.selected.is_some() && !self.columns.is_empty();
                    let plot = ui.add_enabled(can, egui::Button::new("Plot"));
                    let plot = if self.columns.is_empty() {
                        plot.on_disabled_hover_text("Select columns first")
                    } else {
                        plot
                    };
                    if plot.clicked()
                        && let Some(t) = self.selected
                    {
                        outcome = SystemTemplatesOutcome::Plot(t);
                    }
                    if ui.button("Cancel").clicked() {
                        outcome = SystemTemplatesOutcome::Closed;
                    }
                });
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    outcome = SystemTemplatesOutcome::Closed;
                }
            });
        if !open { SystemTemplatesOutcome::Closed } else { outcome }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_template_has_a_listed_category() {
        for t in Template::ALL {
            assert!(CATEGORIES.contains(&category(t)), "{t:?}");
        }
        assert_eq!(category(Template::Line), "Line");
        assert_eq!(category(Template::Histogram), "Statistics");
        assert!(preview_raster(Template::Line).is_some(), "a line preview draws");
    }
}
