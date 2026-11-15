//! The one executor of the tools. The headless server and the desktop app both run a call through
//! [`run_on_document`], so a result cannot differ between them (docs/spec/mcp.md section 11.10).

use crate::files::{self, Protected};
use crate::link::sanitize;
use crate::roots::Roots;
use crate::schemas;
use crate::tools::{self, Res, ToolError};
use anyhow::Context;
use serde_json::Value;
use sheaf_core::template::Template;
use sheaf_core::{Document, Project};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Most mutating calls one attached connection may make.
pub const MAX_CHANGES: u32 = 1000;

/// What a client of the app is told when it calls a tool of the headless server.
pub const NOT_ATTACHED: &str = "not available while attached: the person using Sheaf opens and saves projects";

/// Longest undo step name, in characters.
pub const LABEL_CHARS: usize = 60;

/// What one connection (or the one headless session) remembers between calls.
#[derive(Clone, Debug, Default)]
pub struct Session {
    /// The file the project was opened from or last saved to (headless only).
    pub path: Option<PathBuf>,
    /// Origin projects this session read: saving over one keeps a `.bak` copy first (headless only).
    pub foreign: Vec<PathBuf>,
    /// The template each imported book suggested, by lower-case book name.
    pub suggested: HashMap<String, Template>,
    /// Mutating calls that started on this session; a failed call still counts.
    mutating_calls: u32,
}

impl Session {
    pub fn mutating_calls(&self) -> u32 {
        self.mutating_calls
    }
}

/// What a caller may do: its folders, whether it is attached to the app, and the files it must leave
/// alone.
#[derive(Clone, Debug, Default)]
pub struct Policy {
    pub roots: Roots,
    /// True for a client of the desktop app: the project is the owner's, files are never replaced and
    /// the tools that open, replace or save the project are refused.
    pub attached: bool,
    /// The most mutating calls the session may make; `None` for no limit.
    pub change_cap: Option<u32>,
    /// Names that a new file may not take (attached only). The app computes it for each call from the
    /// live document with [`files::protected_set`].
    pub protected: Protected,
}

impl Policy {
    /// The headless server: replaces files as phase 1 always did, no limit on calls.
    pub fn headless(roots: Roots) -> Policy {
        Policy { roots, ..Policy::default() }
    }

    /// A client of the desktop app. `roots` come from the client and must be absolute.
    pub fn attached(roots: Roots) -> Policy {
        Policy { roots, attached: true, change_cap: Some(MAX_CHANGES), protected: Protected::default() }
    }

    /// Before the work of a call: fails when `path` could not be written. Attached it must be a new file
    /// with a name that does not belong to the project; headless any path under a root will do.
    pub fn check_new_file(&self, path: &Path) -> Result<(), ToolError> {
        if self.attached {
            files::check_new(path, &self.protected)?;
        }
        Ok(())
    }

    /// Writes `bytes` to `path`, which a root check has passed. Attached, the file must be new.
    pub fn write_bytes(&self, path: &Path, bytes: &[u8]) -> Result<(), ToolError> {
        if self.attached {
            files::write_new_bytes(path, bytes, &self.protected)?;
        } else {
            std::fs::write(path, bytes).with_context(|| format!("could not write '{}'", path.display()))?;
        }
        Ok(())
    }

    /// Runs a writer that takes a path. Attached, the writer gets a temporary file next to `path` that is
    /// published only if `path` is still free; the writer must not remember the path it was given.
    pub fn write_with<T>(&self, path: &Path, write: impl FnOnce(&Path) -> anyhow::Result<T>) -> Result<T, ToolError> {
        if self.attached { Ok(files::write_new_with(path, &self.protected, write)?) } else { Ok(write(path)?) }
    }
}

/// What a tool works on: the project (a copy or the live one, as the executor decides), the session and
/// the policy of the caller.
pub struct Ctx<'a> {
    pub project: &'a mut Project,
    pub session: &'a mut Session,
    pub policy: &'a Policy,
}

const PANICKED: &str = "internal error: the tool failed unexpectedly; the project is unchanged";

fn guarded(run: impl FnOnce() -> Res) -> Res {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(run)).unwrap_or_else(|_| Err(ToolError::new(PANICKED)))
}

/// The tools that replace the whole project: they go through `Document::replace`, never `edit`.
fn replaces(name: &str) -> bool {
    matches!(name, "new_project" | "open_project")
}

/// Runs one tool call on `doc`.
///
/// - A tool that `mutates` runs inside `Document::edit`: one undo step named by [`undo_label`], formula
///   columns and analysis links brought up to date, nothing at all when the call fails, panics or
///   changes nothing. The session is put back as it was on failure.
/// - `new_project` and `open_project` build a project on a copy and swap it in with `Document::replace`:
///   no recalculation, the history is cleared.
/// - Every other tool runs on a copy that is thrown away.
/// - Attached, the tools that are `headless_only` are refused, and a session may make only
///   `Policy::change_cap` mutating calls.
pub fn run_on_document(doc: &mut Document, session: &mut Session, policy: &Policy, name: &str, args: &Value) -> Res {
    run_with(doc, session, policy, name, args, tools::call)
}

fn run_with(
    doc: &mut Document,
    session: &mut Session,
    policy: &Policy,
    name: &str,
    args: &Value,
    call: impl FnOnce(&mut Ctx, &str, &Value) -> Res,
) -> Res {
    let Some(def) = schemas::defs().into_iter().find(|d| d.name == name) else {
        return Err(ToolError::new(format!("unknown tool: {name}")));
    };
    if policy.attached && def.headless_only {
        return Err(ToolError::new(NOT_ATTACHED));
    }
    if def.mutates
        && let Some(cap) = policy.change_cap
        && session.mutating_calls >= cap
    {
        return Err(ToolError::new(format!(
            "this connection has made {cap} changes; ask the owner to turn the link off and on, or restart the assistant"
        )));
    }
    let before = session.clone();
    let restore = |session: &mut Session, mut before: Session| {
        // A call that started counts, whatever its result.
        before.mutating_calls = session.mutating_calls;
        *session = before;
    };
    if def.mutates {
        session.mutating_calls += 1;
        let label = undo_label(name, args, doc.project());
        // `Document::edit` puts the project back when the call or the recalculation after it panics; the
        // panic ends here, so the session is put back too.
        let edit = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            doc.edit(&label, |project| {
                let mut ctx = Ctx { project, session: &mut *session, policy };
                guarded(|| call(&mut ctx, name, args))
            })
        }));
        let result = edit.unwrap_or_else(|_| Err(ToolError::new(PANICKED)));
        if result.is_err() {
            restore(session, before);
        }
        return result;
    }
    let mut copy = doc.project().clone();
    let result = guarded(|| call(&mut Ctx { project: &mut copy, session: &mut *session, policy }, name, args));
    match &result {
        Ok(_) if replaces(name) => doc.replace(copy),
        Ok(_) => {}
        Err(_) => restore(session, before),
    }
    result
}

// ----- undo step names ---------------------------------------------------------------

fn text<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key).and_then(Value::as_str).map(str::trim).filter(|t| !t.is_empty())
}

/// A list of names given as a string or an array of strings, at the top of the arguments or, failing
/// that, in `options`.
fn names(args: &Value, key: &str) -> Vec<String> {
    let find = |v: &Value| match v.get(key) {
        Some(Value::String(t)) if !t.trim().is_empty() => Some(vec![t.trim().to_string()]),
        Some(Value::Array(a)) => {
            let list: Vec<String> =
                a.iter().filter_map(Value::as_str).map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect();
            (!list.is_empty()).then_some(list)
        }
        _ => None,
    };
    find(args).or_else(|| args.get("options").and_then(find)).unwrap_or_default()
}

fn option_text<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    text(args, key).or_else(|| args.get("options").and_then(|o| text(o, key)))
}

/// The column as `B(Y)`: its short name and designation as the project says; a name the project does not
/// know is shown as it was given.
fn column_label(project: &Project, book: Option<&str>, sheet: Option<&str>, given: &str) -> String {
    let found = tools::locate_sheet(project, book, sheet).ok().and_then(|(b, i)| {
        let ws = project.workbook(b)?.sheets.get(i)?;
        let given = given.trim();
        ws.columns
            .iter()
            .find(|c| c.short_name.eq_ignore_ascii_case(given))
            .or_else(|| ws.columns.iter().find(|c| c.long_name == given))
    });
    match found {
        Some(c) => format!("{}({})", c.short_name, tools::designation_name(c.designation)),
        None => given.to_string(),
    }
}

/// The name of the undo step for a call, built from the tool, its arguments and the live project:
/// `AI: ` and the tool, then (as they apply) a model or analysis name, the book, the columns as `B(Y)`
/// (at most three, then `+N`), the formula of `set_column_values` or the first file of `import_files`
/// with `+N`. Examples: `AI: plot Book1 B(Y)`, `AI: fit Gauss Book1 B(Y)`,
/// `AI: set_column_values Book1 C = A*2`, `AI: import_files run1.csv +2`. The text came over the wire,
/// so it is sanitized and cut to 60 characters.
pub fn undo_label(tool: &str, args: &Value, project: &Project) -> String {
    let mut parts: Vec<String> = vec![format!("AI: {tool}")];
    let book = text(args, "book").or_else(|| option_text(args, "book"));
    let book_shown = || match book {
        Some(_) => tools::book_name(project, book)
            .ok()
            .flatten()
            .map(|n| project.window_by_name(&n).map_or(n, |w| w.name().to_string())),
        None => project.workbooks().next().map(|b| b.name.clone()),
    };
    let sheet = option_text(args, "sheet");
    let columns = |list: Vec<String>| -> Option<String> {
        if list.is_empty() {
            return None;
        }
        let mut shown: Vec<String> = list.iter().take(3).map(|c| column_label(project, book, sheet, c)).collect();
        if list.len() > 3 {
            shown.push(format!("+{}", list.len() - 3));
        }
        Some(shown.join(" "))
    };
    match tool {
        "import_files" => {
            let files = names(args, "files");
            if let Some(first) = files.first() {
                let base = first.rsplit(['/', '\\']).next().unwrap_or(first).to_string();
                parts.push(base);
                if files.len() > 1 {
                    parts.push(format!("+{}", files.len() - 1));
                }
            }
        }
        "set_column_values" => {
            parts.extend(book_shown());
            if let Some(c) = text(args, "column") {
                let formula = text(args, "formula").unwrap_or_default();
                parts.push(format!("{c} = {formula}"));
            }
        }
        "plot" => {
            if let Some(g) = text(args, "graph") {
                parts.push(g.to_string());
            } else {
                parts.extend(book_shown());
                let mut y = names(args, "y");
                y.extend(names(args, "y2"));
                parts.extend(columns(y));
            }
        }
        "analyze" => {
            parts.extend(text(args, "tool").map(str::to_string));
            parts.extend(book_shown());
            parts.extend(columns(names(args, "columns")));
        }
        "fit" => {
            parts.extend(option_text(args, "model").map(str::to_string));
            parts.extend(book_shown());
            parts.extend(columns(names(args, "y")));
        }
        "simulate_curve" => {
            parts.extend(text(args, "function").map(str::to_string).or_else(|| Some("Gauss".to_string())));
            parts.extend(book_shown());
        }
        "sort_sheet" => {
            parts.extend(book_shown());
            parts.extend(columns(names(args, "key")));
        }
        "filter_sheet" => {
            parts.extend(book_shown());
            parts.extend(text(args, "condition").map(str::to_string));
        }
        _ => {}
    }
    sanitize(&parts.join(" "), LABEL_CHARS)
}

#[cfg(test)]
mod tests;
