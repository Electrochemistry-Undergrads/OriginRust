//! File input and output of the application: instrument files (CH
//! Instruments text, delimited text), text export, and the project file.
//!
//! Importers return plain [`sheaf_core::Workbook`] values with placeholder
//! identifiers; hand them to `Project::adopt_workbook` to give them real ones.

pub mod ascii;
pub mod chi;
mod error;
pub mod export;
pub mod opj;
pub mod project_file;
mod text;

pub use error::IoError;
pub use text::{decode_text, read_text_file};

use sheaf_core::Workbook;
use sheaf_core::template::Template;
use sheaf_core::worksheet::ImportSource;
use std::path::Path;

/// A project read by [`open_project`].
#[derive(Clone, Debug)]
pub struct OpenedProject {
    pub project: sheaf_core::Project,
    /// True when the file is in another program's format (an Origin `.opj`
    /// project). Such a project cannot be saved back to that file; it has
    /// to be saved as a `.sheaf` project under a new name.
    pub foreign: bool,
    /// Things that were skipped or need a look.
    pub warnings: Vec<String>,
}

/// True for file names that [`open_project`] reads: `.sheaf` and `.opj`.
pub fn is_project_file(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    ext.eq_ignore_ascii_case(project_file::EXTENSION) || ext.eq_ignore_ascii_case("opj")
}

/// Opens a Sheaf project (`.sheaf`) or imports an Origin project (`.opj`).
pub fn open_project(path: &Path) -> Result<OpenedProject, IoError> {
    let is_opj = path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("opj"));
    if is_opj {
        let mut import = opj::read_opj(path)?;
        let mut warnings = std::mem::take(&mut import.warnings);
        warnings.insert(0, format!("Imported from an Origin {} project. Only workbooks are read.", import.version));
        Ok(OpenedProject { project: import.into_project(), foreign: true, warnings })
    } else {
        Ok(OpenedProject { project: project_file::load(path)?, foreign: false, warnings: Vec::new() })
    }
}

/// Which importer to use.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ImportKind {
    /// CHI when the file looks like a CH Instruments export, delimited text otherwise.
    Auto,
    Chi,
    Ascii,
}

/// A file read by [`import_file`].
#[derive(Clone, Debug)]
pub struct Imported {
    /// The data; `workbook.source` records the file and importer.
    pub workbook: Workbook,
    /// `"chi"` or `"ascii"`.
    pub format: &'static str,
    /// The graph template that suits the data: cyclic voltammetry for a CHI
    /// CV file, a plain line plot otherwise.
    pub suggested_template: Template,
    /// Header facts of the file (CHI: technique, date, parameters).
    pub meta: Vec<(String, String)>,
    /// Things the user should look at.
    pub warnings: Vec<String>,
}

/// Reads a file with the chosen importer and options.
pub fn import_file(
    path: &Path,
    kind: ImportKind,
    chi_opts: &chi::ChiOptions,
    ascii_opts: &ascii::AsciiOptions,
) -> Result<Imported, IoError> {
    let text = read_text_file(path)?;
    let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let shown = path.display().to_string();
    let use_chi = match kind {
        ImportKind::Auto => chi::looks_like_chi(&text),
        ImportKind::Chi => true,
        ImportKind::Ascii => false,
    };
    let mut imported = if use_chi {
        let r = chi::parse_chi(&text, &stem, chi_opts).map_err(|e| name_file(e, &shown))?;
        let cv = r.scans > 0 || r.technique.to_ascii_lowercase().contains("cyclic voltammetry");
        Imported {
            workbook: r.workbook,
            format: "chi",
            suggested_template: if cv { Template::CyclicVoltammetry } else { Template::Line },
            meta: r.meta,
            warnings: r.warnings,
        }
    } else {
        let r = ascii::parse_ascii(&text, &stem, ascii_opts).map_err(|e| name_file(e, &shown))?;
        Imported {
            workbook: r.workbook,
            format: "ascii",
            suggested_template: Template::Line,
            meta: Vec::new(),
            warnings: r.warnings,
        }
    };
    imported.workbook.source = Some(ImportSource { path: shown, importer: imported.format.to_string() });
    Ok(imported)
}

/// Parsers only know the file stem; messages shown to the user carry the full path.
fn name_file(e: IoError, shown: &str) -> IoError {
    match e {
        IoError::NoData { detail, .. } => IoError::NoData { source_name: shown.to_string(), detail },
        IoError::NotChi { .. } => IoError::NotChi { source_name: shown.to_string() },
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("sheaf-io-import-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn auto_picks_the_importer_and_records_the_source() {
        let dir = temp("auto");
        let mut cv = String::from("Sept. 12, 2026\r\nCyclic Voltammetry\r\n\r\nPotential/V, Current/A\r\n\r\n");
        for e in [0.0, 0.5, 1.0, 0.5, 0.0, 0.5, 1.0] {
            cv.push_str(&format!("{e}, {:e}\r\n", e * 1e-5));
        }
        std::fs::write(dir.join("run1.txt"), &cv).unwrap();
        std::fs::write(dir.join("table.csv"), "x,y\n1,2\n3,4\n5,6\n").unwrap();
        std::fs::write(dir.join("empty.txt"), "").unwrap();
        let (chi, ascii) = (chi::ChiOptions::default(), ascii::AsciiOptions::default());

        let a = import_file(&dir.join("run1.txt"), ImportKind::Auto, &chi, &ascii).unwrap();
        assert_eq!((a.format, a.suggested_template), ("chi", Template::CyclicVoltammetry));
        assert_eq!(a.workbook.source.as_ref().unwrap().importer, "chi");
        assert_eq!(a.workbook.long_name, "run1");

        let b = import_file(&dir.join("table.csv"), ImportKind::Auto, &chi, &ascii).unwrap();
        assert_eq!((b.format, b.suggested_template), ("ascii", Template::Line));
        assert!(b.workbook.source.unwrap().path.ends_with("table.csv"));

        let e = import_file(&dir.join("table.csv"), ImportKind::Chi, &chi, &ascii).unwrap_err();
        assert!(e.to_string().contains("table.csv"), "{e}");
        let e = import_file(&dir.join("empty.txt"), ImportKind::Auto, &chi, &ascii).unwrap_err();
        assert!(e.to_string().contains("empty.txt"), "{e}");
        let e = import_file(&dir.join("nope.txt"), ImportKind::Auto, &chi, &ascii).unwrap_err();
        assert!(matches!(e, IoError::Io { .. }));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
