//! The workbook operation tree (T443; Origin 2027's operations in Object Manager, Sheaf's own design): the analyses
//! of a workbook that follow their input (Recalculate Auto or Manual, [`crate::analysis_link`]) grouped by the sheet
//! that holds their results, each with its input and output columns, its mode and whether it waits for Recalculate.
//! From the tree an operation's input or output is opened, it is recalculated, switched between Auto and Manual, or
//! made to stop following its input (its results stay as plain data).
//!
//! Sheaf's choices (Origin 2027 not measured here): an operation belongs to the book and sheet of its first result
//! column; its parameters are shown as text, not edited in place; switching a waiting operation to Auto runs it.

use crate::analysis_link::LinkTool;
use crate::column::{FormulaState, RecalcMode};
use crate::ids::{ColumnId, WindowId};
use crate::project::{Project, ProjectError};

/// One operation of the tree.
#[derive(Clone, Debug, PartialEq)]
pub struct Operation {
    /// Its index in [`Project::analysis_links`].
    pub index: usize,
    pub name: &'static str,
    pub mode: RecalcMode,
    /// A result waits for Recalculate.
    pub pending: bool,
    pub inputs: Vec<ColumnId>,
    pub outputs: Vec<ColumnId>,
    /// The sheet (index in the book) holding the results.
    pub sheet: usize,
    /// The tool's settings, as text.
    pub parameters: String,
}

/// The menu name of a linked tool.
pub fn tool_name(tool: &LinkTool) -> &'static str {
    match tool {
        LinkTool::Smooth(_) => "Smooth",
        LinkTool::Integrate(_) => "Integrate",
        LinkTool::FindPeaks(_) => "Find Peaks",
        LinkTool::AverageCurves(_) => "Average Multiple Curves",
        LinkTool::Intersect(_) => "Intersect",
        LinkTool::PolygonArea => "Polygon Area",
        LinkTool::SubtractLine(_) => "Subtract Straight Line",
        LinkTool::ReduceX(_) => "Reduce by X",
        LinkTool::ReduceDup(_) => "Reduce Duplicate X",
        LinkTool::Translate(_) => "Translate",
        LinkTool::MedianFilter(_) => "Median Filter",
        LinkTool::Fsd(_) => "Fourier Self-Deconvolution",
        LinkTool::SimulateCurve(_) => "Simulate Curve",
        LinkTool::IirFilter(_) => "IIR Filter",
        LinkTool::FirFilter(_) => "FIR Filter",
    }
}

impl Project {
    /// The operations whose results are in workbook `book`, by sheet and then in the order they were made.
    pub fn book_operations(&self, book: WindowId) -> Vec<Operation> {
        let mut out: Vec<Operation> = self
            .analysis_links
            .iter()
            .enumerate()
            .filter_map(|(index, link)| {
                let (b, sheet, _) = self.locate(*link.outputs.first()?)?;
                if b.id != book {
                    return None;
                }
                let sheet = b.sheets.iter().position(|s| std::ptr::eq(s, sheet))?;
                let pending = link.outputs.iter().any(|id| {
                    self.column(*id).and_then(|c| c.analysis).is_some_and(|m| m.state != FormulaState::Current)
                });
                Some(Operation {
                    index,
                    name: tool_name(&link.tool),
                    mode: link.mode,
                    pending,
                    inputs: link.columns.clone(),
                    outputs: link.outputs.clone(),
                    sheet,
                    parameters: format!("{:?}", link.tool),
                })
            })
            .collect();
        out.sort_by_key(|o| (o.sheet, o.index));
        out
    }

    /// Gives operation `index` new settings for its tool (T469: the operation tree's Parameters...) and runs it
    /// again; its results are replaced in place. The tool must be the same kind (a Smooth stays a Smooth). On an error
    /// nothing changes.
    pub fn set_operation_tool(&mut self, index: usize, tool: LinkTool) -> Result<(), ProjectError> {
        let link = self.analysis_links.get(index).ok_or(ProjectError::NoSuchOperation)?;
        if std::mem::discriminant(&link.tool) != std::mem::discriminant(&tool) {
            return Err(ProjectError::Analysis("the settings are for another tool".into()));
        }
        let before = self.analysis_links[index].tool.clone();
        self.analysis_links[index].tool = tool;
        if let Err(e) = self.recalculate_analysis(index) {
            self.analysis_links[index].tool = before;
            return Err(ProjectError::Analysis(e.to_string()));
        }
        Ok(())
    }

    /// Sets operation `index` to Auto or Manual, or with [`RecalcMode::None`] stops it following its input (its
    /// results stay as data). A waiting operation set to Auto runs at once.
    pub fn set_analysis_mode(&mut self, index: usize, mode: RecalcMode) -> Result<(), ProjectError> {
        let link = self.analysis_links.get_mut(index).ok_or(ProjectError::NoSuchOperation)?;
        if mode == RecalcMode::None {
            self.remove_analysis_link(index);
            return Ok(());
        }
        link.mode = mode;
        let outputs = link.outputs.clone();
        let mut pending = false;
        for id in outputs {
            if let Some(m) = self.column_mut(id).and_then(|c| c.analysis.as_mut()) {
                m.mode = mode;
                pending |= m.state != FormulaState::Current;
            }
        }
        if mode == RecalcMode::Auto && pending {
            self.recalculate_analysis(index).map_err(|e| ProjectError::Analysis(e.to_string()))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::{OutputTarget, Request, RowRange, Tool};
    use crate::column::{ColumnData, Designation};
    use sheaf_calc::{Boundary, SmoothMethod, SmoothOptions};

    #[test]
    fn a_books_operations_list_by_result_sheet_and_change_mode() {
        let mut p = Project::new();
        let b = p.add_workbook("");
        p.add_column(b, 0, Designation::X, ColumnData::numeric((0..10).map(f64::from).collect())).unwrap();
        let y = p
            .add_column(b, 0, Designation::Y, ColumnData::numeric((0..10).map(|i| f64::from(i * i)).collect()))
            .unwrap();
        let o = SmoothOptions {
            method: SmoothMethod::AdjacentAveraging { weighted: false },
            npts: 3,
            boundary: Boundary::None,
        };
        let req = Request { columns: vec![y], rows: RowRange::All, tool: Tool::Smooth(o) };
        let report = p.analyze_linked(&req, &OutputTarget::NextToInput, RecalcMode::Manual).unwrap();
        let ops = p.book_operations(b);
        assert_eq!(ops.len(), 1);
        assert_eq!((ops[0].name, ops[0].mode, ops[0].pending, ops[0].sheet), ("Smooth", RecalcMode::Manual, false, 0));
        assert_eq!((ops[0].inputs.clone(), ops[0].outputs.clone()), (vec![y], report.new_columns.clone()));
        // The input changes: the Manual operation waits; set to Auto it runs.
        let before = p.clone();
        p.column_mut(y).unwrap().data = ColumnData::numeric(vec![1.0; 10]);
        p.update_analysis_links(&before);
        assert!(p.book_operations(b)[0].pending, "waits for Recalculate");
        p.set_analysis_mode(0, RecalcMode::Auto).unwrap();
        let op = &p.book_operations(b)[0];
        assert_eq!((op.mode, op.pending), (RecalcMode::Auto, false));
        // New settings (T469): a 5-point smooth runs again in place.
        let outputs = p.analysis_links[0].outputs.clone();
        let LinkTool::Smooth(mut o) = p.analysis_links[0].tool.clone() else { panic!("a smooth") };
        o.npts = 5;
        p.set_operation_tool(0, LinkTool::Smooth(o)).unwrap();
        assert_eq!(p.analysis_links[0].outputs, outputs, "the same result columns");
        assert!(matches!(p.analysis_links[0].tool, LinkTool::Smooth(SmoothOptions { npts: 5, .. })));
        assert!(p.set_operation_tool(0, LinkTool::PolygonArea).is_err(), "another tool");
        // Stop following: the results stay as data.
        p.set_analysis_mode(0, RecalcMode::None).unwrap();
        assert!(p.book_operations(b).is_empty());
        assert!(p.column(report.new_columns[0]).unwrap().analysis.is_none());
        assert!(p.set_analysis_mode(0, RecalcMode::Auto).is_err());
    }
}
