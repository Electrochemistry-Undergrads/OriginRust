//! Finding the workbook, sheet, graph and columns a request names.

use anyhow::{Result, anyhow, bail};
use sheaf_core::project::Window;
use sheaf_core::{ColumnId, Project, WindowId};

/// Finds the Y columns named on the command line in one sheet.
pub fn named_columns(project: &Project, book: WindowId, sheet: usize, names: &[String]) -> Result<Vec<ColumnId>> {
    let ws = project
        .workbook(book)
        .and_then(|b| b.sheets.get(sheet))
        .ok_or_else(|| anyhow!("the workbook has no such sheet"))?;
    names
        .iter()
        .map(|n| {
            ws.column_by_name(n.trim()).map(|c| c.id).ok_or_else(|| {
                let have: Vec<&str> = ws.columns.iter().map(|c| c.short_name.as_str()).collect();
                anyhow!("column '{}' was not found in sheet '{}' (columns: {})", n, ws.name, have.join(", "))
            })
        })
        .collect()
}

pub fn pick_book(project: &Project, name: Option<&str>) -> Result<WindowId> {
    match name {
        Some(n) => match project.window_by_name(n) {
            Some(Window::Workbook(b)) => Ok(b.id),
            Some(_) => bail!("'{n}' is not a workbook"),
            None => bail!("there is no workbook named '{n}'"),
        },
        None => project.workbooks().next().map(|b| b.id).ok_or_else(|| anyhow!("the project has no workbook")),
    }
}

pub fn pick_sheet(project: &Project, book: WindowId, name: Option<&str>) -> Result<usize> {
    let b = project.workbook(book).ok_or_else(|| anyhow!("workbook not found"))?;
    match name {
        Some(n) => b
            .sheets
            .iter()
            .position(|s| s.name.eq_ignore_ascii_case(n))
            .ok_or_else(|| anyhow!("workbook '{}' has no sheet named '{}'", b.name, n)),
        None => Ok(b.active_sheet.min(b.sheets.len().saturating_sub(1))),
    }
}

pub fn pick_graph(project: &Project, name: Option<&str>) -> Result<WindowId> {
    match name {
        Some(n) => match project.window_by_name(n) {
            Some(Window::Graph(g)) => Ok(g.id),
            Some(_) => bail!("'{n}' is not a graph"),
            None => bail!("there is no graph named '{n}'"),
        },
        None => {
            let mut graphs = project.graphs();
            match (graphs.next(), graphs.next()) {
                (Some(g), None) => Ok(g.id),
                (None, _) => bail!("the project has no graph"),
                _ => {
                    let names: Vec<&str> = project.graphs().map(|g| g.name.as_str()).collect();
                    bail!("the project has several graphs; choose one with --graph ({})", names.join(", "))
                }
            }
        }
    }
}

/// Fails with a clear message unless `value` (when given) is a finite number above zero.
pub fn require_positive(option: &str, value: Option<f64>) -> Result<()> {
    match value {
        Some(v) if !(v.is_finite() && v > 0.0) => {
            bail!("{option} must be a number greater than 0 (got {v})")
        }
        _ => Ok(()),
    }
}
