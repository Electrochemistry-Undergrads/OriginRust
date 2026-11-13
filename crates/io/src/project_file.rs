//! The `.sheaf` project file: a ZIP archive with JSON descriptions and one
//! raw data member per column. The layout is specified in
//! `docs/spec/project-file-format.md`.

use crate::error::IoError;
use serde::{Deserialize, Serialize};
use sheaf_core::{ColumnData, Project, Window};
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
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

/// Saves a project. The archive is written to a temporary file next to
/// `path` and then renamed over it, so a crash never leaves a half-written
/// project behind.
pub fn save(project: &Project, path: &Path) -> Result<(), IoError> {
    save_marked(project, path, false)
}

/// Like [`save`], and marks the file as a companion of an `.opj` that Sheaf wrote, so that Sheaf
/// can tell it from a project the user made and delete it when it is no longer needed.
pub fn save_companion(project: &Project, path: &Path) -> Result<(), IoError> {
    save_marked(project, path, true)
}

/// True when `path` is a project file that Sheaf wrote as a companion of an `.opj`. False for any
/// other file, including one that cannot be read.
pub fn is_companion(path: &Path) -> bool {
    let Ok(file) = File::open(path) else { return false };
    let Ok(mut archive) = ZipArchive::new(BufReader::new(file)) else { return false };
    let Ok(Some(bytes)) = read_member(&mut archive, path, MANIFEST) else { return false };
    serde_json::from_slice::<Manifest>(&bytes).is_ok_and(|m| m.format == FORMAT_NAME && m.companion)
}

fn save_marked(project: &Project, path: &Path, companion: bool) -> Result<(), IoError> {
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "project".to_string());
    let tmp: PathBuf = parent.join(format!(".{name}.{}.tmp", std::process::id()));
    let result = write_archive(project, path, &tmp, companion)
        .and_then(|()| std::fs::rename(&tmp, path).map_err(|e| IoError::io(path, e)));
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

fn write_archive(project: &Project, path: &Path, tmp: &Path, companion: bool) -> Result<(), IoError> {
    let zip_err = |source: ZipError| IoError::Zip { path: path.to_path_buf(), source };
    let io_err = |source: std::io::Error| IoError::io(path, source);
    let json_err = |member: &str, source: serde_json::Error| IoError::Json {
        path: path.to_path_buf(),
        member: member.to_string(),
        source,
    };

    // Refuse before writing anything: two columns with one id would overwrite each other's data.
    let mut seen = HashSet::new();
    for w in &project.windows {
        if let Window::Workbook(b) = w {
            for c in b.sheets.iter().flat_map(|s| &s.columns) {
                if !seen.insert(c.id.0) {
                    return Err(IoError::DuplicateColumnId { path: path.to_path_buf(), id: c.id.0 });
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
        companion,
    };
    put(&mut zip, MANIFEST, &serde_json::to_vec_pretty(&manifest).map_err(|e| json_err(MANIFEST, e))?)?;
    put(&mut zip, PROJECT, &serde_json::to_vec_pretty(project).map_err(|e| json_err(PROJECT, e))?)?;

    for w in &project.windows {
        let Window::Workbook(book) = w else { continue };
        for col in book.sheets.iter().flat_map(|s| &s.columns) {
            match &col.data {
                ColumnData::Numeric(values) => {
                    let mut bytes = Vec::with_capacity(values.len() * 8);
                    for v in values.iter() {
                        bytes.extend_from_slice(&v.to_le_bytes());
                    }
                    put(&mut zip, &numeric_member(col.id.0), &bytes)?;
                }
                ColumnData::Mixed { numbers, texts } => {
                    let mut bytes = Vec::with_capacity(numbers.len() * 8);
                    for v in numbers.iter() {
                        bytes.extend_from_slice(&v.to_le_bytes());
                    }
                    put(&mut zip, &numeric_member(col.id.0), &bytes)?;
                    let member = mixed_member(col.id.0);
                    let json = serde_json::to_vec(texts.as_ref()).map_err(|e| json_err(&member, e))?;
                    put(&mut zip, &member, &json)?;
                }
                ColumnData::Text(values) => {
                    let member = text_member(col.id.0);
                    let bytes = serde_json::to_vec(values.as_ref()).map_err(|e| json_err(&member, e))?;
                    put(&mut zip, &member, &bytes)?;
                }
            }
        }
    }
    let mut out = zip.finish().map_err(zip_err)?;
    out.flush().map_err(io_err)?;
    let file = out.into_inner().map_err(|e| IoError::io(path, e.into_error()))?;
    file.sync_all().map_err(io_err)
}

/// Loads a project and makes sure its identifier counter is ahead of every
/// identifier in the file.
pub fn load(path: &Path) -> Result<Project, IoError> {
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
    Ok(project)
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
            layer.bars = BarSpacing { gap_pct: 40.0, overlap_pct: -10.0 };
        }
        save(&p, &path).unwrap();
        let back = load(&path).unwrap();
        same_data(&p, &back);
        let layer = &back.graph(g).unwrap().layers[0];
        assert_eq!(layer.x_axis.tick_label_source, TickLabelSource::Column { column: names });
        assert_eq!(layer.x_axis.tick_label_angle_deg, -30.0);
        assert_eq!(layer.bars, BarSpacing { gap_pct: 40.0, overlap_pct: -10.0 });
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
