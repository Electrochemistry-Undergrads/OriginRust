//! The folders the server may touch. Every path a tool names is made absolute and canonical
//! (links and `..` resolved) and must lie under one of them.

use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Debug, Default)]
pub struct Roots {
    dirs: Vec<PathBuf>,
}

/// Drops the verbatim prefix Windows puts on canonical paths, where the path is a plain drive path.
pub fn plain(path: PathBuf) -> PathBuf {
    let text = path.to_string_lossy();
    match text.strip_prefix("\\\\?\\") {
        Some(rest) if rest.len() > 2 && rest.as_bytes()[1] == b':' => PathBuf::from(rest),
        _ => path,
    }
}

/// The canonical form of a path that need not exist: its nearest existing parent made canonical,
/// plus the names below it. `None` when the path cannot be resolved safely: a `..` below the
/// existing part, or a link that points nowhere (a write through it could land anywhere).
pub(crate) fn canonical(path: &Path) -> Option<PathBuf> {
    let mut candidate = path.to_path_buf();
    let mut tail: Vec<std::ffi::OsString> = Vec::new();
    loop {
        match fs::canonicalize(&candidate) {
            Ok(mut base) => {
                for name in tail.iter().rev() {
                    base.push(name);
                }
                return Some(base);
            }
            Err(_) => {
                if fs::symlink_metadata(&candidate).is_ok() {
                    return None;
                }
                match candidate.components().next_back()? {
                    Component::Normal(name) => tail.push(name.to_os_string()),
                    Component::CurDir => {}
                    _ => return None,
                }
                if !candidate.pop() {
                    return None;
                }
            }
        }
    }
}

impl Roots {
    /// The folders must exist.
    pub fn new(dirs: &[PathBuf]) -> Result<Roots, String> {
        let mut out = Vec::new();
        for d in dirs {
            let c = fs::canonicalize(d).map_err(|e| format!("--root '{}': {e}", d.display()))?;
            if !c.is_dir() {
                return Err(format!("--root '{}' is not a folder", d.display()));
            }
            out.push(c);
        }
        Ok(Roots { dirs: out })
    }

    /// Like [`Roots::new`], for roots a client sent over the link: each one must already be an
    /// absolute path, because the app's own folder is no base for a client's relative path.
    pub fn from_absolute(dirs: &[PathBuf]) -> Result<Roots, String> {
        if let Some(d) = dirs.iter().find(|d| !d.is_absolute()) {
            return Err(format!("root '{}' is not an absolute path", d.display()));
        }
        Roots::new(dirs)
    }

    pub fn is_empty(&self) -> bool {
        self.dirs.is_empty()
    }

    pub fn dirs(&self) -> &[PathBuf] {
        &self.dirs
    }

    fn outside(text: &str) -> String {
        format!("path is outside the allowed folders: {text}")
    }

    /// The canonical path for `text` (absolute, or relative to the first root) when it lies under a
    /// root.
    pub fn check(&self, text: &str) -> Result<PathBuf, String> {
        if text.trim().is_empty() {
            return Err("the path is empty".to_string());
        }
        self.check_path(Path::new(text), text)
    }

    pub fn check_path(&self, path: &Path, shown: &str) -> Result<PathBuf, String> {
        let Some(first) = self.dirs.first() else {
            return Err("no folder is allowed: start sheaf-mcp with --root DIR".to_string());
        };
        // Network shares and device names are refused before anything touches the file system:
        // resolving them could connect to another machine or open a device.
        if !local_name(path) {
            return Err(Self::outside(shown));
        }
        let absolute = if path.is_absolute() { path.to_path_buf() } else { first.join(path) };
        let resolved = canonical(&absolute).ok_or_else(|| Self::outside(shown))?;
        if self.dirs.iter().any(|r| resolved.starts_with(r)) { Ok(plain(resolved)) } else { Err(Self::outside(shown)) }
    }

    /// A project file to read: the file and, when it exists, its companion `<name>.opj.sheaf`.
    pub fn check_open(&self, text: &str) -> Result<PathBuf, String> {
        let path = self.check(text)?;
        let companion = with_suffix(&path, ".sheaf");
        if companion.exists() {
            self.check_path(&companion, &companion.display().to_string())?;
        }
        Ok(path)
    }

    /// A project file to write: the file (with `.opj` added to a name without extension), its
    /// companion and the `.bak` copy.
    pub fn check_save(&self, text: &str) -> Result<PathBuf, String> {
        let path = self.check(text)?;
        let target = sheaf_io::save::default_extension(&path);
        for extra in [with_suffix(&target, ".sheaf"), with_suffix(&target, ".bak")] {
            self.check_path(&extra, &extra.display().to_string()).map_err(|e| {
                if extra.symlink_metadata().is_ok_and(|m| m.file_type().is_symlink()) {
                    format!("the path is a symbolic link; remove it first: {}", extra.display())
                } else {
                    e
                }
            })?;
        }
        Ok(path)
    }
}

/// False for a path that names a network share (`\\host\share`, `//host/share`), a device
/// (`\\.\pipe\x`, `\\?\UNC\...`, `\\?\GLOBALROOT\...`) or a reserved Windows device name in any part
/// (`CON`, `NUL`, `COM1`, `LPT1.txt`, ...). Drive paths, `\\?\C:\` paths and relative paths pass.
fn local_name(path: &Path) -> bool {
    let text = path.to_string_lossy();
    let verbatim_drive = text
        .strip_prefix("\\\\?\\")
        .is_some_and(|rest| rest.len() > 2 && rest.as_bytes()[0].is_ascii_alphabetic() && rest.as_bytes()[1] == b':');
    let starts_double =
        text.starts_with("\\\\") || text.starts_with("//") || text.starts_with("\\/") || text.starts_with("/\\");
    if starts_double && !verbatim_drive {
        return false;
    }
    const DEVICES: [&str; 4] = ["CON", "PRN", "AUX", "NUL"];
    text.split(['/', '\\']).all(|part| {
        let stem = part.split('.').next().unwrap_or("").trim_end_matches([' ', ':']).to_ascii_uppercase();
        let numbered = (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.len() == 4
            && stem.as_bytes()[3].is_ascii_digit()
            && stem.as_bytes()[3] != b'0';
        !(DEVICES.contains(&stem.as_str()) || numbered || stem.contains("CONIN$") || stem.contains("CONOUT$"))
    })
}

/// `path` with `suffix` appended to the whole file name (`a.opj` to `a.opj.sheaf`).
pub fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(suffix);
    PathBuf::from(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Dir(PathBuf);
    impl Dir {
        fn new(tag: &str) -> Dir {
            let p = std::env::temp_dir().join(format!("sheaf-mcp-{tag}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&p);
            fs::create_dir_all(&p).unwrap();
            Dir(p)
        }
    }
    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn setup(tag: &str) -> (Dir, Roots) {
        let dir = Dir::new(tag);
        fs::create_dir_all(dir.0.join("data")).unwrap();
        fs::write(dir.0.join("data").join("a.csv"), "x\n1\n").unwrap();
        fs::write(dir.0.join("secret.txt"), "s").unwrap();
        let roots = Roots::new(&[dir.0.join("data")]).unwrap();
        (dir, roots)
    }

    #[test]
    fn paths_under_a_root_are_accepted() {
        let (dir, roots) = setup("ok");
        assert!(roots.check("a.csv").is_ok());
        assert!(roots.check(&dir.0.join("data").join("a.csv").display().to_string()).is_ok());
        // A file that does not exist yet: its parent is what counts.
        assert!(roots.check("new.csv").is_ok());
        assert!(roots.check("sub/new.csv").is_ok());
    }

    #[test]
    fn dot_dot_cannot_leave_the_root() {
        let (dir, roots) = setup("dots");
        let e = roots.check("../secret.txt").unwrap_err();
        assert!(e.starts_with("path is outside the allowed folders: "), "{e}");
        assert!(roots.check("../nothing-here.txt").is_err());
        assert!(roots.check("sub/../../secret.txt").is_err());
        assert!(roots.check("new/../../x.csv").is_err());
        let absolute = dir.0.join("secret.txt");
        assert!(roots.check(&absolute.display().to_string()).is_err());
        assert!(roots.check(&dir.0.join("data").join("..").join("secret.txt").display().to_string()).is_err());
    }

    #[test]
    fn a_sibling_with_the_same_prefix_is_outside() {
        let (dir, roots) = setup("prefix");
        fs::create_dir_all(dir.0.join("data2")).unwrap();
        assert!(roots.check(&dir.0.join("data2").join("x.csv").display().to_string()).is_err());
    }

    #[test]
    fn nothing_is_allowed_without_a_root() {
        let roots = Roots::new(&[]).unwrap();
        assert!(roots.is_empty());
        assert!(roots.check("a.csv").unwrap_err().contains("--root"));
    }

    #[test]
    fn saving_checks_the_companion_and_the_backup_too() {
        let (_dir, roots) = setup("save");
        assert!(roots.check_save("p").is_ok());
        assert!(roots.check_save("p.opj").is_ok());
        assert!(roots.check_save("../p.opj").is_err());
    }

    #[test]
    fn shares_and_devices_are_refused_without_being_opened() {
        let (_dir, roots) = setup("devices");
        for bad in [
            "\\\\host\\share\\a.csv",
            "//host/share/a.csv",
            "\\\\.\\pipe\\x",
            "\\\\.\\COM1",
            "\\\\?\\UNC\\host\\share\\a.csv",
            "\\\\?\\GLOBALROOT\\Device\\x",
            "NUL",
            "sub/con.txt",
            "COM3",
            "lpt1.csv",
            "CONIN$",
        ] {
            assert!(roots.check(bad).is_err(), "{bad}");
        }
        // Names that only look like devices are fine.
        for good in ["console.csv", "com10.csv", "com0.csv", "nul2.txt", "data.csv"] {
            assert!(roots.check(good).is_ok(), "{good}");
        }
    }

    #[test]
    fn roots_from_a_client_must_be_absolute() {
        let (dir, _) = setup("absolute");
        let data = dir.0.join("data");
        assert_eq!(Roots::from_absolute(std::slice::from_ref(&data)).unwrap().dirs().len(), 1);
        let e = Roots::from_absolute(&[PathBuf::from("data")]).unwrap_err();
        assert!(e.contains("not an absolute path"), "{e}");
        assert!(Roots::from_absolute(&[]).unwrap().is_empty());
        assert!(Roots::from_absolute(&[dir.0.join("missing")]).is_err());
    }

    #[test]
    fn missing_roots_are_refused() {
        assert!(Roots::new(&[PathBuf::from("definitely/not/here")]).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn a_link_that_leaves_the_root_is_followed_and_refused() {
        let (dir, roots) = setup("link");
        std::os::unix::fs::symlink(dir.0.join("secret.txt"), dir.0.join("data").join("link.txt")).unwrap();
        assert!(roots.check("link.txt").is_err());
        std::os::unix::fs::symlink(&dir.0, dir.0.join("data").join("up")).unwrap();
        assert!(roots.check("up/secret.txt").is_err());
        // A link to nowhere cannot be proven safe to write through.
        std::os::unix::fs::symlink(dir.0.join("gone"), dir.0.join("data").join("dangling")).unwrap();
        assert!(roots.check("dangling").is_err());
    }
}
