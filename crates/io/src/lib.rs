//! File input and output of the application: instrument files (CH
//! Instruments text and binary, delimited text), text export, and the project file.
//!
//! Importers return plain [`sheaf_core::Workbook`] values with placeholder
//! identifiers; hand them to `Project::adopt_workbook` to give them real ones.

pub mod ascii;
pub mod batch;
pub mod chi;
pub mod chi_bin;
mod error;
pub mod excel;
pub mod export;
pub mod opj;
pub mod opju;
pub mod project_file;
pub mod recent;
pub mod recovery;
pub mod save;
mod text;
pub mod themes;
pub mod window_file;

pub use error::IoError;
pub use excel::{is_excel_file, is_excel_workbook};
pub use text::{decode_text, read_text_file};

use sheaf_core::Workbook;
use sheaf_core::template::Template;
use sheaf_core::worksheet::ImportSource;
use std::path::{Path, PathBuf};

/// A project read by [`open_project`].
#[derive(Clone, Debug)]
pub struct OpenedProject {
    pub project: sheaf_core::Project,
    /// True when the file is in another program's format (an Origin `.opj`
    /// project) and no companion was used. Such a project is not saved back to
    /// that file by itself; the user saves it under a name of their choice.
    pub foreign: bool,
    /// The file a plain Save should write: the file itself for a `.sheaf` project, the `.opj` for
    /// a companion, `None` for a foreign project.
    pub save_path: Option<std::path::PathBuf>,
    /// True when the companion `x.opj.sheaf` was opened instead of `x.opj`.
    pub companion_used: bool,
    /// Things that were skipped or need a look.
    pub warnings: Vec<String>,
}

/// True for files that [`open_project`] reads: `.sheaf`, `.opj` and `.opju`.
/// A `.sheaf` file that is a window file (File > Save Window As) is not a project: see
/// [`window_file::is_window_file`].
pub fn is_project_file(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    if ext.eq_ignore_ascii_case(project_file::EXTENSION) {
        return !window_file::is_window_file(path);
    }
    ext.eq_ignore_ascii_case("opj") || ext.eq_ignore_ascii_case("opju")
}

fn modified(path: &Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// Opens a Sheaf project (`.sheaf`) or imports an Origin project (`.opj`).
///
/// For `x.opj` with a companion `x.opj.sheaf` that Sheaf wrote beside it: the companion (the whole
/// project) is opened when it is at least as new as the `.opj`; when the `.opj` is newer, it was
/// changed in Origin, so the `.opj` is read and a warning says the companion was not used.
pub fn open_project(path: &Path) -> Result<OpenedProject, IoError> {
    if path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("opju")) {
        let mut import = opju::read_opju(path)?;
        let mut warnings = std::mem::take(&mut import.warnings);
        warnings.insert(
            0,
            format!(
                "Imported from an Origin Unicode project ({}). Workbooks are read; graphs are not yet.",
                import.version
            ),
        );
        return Ok(OpenedProject {
            project: import.into_project(),
            foreign: true,
            save_path: None,
            companion_used: false,
            warnings,
        });
    }
    let is_opj = path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("opj"));
    if is_opj {
        let companion = save::companion_path(path);
        let mut extra = Vec::new();
        if companion.exists() && project_file::is_companion(&companion) {
            let newer_or_same = match (modified(&companion), modified(path)) {
                (Some(c), Some(o)) => c >= o,
                _ => false,
            };
            if newer_or_same {
                match project_file::load(&companion) {
                    Ok(project) => {
                        return Ok(OpenedProject {
                            project,
                            foreign: false,
                            save_path: Some(path.to_path_buf()),
                            companion_used: true,
                            warnings: vec![format!(
                                "Opened {} (the whole project, with graphs) instead of {}, because it is at least as new.",
                                companion.display(),
                                path.display()
                            )],
                        });
                    }
                    Err(e) => extra.push(format!(
                        "The companion {} could not be read ({e}); the .opj was opened instead.",
                        companion.display()
                    )),
                }
            } else {
                extra.push(format!(
                    "{} is older than {} (which was changed after Sheaf saved it, for example in Origin), so it was not used. Graphs that only the companion held are not in this project.",
                    companion.display(),
                    path.display()
                ));
            }
        }
        let mut import = opj::read_opj(path)?;
        let mut warnings = std::mem::take(&mut import.warnings);
        warnings.insert(
            0,
            format!("Imported from an Origin {} project. Workbooks and basic graphs are read.", import.version),
        );
        warnings.splice(1..1, extra);
        Ok(OpenedProject {
            project: import.into_project(),
            foreign: true,
            save_path: None,
            companion_used: false,
            warnings,
        })
    } else {
        // A companion opened by name still saves to the .opj it belongs to.
        let save_path = if project_file::is_companion(path) { save::companion_target(path) } else { None };
        let save_path = save_path.or_else(|| Some(path.to_path_buf()));
        Ok(OpenedProject {
            project: project_file::load(path)?,
            foreign: false,
            save_path,
            companion_used: false,
            warnings: Vec::new(),
        })
    }
}

/// Which importer to use.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum ImportKind {
    /// CHI when the file looks like a CH Instruments export, delimited text otherwise.
    #[default]
    Auto,
    Chi,
    Ascii,
}

/// A file read by [`import_file`].
#[derive(Clone, Debug)]
pub struct Imported {
    /// The file that was read, as it was given (`workbook.source.path` is its text for display and
    /// loses bytes that are not valid text).
    pub path: PathBuf,
    /// The data; `workbook.source` records the file and importer.
    pub workbook: Workbook,
    /// `"chi"`, `"ascii"` or `"excel"`.
    pub format: &'static str,
    /// The graph template that suits the data: cyclic voltammetry for a CHI
    /// CV file, a plain line plot otherwise.
    pub suggested_template: Template,
    /// Header facts of the file (CHI: technique, date, parameters).
    pub meta: Vec<(String, String)>,
    /// Things the user should look at.
    pub warnings: Vec<String>,
}

/// Reads a file with the chosen importer and options. An Excel workbook (`.xls`, `.xlsx`,
/// `.xlsm`, recognised by its first bytes as well as its name) is read with the default
/// [`excel::ExcelOptions`] when the kind is `Auto`; use [`import_file_with_excel`] to choose its
/// sheets and header rows.
pub fn import_file(
    path: &Path,
    kind: ImportKind,
    chi_opts: &chi::ChiOptions,
    ascii_opts: &ascii::AsciiOptions,
) -> Result<Imported, IoError> {
    import_file_with_excel(path, kind, chi_opts, ascii_opts, &excel::ExcelOptions::default())
}

/// [`import_file`] with the options for Excel workbooks.
pub fn import_file_with_excel(
    path: &Path,
    kind: ImportKind,
    chi_opts: &chi::ChiOptions,
    ascii_opts: &ascii::AsciiOptions,
    excel_opts: &excel::ExcelOptions,
) -> Result<Imported, IoError> {
    if kind == ImportKind::Auto && excel::is_excel_workbook(path) {
        let r = excel::read_workbook(path, excel_opts)?;
        let names: Vec<&str> = r.workbook.sheets.iter().map(|s| s.name.as_str()).collect();
        return Ok(Imported {
            path: path.to_path_buf(),
            meta: vec![("Sheets".to_string(), names.join(", "))],
            workbook: r.workbook,
            format: "excel",
            suggested_template: Template::Line,
            warnings: r.warnings,
        });
    }
    let bytes = std::fs::read(path).map_err(|e| IoError::io(path, e))?;
    let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let shown = path.display().to_string();
    // A CH Instruments binary file is recognised by its first bytes, whatever its extension.
    if kind != ImportKind::Ascii && chi_bin::looks_like_chi_bin(&bytes) {
        let r = chi_bin::parse_chi_bin(&bytes, &stem, chi_opts).map_err(|e| name_file(e, &shown))?;
        return Ok(finish_chi(r, shown, path));
    }
    let text = decode_text(&bytes, path)?;
    let use_chi = match kind {
        ImportKind::Auto => chi::looks_like_chi(&text),
        ImportKind::Chi => true,
        ImportKind::Ascii => false,
    };
    if use_chi {
        let r = chi::parse_chi(&text, &stem, chi_opts).map_err(|e| name_file(e, &shown))?;
        return Ok(finish_chi(r, shown, path));
    }
    let file_name = path.file_name().map_or_else(|| stem.clone(), |s| s.to_string_lossy().into_owned());
    let r = ascii::parse_ascii_named(&text, &file_name, ascii_opts).map_err(|e| name_file(e, &shown))?;
    let mut workbook = r.workbook;
    workbook.source = Some(ImportSource { path: shown, importer: "ascii".to_string() });
    let mut warnings = r.warnings;
    if excel::is_excel_file(path) {
        warnings
            .push("the file is named like an Excel workbook but holds text; it was read as a text file".to_string());
    }
    Ok(Imported {
        path: path.to_path_buf(),
        workbook,
        format: "ascii",
        suggested_template: Template::Line,
        meta: Vec::new(),
        warnings,
    })
}

/// Wraps the result of either CH Instruments reader (text or binary).
fn finish_chi(r: chi::ChiImport, shown: String, path: &Path) -> Imported {
    let cv = r.scans > 0 || r.technique.to_ascii_lowercase().contains("cyclic voltammetry");
    let mut workbook = r.workbook;
    // Origin reads these text files with its ASCII import, which keeps the extension in the book's
    // long name (an imported run1.txt is titled run1.txt).
    if let Some(name) = path.file_name() {
        workbook.long_name = name.to_string_lossy().into_owned();
    }
    workbook.source = Some(ImportSource { path: shown, importer: "chi".to_string() });
    Imported {
        path: path.to_path_buf(),
        workbook,
        format: "chi",
        suggested_template: if cv { Template::CyclicVoltammetry } else { Template::Line },
        meta: r.meta,
        warnings: r.warnings,
    }
}

/// Parsers only know the file stem; messages shown to the user carry the full path.
fn name_file(e: IoError, shown: &str) -> IoError {
    match e {
        IoError::NoData { detail, .. } => IoError::NoData { source_name: shown.to_string(), detail },
        IoError::NotChi { .. } => IoError::NotChi { source_name: shown.to_string() },
        IoError::ChiBinary { detail, .. } => IoError::ChiBinary { source_name: shown.to_string(), detail },
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
        assert_eq!(a.workbook.long_name, "run1.txt", "the title keeps the extension, as in Origin");

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

    #[test]
    fn a_chi_binary_file_is_recognised_by_its_first_bytes_not_its_name() {
        let dir = temp("bin");
        let bytes = chi_bin::tests::cv_file(1.2, 1.2, 0.2, 0.001, false, 4000, |i| i as f32 * 1e-7);
        // The extension says nothing: the bytes decide.
        for name in ["run.bin", "run.dat", "run"] {
            std::fs::write(dir.join(name), &bytes).unwrap();
        }
        let (chi, ascii) = (chi::ChiOptions::default(), ascii::AsciiOptions::default());
        for name in ["run.bin", "run.dat", "run"] {
            for kind in [ImportKind::Auto, ImportKind::Chi] {
                let a = import_file(&dir.join(name), kind, &chi, &ascii).unwrap();
                assert_eq!((a.format, a.suggested_template), ("chi", Template::CyclicVoltammetry), "{name}");
                assert_eq!(a.workbook.source.as_ref().unwrap().importer, "chi");
                assert_eq!(a.workbook.long_name, name, "the title is the whole file name");
                assert!(a.meta.iter().any(|(k, v)| k == "Technique" && v == "Cyclic Voltammetry"));
            }
        }
        // Reading it as delimited text is refused with the usual message about binary data.
        let e = import_file(&dir.join("run.bin"), ImportKind::Ascii, &chi, &ascii).unwrap_err();
        assert!(matches!(e, IoError::NotText { .. }), "{e}");
        // A damaged file names the file in full.
        std::fs::write(dir.join("cut.bin"), &bytes[..40]).unwrap();
        let e = import_file(&dir.join("cut.bin"), ImportKind::Auto, &chi, &ascii).unwrap_err();
        assert!(matches!(e, IoError::ChiBinary { .. }) && e.to_string().contains("cut.bin"), "{e}");
        // Other bytes that merely end in .bin are not taken for CHI files.
        std::fs::write(
            dir.join("table.bin"),
            "x,y
1,2
3,4
5,6
",
        )
        .unwrap();
        assert_eq!(import_file(&dir.join("table.bin"), ImportKind::Auto, &chi, &ascii).unwrap().format, "ascii");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
