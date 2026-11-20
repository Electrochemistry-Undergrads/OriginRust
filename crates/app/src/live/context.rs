//! A read-only snapshot of the desktop focus and selections for `sheaf://app`.

use crate::actions::LabelRow;
use crate::graph_view::GraphUi;
use crate::grid::{BookUi, Sel};
use serde_json::{Value, json};
use sheaf_core::project::Window;
use sheaf_core::{Column, Document, WindowId, Worksheet};
use sheaf_plot::scene::ObjectKind;
use std::collections::{HashMap, HashSet};

/// Bounds metadata for a very wide worksheet; the reply still reports the full count.
const MAX_COLUMNS: usize = 1024;

pub struct Context<'a> {
    active: Option<WindowId>,
    sheet: Option<usize>,
    books: &'a HashMap<WindowId, BookUi>,
    graphs: &'a HashMap<WindowId, GraphUi>,
    editing: bool,
    dialog_open: bool,
}

impl<'a> Context<'a> {
    pub fn new(
        active: Option<WindowId>,
        doc: &Document,
        books: &'a HashMap<WindowId, BookUi>,
        graphs: &'a HashMap<WindowId, GraphUi>,
        editing: bool,
        dialog_open: bool,
    ) -> Self {
        let sheet = active.and_then(|id| doc.project().workbook(id)).map(|b| b.active_sheet);
        Self { active, sheet, books, graphs, editing, dialog_open }
    }

    /// Runs when a resource read is serviced, with the document after any earlier queued call.
    /// The UI references are borrowed only; editor text and cell values are never read.
    pub fn snapshot(&self, doc: &Document) -> Value {
        let project = doc.project();
        let mut out = json!({
            "document_revision": doc.revision(), "project_generation": doc.generation(),
            "modified": doc.dirty, "active_window": null, "worksheet": null,
            "active_layer": null, "selection": null, "data_reader": null,
            "editing": self.editing, "dialog_open": self.dialog_open
        });
        let Some(window) = self.active.and_then(|id| project.window(id)) else { return out };
        let kind = match window {
            Window::Workbook(_) => "workbook",
            Window::Graph(_) => "graph",
            Window::Notes(_) => "notes",
            Window::Matrix(_) => "matrix",
        };
        out["active_window"] = json!({"id": window.id(), "name": window.name(), "kind": kind});
        match window {
            Window::Workbook(book) => {
                if let Some(sheet) = book.sheets.get(book.active_sheet) {
                    out["worksheet"] = json!({"name": sheet.name, "index": book.active_sheet + 1,
                        "column_count": sheet.columns.len(), "row_count": sheet.row_count()});
                    // An earlier AI call in this frame may have selected a new output sheet. The
                    // old grid selection belongs to the sheet drawn before that call, not the new one.
                    if self.sheet == Some(book.active_sheet)
                        && let Some(state) = self.books.get(&book.id)
                    {
                        out["selection"] = worksheet_selection(sheet, state);
                    }
                }
            }
            Window::Notes(notes) => {
                out["notes"] = json!({"text": notes.text});
            }
            Window::Matrix(m) => {
                if let Some(s) = m.sheets.get(m.active_sheet) {
                    out["matrix"] = json!({"name": s.name, "index": m.active_sheet + 1,
                        "rows": s.rows, "columns": s.cols});
                }
            }
            Window::Graph(graph) => {
                if let Some(state) = self.graphs.get(&graph.id) {
                    if graph.layers.get(state.active_layer).is_some() {
                        out["active_layer"] = json!(state.active_layer + 1);
                    }
                    if let Some((layer_index, object)) = state.selected
                        && let Some(layer) = graph.layers.get(layer_index)
                    {
                        let selected = match object {
                            ObjectKind::XAxis => Some(json!({"kind": "x_axis"})),
                            ObjectKind::YAxis => Some(json!({"kind": "y_axis"})),
                            ObjectKind::XTitle => Some(json!({"kind": "x_title"})),
                            ObjectKind::YTitle => Some(json!({"kind": "y_title"})),
                            ObjectKind::Legend => Some(json!({"kind": "legend"})),
                            ObjectKind::Plot(i) => layer.plots.get(i).map(|_| json!({"kind": "plot", "index": i + 1})),
                            ObjectKind::Label(i) => {
                                layer.labels.get(i).map(|_| json!({"kind": "label", "index": i + 1}))
                            }
                            ObjectKind::Slope(i) => {
                                layer.slopes.get(i).map(|_| json!({"kind": "slope", "index": i + 1}))
                            }
                            ObjectKind::StraightLine(i) => {
                                layer.straight_lines.get(i).map(|_| json!({"kind": "straight_line", "index": i + 1}))
                            }
                            ObjectKind::Scale(i) => {
                                layer.scales.get(i).map(|_| json!({"kind": "scale", "index": i + 1}))
                            }
                            ObjectKind::Shape(i) => {
                                layer.shapes.get(i).map(|_| json!({"kind": "shape", "index": i + 1}))
                            }
                        };
                        if let Some(mut selected) = selected {
                            selected["layer"] = json!(layer_index + 1);
                            out["selection"] = selected;
                        }
                    }
                    let point = state
                        .reader_point
                        .filter(|p| {
                            graph
                                .layers
                                .get(p.layer)
                                .and_then(|l| l.plots.get(p.plot))
                                .and_then(|plot| project.column(plot.y))
                                .is_some_and(|c| p.row < c.len())
                        })
                        .map(|p| json!({"layer": p.layer + 1, "plot": p.plot + 1, "row": p.row.saturating_add(1)}));
                    out["data_reader"] = json!({"enabled": state.reader, "point": point});
                } else if !graph.layers.is_empty() {
                    out["active_layer"] = json!(1);
                }
            }
        }
        out
    }
}

fn rows(a: usize, b: usize) -> Value {
    json!({"from": a.saturating_add(1), "to": b.saturating_add(1)})
}

fn selection<'a>(kind: &str, columns: impl Iterator<Item = &'a Column>, rows: Value) -> Value {
    let mut names = Vec::new();
    let mut count = 0;
    for column in columns {
        count += 1;
        if names.len() < MAX_COLUMNS {
            names.push(column.short_name.as_str());
        }
    }
    if count == 0 {
        return Value::Null;
    }
    json!({"kind": kind, "columns": names, "column_count": count,
        "columns_truncated": count > MAX_COLUMNS, "rows": rows})
}

fn worksheet_selection(sheet: &Worksheet, state: &BookUi) -> Value {
    match &state.sel {
        Sel::None => Value::Null,
        Sel::Columns => {
            let selected: HashSet<_> = state.selected.iter().copied().collect();
            selection("columns", sheet.columns.iter().filter(|c| selected.contains(&c.id)), Value::Null)
        }
        Sel::Rows { anchor, end } => selection("rows", sheet.columns.iter(), rows(*anchor.min(end), *anchor.max(end))),
        Sel::All => selection("all", sheet.columns.iter(), Value::Null),
        Sel::Cells { anchor, end } => {
            let Some(last) = sheet.columns.len().checked_sub(1) else { return Value::Null };
            let lo = anchor.0.min(end.0).min(last);
            let hi = anchor.0.max(end.0).min(last);
            let mut out =
                selection("cells", sheet.columns[lo..=hi].iter(), rows(anchor.1.min(end.1), anchor.1.max(end.1)));
            out["active_cell"] = state
                .active_cell()
                .and_then(|(c, r)| {
                    sheet.columns.get(c).map(|column| json!({"column": column.short_name, "row": r.saturating_add(1)}))
                })
                .unwrap_or(Value::Null);
            out
        }
        Sel::Label { column, row } => {
            let Some(column) = sheet.column(*column) else { return Value::Null };
            let mut out = selection("label", std::iter::once(column), Value::Null);
            out["label_row"] = match row {
                LabelRow::LongName => json!({"kind": "long_name"}),
                LabelRow::Units => json!({"kind": "units"}),
                LabelRow::Comments => json!({"kind": "comments"}),
                LabelRow::Formula => json!({"kind": "formula"}),
                LabelRow::SamplingInterval => json!({"kind": "sampling_interval"}),
                LabelRow::Sparklines => json!({"kind": "sparklines"}),
                LabelRow::Param(name) => json!({"kind": "parameter", "name": name}),
            };
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sheaf_core::{ColumnData, Designation, Project};

    #[test]
    fn app_context_does_not_reuse_a_selection_after_an_earlier_job_switches_sheets() {
        let mut p = Project::new();
        let b = p.add_workbook("Data");
        p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
        p.workbook_mut(b).unwrap().sheets.push(Worksheet::new("Output"));
        let mut doc = Document::new(p);
        let mut books = HashMap::new();
        let mut ui = BookUi::default();
        ui.select_cells(doc.project().workbook(b).unwrap().sheet(), (0, 0), (0, 1));
        books.insert(b, ui);
        let graphs = HashMap::new();
        let context = Context::new(Some(b), &doc, &books, &graphs, false, false);
        assert_eq!(context.snapshot(&doc)["selection"]["kind"], "cells");
        doc.apply("show output", |p| p.workbook_mut(b).unwrap().active_sheet = 1);
        let before = doc.project().clone();
        let snapshot = context.snapshot(&doc);
        assert_eq!(snapshot["worksheet"]["name"], "Output");
        assert_eq!(snapshot["worksheet"]["index"], 2);
        assert!(snapshot["selection"].is_null());
        assert_eq!(snapshot["document_revision"], doc.revision());
        assert_eq!(doc.project(), &before);
    }

    #[test]
    fn app_context_bounds_wide_column_lists_and_names_parameter_rows_without_values() {
        let mut p = Project::new();
        let b = p.add_workbook("Wide");
        for _ in 0..MAX_COLUMNS + 2 {
            p.add_column(b, 0, Designation::Y, ColumnData::numeric(Vec::new())).unwrap();
        }
        let column = p.workbook(b).unwrap().sheet().columns[0].id;
        let doc = Document::new(p);
        let mut books = HashMap::new();
        let mut ui = BookUi::default();
        ui.sel = Sel::All;
        books.insert(b, ui);
        let graphs = HashMap::new();
        let context = Context::new(Some(b), &doc, &books, &graphs, false, false);
        let snapshot = context.snapshot(&doc);
        assert_eq!(snapshot["selection"]["columns"].as_array().unwrap().len(), MAX_COLUMNS);
        assert_eq!(snapshot["selection"]["column_count"], MAX_COLUMNS + 2);
        assert_eq!(snapshot["selection"]["columns_truncated"], true);
        let ui = books.get_mut(&b).unwrap();
        ui.sel = Sel::Label { column, row: LabelRow::Param("dose \u{03bc}g".into()) };
        ui.editing_label = Some((column, LabelRow::Param("dose \u{03bc}g".into()), "uncommitted-private-label".into()));
        let context = Context::new(Some(b), &doc, &books, &graphs, true, false);
        let snapshot = context.snapshot(&doc);
        assert_eq!(snapshot["selection"]["label_row"], json!({"kind": "parameter", "name": "dose \u{03bc}g"}));
        assert!(!snapshot.to_string().contains("uncommitted-private-label"));
        assert!(!doc.dirty && doc.undo_label().is_none());
    }
}
