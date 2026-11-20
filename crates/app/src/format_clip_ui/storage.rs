//! Private, versioned profile snapshot. Last successful writer is loaded at the next startup.
use sheaf_core::format_clip::FormatClip;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

pub(super) const FILE_NAME: &str = "format-clipboard.json";
const MAX_BYTES: u64 = 16 * 1024 * 1024;
static NEXT: AtomicU64 = AtomicU64::new(0);

/// How long an open or a rename refused because another write is replacing the file is tried
/// again (T236): Windows says "access denied" for the moment a rename over the file is under way.
const BUSY_RETRY: Duration = Duration::from_secs(1);

/// Runs `op` again while it fails with access denied, for up to [`BUSY_RETRY`].
fn retry_busy<T>(mut op: impl FnMut() -> io::Result<T>) -> io::Result<T> {
    let start = Instant::now();
    loop {
        match op() {
            Err(e) if e.kind() == io::ErrorKind::PermissionDenied && start.elapsed() < BUSY_RETRY => {
                std::thread::sleep(Duration::from_millis(2));
            }
            r => return r,
        }
    }
}

pub(super) fn load(path: &Path) -> Option<FormatClip> {
    let mut bytes = Vec::new();
    retry_busy(|| File::open(path)).ok()?.take(MAX_BYTES + 1).read_to_end(&mut bytes).ok()?;
    if bytes.len() as u64 > MAX_BYTES {
        return None;
    }
    let mut record: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    if record.get("version")?.as_u64() != Some(1) {
        return None;
    }
    serde_json::from_value(record.get_mut("format")?.take()).ok()
}

pub(super) fn save(path: &Path, clip: &FormatClip) -> io::Result<()> {
    let bytes = serde_json::to_vec(&serde_json::json!({ "version": 1, "format": clip })).map_err(io::Error::other)?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(io::Error::other("format clipboard exceeds 16 MiB"));
    }
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    for _ in 0..16 {
        let seq = NEXT.fetch_add(1, Ordering::Relaxed);
        let tmp = path.with_extension(format!("{}.{seq}.tmp", std::process::id()));
        let mut file = match OpenOptions::new().write(true).create_new(true).open(&tmp) {
            Ok(file) => file,
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        };
        let result = (|| {
            file.write_all(&bytes)?;
            file.sync_all()?;
            drop(file);
            retry_busy(|| fs::rename(&tmp, path))
        })();
        if result.is_err() {
            let _ = fs::remove_file(&tmp);
        }
        return result;
    }
    Err(io::Error::other("no unique temporary format-clipboard file was available"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sheaf_core::{
        Project,
        format_clip::{self, AxisPart, GraphPart},
        template::Template,
    };
    use std::path::PathBuf;

    fn file() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "sheaf-format-clip-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&dir).unwrap();
        dir.join(FILE_NAME)
    }

    fn clips() -> [FormatClip; 2] {
        let mut p = Project::new();
        let g = p.add_graph(Template::Line);
        p.graph_mut(g).unwrap().layers[0].x_axis.min = Some(2.0);
        [
            format_clip::copy_graph(p.graph(g).unwrap(), GraphPart::Colors),
            format_clip::copy_axis(p.graph(g).unwrap(), 0, false, AxisPart::Scales).unwrap(),
        ]
    }

    #[test]
    fn storage_round_trips_graph_and_partial_axis_and_replaces_the_complete_record() {
        let path = file();
        for clip in clips() {
            save(&path, &clip).unwrap();
            assert_eq!(load(&path), Some(clip));
            assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1, "no temporary file remains");
        }
        fs::remove_file(&path).unwrap();
        fs::remove_dir(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn missing_corrupt_unknown_version_and_oversized_records_are_empty_without_being_modified() {
        let path = file();
        assert_eq!(load(&path), None);
        for bytes in [b"broken".as_slice(), br#"{"version":2,"format":{}}"#, br#"{"version":1,"format":null}"#] {
            fs::write(&path, bytes).unwrap();
            assert_eq!(load(&path), None);
            assert_eq!(fs::read(&path).unwrap(), bytes);
        }
        File::create(&path).unwrap().set_len(MAX_BYTES + 1).unwrap();
        assert_eq!(load(&path), None);
        assert_eq!(fs::metadata(&path).unwrap().len(), MAX_BYTES + 1);
        fs::remove_file(&path).unwrap();
        fs::remove_dir(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn failed_atomic_replace_keeps_the_destination_and_cleans_only_its_temporary_file() {
        let path = file();
        fs::create_dir(&path).unwrap();
        assert!(save(&path, &clips()[0]).is_err());
        assert!(path.is_dir());
        assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
        fs::remove_dir(&path).unwrap();
        fs::remove_dir(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn concurrent_replacements_leave_one_complete_typed_record() {
        let path = file();
        let [a, b] = clips();
        save(&path, &a).unwrap();
        let workers: Vec<_> = [a.clone(), b.clone()]
            .into_iter()
            .map(|clip| {
                let path = path.clone();
                std::thread::spawn(move || {
                    for _ in 0..12 {
                        save(&path, &clip).unwrap();
                    }
                })
            })
            .collect();
        while workers.iter().any(|w| !w.is_finished()) {
            let loaded = load(&path).expect("replacement never exposes a partial record");
            assert!(loaded == a || loaded == b);
        }
        for worker in workers {
            worker.join().unwrap();
        }
        let loaded = load(&path).unwrap();
        assert!(loaded == a || loaded == b);
        assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
        fs::remove_file(&path).unwrap();
        fs::remove_dir(path.parent().unwrap()).unwrap();
    }
}
