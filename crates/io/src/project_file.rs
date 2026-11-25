//! The `.sheaf` project file: a ZIP archive with JSON descriptions and one
//! raw data member per column. The layout is specified in
//! `docs/spec/project-file-format.md`.

use crate::error::IoError;
use serde::{Deserialize, Serialize};
use sheaf_core::{ColumnData, Project, Window};
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufReader, BufWriter, Cursor, Read, Write};
use std::path::{Path, PathBuf};
use zip::result::ZipError;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

/// File extension of project files, without the dot.
pub const EXTENSION: &str = "sheaf";
/// The newest format version this program reads and writes.
pub const FORMAT_VERSION: u32 = 1;

const FORMAT_NAME: &str = "sheaf-project";
const MANIFEST: &str = "manifest.json";
const PROJECT: &str = "project.json";

#[derive(Serialize, Deserialize)]
struct Manifest {
    format: String,
    version: u32,
    #[serde(default)]
    app: String,
    /// Set on a companion file that Sheaf wrote next to an `.opj` (see `crate::save`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    companion: bool,
    /// Set on a window file (File > Save Window As): `workbook` or `graph`. See `crate::window_file`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    window: Option<String>,
    /// Set on a user graph template (File > Save Template As): `graph`. See `crate::user_templates`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    template: Option<String>,
}

/// What a file says it is, besides a project.
#[derive(Clone, Copy)]
enum Marker<'a> {
    Project,
    Companion,
    Window(&'a str),
    Template(&'a str),
}

/// What a reader wants a file to be; anything else is refused before data is read.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Want {
    Project,
    Window,
    Template,
}

fn numeric_member(id: u64) -> String {
    format!("data/{id}.f64")
}

fn text_member(id: u64) -> String {
    format!("data/{id}.txt.json")
}

/// The text cells of a Text & Numeric column (a JSON object, row -> text). The numbers of such a
/// column are in the `.f64` member; this member marks the column as Text & Numeric.
fn mixed_member(id: u64) -> String {
    format!("data/{id}.mixed.json")
}

/// The member that holds the numbers of a matrix sheet: `rows * cols` doubles, row by row.
fn matrix_member(data_id: u64) -> String {
    format!("data/m{data_id}.f64")
}

/// Saves a project. The archive is written to a temporary file next to
/// `path` and then renamed over it, so a crash never leaves a half-written
/// project behind.
pub fn save(project: &Project, path: &Path) -> Result<(), IoError> {
    save_marked(project, path, Marker::Project)
}

/// Like [`save`], and marks the file as a companion of an `.opj` that Sheaf wrote, so that Sheaf
/// can tell it from a project the user made and delete it when it is no longer needed.
pub fn save_companion(project: &Project, path: &Path) -> Result<(), IoError> {
    save_marked(project, path, Marker::Companion)
}

/// Writes a window file: the same archive as a project, with `window` (`workbook` or `graph`) in the
/// manifest so that it is never taken for a project.
pub(crate) fn save_window_file(project: &Project, path: &Path, window: &str) -> Result<(), IoError> {
    save_marked(project, path, Marker::Window(window))
}

/// Writes a user template file: the same archive as a project, with `template` (`graph`) in the
/// manifest so that it is never taken for a project or a window file.
pub(crate) fn save_template_file(project: &Project, path: &Path, template: &str) -> Result<(), IoError> {
    save_marked(project, path, Marker::Template(template))
}

/// The manifest of `path`; `None` for a file that is not a Sheaf archive and a file that cannot be read.
fn sheaf_manifest(path: &Path) -> Option<Manifest> {
    let file = File::open(path).ok()?;
    let mut archive = ZipArchive::new(BufReader::new(file)).ok()?;
    let bytes = read_member(&mut archive, path, MANIFEST).ok()??;
    let manifest = serde_json::from_slice::<Manifest>(&bytes).ok()?;
    (manifest.format == FORMAT_NAME).then_some(manifest)
}

/// The `window` marker of the manifest of `path`: `None` for a project, a companion, a file that is
/// not a Sheaf archive and a file that cannot be read.
pub(crate) fn window_marker(path: &Path) -> Option<String> {
    sheaf_manifest(path)?.window
}

/// The `template` marker of the manifest of `path`: `None` for a project, a window file, a file that
/// is not a Sheaf archive and a file that cannot be read.
pub(crate) fn template_marker(path: &Path) -> Option<String> {
    sheaf_manifest(path)?.template
}

/// True when `path` is a project file that Sheaf wrote as a companion of an `.opj`. False for any
/// other file, including one that cannot be read.
pub fn is_companion(path: &Path) -> bool {
    let Ok(file) = File::open(path) else { return false };
    let Ok(mut archive) = ZipArchive::new(BufReader::new(file)) else { return false };
    let Ok(Some(bytes)) = read_member(&mut archive, path, MANIFEST) else { return false };
    serde_json::from_slice::<Manifest>(&bytes).is_ok_and(|m| m.format == FORMAT_NAME && m.companion)
}

fn save_marked(project: &Project, path: &Path, marker: Marker<'_>) -> Result<(), IoError> {
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "project".to_string());
    let tmp: PathBuf = parent.join(format!(".{name}.{}.tmp", std::process::id()));
    let result = write_archive(project, path, &tmp, marker)
        .and_then(|()| std::fs::rename(&tmp, path).map_err(|e| IoError::io(path, e)));
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

fn write_archive(project: &Project, path: &Path, tmp: &Path, marker: Marker<'_>) -> Result<(), IoError> {
    let zip_err = |source: ZipError| IoError::Zip { path: path.to_path_buf(), source };
    let io_err = |source: std::io::Error| IoError::io(path, source);
    let json_err = |member: &str, source: serde_json::Error| IoError::Json {
        path: path.to_path_buf(),
        member: member.to_string(),
        source,
    };

    // Refuse before writing anything: two columns with one id would overwrite each other's data.
    let mut seen = HashSet::new();
    let mut seen_matrix = HashSet::new();
    for w in &project.windows {
        if let Window::Workbook(b) = w {
            for c in b.sheets.iter().flat_map(|s| &s.columns) {
                if !seen.insert(c.id.0) {
                    return Err(IoError::DuplicateColumnId { path: path.to_path_buf(), id: c.id.0 });
                }
            }
        }
        if let Window::Matrix(m) = w {
            for s in &m.sheets {
                if !seen_matrix.insert(s.data_id) {
                    return Err(IoError::DuplicateColumnId { path: path.to_path_buf(), id: s.data_id });
                }
            }
        }
    }

    let file = File::create(tmp).map_err(|e| IoError::io(tmp, e))?;
    let mut zip = ZipWriter::new(BufWriter::new(file));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let put = |zip: &mut ZipWriter<BufWriter<File>>, name: &str, bytes: &[u8]| -> Result<(), IoError> {
        let opts = options.large_file(bytes.len() as u64 >= u64::from(u32::MAX));
        zip.start_file(name, opts).map_err(zip_err)?;
        zip.write_all(bytes).map_err(io_err)
    };

    let manifest = Manifest {
        format: FORMAT_NAME.to_string(),
        version: FORMAT_VERSION,
        app: env!("CARGO_PKG_VERSION").to_string(),
        companion: matches!(marker, Marker::Companion),
        window: match marker {
            Marker::Window(w) => Some(w.to_string()),
            _ => None,
        },
        template: match marker {
            Marker::Template(t) => Some(t.to_string()),
            _ => None,
        },
    };
    put(&mut zip, MANIFEST, &serde_json::to_vec_pretty(&manifest).map_err(|e| json_err(MANIFEST, e))?)?;
    put(&mut zip, PROJECT, &serde_json::to_vec_pretty(project).map_err(|e| json_err(PROJECT, e))?)?;

    // The data members, in the order of the windows and their columns. Deflating them is most of
    // the time of a save (T426), so large projects compress them on several threads, each into an
    // archive in memory, and copy the compressed members into the file in the same order: the
    // file is the same as one written member by member.
    let mut jobs: Vec<DataMember<'_>> = Vec::new();
    for w in &project.windows {
        if let Window::Matrix(book) = w {
            jobs.extend(book.sheets.iter().map(DataMember::Matrix));
        }
        if let Window::Workbook(book) = w {
            jobs.extend(book.sheets.iter().flat_map(|s| &s.columns).map(DataMember::Column));
        }
    }
    let total: usize = jobs.iter().map(DataMember::size).sum();
    let threads = if total < PARALLEL_FROM {
        1
    } else {
        std::thread::available_parallelism().map_or(1, |n| n.get()).clamp(1, 8).min(jobs.len().max(1))
    };
    if threads <= 1 {
        for job in &jobs {
            job.write(&mut |name, bytes| put(&mut zip, name, bytes), &json_err)?;
        }
    } else {
        let groups = split_by_size(&jobs, threads);
        let parts: Vec<Result<Vec<u8>, IoError>> = std::thread::scope(|scope| {
            let handles: Vec<_> = groups
                .iter()
                .map(|group| {
                    scope.spawn(|| -> Result<Vec<u8>, IoError> {
                        let mut part = ZipWriter::new(Cursor::new(Vec::new()));
                        for job in *group {
                            job.write(
                                &mut |name, bytes| {
                                    let opts = options.large_file(bytes.len() as u64 >= u64::from(u32::MAX));
                                    part.start_file(name, opts).map_err(zip_err)?;
                                    part.write_all(bytes).map_err(io_err)
                                },
                                &json_err,
                            )?;
                        }
                        Ok(part.finish().map_err(zip_err)?.into_inner())
                    })
                })
                .collect();
            handles.into_iter().map(|h| h.join().expect("a save thread panicked")).collect()
        });
        for part in parts {
            let archive = ZipArchive::new(Cursor::new(part?)).map_err(zip_err)?;
            zip.merge_archive(archive).map_err(zip_err)?;
        }
    }
    let mut out = zip.finish().map_err(zip_err)?;
    out.flush().map_err(io_err)?;
    let file = out.into_inner().map_err(|e| IoError::io(path, e.into_error()))?;
    file.sync_all().map_err(io_err)
}

/// Below this many bytes of data a project is compressed on one thread (threads cost more than they
/// save on small files).
const PARALLEL_FROM: usize = 4 << 20;

/// One window's data that goes to the archive as one or two members.
enum DataMember<'a> {
    Matrix(&'a sheaf_core::matrix::MatrixSheet),
    Column(&'a sheaf_core::Column),
}

impl DataMember<'_> {
    /// About how many bytes the member's data has (for sharing the work between threads).
    fn size(&self) -> usize {
        match self {
            DataMember::Matrix(s) => s.values().len() * 8,
            DataMember::Column(c) => match &c.data {
                ColumnData::Numeric(v) => v.len() * 8,
                ColumnData::Mixed { numbers, texts } => numbers.len() * 8 + texts.len() * 16,
                ColumnData::Text(v) => v.iter().map(|t| t.len() + 3).sum(),
            },
        }
    }

    /// Writes the member(s) with `put(name, bytes)`.
    fn write(
        &self,
        put: &mut dyn FnMut(&str, &[u8]) -> Result<(), IoError>,
        json_err: &dyn Fn(&str, serde_json::Error) -> IoError,
    ) -> Result<(), IoError> {
        let doubles = |values: &[f64]| {
            let mut bytes = Vec::with_capacity(values.len() * 8);
            for v in values {
                bytes.extend_from_slice(&v.to_le_bytes());
            }
            bytes
        };
        match self {
            DataMember::Matrix(sheet) => put(&matrix_member(sheet.data_id), &doubles(sheet.values())),
            DataMember::Column(col) => match &col.data {
                ColumnData::Numeric(values) => put(&numeric_member(col.id.0), &doubles(values)),
                ColumnData::Mixed { numbers, texts } => {
                    put(&numeric_member(col.id.0), &doubles(numbers))?;
                    let member = mixed_member(col.id.0);
                    let json = serde_json::to_vec(texts.as_ref()).map_err(|e| json_err(&member, e))?;
                    put(&member, &json)
                }
                ColumnData::Text(values) => {
                    let member = text_member(col.id.0);
                    let bytes = serde_json::to_vec(values.as_ref()).map_err(|e| json_err(&member, e))?;
                    put(&member, &bytes)
                }
            },
        }
    }
}

/// Cuts `jobs` into at most `n` runs in order, of about the same total size.
fn split_by_size<'a, 'b>(jobs: &'a [DataMember<'b>], n: usize) -> Vec<&'a [DataMember<'b>]> {
    let total: usize = jobs.iter().map(DataMember::size).sum::<usize>().max(1);
    let mut out = Vec::new();
    let (mut start, mut acc) = (0, 0usize);
    for (i, job) in jobs.iter().enumerate() {
        acc += job.size();
        // Close the run when it has reached its share of the total.
        if acc * n >= total * (out.len() + 1) && out.len() + 1 < n {
            out.push(&jobs[start..=i]);
            start = i + 1;
        }
    }
    if start < jobs.len() {
        out.push(&jobs[start..]);
    }
    out
}

/// Loads a project and makes sure its identifier counter is ahead of every
/// identifier in the file. A window file (File > Save Window As) and a user template file (File >
/// Save Template As) are refused: they are never opened as a project, so they cannot replace one
/// (see [`crate::window_file`] and [`crate::user_templates`]).
pub fn load(path: &Path) -> Result<Project, IoError> {
    load_any(path, Want::Project).map(|(project, _)| project)
}

/// Loads a window file: the project it holds and the kind in its manifest. A project, or a file
/// without the marker, is refused.
pub(crate) fn load_window_file(path: &Path) -> Result<(Project, String), IoError> {
    let (project, kind) = load_any(path, Want::Window)?;
    Ok((project, kind.unwrap_or_default()))
}

/// Loads a user template file: the project it holds and the kind in its manifest. Anything without
/// the template marker is refused.
pub(crate) fn load_template_file(path: &Path) -> Result<(Project, String), IoError> {
    let (project, kind) = load_any(path, Want::Template)?;
    Ok((project, kind.unwrap_or_default()))
}

/// Reads the archive. The manifest is checked before any data is read: a window or template file is
/// refused when a project is wanted, a file without the wanted marker otherwise. Returns the marker.
fn load_any(path: &Path, want: Want) -> Result<(Project, Option<String>), IoError> {
    let file = File::open(path).map_err(|e| IoError::io(path, e))?;
    let mut archive =
        ZipArchive::new(BufReader::new(file)).map_err(|source| IoError::Zip { path: path.to_path_buf(), source })?;

    let manifest_bytes =
        read_member(&mut archive, path, MANIFEST)?.ok_or_else(|| IoError::NotAProject { path: path.to_path_buf() })?;
    let manifest: Manifest = serde_json::from_slice(&manifest_bytes).map_err(|source| IoError::Json {
        path: path.to_path_buf(),
        member: MANIFEST.to_string(),
        source,
    })?;
    if manifest.format != FORMAT_NAME {
        return Err(IoError::NotAProject { path: path.to_path_buf() });
    }
    let here = path.to_path_buf();
    match want {
        Want::Project if manifest.window.is_some() => return Err(IoError::IsAWindowFile { path: here }),
        Want::Project if manifest.template.is_some() => return Err(IoError::IsATemplateFile { path: here }),
        Want::Window if manifest.window.is_none() => return Err(IoError::NotAWindowFile { path: here }),
        Want::Template if manifest.template.is_none() => return Err(IoError::NotATemplateFile { path: here }),
        _ => {}
    }
    if manifest.version > FORMAT_VERSION {
        return Err(IoError::UnsupportedVersion {
            path: path.to_path_buf(),
            found: manifest.version,
            supported: FORMAT_VERSION,
        });
    }

    let project_bytes = read_member(&mut archive, path, PROJECT)?.ok_or_else(|| IoError::Corrupt {
        path: path.to_path_buf(),
        member: PROJECT.to_string(),
        detail: "is missing".to_string(),
    })?;
    let mut project: Project = serde_json::from_slice(&project_bytes).map_err(|source| IoError::Json {
        path: path.to_path_buf(),
        member: PROJECT.to_string(),
        source,
    })?;

    for w in &mut project.windows {
        if let Window::Matrix(book) = w {
            if book.sheets.is_empty() {
                book.sheets.push(sheaf_core::MatrixSheet::new("MSheet1", 0, 1, 1));
                book.active_sheet = 0;
            }
            for sheet in &mut book.sheets {
                let member = matrix_member(sheet.data_id);
                let Some(bytes) = read_member(&mut archive, path, &member)? else {
                    return Err(IoError::MissingColumnData {
                        path: path.to_path_buf(),
                        column: format!("matrix sheet '{}' (id {})", sheet.name, sheet.data_id),
                    });
                };
                let expected = sheet.rows.saturating_mul(sheet.cols);
                if bytes.len() % 8 != 0 || bytes.len() / 8 != expected {
                    return Err(IoError::Corrupt {
                        path: path.to_path_buf(),
                        member,
                        detail: format!(
                            "has {} bytes, but a matrix of {} x {} needs {}",
                            bytes.len(),
                            sheet.rows,
                            sheet.cols,
                            expected * 8
                        ),
                    });
                }
                let values = bytes.as_chunks::<8>().0.iter().map(|c| f64::from_le_bytes(*c)).collect();
                sheet.data = ColumnData::numeric(values);
            }
            continue;
        }
        let Window::Workbook(book) = w else { continue };
        // A damaged or hand-edited file may list no sheets; the rest of the program assumes at
        // least one, so an empty first sheet is added.
        if book.sheets.is_empty() {
            book.sheets.push(sheaf_core::Worksheet::new("Sheet1"));
            book.active_sheet = 0;
        }
        for col in book.sheets.iter_mut().flat_map(|s| &mut s.columns) {
            let id = col.id.0;
            let numeric = numeric_member(id);
            let text = text_member(id);
            col.data = if let Some(bytes) = read_member(&mut archive, path, &numeric)? {
                if bytes.len() % 8 != 0 {
                    return Err(IoError::Corrupt {
                        path: path.to_path_buf(),
                        member: numeric,
                        detail: format!("has {} bytes, which is not a whole number of 8-byte values", bytes.len()),
                    });
                }
                let values = bytes
                    .as_chunks::<8>()
                    .0
                    .iter()
                    .map(|c| f64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]))
                    .collect();
                match read_member(&mut archive, path, &mixed_member(id))? {
                    Some(json) => {
                        let texts: std::collections::BTreeMap<usize, String> =
                            serde_json::from_slice(&json).map_err(|source| IoError::Json {
                                path: path.to_path_buf(),
                                member: mixed_member(id),
                                source,
                            })?;
                        ColumnData::mixed(values, texts)
                    }
                    None => ColumnData::numeric(values),
                }
            } else if let Some(bytes) = read_member(&mut archive, path, &text)? {
                let values: Vec<String> = serde_json::from_slice(&bytes).map_err(|source| IoError::Json {
                    path: path.to_path_buf(),
                    member: text,
                    source,
                })?;
                ColumnData::text(values)
            } else {
                return Err(IoError::MissingColumnData {
                    path: path.to_path_buf(),
                    column: format!("'{}' (id {id})", col.display_name()),
                });
            };
        }
    }
    project.repair_ids();
    Ok((project, if want == Want::Template { manifest.template } else { manifest.window }))
}

/// Reads one archive member; `None` when the archive has no such member.
fn read_member<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    path: &Path,
    name: &str,
) -> Result<Option<Vec<u8>>, IoError> {
    match archive.by_name(name) {
        Ok(mut member) => {
            let mut bytes = Vec::new();
            member.read_to_end(&mut bytes).map_err(|e| IoError::Corrupt {
                path: path.to_path_buf(),
                member: name.to_string(),
                detail: format!("cannot be decompressed: {e}"),
            })?;
            Ok(Some(bytes))
        }
        Err(ZipError::FileNotFound) => Ok(None),
        Err(source) => Err(IoError::Zip { path: path.to_path_buf(), source }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sheaf_core::template::Template;
    use sheaf_core::{Column, ColumnId, Designation};

    struct TempDir(PathBuf);
    impl TempDir {
        fn new(tag: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("sheaf-io-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            TempDir(dir)
        }
        fn file(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn sample() -> Project {
        let mut p = Project::new();
        let book = p.add_workbook("demo \u{3b1}");
        let x = p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![0.0, 1.0, f64::NAN, 3.5])).unwrap();
        let y =
            p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0, f64::INFINITY, 9.0, -0.0])).unwrap();
        let t = p
            .add_column(
                book,
                0,
                Designation::Label,
                ColumnData::text(vec!["a".into(), "".into(), "line\nbreak \"q\"".into(), "\u{e9}".into()]),
            )
            .unwrap();
        p.column_mut(y).unwrap().units = "mA".into();
        p.column_mut(t).unwrap().long_name = "names".into();
        let g = p.add_graph(Template::CyclicVoltammetry);
        p.add_plot(g, 0, y).unwrap();
        assert_eq!(p.graph(g).unwrap().layers[0].plots[0].x, Some(x));
        p
    }

    fn same_data(a: &Project, b: &Project) {
        assert_eq!(a, b, "project description and column data are equal");
    }

    #[test]
    fn round_trip_keeps_columns_graphs_and_ids() {
        let dir = TempDir::new("roundtrip");
        let path = dir.file("demo.sheaf");
        let p = sample();
        save(&p, &path).unwrap();
        let mut back = load(&path).unwrap();
        same_data(&p, &back);
        let book = p.workbooks().next().unwrap();
        let y = &book.sheets[0].columns[1];
        assert!(back.column(y.id).unwrap().numeric().unwrap()[3].is_sign_negative(), "-0.0 survives");
        // The counter is ahead of every id in the file.
        let new_id = back.new_column_id();
        assert!(back.column(new_id).is_none());
        // No temporary file is left behind.
        let names: Vec<_> = std::fs::read_dir(&dir.0).unwrap().map(|e| e.unwrap().file_name()).collect();
        assert_eq!(names.len(), 1, "{names:?}");
        // Saving again over an existing file works.
        save(&p, &path).unwrap();
    }

    /// T426: a project big enough for the parallel save (several threads, merged in order) reads back
    /// equal, with every member in the order of a one-thread save.
    #[test]
    fn a_large_project_saved_on_several_threads_reads_back_the_same() {
        let dir = TempDir::new("parallel");
        let path = dir.file("big.sheaf");
        let mut p = sample();
        let book = p.add_workbook("big");
        for c in 0..6 {
            let v: Vec<f64> = (0..120_000).map(|i| (i as f64 * 0.01 + c as f64).sin()).collect();
            p.add_column(book, 0, Designation::Y, ColumnData::numeric(v)).unwrap();
        }
        let texts: Vec<String> = (0..20_000).map(|i| format!("t{i}")).collect();
        p.add_column(book, 0, Designation::Label, ColumnData::text(texts)).unwrap();
        let m = p.add_matrix_book("field");
        p.matrix_book_mut(m).unwrap().sheets[0].set_dimensions(200, 300, None).unwrap();
        p.matrix_book_mut(m).unwrap().sheets[0].set_values("i*j").unwrap();
        let jobs: usize = p
            .windows
            .iter()
            .map(|w| match w {
                Window::Workbook(b) => {
                    b.sheets.iter().flat_map(|s| &s.columns).map(|c| DataMember::Column(c).size()).sum()
                }
                Window::Matrix(b) => b.sheets.iter().map(|s| DataMember::Matrix(s).size()).sum(),
                _ => 0,
            })
            .sum();
        assert!(jobs > PARALLEL_FROM, "the test project takes the parallel path ({jobs} bytes)");
        save(&p, &path).unwrap();
        let back = load(&path).unwrap();
        same_data(&p, &back);
        // The members are in window and column order, as one thread writes them.
        let names: Vec<String> = {
            let mut a = ZipArchive::new(BufReader::new(File::open(&path).unwrap())).unwrap();
            (0..a.len()).map(|i| a.by_index(i).unwrap().name().unwrap().to_string()).collect()
        };
        let mut expected = vec![MANIFEST.to_string(), PROJECT.to_string()];
        for w in &p.windows {
            if let Window::Matrix(b) = w {
                expected.extend(b.sheets.iter().map(|s| matrix_member(s.data_id)));
            }
            if let Window::Workbook(b) = w {
                for c in b.sheets.iter().flat_map(|s| &s.columns) {
                    match &c.data {
                        ColumnData::Numeric(_) => expected.push(numeric_member(c.id.0)),
                        ColumnData::Mixed { .. } => expected.extend([numeric_member(c.id.0), mixed_member(c.id.0)]),
                        ColumnData::Text(_) => expected.push(text_member(c.id.0)),
                    }
                }
            }
        }
        assert_eq!(names, expected);
    }

    /// The work is cut into runs in order that cover every member once.
    #[test]
    fn the_members_are_split_in_order_without_gaps() {
        let p = sample();
        let cols: Vec<&Column> = p.workbooks().flat_map(|b| b.sheets.iter().flat_map(|s| &s.columns)).collect();
        let jobs: Vec<DataMember<'_>> = cols.iter().map(|c| DataMember::Column(c)).collect();
        for n in 1..=5 {
            let runs = split_by_size(&jobs, n);
            assert!(runs.len() <= n.max(1));
            assert_eq!(runs.iter().map(|r| r.len()).sum::<usize>(), jobs.len(), "n = {n}");
        }
    }

    #[test]
    fn round_trip_keeps_matrix_books_and_their_labels() {
        let dir = TempDir::new("matrix");
        let path = dir.file("m.sheaf");
        let mut p = sample();
        let m = p.add_matrix_book("field");
        {
            let s = &mut p.matrix_book_mut(m).unwrap().sheets[0];
            s.set_dimensions(3, 4, Some((10.0, 40.0, 1.0, 3.0))).unwrap();
            s.set_values("i*10+j").unwrap();
            s.set_value(1, 1, f64::NAN).unwrap();
            s.x.long_name = "Potential".into();
            s.x.units = "V".into();
            s.z.comments = "current density".into();
        }
        let second = {
            let mut sh = sheaf_core::MatrixSheet::new("MSheet2", 0, 2, 2);
            sh.set_values("x+y").unwrap();
            sh
        };
        let m2 = p.adopt_matrix_book("second", vec![second]);
        save(&p, &path).unwrap();
        let back = load(&path).unwrap();
        same_data(&p, &back);
        let s = &back.matrix_book(m).unwrap().sheets[0];
        assert_eq!((s.rows, s.cols), (3, 4));
        assert!(s.value(1, 1).is_nan());
        assert_eq!(s.value(2, 3), 34.0);
        assert_eq!(s.x.units, "V");
        assert_eq!(back.matrix_book(m2).unwrap().sheets[0].value(1, 1), 20.0);
    }

    #[test]
    fn round_trip_keeps_heat_map_and_contour_graphs() {
        use sheaf_core::template::FieldTemplate;
        let dir = TempDir::new("field-graphs");
        let path = dir.file("f.sheaf");
        let mut p = sample();
        let m = p.add_matrix_book("field");
        p.matrix_book_mut(m).unwrap().sheets[0].set_values("i+j").unwrap();
        let hm = p.plot_matrix(FieldTemplate::HeatMap, m, 0).unwrap();
        let book = p.workbooks().next().unwrap().id;
        let cols: Vec<_> = p.workbooks().next().unwrap().sheets[0].columns.iter().map(|c| c.id).collect();
        let _ = book;
        let xyz = p.plot_xyz(FieldTemplate::ContourFill, cols[0], cols[1], cols[1]).unwrap();
        p.graph_mut(xyz).unwrap().layers[0].field.as_mut().unwrap().levels.min = Some(-2.5);
        save(&p, &path).unwrap();
        let back = load(&path).unwrap();
        same_data(&p, &back);
        assert!(back.graph(hm).unwrap().layers[0].field.is_some());
        assert_eq!(back.graph(xyz).unwrap().layers[0].field.as_ref().unwrap().levels.min, Some(-2.5));
        // The XYZ graph's columns count as read by the graph.
        assert_eq!(back.graph(xyz).unwrap().referenced_columns().len(), 3);
    }

    #[test]
    fn a_project_file_without_matrices_still_loads() {
        // Files written before matrix windows existed have no matrix members and no matrix windows.
        let dir = TempDir::new("nomatrix");
        let path = dir.file("old.sheaf");
        save(&sample(), &path).unwrap();
        assert!(load(&path).unwrap().matrix_books().next().is_none());
    }

    #[test]
    fn a_matrix_member_of_the_wrong_size_is_refused() {
        let dir = TempDir::new("badmatrix");
        let path = dir.file("bad.sheaf");
        let mut p = Project::new();
        let m = p.add_matrix_book("");
        p.matrix_book_mut(m).unwrap().sheets[0].rows = 40; // data (32 x 32) no longer fits
        save(&p, &path).unwrap();
        assert!(matches!(load(&path), Err(IoError::Corrupt { .. })));
    }

    #[test]
    fn round_trip_keeps_a_double_y_graph() {
        let dir = TempDir::new("double-y");
        let path = dir.file("dy.sheaf");
        let mut p = sample();
        let book = p.workbooks().next().unwrap().id;
        let charge = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![0.0, 0.5, 1.5, 3.0])).unwrap();
        let current = p.workbooks().next().unwrap().sheets[0].columns[1].id;
        let g = p.plot_columns(Template::DoubleY, &[current, charge]).unwrap();
        p.graph_mut(g).unwrap().layers[1].y_axis.title_color_follows_plot = true;
        save(&p, &path).unwrap();
        let back = load(&path).unwrap();
        same_data(&p, &back);
        let graph = back.graph(g).unwrap();
        assert_eq!(graph.right_y_member(0), Some(1), "the link and the axis side survive");
        assert!(graph.layers[1].y_axis.title_color_follows_plot);
    }

    #[test]
    fn round_trip_keeps_a_stack_of_panels() {
        use sheaf_core::panels::PanelLayout;
        let dir = TempDir::new("panels");
        let path = dir.file("stack.sheaf");
        let mut p = sample();
        let book = p.workbooks().next().unwrap().id;
        let second = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![5.0, 6.0, 7.0, 8.0])).unwrap();
        let first = p.workbooks().next().unwrap().sheets[0].columns[1].id;
        let g =
            p.plot_panels(Template::CyclicVoltammetry, &[vec![first], vec![second]], &PanelLayout::stack(2)).unwrap();
        save(&p, &path).unwrap();
        let back = load(&path).unwrap();
        same_data(&p, &back);
        let graph = back.graph(g).unwrap();
        assert_eq!(back.graph(g), p.graph(g));
        assert_eq!(graph.link_parent(1), Some(0), "the link survives");
        assert_eq!(graph.panels(), vec![0, 1]);
        assert!(graph.layers[0].y_axis.hide_end_label && !graph.layers[1].x_axis.show_tick_labels);
    }

    #[test]
    fn round_trip_keeps_stack_settings() {
        let dir = TempDir::new("stack");
        let path = dir.file("stack.sheaf");
        let mut p = sample();
        let book = p.workbooks().next().unwrap().id;
        let second = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![5.0, 6.0, 7.0, 8.0])).unwrap();
        let first = p.workbooks().next().unwrap().sheets[0].columns[1].id;
        let g = p.plot_columns(Template::StackedLines, &[first, second]).unwrap();
        {
            let layer = &mut p.graph_mut(g).unwrap().layers[0];
            layer.plot_offset_mut(second).y = 250.0;
            layer.plot_offset_mut(first).follows = Some(second);
        }
        save(&p, &path).unwrap();
        let back = load(&path).unwrap();
        same_data(&p, &back);
        let layer = &back.graph(g).unwrap().layers[0];
        assert_eq!(layer.stack, sheaf_core::graph::Stack::Auto { gap_percent: 10.0 });
        assert_eq!(layer.plot_offset(second).map(|o| o.y), Some(250.0));
        assert_eq!(layer.follows(first), Some(second));
        assert_eq!(back.graph(g), p.graph(g));
    }

    #[test]
    fn round_trip_keeps_the_plot_details_layer_and_page_settings() {
        use sheaf_core::Color;
        use sheaf_core::graph::*;
        let dir = TempDir::new("details");
        let path = dir.file("details.sheaf");
        let mut p = sample();
        let book = p.workbooks().next().unwrap();
        let (y, names) = (book.sheets[0].columns[1].id, book.sheets[0].columns[2].id);
        let g = p.plot_columns(Template::Column, &[y]).unwrap();
        {
            let graph = p.graph_mut(g).unwrap();
            graph.page.style.border = Some(BorderStyle::solid(Color::BLACK, 1.0));
            graph.page.style.comments = "a note".into();
            graph.page.style.layer_icons = false;
            graph.page.style.title_display = TitleDisplay::LongName;
            let layer = &mut graph.layers[0];
            layer.bars.width_pct = Some(60.0);
            layer.group_style.subgrouping = Subgrouping::BySize { size: 2 };
            layer.group_style.subgroup_on = SubgroupOn::LineStyle;
            layer.display.show_y_axes = false;
            layer.background = Some(Color::rgb(1, 2, 3));
            layer.background_style.gradient = Some(Gradient::new(Color::WHITE));
            layer.background_style.border = Some(BorderStyle::solid(Color::RED, 0.5));
            let d = &mut layer.plots[0].decor;
            d.fill.pattern = FillPattern::DiagonalCross;
            d.fill.transparency_pct = 20;
            d.labels.enable = true;
            d.labels.form = LabelForm::Column { column: names };
            d.labels.only_rows = "1-2".into();
            d.labels.decimals = Some(2);
            d.labels.dx_pt = 3.0;
        }
        save(&p, &path).unwrap();
        let back = load(&path).unwrap();
        assert_eq!(back.graph(g), p.graph(g));
        assert!(back.graph(g).unwrap().referenced_columns().contains(&names));
    }

    #[test]
    fn round_trip_keeps_tick_label_sources_and_bar_spacing() {
        use sheaf_core::graph::{BarSpacing, TickLabelSource};
        let dir = TempDir::new("column");
        let path = dir.file("column.sheaf");
        let mut p = sample();
        let book = p.workbooks().next().unwrap();
        let (y, names) = (book.sheets[0].columns[1].id, book.sheets[0].columns[2].id);
        let g = p.plot_columns(Template::Column, &[y]).unwrap();
        {
            let layer = &mut p.graph_mut(g).unwrap().layers[0];
            layer.x_axis.tick_label_source = TickLabelSource::Column { column: names };
            layer.x_axis.tick_label_angle_deg = -30.0;
            layer.bars = BarSpacing { gap_pct: 40.0, overlap_pct: -10.0, width_pct: None };
        }
        save(&p, &path).unwrap();
        let back = load(&path).unwrap();
        same_data(&p, &back);
        let layer = &back.graph(g).unwrap().layers[0];
        assert_eq!(layer.x_axis.tick_label_source, TickLabelSource::Column { column: names });
        assert_eq!(layer.x_axis.tick_label_angle_deg, -30.0);
        assert_eq!(layer.bars, BarSpacing { gap_pct: 40.0, overlap_pct: -10.0, width_pct: None });
        assert!(back.graph(g).unwrap().referenced_columns().contains(&names));
    }

    #[test]
    fn a_recalculating_column_keeps_its_mode_and_pending_mark_in_a_saved_project() {
        use sheaf_core::{Document, FormulaState, RecalcMode};
        let dir = TempDir::new("recalc");
        let path = dir.file("demo.sheaf");
        let mut doc = Document::new(sample());
        let (a, b) = {
            let cols = &doc.project().workbooks().next().unwrap().sheets[0].columns;
            (cols[0].id, cols[1].id)
        };
        doc.edit("formula", |p| p.set_column_values_with(b, "col(A)*2", None, RecalcMode::Manual)).unwrap();
        doc.apply("edit", |p| {
            p.column_mut(a).unwrap().set_cell_text(0, "5");
        });
        assert_eq!(doc.project().column(b).unwrap().formula_state, FormulaState::Pending);
        save(doc.project(), &path).unwrap();
        let back = load(&path).unwrap();
        let col = back.column(b).unwrap();
        assert_eq!((col.recalc, col.formula_state), (RecalcMode::Manual, FormulaState::Pending));
        same_data(doc.project(), &back);
        // The loaded project still recalculates.
        let mut doc = Document::new(back);
        doc.edit("recalculate", |p| p.recalculate(b)).unwrap();
        assert_eq!(doc.project().numeric(b).unwrap()[0], 10.0);
    }

    #[test]
    fn a_formula_with_j_set_on_several_columns_survives_saving_and_still_recalculates_by_column() {
        use sheaf_core::{Document, RecalcMode};
        let dir = TempDir::new("multicol");
        let path = dir.file("multi.sheaf");
        let mut p = Project::new();
        let book = p.add_workbook("");
        let a = p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
        let ids: Vec<_> =
            (0..3).map(|_| p.add_column(book, 0, Designation::Y, ColumnData::numeric(Vec::new())).unwrap()).collect();
        let mut doc = Document::new(p);
        doc.set_columns_values(&ids, "col(A)*j", None, RecalcMode::Auto).unwrap();
        save(doc.project(), &path).unwrap();
        let back = load(&path).unwrap();
        same_data(doc.project(), &back);
        assert!(ids.iter().all(|c| back.column(*c).unwrap().formula.as_deref() == Some("col(A)*j")));
        let mut doc = Document::new(back);
        doc.apply("edit", |p| {
            p.column_mut(a).unwrap().set_cell_text(0, "10");
        });
        let first: Vec<f64> = ids.iter().map(|c| doc.project().numeric(*c).unwrap()[0]).collect();
        assert_eq!(first, [20.0, 30.0, 40.0]);
    }

    #[test]
    fn an_analysis_that_recalculates_survives_saving_and_loading() {
        use sheaf_core::analysis::{OutputTarget, Request, RowRange, Tool};
        use sheaf_core::column::RecalcMode;
        let dir = TempDir::new("analysis-link");
        let path = dir.file("link.sheaf");
        let mut p = sample();
        let y = p.workbooks().next().unwrap().sheets[0].columns[1].id;
        let request = Request { columns: vec![y], rows: RowRange::All, tool: Tool::Smooth(Default::default()) };
        let report = p.analyze_linked(&request, &OutputTarget::NextToInput, RecalcMode::Manual).unwrap();
        save(&p, &path).unwrap();
        let back = load(&path).unwrap();
        assert_eq!(back.analysis_links, p.analysis_links);
        let out = report.new_columns[0];
        assert_eq!(back.column(out).unwrap().analysis, p.column(out).unwrap().analysis);
        assert!(back.column(out).unwrap().analysis.is_some());
    }

    #[test]
    fn text_and_numeric_columns_keep_their_text_cells() {
        let dir = TempDir::new("mixed");
        let path = dir.file("mixed.sheaf");
        let mut p = Project::new();
        let book = p.add_workbook("m");
        let texts = [(1, "<LOD".to_string()), (5, "four".to_string())].into_iter().collect();
        let a = p.add_column(book, 0, Designation::Y, ColumnData::mixed(vec![1.0, f64::NAN, 3.0], texts)).unwrap();
        // a Text & Numeric column without text keeps its format too
        let b = p.add_column(book, 0, Designation::Y, ColumnData::mixed_numbers(vec![1.0, 2.0])).unwrap();
        let c = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0])).unwrap();
        save(&p, &path).unwrap();
        let back = load(&path).unwrap();
        same_data(&p, &back);
        assert_eq!(back.column(a).unwrap().cell_text(5), "four");
        assert!(back.column(b).unwrap().data.is_mixed());
        assert!(!back.column(c).unwrap().data.is_mixed());
    }

    #[test]
    fn member_names_follow_the_spec() {
        let dir = TempDir::new("members");
        let path = dir.file("demo.sheaf");
        let p = sample();
        save(&p, &path).unwrap();
        let mut zip = ZipArchive::new(File::open(&path).unwrap()).unwrap();
        let names: Vec<String> =
            (0..zip.len()).map(|i| zip.by_index(i).unwrap().name().unwrap().into_owned()).collect();
        let ids: Vec<u64> = p.workbooks().next().unwrap().sheets[0].columns.iter().map(|c| c.id.0).collect();
        assert_eq!(
            names,
            vec![
                "manifest.json".to_string(),
                "project.json".to_string(),
                format!("data/{}.f64", ids[0]),
                format!("data/{}.f64", ids[1]),
                format!("data/{}.txt.json", ids[2]),
            ]
        );
        let mut manifest = String::new();
        zip.by_name("manifest.json").unwrap().read_to_string(&mut manifest).unwrap();
        let m: serde_json::Value = serde_json::from_str(&manifest).unwrap();
        assert_eq!(m["format"], "sheaf-project");
        assert_eq!(m["version"], 1);
        assert!(manifest.contains("\n  \"format\""), "pretty-printed with two spaces");
        let mut raw = Vec::new();
        zip.by_name(&format!("data/{}.f64", ids[0])).unwrap().read_to_end(&mut raw).unwrap();
        assert_eq!(raw.len(), 32);
        assert_eq!(&raw[8..16], &1.0f64.to_le_bytes());
    }

    /// Writes an archive with the given members.
    fn archive(path: &Path, members: &[(&str, &[u8])]) {
        let mut zip = ZipWriter::new(File::create(path).unwrap());
        for (name, bytes) in members {
            zip.start_file(*name, SimpleFileOptions::default()).unwrap();
            zip.write_all(bytes).unwrap();
        }
        zip.finish().unwrap();
    }

    #[test]
    fn newer_version_is_refused_with_both_numbers() {
        let dir = TempDir::new("version");
        let path = dir.file("future.sheaf");
        archive(&path, &[("manifest.json", br#"{"format":"sheaf-project","version":99,"app":"9.9"}"#)]);
        let e = load(&path).unwrap_err();
        assert!(matches!(e, IoError::UnsupportedVersion { found: 99, supported: 1, .. }));
        let msg = e.to_string();
        assert!(msg.contains("99") && msg.contains('1') && msg.contains("future.sheaf"), "{msg}");
    }

    #[test]
    fn wrong_format_and_damaged_files_are_errors() {
        let dir = TempDir::new("damaged");
        let other = dir.file("other.sheaf");
        archive(&other, &[("manifest.json", br#"{"format":"something","version":1}"#)]);
        assert!(matches!(load(&other).unwrap_err(), IoError::NotAProject { .. }));
        let no_manifest = dir.file("nomanifest.sheaf");
        archive(&no_manifest, &[("readme.txt", b"hi")]);
        assert!(matches!(load(&no_manifest).unwrap_err(), IoError::NotAProject { .. }));

        let garbage = dir.file("garbage.sheaf");
        std::fs::write(&garbage, b"this is not a zip file at all").unwrap();
        let e = load(&garbage).unwrap_err();
        assert!(matches!(e, IoError::Zip { .. }) && e.to_string().contains("garbage.sheaf"), "{e}");

        let good = dir.file("good.sheaf");
        save(&sample(), &good).unwrap();
        let bytes = std::fs::read(&good).unwrap();
        let truncated = dir.file("truncated.sheaf");
        std::fs::write(&truncated, &bytes[..bytes.len() / 2]).unwrap();
        assert!(load(&truncated).is_err());
        std::fs::write(&truncated, b"").unwrap();
        assert!(load(&truncated).is_err());
        assert!(matches!(load(&dir.file("missing.sheaf")).unwrap_err(), IoError::Io { .. }));
    }

    #[test]
    fn a_column_without_data_names_the_column() {
        let dir = TempDir::new("nodata");
        let good = dir.file("good.sheaf");
        let p = sample();
        save(&p, &good).unwrap();
        // Rebuild the archive without the text column's member.
        let mut src = ZipArchive::new(File::open(&good).unwrap()).unwrap();
        let bad = dir.file("bad.sheaf");
        let mut out = ZipWriter::new(File::create(&bad).unwrap());
        for i in 0..src.len() {
            let mut m = src.by_index(i).unwrap();
            if m.name().unwrap().ends_with(".txt.json") {
                continue;
            }
            let mut bytes = Vec::new();
            m.read_to_end(&mut bytes).unwrap();
            out.start_file(m.name().unwrap(), SimpleFileOptions::default()).unwrap();
            out.write_all(&bytes).unwrap();
        }
        out.finish().unwrap();
        let e = load(&bad).unwrap_err();
        assert!(matches!(e, IoError::MissingColumnData { .. }));
        assert!(e.to_string().contains("names"), "{e}");
    }

    #[test]
    fn odd_sized_numeric_member_is_corrupt() {
        let dir = TempDir::new("odd");
        let path = dir.file("odd.sheaf");
        let mut p = Project::new();
        let b = p.add_workbook("");
        let id = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0])).unwrap();
        let project_json = serde_json::to_vec(&p).unwrap();
        archive(
            &path,
            &[
                ("manifest.json", br#"{"format":"sheaf-project","version":1}"#),
                ("project.json", &project_json),
                (&format!("data/{}.f64", id.0), &[1, 2, 3]),
            ],
        );
        assert!(matches!(load(&path).unwrap_err(), IoError::Corrupt { .. }));
    }

    #[test]
    fn a_workbook_without_sheets_gets_an_empty_first_sheet() {
        let dir = TempDir::new("nosheets");
        let mut p = Project::new();
        let b = p.add_workbook("");
        p.workbook_mut(b).unwrap().sheets.clear();
        let path = dir.file("nosheets.sheaf");
        save(&p, &path).unwrap();
        let back = load(&path).unwrap();
        let wb = back.workbooks().next().unwrap();
        assert_eq!(wb.sheets.len(), 1);
        assert_eq!(wb.sheet().name, "Sheet1");
    }

    #[test]
    fn duplicate_column_ids_are_refused() {
        let dir = TempDir::new("dup");
        let mut p = Project::new();
        let b = p.add_workbook("");
        let wb = p.workbook_mut(b).unwrap();
        for name in ["A", "B"] {
            wb.sheets[0].columns.push(Column::new(ColumnId(7), name, Designation::Y, ColumnData::numeric(vec![1.0])));
        }
        let path = dir.file("dup.sheaf");
        assert!(matches!(save(&p, &path).unwrap_err(), IoError::DuplicateColumnId { id: 7, .. }));
        assert!(!path.exists());
    }
}
