//! Layout > Add Text... and a double-click on layout text (T237 slice 8): the text (several lines)
//! and its size. OK adds it centred at the top of the page, or changes the clicked item (its box
//! takes the new text's size, its top left corner stays), as one undo step.

use eframe::egui;
use sheaf_core::layout_page::LayoutContent;
use sheaf_core::{Document, WindowId};

pub enum Outcome {
    Open,
    Closed,
    Done,
}

pub struct LayoutText {
    pub layout: WindowId,
    /// The item being edited; `None` adds a new one.
    pub index: Option<usize>,
    pub text: String,
    pub size: u16,
}

/// Adds `text` to `layout`, or puts it in item `index`, as one undo step.
pub fn set_text(
    doc: &mut Document,
    layout: WindowId,
    index: Option<usize>,
    text: &str,
    size: u16,
) -> Result<(), String> {
    if text.trim().is_empty() {
        return Err("type some text".into());
    }
    let size = size.clamp(4, 200);
    let (_, box_pt) = sheaf_plot::layout_text_svg(text, size);
    let label = if index.is_some() { "Edit layout text" } else { "Add text to layout" };
    doc.edit(label, |p| {
        let l = p.layout_mut(layout).ok_or("the layout page is gone")?;
        let (pw, ph) = (l.width_pt, l.height_pt);
        match index {
            None => {
                l.add_text(text.to_string(), size, box_pt);
            }
            Some(k) => {
                let it = l.items.get_mut(k).ok_or("the text is gone")?;
                it.content = LayoutContent::Text { text: text.to_string(), size };
                (it.w, it.h) = ((box_pt.0 / pw).min(1.0), (box_pt.1 / ph).min(1.0));
            }
        }
        Ok::<_, String>(())
    })
}

impl LayoutText {
    /// A new text for `layout`.
    pub fn new(layout: WindowId) -> Self {
        LayoutText { layout, index: None, text: String::new(), size: 18 }
    }

    /// Editing item `index` of `layout`; `None` when it is not text.
    pub fn edit(doc: &Document, layout: WindowId, index: usize) -> Option<Self> {
        match &doc.project().layout(layout)?.items.get(index)?.content {
            LayoutContent::Text { text, size } => {
                Some(LayoutText { layout, index: Some(index), text: text.clone(), size: *size })
            }
            _ => None,
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> Outcome {
        if doc.project().layout(self.layout).is_none() || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            return Outcome::Closed;
        }
        let mut open = true;
        let (mut ok, mut cancel) = (false, false);
        egui::Window::new(if self.index.is_some() { "Edit Text" } else { "Add Text" })
            .id(egui::Id::new("layout_text"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.add(egui::TextEdit::multiline(&mut self.text).desired_rows(3).desired_width(280.0));
                ui.horizontal(|ui| {
                    ui.label("Size");
                    ui.add(egui::DragValue::new(&mut self.size).range(4..=200));
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ok = ui.button("OK").clicked();
                    cancel = ui.button("Cancel").clicked();
                });
            });
        if !open || cancel {
            return Outcome::Closed;
        }
        if ok {
            return match set_text(doc, self.layout, self.index, &self.text, self.size) {
                Ok(()) => Outcome::Done,
                Err(_) => Outcome::Open,
            };
        }
        Outcome::Open
    }
}
