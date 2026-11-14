//! Importing data files into a project.

use crate::read::ReadSettings;
use crate::session::{is_project, load_project};
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use sheaf_core::Project;
use sheaf_core::project::Window;
use sheaf_io::batch::{BatchLayout, BatchOptions};
use std::path::PathBuf;

/// Where each file goes.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImportLayout {
    /// One workbook per file.
    #[default]
    Books,
    /// One sheet per file, in one workbook.
    Sheets,
    /// The files side by side as columns of one sheet.
    Columns,
    /// The files one below the other as rows of one sheet.
    Rows,
}

impl ImportLayout {
    pub fn batch(self) -> BatchLayout {
        match self {
            ImportLayout::Books => BatchLayout::Books,
            ImportLayout::Sheets => BatchLayout::Sheets,
            ImportLayout::Columns => BatchLayout::Columns,
            ImportLayout::Rows => BatchLayout::Rows,
        }
    }
}

/// Which files to read and how.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ImportRequest {
    /// Data files, folders, patterns, or projects (their workbooks are taken over).
    pub files: Vec<PathBuf>,
    pub layout: ImportLayout,
    pub read: ReadSettings,
    /// Long name of the shared workbook of the layouts other than `books`.
    pub book_name: Option<String>,
    pub file_name_comments: bool,
    pub partial_except_first: bool,
    /// Make a graph of each imported book (of each sheet with the `sheets` layout).
    pub plot: bool,
}

/// One imported file.
#[derive(Clone, Debug, Serialize)]
pub struct ImportedItem {
    pub file: PathBuf,
    /// The workbook it went into.
    pub book: String,
    pub sheet: usize,
    pub format: String,
    pub suggested_template: String,
    pub warnings: Vec<String>,
}

/// What an import did. A file that could not be read is in `errors`; the others are in the project.
#[derive(Clone, Debug, Default, Serialize)]
pub struct ImportReport {
    pub items: Vec<ImportedItem>,
    pub errors: Vec<String>,
    /// Lines about project inputs and patterns, in order ("imported 2 workbook(s) from ...").
    pub notes: Vec<String>,
    /// Warnings of the projects that were opened as inputs.
    pub project_warnings: Vec<String>,
    /// The Origin projects among the inputs.
    pub foreign_inputs: Vec<PathBuf>,
    /// How many data files the inputs expanded to.
    pub data_files: usize,
    /// Names of the graphs made with `plot`.
    pub graphs: Vec<String>,
}

/// Reads the files into the project.
pub fn import_files(project: &mut Project, req: &ImportRequest) -> Result<ImportReport> {
    let mut report = ImportReport::default();
    // Projects are taken over as they are; everything else is a data file, folder or pattern.
    let mut data_files = Vec::new();
    for arg in &req.files {
        if is_project(arg) {
            let mut count = 0;
            let loaded = load_project(arg)?;
            report.project_warnings.extend(loaded.warnings);
            if loaded.foreign {
                report.foreign_inputs.push(arg.clone());
            }
            for window in loaded.project.windows {
                if let Window::Workbook(book) = window {
                    project.adopt_named_workbook(book);
                    count += 1;
                }
            }
            report.notes.push(format!("imported {count} workbook(s) from {}", arg.display()));
            continue;
        }
        let found = sheaf_io::batch::expand_inputs(arg).map_err(|e| anyhow!(e))?;
        if found.len() > 1 {
            report.notes.push(format!("{} matches {} files", arg.display(), found.len()));
        }
        data_files.extend(found);
    }
    report.data_files = data_files.len();
    let layout = req.layout.batch();
    let opts = BatchOptions {
        layout,
        kind: req.read.format.kind(),
        chi: req.read.chi_options(),
        ascii: req.read.ascii_options()?,
        book_name: req.book_name.clone().unwrap_or_default(),
        file_name_in_comments: req.file_name_comments,
        partial_except_first: req.partial_except_first,
    };
    let result = sheaf_io::batch::import_batch(project, &data_files, &opts);
    let mut plotted = std::collections::HashSet::new();
    for item in &result.items {
        let name = project.workbook(item.book).map(|b| b.name.clone()).unwrap_or_default();
        report.items.push(ImportedItem {
            file: item.path.clone(),
            book: name,
            sheet: item.sheet,
            format: item.format.to_string(),
            suggested_template: item.suggested_template.name().to_string(),
            warnings: item.warnings.clone(),
        });
        // One graph per book, or per sheet when each file has its own sheet.
        let key = if layout == BatchLayout::Sheets { (item.book, item.sheet) } else { (item.book, 0) };
        if req.plot && plotted.insert(key) {
            let ys = project.default_plot_columns(item.book, item.sheet, item.suggested_template);
            if !ys.is_empty() {
                let graph = project.plot_columns(item.suggested_template, &ys)?;
                if let Some(g) = project.graph(graph) {
                    report.graphs.push(g.name.clone());
                }
            }
        }
    }
    report.errors = result.errors;
    Ok(report)
}
