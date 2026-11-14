//! Analysis results that follow their input (the Recalculate choice of the analysis windows).
//!
//! A run made with Recalculate Auto or Manual is remembered as an [`AnalysisLink`]: the input columns,
//! the rows, the tool with its options, where the results went and which columns hold them. When a
//! column the run read changes, an Auto link is run again at once (inside the same undo step as the
//! edit) and a Manual link is marked pending until [`Project::recalculate_analysis`] is called. The
//! results are replaced in place, so graphs and formulas that refer to the result columns keep working.
//! Smooth, Integrate, Find Peaks, Average Multiple Curves, Intersect, Polygon Area and the data
//! manipulation tools (T23) can be linked; the other tools run once.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use sheaf_calc::{PeakOptions, ReduceOptions, SmoothOptions};

use crate::analysis::{
    AnalysisError, AverageSpec, IntegrateSpec, IntersectSpec, MedianFilterSpec, OutputTarget, ReduceDupSpec, Report,
    Request, RowRange, SubtractLineSpec, Tool, TranslateSpec,
};
use crate::column::{AnalysisMark, FormulaState, RecalcMode};
use crate::{ColumnId, Project};

/// The tools whose runs can be repeated, in a form that is saved with the project.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LinkTool {
    Smooth(SmoothOptions),
    Integrate(IntegrateSpec),
    FindPeaks(PeakOptions),
    AverageCurves(AverageSpec),
    Intersect(IntersectSpec),
    PolygonArea,
    SubtractLine(SubtractLineSpec),
    ReduceX(ReduceOptions),
    ReduceDup(ReduceDupSpec),
    Translate(TranslateSpec),
    MedianFilter(MedianFilterSpec),
}

impl LinkTool {
    pub fn from_tool(tool: &Tool) -> Option<Self> {
        match tool {
            Tool::Smooth(o) => Some(LinkTool::Smooth(*o)),
            Tool::Integrate(s) => Some(LinkTool::Integrate(*s)),
            Tool::FindPeaks(o) => Some(LinkTool::FindPeaks(*o)),
            Tool::AverageCurves(s) => Some(LinkTool::AverageCurves(*s)),
            Tool::Intersect(s) => Some(LinkTool::Intersect(*s)),
            Tool::PolygonArea => Some(LinkTool::PolygonArea),
            Tool::SubtractLine(s) => Some(LinkTool::SubtractLine(*s)),
            Tool::ReduceX(o) => Some(LinkTool::ReduceX(*o)),
            Tool::ReduceDup(s) => Some(LinkTool::ReduceDup(*s)),
            Tool::Translate(s) => Some(LinkTool::Translate(*s)),
            Tool::MedianFilter(s) => Some(LinkTool::MedianFilter(*s)),
            _ => None,
        }
    }

    pub fn tool(&self) -> Tool {
        match self {
            LinkTool::Smooth(o) => Tool::Smooth(*o),
            LinkTool::Integrate(s) => Tool::Integrate(*s),
            LinkTool::FindPeaks(o) => Tool::FindPeaks(*o),
            LinkTool::AverageCurves(s) => Tool::AverageCurves(*s),
            LinkTool::Intersect(s) => Tool::Intersect(*s),
            LinkTool::PolygonArea => Tool::PolygonArea,
            LinkTool::SubtractLine(s) => Tool::SubtractLine(*s),
            LinkTool::ReduceX(o) => Tool::ReduceX(*o),
            LinkTool::ReduceDup(s) => Tool::ReduceDup(*s),
            LinkTool::Translate(s) => Tool::Translate(*s),
            LinkTool::MedianFilter(s) => Tool::MedianFilter(*s),
        }
    }
}

/// One analysis run that is repeated when its input changes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalysisLink {
    /// Auto or Manual.
    pub mode: RecalcMode,
    pub columns: Vec<ColumnId>,
    pub rows: RowRange,
    pub tool: LinkTool,
    pub output: OutputTarget,
    /// Every column the run wrote, in the order of [`Report::new_columns`].
    pub outputs: Vec<ColumnId>,
}

impl AnalysisLink {
    pub fn request(&self) -> Request {
        Request { columns: self.columns.clone(), rows: self.rows, tool: self.tool.tool() }
    }
}

impl Project {
    /// True if the tool can be run with Recalculate Auto or Manual.
    pub fn can_link(tool: &Tool) -> bool {
        LinkTool::from_tool(tool).is_some()
    }

    /// Runs a tool like [`Project::analyze_to`]; with `mode` Auto or Manual (and a tool that can be
    /// linked) the run is remembered so that it follows its input.
    pub fn analyze_linked(
        &mut self,
        request: &Request,
        output: &OutputTarget,
        mode: RecalcMode,
    ) -> Result<Report, AnalysisError> {
        self.analyze_linked_into(None, request, output, mode).map(|(report, _)| report)
    }

    /// The run of an analysis window: like [`Project::analyze_linked`], but when `previous` names the
    /// columns of the window's earlier run (Apply, Auto Apply) they are replaced instead of adding
    /// new ones, and the link of that run is updated or dropped. Returns the report and the columns
    /// that now hold the results.
    pub fn analyze_linked_into(
        &mut self,
        previous: Option<&[ColumnId]>,
        request: &Request,
        output: &OutputTarget,
        mode: RecalcMode,
    ) -> Result<(Report, Vec<ColumnId>), AnalysisError> {
        let (report, outputs) = match previous {
            Some(prev) => self.analyze_into(prev, request, output)?,
            None => {
                let report = self.analyze_to(request, output)?;
                let ids = report.new_columns.clone();
                (report, ids)
            }
        };
        let existing = previous.and_then(|p| p.first()).and_then(|c| self.analysis_link_of(*c));
        match (mode != RecalcMode::None).then(|| LinkTool::from_tool(&request.tool)).flatten() {
            Some(tool) => {
                let link = AnalysisLink {
                    mode,
                    columns: request.columns.clone(),
                    rows: request.rows,
                    tool,
                    output: output.clone(),
                    outputs: outputs.clone(),
                };
                let at = match existing {
                    Some(i) => {
                        self.analysis_links[i] = link;
                        i
                    }
                    None => {
                        self.analysis_links.push(link);
                        self.analysis_links.len() - 1
                    }
                };
                self.mark_link(at, FormulaState::Current);
            }
            None => {
                if let Some(i) = existing {
                    self.remove_analysis_link(i);
                }
            }
        }
        Ok((report, outputs))
    }

    /// The link whose results include `column`.
    pub fn analysis_link_of(&self, column: ColumnId) -> Option<usize> {
        self.analysis_links.iter().position(|l| l.outputs.contains(&column))
    }

    /// Runs the tool again and replaces the columns in `outputs` with the new results, so that
    /// everything that refers to them keeps working. When the old columns are gone or the number of
    /// result columns changed, the new columns stay and the returned list names them.
    pub fn analyze_into(
        &mut self,
        outputs: &[ColumnId],
        request: &Request,
        output: &OutputTarget,
    ) -> Result<(Report, Vec<ColumnId>), AnalysisError> {
        let mut report = self.analyze_to(request, output)?;
        let new_ids = report.new_columns.clone();
        let usable = outputs.len() == new_ids.len() && outputs.iter().all(|c| self.column(*c).is_some());
        if !usable {
            self.retire_outputs(outputs, &new_ids, request, output, &mut report);
            return Ok((report, new_ids));
        }
        // Columns that the run wrote in place (an existing-columns target) need no copying.
        let transplant: Vec<(ColumnId, ColumnId)> =
            outputs.iter().copied().zip(new_ids.iter().copied()).filter(|(old, new)| old != new).collect();
        for (old, new) in &transplant {
            let (data, long_name, units, comments, designation) = {
                let c = self.column(*new).expect("the run just wrote it");
                (c.data.clone(), c.long_name.clone(), c.units.clone(), c.comments.clone(), c.designation)
            };
            let c = self.column_mut(*old).expect("checked above");
            c.data = data;
            c.long_name = long_name;
            c.units = units;
            c.comments = comments;
            c.designation = designation;
        }
        let gone: HashSet<ColumnId> = transplant.iter().map(|(_, new)| *new).collect();
        let book = report.book;
        let added_sheet = report.new_sheet;
        if let Some(b) = self.workbook_mut(book) {
            for s in &mut b.sheets {
                s.columns.retain(|c| !gone.contains(&c.id));
            }
            // The sheet the run added is empty now when its columns were all copied into old ones.
            if let Some(i) = added_sheet.filter(|&i| i < b.sheets.len() && b.sheets[i].columns.is_empty()) {
                b.sheets.remove(i);
            }
        }
        report.new_columns = outputs.to_vec();
        report.new_sheet = added_sheet.and_then(|_| {
            let last = *outputs.last()?;
            self.workbook(book)?.sheets.iter().position(|s| s.column(last).is_some())
        });
        Ok((report, outputs.to_vec()))
    }

    /// The number of result columns changed, so the new columns stay and the old linked results
    /// would be left behind with a mark that no link names any more. Their columns (and a result
    /// sheet that becomes empty) are removed; columns of an `Existing` target are the user's own, so
    /// only their mark is cleared. Old columns without a mark were never linked and are left alone.
    fn retire_outputs(
        &mut self,
        old: &[ColumnId],
        new: &[ColumnId],
        request: &Request,
        output: &OutputTarget,
        report: &mut Report,
    ) {
        let stale: Vec<ColumnId> = old
            .iter()
            .copied()
            .filter(|id| !new.contains(id) && self.column(*id).is_some_and(|c| c.analysis.is_some()))
            .collect();
        if stale.is_empty() {
            return;
        }
        let remove = !matches!(output, OutputTarget::Existing(_));
        for id in &stale {
            if let Some(c) = self.column_mut(*id) {
                c.analysis = None;
            }
        }
        if !remove {
            return;
        }
        let book = report.book;
        let Some(b) = self.workbook_mut(book) else { return };
        let mut keep: Vec<bool> = Vec::new();
        for (i, sheet) in b.sheets.iter_mut().enumerate() {
            let had = sheet.columns.len();
            sheet.columns.retain(|c| !stale.contains(&c.id));
            // A sheet that held only old results is gone; the indices in the report move with it.
            keep.push(sheet.columns.len() == had || !sheet.columns.is_empty() || i == report.sheet);
        }
        let mut k = keep.iter();
        b.sheets.retain(|_| *k.next().unwrap_or(&true));
        let position_of = |b: &crate::Workbook, id: ColumnId| b.sheets.iter().position(|s| s.column(id).is_some());
        if let Some(i) = request.columns.first().and_then(|c| position_of(b, *c)) {
            report.sheet = i;
        }
        if report.new_sheet.is_some() {
            report.new_sheet = new.last().and_then(|c| position_of(b, *c));
        }
    }

    /// Runs the link `index` again now (Manual links wait for this) and marks its columns current.
    pub fn recalculate_analysis(&mut self, index: usize) -> Result<(), AnalysisError> {
        let link = self.analysis_links.get(index).cloned().ok_or(AnalysisError::NoSuchSheet)?;
        match self.analyze_into(&link.outputs, &link.request(), &link.output) {
            Ok((_, outputs)) => {
                self.analysis_links[index].outputs = outputs;
                self.mark_link(index, FormulaState::Current);
                Ok(())
            }
            Err(e) => {
                self.mark_link(index, FormulaState::Broken);
                Err(e)
            }
        }
    }

    /// Runs every link of the workbook that is waiting. Returns what could not run.
    pub fn recalculate_pending_analyses(&mut self, book: crate::WindowId) -> Vec<String> {
        let pending: Vec<usize> = (0..self.analysis_links.len())
            .filter(|&i| {
                self.analysis_links[i].outputs.first().and_then(|c| self.locate(*c)).is_some_and(|(b, _, c)| {
                    b.id == book && c.analysis.is_some_and(|m| m.state != FormulaState::Current)
                })
            })
            .collect();
        let mut problems = Vec::new();
        for i in pending {
            if let Err(e) = self.recalculate_analysis(i) {
                problems.push(e.to_string());
            }
        }
        problems
    }

    /// Forgets the link `index` (its columns become plain data).
    pub fn remove_analysis_link(&mut self, index: usize) {
        if index < self.analysis_links.len() {
            let link = self.analysis_links.remove(index);
            for id in link.outputs {
                if let Some(c) = self.column_mut(id) {
                    c.analysis = None;
                }
            }
        }
    }

    /// Brings the links up to date after an edit that turned `before` into `self`: Auto links whose
    /// input columns changed run again, Manual links are marked pending. Used by
    /// [`crate::Document`] inside the undo step of the edit. Returns what could not run.
    pub fn update_analysis_links(&mut self, before: &Project) -> Vec<String> {
        if self.analysis_links.is_empty() {
            return Vec::new();
        }
        let mut changed: HashSet<ColumnId> = HashSet::new();
        for b in self.workbooks() {
            for s in &b.sheets {
                for c in &s.columns {
                    if before.column(c.id).is_some_and(|o| o.data != c.data) {
                        changed.insert(c.id);
                    }
                }
            }
        }
        let mut problems = Vec::new();
        for i in 0..self.analysis_links.len() {
            let sources = self.link_sources(i);
            if !sources.iter().any(|c| changed.contains(c)) {
                continue;
            }
            match self.analysis_links[i].mode {
                RecalcMode::Auto => match self.recalculate_analysis(i) {
                    Ok(()) => changed.extend(self.analysis_links[i].outputs.iter().copied()),
                    Err(e) => problems.push(e.to_string()),
                },
                _ => self.mark_link(i, FormulaState::Pending),
            }
        }
        problems
    }

    /// The input columns of a link and the X columns they read.
    fn link_sources(&self, index: usize) -> Vec<ColumnId> {
        let mut out = Vec::new();
        for id in &self.analysis_links[index].columns {
            out.push(*id);
            if let Some((_, sheet, _)) = self.locate(*id)
                && let Some(at) = sheet.column_index(*id)
                && let Some(x) = sheet.x_for(at)
            {
                out.push(x.id);
            }
        }
        out
    }

    fn mark_link(&mut self, index: usize, state: FormulaState) {
        let mode = self.analysis_links[index].mode;
        let outputs = self.analysis_links[index].outputs.clone();
        for id in outputs {
            if let Some(c) = self.column_mut(id) {
                c.analysis = Some(AnalysisMark { mode, state });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ColumnData, Designation};
    use sheaf_calc::{Boundary, SmoothMethod};

    fn setup() -> (Project, ColumnId, ColumnId, crate::WindowId) {
        let mut p = Project::new();
        let book = p.add_workbook("Book1");
        let x =
            p.add_column(book, 0, Designation::X, ColumnData::numeric((0..20).map(|i| i as f64).collect())).unwrap();
        let y = p
            .add_column(book, 0, Designation::Y, ColumnData::numeric((0..20).map(|i| (i % 4) as f64).collect()))
            .unwrap();
        (p, x, y, book)
    }

    fn smooth_request(y: ColumnId) -> Request {
        let o = SmoothOptions {
            method: SmoothMethod::AdjacentAveraging { weighted: false },
            npts: 3,
            boundary: Boundary::None,
        };
        Request { columns: vec![y], rows: RowRange::All, tool: Tool::Smooth(o) }
    }

    #[test]
    fn an_auto_link_follows_its_input_and_keeps_the_result_column() {
        let (mut p, _x, y, book) = setup();
        let report = p.analyze_linked(&smooth_request(y), &OutputTarget::NextToInput, RecalcMode::Auto).unwrap();
        let out = report.new_columns[0];
        let before = p.clone();
        p.column_mut(y).unwrap().data = ColumnData::numeric(vec![9.0; 20]);
        let problems = p.update_analysis_links(&before);
        assert!(problems.is_empty(), "{problems:?}");
        assert_eq!(p.numeric(out).unwrap(), vec![9.0; 20].as_slice(), "same column, new values");
        assert_eq!(p.workbook(book).unwrap().sheets[0].columns.len(), 3, "no column was added");
        assert_eq!(p.column(out).unwrap().analysis.unwrap().state, FormulaState::Current);
    }

    #[test]
    fn a_manual_link_waits_until_it_is_recalculated() {
        let (mut p, _x, y, _) = setup();
        let report = p.analyze_linked(&smooth_request(y), &OutputTarget::NextToInput, RecalcMode::Manual).unwrap();
        let out = report.new_columns[0];
        let old = p.numeric(out).unwrap().to_vec();
        let before = p.clone();
        p.column_mut(y).unwrap().data = ColumnData::numeric(vec![9.0; 20]);
        p.update_analysis_links(&before);
        assert_eq!(p.numeric(out).unwrap(), old.as_slice(), "not recalculated yet");
        assert_eq!(p.column(out).unwrap().analysis.unwrap().state, FormulaState::Pending);
        let link = p.analysis_link_of(out).unwrap();
        p.recalculate_analysis(link).unwrap();
        assert_eq!(p.numeric(out).unwrap(), vec![9.0; 20].as_slice());
        assert_eq!(p.column(out).unwrap().analysis.unwrap().state, FormulaState::Current);
    }

    #[test]
    fn a_table_result_is_replaced_in_the_same_sheet() {
        let (mut p, _x, y, book) = setup();
        let req = Request {
            columns: vec![y],
            rows: RowRange::All,
            tool: Tool::Integrate(IntegrateSpec { cumulative: false, ..IntegrateSpec::default() }),
        };
        let report = p.analyze_linked(&req, &OutputTarget::NextToInput, RecalcMode::Auto).unwrap();
        let area = report.new_columns[1];
        let before = p.clone();
        p.column_mut(y).unwrap().data = ColumnData::numeric(vec![1.0; 20]);
        p.update_analysis_links(&before);
        assert_eq!(p.workbook(book).unwrap().sheets.len(), 2, "the result sheet was reused, not added");
        assert_eq!(p.numeric(area).unwrap()[0], 19.0);
    }

    #[test]
    fn output_can_go_to_a_new_sheet_or_to_existing_columns() {
        let (mut p, x, y, book) = setup();
        let report = p.analyze_to(&smooth_request(y), &OutputTarget::NewSheet).unwrap();
        let b = p.workbook(book).unwrap();
        assert_eq!((b.sheets.len(), b.sheets[0].columns.len()), (2, 2), "the input sheet is untouched");
        let sheet = &b.sheets[report.new_sheet.unwrap()];
        assert_eq!(sheet.columns.iter().map(|c| c.designation).collect::<Vec<_>>(), [Designation::X, Designation::Y]);
        assert_eq!(sheet.columns[0].numeric().unwrap(), p.numeric(x).unwrap());
        // Existing columns: the result replaces the values of the chosen column.
        let empty = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![0.0; 20])).unwrap();
        let report = p.analyze_to(&smooth_request(y), &OutputTarget::Existing(vec![empty])).unwrap();
        assert_eq!(report.new_columns, [empty]);
        assert!(p.numeric(empty).unwrap().iter().any(|v| *v != 0.0));
        assert!(p.column(empty).unwrap().long_name.starts_with("Smoothed"));
        // A wrong number of columns is refused and nothing changes.
        let before = p.clone();
        assert!(p.analyze_to(&smooth_request(y), &OutputTarget::Existing(vec![empty, y])).is_err());
        assert_eq!(p, before);
    }

    #[test]
    fn applying_again_replaces_the_previous_result() {
        let (mut p, _x, y, book) = setup();
        let (_, first) =
            p.analyze_linked_into(None, &smooth_request(y), &OutputTarget::NextToInput, RecalcMode::Manual).unwrap();
        let (_, second) = p
            .analyze_linked_into(Some(&first), &smooth_request(y), &OutputTarget::NextToInput, RecalcMode::None)
            .unwrap();
        assert_eq!(first, second);
        assert_eq!(p.workbook(book).unwrap().sheets[0].columns.len(), 3);
        assert!(p.analysis_links.is_empty(), "mode None drops the link");
        assert!(p.column(first[0]).unwrap().analysis.is_none());
    }

    #[test]
    fn links_survive_a_save_as_json() {
        let (mut p, _x, y, _) = setup();
        p.analyze_linked(&smooth_request(y), &OutputTarget::NewSheet, RecalcMode::Manual).unwrap();
        let text = serde_json::to_string(&p).unwrap();
        let back: Project = serde_json::from_str(&text).unwrap();
        assert_eq!(back.analysis_links, p.analysis_links);
    }
}
