//! Page Setup and Print Preview (File menu). Page Setup follows the layout of Origin's window: a
//! picture of a sheet, the Paper group (Size), the Orientation group and the Margins group in
//! inches, with OK and Cancel. Print Preview draws the paper, the margins and the graph page where
//! `PageSetup::place` puts it, the function the printer path uses too.

use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Vec2};
use sheaf_core::page_setup::{Margins, Orientation, PageSetup, Paper, Rect as PtRect};
use sheaf_core::{Document, WindowId};

const PT_PER_IN: f64 = 72.0;

pub enum PageSetupOutcome {
    Open,
    Closed,
    /// OK: the new setup.
    Apply(PageSetup),
}

/// The Page Setup window; it edits a copy until OK.
pub struct PageSetupDialog {
    setup: PageSetup,
}

impl PageSetupDialog {
    pub fn new(setup: PageSetup) -> Self {
        PageSetupDialog { setup }
    }

    pub fn show(&mut self, ctx: &egui::Context) -> PageSetupOutcome {
        let mut open = true;
        let mut outcome = PageSetupOutcome::Open;
        egui::Window::new("Page Setup").open(&mut open).collapsible(false).resizable(false).show(ctx, |ui| {
            // The picture of a sheet with its margins (a wide page decides "Automatic").
            let (rect, _) = ui.allocate_exact_size(Vec2::new(120.0_f32, 90.0_f32), Sense::hover());
            let (pw, ph) = self.setup.paper_pt(2.0, 1.0);
            draw_sheet(ui, rect, (pw, ph), &self.setup.margins, None);
            ui.group(|ui| {
                ui.label("Paper");
                ui.horizontal(|ui| {
                    ui.label("Size:");
                    egui::ComboBox::from_id_salt("page_setup_paper").selected_text(self.setup.paper.label()).show_ui(
                        ui,
                        |ui| {
                            for p in Paper::ALL {
                                ui.selectable_value(&mut self.setup.paper, p, p.label());
                            }
                        },
                    );
                });
            });
            ui.group(|ui| {
                ui.label("Orientation");
                for o in Orientation::ALL {
                    ui.radio_value(&mut self.setup.orientation, o, o.label());
                }
            });
            ui.group(|ui| {
                ui.label("Margins (inches)");
                egui::Grid::new("page_setup_margins").num_columns(4).show(ui, |ui| {
                    let m = &mut self.setup.margins;
                    inch_field(ui, "Left:", &mut m.left);
                    inch_field(ui, "Right:", &mut m.right);
                    ui.end_row();
                    inch_field(ui, "Top:", &mut m.top);
                    inch_field(ui, "Bottom:", &mut m.bottom);
                    ui.end_row();
                });
            });
            ui.horizontal(|ui| {
                if ui.button("OK").clicked() {
                    outcome = PageSetupOutcome::Apply(self.setup);
                }
                if ui.button("Cancel").clicked() {
                    outcome = PageSetupOutcome::Closed;
                }
            });
        });
        if !open {
            outcome = PageSetupOutcome::Closed;
        }
        outcome
    }
}

fn inch_field(ui: &mut egui::Ui, label: &str, pt: &mut f64) {
    ui.label(label);
    let mut inches = *pt / PT_PER_IN;
    if ui.add(egui::DragValue::new(&mut inches).range(0.0..=5.0).speed(0.05).fixed_decimals(2)).changed() {
        *pt = inches * PT_PER_IN;
    }
}

/// Draws a sheet of `paper` points inside `rect` (scaled to fit, centred) with its margin frame
/// dashed and, when given, the page rectangle (points from the sheet's top left) filled with the
/// texture.
fn draw_sheet(
    ui: &egui::Ui,
    rect: Rect,
    paper: (f64, f64),
    margins: &Margins,
    page: Option<(PtRect, &egui::TextureHandle)>,
) {
    let scale = (f64::from(rect.width()) / paper.0).min(f64::from(rect.height()) / paper.1) as f32;
    let size = Vec2::new(paper.0 as f32 * scale, paper.1 as f32 * scale);
    let sheet = Rect::from_center_size(rect.center(), size);
    let p = ui.painter();
    p.rect_filled(sheet.translate(Vec2::splat(3.0)), 0.0, Color32::from_black_alpha(60));
    p.rect_filled(sheet, 0.0, Color32::WHITE);
    p.rect_stroke(sheet, 0.0, Stroke::new(1.0_f32, Color32::GRAY), egui::StrokeKind::Inside);
    let at = |x: f64, y: f64| Pos2::new(sheet.min.x + x as f32 * scale, sheet.min.y + y as f32 * scale);
    let frame =
        Rect::from_min_max(at(margins.left, margins.top), at(paper.0 - margins.right, paper.1 - margins.bottom));
    if frame.width() > 0.0 && frame.height() > 0.0 {
        let dash = Stroke::new(1.0_f32, Color32::from_rgb(90, 90, 160));
        let corners =
            [frame.left_top(), frame.right_top(), frame.right_bottom(), frame.left_bottom(), frame.left_top()];
        p.extend(egui::Shape::dashed_line(&corners, dash, 4.0, 3.0));
    }
    if let Some((r, tex)) = page {
        let dest = Rect::from_min_max(at(r.x, r.y), at(r.x + r.w, r.y + r.h));
        p.image(tex.id(), dest, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
        p.rect_stroke(dest, 0.0, Stroke::new(1.0_f32, Color32::from_gray(190)), egui::StrokeKind::Outside);
    }
}

pub enum PreviewOutcome {
    Open,
    Closed,
    /// The Print button: print this graph now.
    Print,
    /// The Page Setup button.
    Setup,
}

/// The Print Preview window of one graph or workbook.
pub struct PrintPreview {
    /// A graph window, or a workbook window (its active sheet prints as a table).
    pub graph: WindowId,
    texture: Option<egui::TextureHandle>,
    /// The project revision and page the texture was drawn for; the picture is redrawn when one
    /// changes.
    drawn_for: Option<(u64, usize)>,
    /// The sheet page shown, counting from 0 (workbooks only).
    page: usize,
    /// The revision, printed area and page count of the sheet, so a frame does not paginate again.
    sheet_info: Option<(u64, (f64, f64), usize)>,
}

impl PrintPreview {
    pub fn new(graph: WindowId) -> Self {
        PrintPreview { graph, texture: None, drawn_for: None, page: 0, sheet_info: None }
    }

    /// The page being shown, counting from 0 (a workbook's table pages).
    #[cfg(test)]
    pub fn page(&self) -> usize {
        self.page
    }

    pub fn show(&mut self, ctx: &egui::Context, doc: &Document) -> PreviewOutcome {
        let project = doc.project();
        let setup = project.page_setup;
        // What is shown: the graph page placed on the paper, or one page of the sheet's table
        // (the printed area, already inside the margins), and how many pages there are.
        let (page, pages) = if let Some(g) = project.graph(self.graph) {
            ((g.page.width_pt, g.page.height_pt), 1)
        } else if let Some(sheet) = project.workbook(self.graph).map(|b| b.sheet()) {
            // Paginating walks every row, so it is done once per revision, not once per frame.
            let key = doc.revision();
            if self.sheet_info.is_none_or(|(k, _, _)| k != key) {
                let (area, count) = match sheaf_plot::sheet_pages(sheet, &setup) {
                    Ok(p) => (p.area, p.count),
                    Err(_) => (setup.paper_pt(0.0, 1.0), 1),
                };
                self.sheet_info = Some((key, area, count));
            }
            self.sheet_info.map_or(((0.0, 0.0), 1), |(_, a, c)| (a, c))
        } else {
            return PreviewOutcome::Closed;
        };
        let is_sheet = project.graph(self.graph).is_none();
        self.page = self.page.min(pages.saturating_sub(1));
        let stamp = (doc.revision(), self.page);
        if self.drawn_for != Some(stamp) {
            // 150 dpi is plenty for a window; the printer gets the vector page.
            let raster = if is_sheet {
                project
                    .workbook(self.graph)
                    .map(|b| b.sheet())
                    .map(|s| sheaf_plot::render_sheet_raster(s, &setup, self.page, 150.0 / 72.0))
            } else {
                Some(sheaf_plot::render_raster(project, self.graph, 150.0 / 72.0))
            };
            self.texture = raster.and_then(Result::ok).map(|r| {
                let img = egui::ColorImage::from_rgba_unmultiplied([r.width as usize, r.height as usize], &r.rgba);
                ctx.load_texture("print_preview", img, egui::TextureOptions::LINEAR)
            });
            self.drawn_for = Some(stamp);
        }
        let mut open = true;
        let mut outcome = PreviewOutcome::Open;
        let mut turn: isize = 0;
        egui::Window::new("Print Preview").open(&mut open).collapsible(false).default_size([420.0, 520.0]).show(
            ctx,
            |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Print...").clicked() {
                        outcome = PreviewOutcome::Print;
                    }
                    if ui.button("Page Setup...").clicked() {
                        outcome = PreviewOutcome::Setup;
                    }
                    if is_sheet {
                        ui.separator();
                        if ui.add_enabled(self.page > 0, egui::Button::new("< Prev")).clicked() {
                            turn = -1;
                        }
                        ui.label(format!("Page {} of {pages}", self.page + 1));
                        if ui.add_enabled(self.page + 1 < pages, egui::Button::new("Next >")).clicked() {
                            turn = 1;
                        }
                        ui.separator();
                    }
                    if ui.button("Close").clicked() {
                        outcome = PreviewOutcome::Closed;
                    }
                });
                let paper_for = if is_sheet { (0.0, 1.0) } else { page };
                ui.weak(format!(
                    "{}, {}, margins {:.2} / {:.2} / {:.2} / {:.2} in (left, top, right, bottom)",
                    setup.paper.label(),
                    setup.orientation_for(paper_for.0, paper_for.1).label().to_lowercase(),
                    setup.margins.left / PT_PER_IN,
                    setup.margins.top / PT_PER_IN,
                    setup.margins.right / PT_PER_IN,
                    setup.margins.bottom / PT_PER_IN,
                ));
                let avail = ui.available_size().max(Vec2::splat(120.0));
                let (rect, _) = ui.allocate_exact_size(avail, Sense::hover());
                let (paper, placed) = if is_sheet {
                    // The table page is the printed area: exactly the rectangle inside the margins.
                    let m = setup.margins;
                    (setup.paper_pt(0.0, 1.0), Some(PtRect { x: m.left, y: m.top, w: page.0, h: page.1 }))
                } else {
                    (setup.paper_pt(page.0, page.1), setup.place(page.0, page.1))
                };
                let page_tex = placed.zip(self.texture.as_ref());
                draw_sheet(ui, rect.shrink(6.0), paper, &setup.margins, page_tex);
            },
        );
        if turn != 0 {
            self.page = self.page.saturating_add_signed(turn).min(pages.saturating_sub(1));
        }
        if !open {
            outcome = PreviewOutcome::Closed;
        }
        outcome
    }
}
