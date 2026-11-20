//! What a project holds, as data.

use serde::Serialize;
use sheaf_core::Project;
use sheaf_core::project::Window;

/// One column of a worksheet.
#[derive(Clone, Debug, Serialize)]
pub struct ColumnSummary {
    pub short_name: String,
    /// The designation tag (X, Y, yEr, ...).
    pub designation: String,
    pub rows: usize,
    /// `text`, `date` or empty for numbers.
    pub kind: String,
    /// Long name, then a date column's first and last value, units and comments, as the
    /// command line shows them.
    pub label: String,
    pub long_name: String,
    pub units: String,
    pub comments: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct SheetSummary {
    pub name: String,
    pub rows: usize,
    pub columns: Vec<ColumnSummary>,
}

#[derive(Clone, Debug, Serialize)]
pub struct WorkbookSummary {
    pub name: String,
    pub title: String,
    pub sheets: Vec<SheetSummary>,
}

#[derive(Clone, Debug, Serialize)]
pub struct GraphSummary {
    pub name: String,
    pub title: String,
    pub layers: usize,
    pub plots: usize,
    pub width_in: f64,
    pub height_in: f64,
}

/// One sheet of a matrix book.
#[derive(Clone, Debug, Serialize)]
pub struct MatrixSheetSummary {
    pub name: String,
    pub rows: usize,
    pub cols: usize,
    pub x_from: f64,
    pub x_to: f64,
    pub y_from: f64,
    pub y_to: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct MatrixSummary {
    pub name: String,
    pub title: String,
    pub sheets: Vec<MatrixSheetSummary>,
}

/// The windows of a project in creation order.
#[derive(Clone, Debug, Serialize)]
pub struct ProjectSummary {
    pub workbooks: Vec<WorkbookSummary>,
    pub graphs: Vec<GraphSummary>,
    /// Matrix books (T192), in creation order; they are not in `order`.
    pub matrices: Vec<MatrixSummary>,
    /// The windows in their order: `("workbook", index)` or `("graph", index)`.
    #[serde(skip)]
    pub order: Vec<(bool, usize)>,
}

/// Lists the workbooks, sheets, columns and graphs of a project.
pub fn summarize(project: &Project) -> ProjectSummary {
    let mut out = ProjectSummary { workbooks: Vec::new(), graphs: Vec::new(), matrices: Vec::new(), order: Vec::new() };
    for w in &project.windows {
        match w {
            Window::Workbook(b) => {
                let sheets = b
                    .sheets
                    .iter()
                    .map(|s| SheetSummary {
                        name: s.name.clone(),
                        rows: s.row_count(),
                        columns: s
                            .columns
                            .iter()
                            .map(|c| {
                                let kind = if !c.data.is_numeric() {
                                    "text"
                                } else if c.format.is_date_time() {
                                    "date"
                                } else {
                                    ""
                                };
                                let mut label = c.long_name.clone();
                                // A date column shows its first and last value as they appear in the worksheet.
                                if c.format.is_date_time()
                                    && let Some(v) = c.numeric()
                                    && let (Some(first), Some(last)) =
                                        (v.iter().find(|x| x.is_finite()), v.iter().rev().find(|x| x.is_finite()))
                                {
                                    let sep = if label.is_empty() { "" } else { " " };
                                    label.push_str(&format!(
                                        "{sep}<{} to {}>",
                                        c.format.format(*first),
                                        c.format.format(*last)
                                    ));
                                }
                                if !c.units.is_empty() {
                                    label.push_str(&format!(" ({})", c.units));
                                }
                                if !c.comments.is_empty() {
                                    label.push_str(&format!(" [{}]", c.comments.replace('\n', " / ")));
                                }
                                ColumnSummary {
                                    short_name: c.short_name.clone(),
                                    designation: c.designation.tag().to_string(),
                                    rows: c.len(),
                                    kind: kind.to_string(),
                                    label,
                                    long_name: c.long_name.clone(),
                                    units: c.units.clone(),
                                    comments: c.comments.clone(),
                                }
                            })
                            .collect(),
                    })
                    .collect();
                out.order.push((true, out.workbooks.len()));
                out.workbooks.push(WorkbookSummary { name: b.name.clone(), title: b.title().to_string(), sheets });
            }
            // Notes windows (T154) hold no data.
            Window::Notes(_) => {}
            // Layout pages (T237) hold no data of their own.
            Window::Layout(_) => {}
            Window::Matrix(m) => out.matrices.push(MatrixSummary {
                name: m.name.clone(),
                title: m.title().to_string(),
                sheets: m
                    .sheets
                    .iter()
                    .map(|s| MatrixSheetSummary {
                        name: s.name.clone(),
                        rows: s.rows,
                        cols: s.cols,
                        x_from: s.x_from,
                        x_to: s.x_to,
                        y_from: s.y_from,
                        y_to: s.y_to,
                    })
                    .collect(),
            }),
            Window::Graph(g) => {
                out.order.push((false, out.graphs.len()));
                out.graphs.push(GraphSummary {
                    name: g.name.clone(),
                    title: g.title().to_string(),
                    layers: g.layers.len(),
                    plots: g.layers.iter().map(|l| l.plots.len()).sum(),
                    width_in: g.page.width_pt / 72.0,
                    height_in: g.page.height_pt / 72.0,
                });
            }
        }
    }
    out
}
