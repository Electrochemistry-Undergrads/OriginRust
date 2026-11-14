//! Layer Contents and Plot Setup, the two dialogs behind F12 and the layer icon.
//!
//! * **Layer Contents**: the columns of the project that can be added (a sheet at a time) at the left,
//!   the plots of the layer at the right, buttons to move a selection across (add, remove), to
//!   reorder, and to Group (the plots follow the layer's increment rules) or Ungroup (they keep
//!   their style).
//! * **Plot Setup**: one row per plot of the layer with the X, Y and error columns it draws and its
//!   plot type, each a drop-down.
//!
//! Both edit the graph; the buttons OK, Cancel and Apply belong to the window that holds them.

use std::collections::BTreeSet;

use eframe::egui;
use sheaf_core::ColumnId;
use sheaf_core::graph::*;

use super::graph_details::{Info, KINDS, kind_name};
use super::graph_props::{Addable, Source, section};

/// A plot of the layer drawing this column, in the layer's default look.
pub fn new_plot(layer: &Layer, a: &Addable) -> Plot {
    Plot {
        x: a.x,
        y: a.y,
        y_error: a.y_error,
        rows: None,
        kind: layer.default_kind,
        line: layer.default_line.clone(),
        symbol: layer.default_symbol.clone(),
        label: None,
        visible: true,
        auto_color: true,
        group: None,
        x_error: None,
        y_base: None,
        error: ErrorStyle::default(),
        hist: Default::default(),
        size_from: None,
        color_from: None,
        details: Default::default(),
    }
}

/// What the two lists have selected, and the sheet chosen at the left.
#[derive(Default)]
pub struct ContentsUi {
    pub add_from: usize,
    available: BTreeSet<usize>,
    plots: BTreeSet<usize>,
}

/// Where the user wants to go from Layer Contents or Plot Setup.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Goto {
    Contents,
    Setup,
    Details,
}

fn toggle(set: &mut BTreeSet<usize>, i: usize, multi: bool) {
    if multi {
        if !set.remove(&i) {
            set.insert(i);
        }
    } else {
        *set = BTreeSet::from([i]);
    }
}

pub fn contents_body(
    ui: &mut egui::Ui,
    graph: &mut Graph,
    l: usize,
    info: &Info,
    sources: &[Source],
    st: &mut ContentsUi,
) -> Option<Goto> {
    let mut go = None;
    let layer = &mut graph.layers[l];
    st.plots.retain(|&i| i < layer.plots.len());
    let multi = ui.input(|i| i.modifiers.command || i.modifiers.shift);
    let texts: &[String] = info.texts.get(l).map(Vec::as_slice).unwrap_or(&[]);
    let mut add: Vec<usize> = Vec::new();
    let mut remove = false;
    let mut move_by = 0_i32;
    let mut group: Option<bool> = None;

    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.set_width(300.0);
            section(ui, "Available Data");
            if sources.is_empty() {
                ui.weak("The project has no numeric Y columns.");
                return;
            }
            st.add_from = st.add_from.min(sources.len() - 1);
            let before = st.add_from;
            egui::ComboBox::from_id_salt("contents_source")
                .selected_text(&sources[st.add_from].title)
                .width(290.0)
                .show_ui(ui, |ui| {
                    for (i, s) in sources.iter().enumerate() {
                        ui.selectable_value(&mut st.add_from, i, &s.title);
                    }
                });
            if st.add_from != before {
                st.available.clear();
            }
            egui::ScrollArea::vertical()
                .id_salt("contents_available")
                .max_height(300.0)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for (i, a) in sources[st.add_from].columns.iter().enumerate() {
                        let shown = st.available.contains(&i);
                        let r = ui.selectable_label(shown, &a.label);
                        if r.clicked() {
                            toggle(&mut st.available, i, multi);
                        }
                        if r.double_clicked() {
                            add = vec![i];
                        }
                    }
                });
            ui.weak("Ctrl or Shift: select several");
        });
        ui.vertical(|ui| {
            ui.add_space(80.0);
            if ui
                .add_enabled(!st.available.is_empty(), egui::Button::new("=>").min_size(egui::vec2(36.0, 0.0)))
                .on_hover_text("Add to the layer")
                .clicked()
            {
                add = st.available.iter().copied().collect();
            }
            if ui
                .add_enabled(!st.plots.is_empty(), egui::Button::new("<=").min_size(egui::vec2(36.0, 0.0)))
                .on_hover_text("Remove from the layer")
                .clicked()
            {
                remove = true;
            }
            ui.add_space(8.0);
            let one = st.plots.len() == 1;
            let first = st.plots.iter().next().copied().unwrap_or(0);
            if ui.add_enabled(one && first > 0, egui::Button::new("Up")).clicked() {
                move_by = -1;
            }
            if ui.add_enabled(one && first + 1 < layer.plots.len(), egui::Button::new("Down")).clicked() {
                move_by = 1;
            }
        });
        ui.vertical(|ui| {
            section(ui, "Layer Contents");
            egui::ScrollArea::both().id_salt("contents_plots").max_height(300.0).auto_shrink([false, false]).show(
                ui,
                |ui| {
                    for (i, plot) in layer.plots.iter().enumerate() {
                        ui.horizontal(|ui| {
                            crate::grid::color_swatch(ui, plot.line.color);
                            let text = texts.get(i).cloned().unwrap_or_else(|| format!("Plot {}", i + 1));
                            let text = if plot.auto_color { text } else { format!("{text}  (ungrouped)") };
                            if ui.selectable_label(st.plots.contains(&i), text).clicked() {
                                toggle(&mut st.plots, i, multi);
                            }
                        });
                    }
                    if layer.plots.is_empty() {
                        ui.weak("The layer has no plots.");
                    }
                },
            );
            ui.horizontal(|ui| {
                let some = !st.plots.is_empty();
                if ui
                    .add_enabled(some, egui::Button::new("Group"))
                    .on_hover_text("The plots follow the layer's increment rules")
                    .clicked()
                {
                    group = Some(true);
                }
                if ui
                    .add_enabled(some, egui::Button::new("Ungroup"))
                    .on_hover_text("The plots keep the style they have")
                    .clicked()
                {
                    group = Some(false);
                }
            });
        });
    });
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        if ui.button("Plot Setup...").clicked() {
            go = Some(Goto::Setup);
        }
        if ui.button("Plot Details...").clicked() {
            go = Some(Goto::Details);
        }
    });

    if !add.is_empty() {
        for &i in &add {
            if let Some(a) = sources[st.add_from].columns.get(i) {
                let plot = new_plot(layer, a);
                layer.plots.push(plot);
            }
        }
        layer.recolor_automatic();
        st.available.clear();
    }
    if remove {
        for &i in st.plots.iter().rev() {
            layer.plots.remove(i);
        }
        st.plots.clear();
        layer.recolor_automatic();
    }
    if move_by != 0
        && let Some(&i) = st.plots.iter().next()
    {
        let j = (i as i32 + move_by) as usize;
        layer.plots.swap(i, j);
        st.plots = BTreeSet::from([j]);
        layer.recolor_automatic();
    }
    if let Some(on) = group {
        for &i in &st.plots {
            layer.plots[i].auto_color = on;
        }
        if on {
            layer.recolor_automatic();
        }
    }
    go
}

fn column_name(columns: &[(ColumnId, String)], id: ColumnId) -> String {
    columns.iter().find(|(c, _)| *c == id).map(|(_, n)| n.clone()).unwrap_or_else(|| "(missing)".into())
}

/// A drop-down over the columns of the project; `none` is the text of "no column", if one is allowed.
fn column_combo(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash,
    value: &mut Option<ColumnId>,
    columns: &[(ColumnId, String)],
    none: Option<&str>,
) {
    let shown = match (*value, none) {
        (Some(c), _) => column_name(columns, c),
        (None, Some(n)) => n.to_string(),
        (None, None) => String::new(),
    };
    egui::ComboBox::from_id_salt(id).selected_text(shown).width(170.0).show_ui(ui, |ui| {
        if let Some(n) = none {
            ui.selectable_value(value, None, n);
        }
        for (c, n) in columns {
            ui.selectable_value(value, Some(*c), n);
        }
    });
}

pub fn setup_body(ui: &mut egui::Ui, graph: &mut Graph, l: usize, info: &Info) -> Option<Goto> {
    let mut go = None;
    let layer = &mut graph.layers[l];
    ui.label("The columns each plot of the layer draws. Rows of data and looks are in Plot Details.");
    ui.add_space(6.0);
    egui::ScrollArea::both()
        .id_salt("setup_table")
        .max_height((ui.available_height() - 40.0).max(120.0))
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("setup_grid").num_columns(5).striped(true).spacing([10.0, 6.0]).show(ui, |ui| {
                for h in ["Plot", "X", "Y", "Y Error", "Type"] {
                    ui.label(egui::RichText::new(h).strong());
                }
                ui.end_row();
                for (i, plot) in layer.plots.iter_mut().enumerate() {
                    ui.label(format!("{}", i + 1));
                    column_combo(ui, ("setup_x", i), &mut plot.x, info.columns, Some("(row number)"));
                    let mut y = Some(plot.y);
                    column_combo(ui, ("setup_y", i), &mut y, info.columns, None);
                    if let Some(y) = y {
                        plot.y = y;
                    }
                    column_combo(ui, ("setup_e", i), &mut plot.y_error, info.columns, Some("(none)"));
                    egui::ComboBox::from_id_salt(("setup_kind", i)).selected_text(kind_name(plot.kind)).show_ui(
                        ui,
                        |ui| {
                            for k in KINDS {
                                ui.selectable_value(&mut plot.kind, k, kind_name(k));
                            }
                        },
                    );
                    ui.end_row();
                }
            });
            if layer.plots.is_empty() {
                ui.weak("The layer has no plots. Add data in Layer Contents.");
            }
        });
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        if ui.button("Layer Contents...").clicked() {
            go = Some(Goto::Contents);
        }
        if ui.button("Plot Details...").clicked() {
            go = Some(Goto::Details);
        }
    });
    go
}
