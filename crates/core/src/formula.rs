//! Column formulas ("set column values"): fill a column from an expression
//! over the other columns of its sheet, for example `col(B)*1000`.
//!
//! The expression language is in the `sheaf-expr` crate and described in
//! `docs/spec/expression-language.md`. This module connects it to worksheets.

use crate::column::{Column, ColumnData, FormulaState, RecalcMode};
use crate::ids::ColumnId;
use crate::project::Project;
use crate::worksheet::{Workbook, Worksheet};
use sheaf_expr::{CellReads, ColumnLabel, ColumnRef, Expr, ExprError, Host, Value, WorksheetRef};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::ops::Range;

/// The most rows a formula may fill; a larger end row is refused instead of allocating that much.
pub const MAX_FORMULA_ROWS: usize = 10_000_000;

#[derive(Debug, thiserror::Error)]
pub enum FormulaError {
    #[error("there is no column with id {0:?}")]
    NoSuchColumn(ColumnId),
    #[error("the sheet has no rows yet; give a row range or enter data first")]
    NoRows,
    #[error("row {0} is beyond the limit of {MAX_FORMULA_ROWS} rows for a formula")]
    TooManyRows(usize),
    #[error(
        "column {0} holds text outside the chosen rows, and a column is either all text or all \
         numbers; set values on all its rows to make it numeric, or use another column"
    )]
    TextOutsideRange(String),
    #[error(
        "this formula would make the column depend on itself ({0}); a column that recalculates \
         cannot read a column that reads it"
    )]
    Cycle(String),
    #[error("column {0} has no formula that recalculates")]
    NotRecalculating(String),
    #[error("{0}")]
    Broken(String),
    #[error("{0}")]
    Expr(#[from] ExprError),
}

/// Reads local columns and explicit references to other worksheets in this project.
///
/// `col(A)` finds a column by short name (ignoring case) and, if there is
/// none, by long name; `col("Long Name")` by long name; `col(3)` by position.
/// Text columns read as missing values.
struct SheetHost<'a> {
    sheet: &'a Worksheet,
    book: Option<&'a Workbook>,
    project: Option<&'a Project>,
}

impl<'a> SheetHost<'a> {
    fn at(project: &'a Project, column: ColumnId) -> Option<Self> {
        let (book, sheet, _) = project.locate(column)?;
        Some(Self { sheet, book: Some(book), project: Some(project) })
    }

    fn source<'r>(&self, r: &'r ColumnRef) -> Option<(&'a Worksheet, &'r ColumnRef)> {
        let ColumnRef::Scoped { book, sheet, column } = r else {
            return Some((self.sheet, r));
        };
        let book = match book {
            Some(name) => {
                let project = self.project?;
                if let Some(book) = project.workbooks().find(|b| b.name.eq_ignore_ascii_case(name)) {
                    book
                } else {
                    let mut titles = project.workbooks().filter(|b| b.long_name.eq_ignore_ascii_case(name));
                    let book = titles.next()?;
                    if titles.next().is_some() {
                        return None;
                    }
                    book
                }
            }
            None => self.book?,
        };
        let sheet = match sheet {
            WorksheetRef::Name(name) => book.sheets.iter().find(|s| s.name.eq_ignore_ascii_case(name))?,
            WorksheetRef::Index(index) => index.checked_sub(1).and_then(|i| book.sheets.get(i))?,
        };
        Some((sheet, column))
    }

    fn find(&self, r: &ColumnRef) -> Option<&'a Column> {
        let (sheet, r) = self.source(r)?;
        let cols = &sheet.columns;
        match r {
            ColumnRef::Short(name) => cols
                .iter()
                .find(|c| c.short_name.eq_ignore_ascii_case(name))
                .or_else(|| cols.iter().find(|c| c.long_name == *name)),
            ColumnRef::Long(name) => cols.iter().find(|c| c.long_name == *name),
            ColumnRef::Index(n) => n.checked_sub(1).and_then(|i| cols.get(i)),
            ColumnRef::Scoped { .. } => None,
        }
    }
}

impl Host for SheetHost<'_> {
    fn column_parameter(&self, col: &ColumnRef, name: &str) -> Option<String> {
        let column = self.find(col)?;
        let names = self.source(col)?.0.parameter_names();
        let key = names
            .iter()
            .find(|k| k.as_str() == name)
            .or_else(|| names.iter().find(|k| k.eq_ignore_ascii_case(name)))
            .or_else(|| {
                let ordinal = name.strip_prefix(['D', 'd'])?;
                if ordinal.is_empty() || !ordinal.bytes().all(|c| c.is_ascii_digit()) {
                    return None;
                }
                let index: usize = ordinal.parse().ok()?;
                index.checked_sub(1).and_then(|i| names.get(i))
            })?;
        Some(column.params.get(key).cloned().unwrap_or_default())
    }

    fn formatted_numeric_cell(&self, col: &ColumnRef, row: usize) -> Option<String> {
        let c = self.find(col)?;
        let value = c.numeric().and_then(|v| v.get(row)).copied().unwrap_or(f64::NAN);
        if value.is_nan() {
            return Some("--".into());
        }
        // General keeps the expression formatter's measured general-number behavior.
        (c.format != crate::format::NumberFormat::General).then(|| c.format.format(value))
    }

    fn column_label(&self, col: &ColumnRef, label: ColumnLabel) -> Option<String> {
        let c = self.find(col)?;
        Some(match label {
            ColumnLabel::ShortName => c.short_name.clone(),
            ColumnLabel::LongName => c.long_name.clone(),
            ColumnLabel::Units => c.units.clone(),
            ColumnLabel::Comments => c.comments.clone(),
            ColumnLabel::Formula => c.formula.clone().unwrap_or_default(),
        })
    }

    fn variable(&self, _name: &str) -> Option<f64> {
        None
    }

    fn cell(&self, col: &ColumnRef, row: usize) -> Option<f64> {
        let c = self.find(col)?;
        match c.numeric() {
            Some(v) => v.get(row).copied(),
            None => (row < c.len()).then_some(f64::NAN),
        }
    }

    fn has_column(&self, col: &ColumnRef) -> bool {
        self.find(col).is_some()
    }

    fn column_data(&self, col: &ColumnRef) -> Option<&[f64]> {
        self.find(col)?.numeric()
    }

    fn column_len(&self, col: &ColumnRef) -> usize {
        self.find(col).map_or(0, Column::len)
    }

    fn text_cell(&self, col: &ColumnRef, row: usize) -> Option<String> {
        let c = self.find(col)?;
        match &c.data {
            ColumnData::Text(v) => v.get(row).cloned(),
            data => data.text_cell(row).map(str::to_string),
        }
    }

    fn column_has_text(&self, col: &ColumnRef) -> bool {
        self.find(col).is_some_and(|c| match &c.data {
            ColumnData::Text(_) => true,
            ColumnData::Mixed { texts, .. } => !texts.is_empty(),
            ColumnData::Numeric(_) => false,
        })
    }

    /// `now()` and `today()` are in local time, as in Origin.
    fn now(&self) -> f64 {
        let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0.0, |d| d.as_secs_f64());
        (secs + crate::localtime::local_utc_offset_seconds() as f64) / 86_400.0 + crate::datetime::UNIX_EPOCH_VALUE
    }
}

/// Evaluates an expression for the first `rows` rows of a sheet.
pub(crate) fn eval_on_sheet(expr: &Expr, sheet: &Worksheet, rows: usize) -> Result<Vec<f64>, ExprError> {
    expr.eval_rows(&SheetHost { sheet, book: None, project: None }, rows)
}

/// Cell reads determine calculation order; header reads only invalidate a result.
#[derive(PartialEq, Eq)]
struct FormulaDeps {
    data: Vec<ColumnId>,
    labels: Vec<ColumnId>,
    parameters: Vec<ColumnId>,
}

/// Resolves cell and header sources, or returns the first reference that matches no column.
/// A computed `cell(r,c)` may read any other column of the sheet.
fn resolve_deps(expr: &Expr, host: &SheetHost<'_>, me: ColumnId) -> Result<FormulaDeps, ColumnRef> {
    let sheet = host.sheet;
    let expr = expr.resolve_parameters(host);
    let mut out = Vec::new();
    for r in expr.data_columns() {
        let id = host.find(&r).ok_or(r)?.id;
        if !out.contains(&id) {
            out.push(id);
        }
    }
    match expr.cell_reads() {
        CellReads::None => {}
        CellReads::Columns(positions) => {
            for p in positions {
                if let Some(c) = sheet.columns.get(p - 1)
                    && !out.contains(&c.id)
                {
                    out.push(c.id);
                }
            }
        }
        CellReads::Any => {
            for c in sheet.columns.iter().filter(|c| c.id != me) {
                if !out.contains(&c.id) {
                    out.push(c.id);
                }
            }
        }
    }
    let mut labels = Vec::new();
    for r in expr.label_columns() {
        let id = host.find(&r).ok_or(r)?.id;
        if !labels.contains(&id) {
            labels.push(id);
        }
    }
    let parameters = expr.parameter_columns().iter().filter_map(|r| host.find(r).map(|c| c.id)).collect();
    Ok(FormulaDeps { data: out, labels, parameters })
}

/// The new data of `target` after its formula filled `range`. Rows outside the range keep their
/// values. Checks the limits and the text rule of [`Project::set_column_values`].
fn compute_fill(
    host: &SheetHost<'_>,
    target: &Column,
    expr: &Expr,
    range: Range<usize>,
) -> Result<ColumnData, FormulaError> {
    if range.is_empty() {
        return Err(FormulaError::NoRows);
    }
    if range.end > MAX_FORMULA_ROWS {
        return Err(FormulaError::TooManyRows(range.end));
    }
    let computed = if expr.involves_text(host) {
        // A formula with text may give text: such a result is a Text & Numeric column.
        let values = expr.eval_values(host, range.end)?;
        if values[range.clone()].iter().any(|v| matches!(v, Value::Text(t) if !t.is_empty())) {
            return Ok(fill_with_text(target, &values, range));
        }
        values.iter().map(Value::number).collect()
    } else {
        expr.eval_rows(host, range.end)?
    };
    if let Some(text) = target.text() {
        let outside = text.iter().enumerate().any(|(i, t)| !range.contains(&i) && !t.trim().is_empty());
        if outside {
            return Err(FormulaError::TextOutsideRange(target.short_name.clone()));
        }
    }
    if range.start == 0 && range.end >= target.len() {
        return Ok(filled(target, computed, None));
    }
    let mut values: Vec<f64> = target.numeric().map(<[f64]>::to_vec).unwrap_or_default();
    if values.len() < range.end {
        values.resize(range.end, f64::NAN);
    }
    values[range.clone()].copy_from_slice(&computed[range.clone()]);
    Ok(filled(target, values, Some(range)))
}

/// The column after a formula gave text in some rows: numbers and texts side by side (Origin's
/// Text & Numeric format). Rows outside `range` keep their numbers and texts.
fn fill_with_text(target: &Column, values: &[Value], range: Range<usize>) -> ColumnData {
    let mut numbers: Vec<f64> = target.numeric().map(<[f64]>::to_vec).unwrap_or_default();
    if numbers.len() < range.end {
        numbers.resize(range.end, f64::NAN);
    }
    let mut texts: BTreeMap<usize, String> = match &target.data {
        ColumnData::Mixed { texts, .. } => texts.iter().map(|(r, t)| (*r, t.clone())).collect(),
        ColumnData::Text(v) => {
            v.iter().enumerate().filter(|(_, t)| !t.is_empty()).map(|(r, t)| (r, t.clone())).collect()
        }
        ColumnData::Numeric(_) => BTreeMap::new(),
    };
    for r in range.clone() {
        texts.remove(&r);
        match &values[r] {
            Value::Num(x) => numbers[r] = *x,
            Value::Text(t) => {
                numbers[r] = f64::NAN;
                texts.insert(r, t.clone());
            }
        }
    }
    ColumnData::mixed(numbers, texts)
}

/// The column after a fill: a Text & Numeric column stays one and keeps the text cells outside
/// the filled rows (Origin's Set Column Values leaves the other cells alone); a Numeric column
/// stays Numeric.
fn filled(target: &Column, values: Vec<f64>, range: Option<Range<usize>>) -> ColumnData {
    let ColumnData::Mixed { texts, .. } = &target.data else {
        return ColumnData::numeric(values);
    };
    let kept = texts.iter().filter(|(r, _)| range.as_ref().is_some_and(|g| !g.contains(*r)));
    ColumnData::mixed(values, kept.map(|(r, t)| (*r, t.clone())).collect())
}

/// Looks for a chain of recalculating columns that leads from the columns `deps` back to
/// `target`. Returns the chain, starting and ending with `target`.
fn find_cycle(project: &Project, target: ColumnId, deps: &[ColumnId]) -> Option<Vec<ColumnId>> {
    let mut parent: HashMap<ColumnId, ColumnId> = HashMap::new();
    let mut queue = VecDeque::new();
    for d in deps {
        if *d == target {
            return Some(vec![target, target]);
        }
        if let std::collections::hash_map::Entry::Vacant(entry) = parent.entry(*d) {
            entry.insert(target);
            queue.push_back(*d);
        }
    }
    while let Some(id) = queue.pop_front() {
        let Some(c) = project.column(id).filter(|c| c.is_locked()) else {
            continue;
        };
        let Some(expr) = c.formula.as_deref().and_then(|f| Expr::parse(f).ok()) else {
            continue;
        };
        let host = SheetHost::at(project, id)?;
        let Ok(next) = resolve_deps(&expr, &host, id) else {
            continue;
        };
        for n in next.data {
            if n == target {
                let mut chain = vec![id];
                let mut cur = id;
                while let Some(&p) = parent.get(&cur) {
                    if p == target {
                        break;
                    }
                    chain.push(p);
                    cur = p;
                }
                chain.reverse();
                let mut path = vec![target];
                path.extend(chain);
                path.push(target);
                return Some(path);
            }
            if let std::collections::hash_map::Entry::Vacant(entry) = parent.entry(n) {
                entry.insert(id);
                queue.push_back(n);
            }
        }
    }
    None
}

/// A recalculating column that could not be brought up to date, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecalcProblem {
    pub column: ColumnId,
    pub message: String,
}

struct FormulaNode {
    id: ColumnId,
    deps: Result<FormulaDeps, String>,
    expr: Option<Expr>,
}

/// Brings the project's recalculating columns up to date after the columns in `changed` were
/// modified. `force` lists columns to recalculate even when they are Manual.
///
/// Columns run in dependency order: an Auto column runs when something it reads is in the dirty
/// set (the changed columns plus every column recalculated so far), a Manual column is marked
/// pending instead. A column that cannot run (a column it reads is missing, the formula does not
/// parse, a circular chain) is marked broken and keeps its old values.
fn recalc_project(
    project: &mut Project,
    changed: &HashSet<ColumnId>,
    force: &HashSet<ColumnId>,
    parameter_rows_changed: &HashSet<ColumnId>,
    bindings_changed: &HashSet<ColumnId>,
) -> Vec<RecalcProblem> {
    let mut problems = Vec::new();
    let nodes: Vec<FormulaNode> = project
        .workbooks()
        .flat_map(|b| &b.sheets)
        .flat_map(|s| &s.columns)
        .filter(|c| c.is_locked())
        .map(|c| {
            let source = c.formula.as_deref().unwrap_or_default();
            match Expr::parse(source) {
                Err(e) => FormulaNode { id: c.id, deps: Err(format!("the formula is not valid: {e}")), expr: None },
                Ok(expr) => {
                    let host = SheetHost::at(project, c.id).expect("column is in the project");
                    let deps = resolve_deps(&expr, &host, c.id)
                        .map_err(|r| format!("it reads {r}, which cannot be found in the project"));
                    FormulaNode { id: c.id, deps, expr: Some(expr) }
                }
            }
        })
        .collect();
    if nodes.is_empty() {
        return problems;
    }

    // Dependency order (Kahn), using cell data only: headers are scalars independent of their
    // column's formula result. Nodes left over take part in a circular data chain.
    let position: HashMap<ColumnId, usize> = nodes.iter().enumerate().map(|(i, n)| (n.id, i)).collect();
    let mut indegree = vec![0usize; nodes.len()];
    let mut readers: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
    for (j, n) in nodes.iter().enumerate() {
        if let Ok(deps) = &n.deps {
            for d in &deps.data {
                if let Some(&i) = position.get(d) {
                    indegree[j] += 1;
                    readers[i].push(j);
                }
            }
        }
    }
    let mut order: Vec<usize> = (0..nodes.len()).filter(|&j| indegree[j] == 0).collect();
    let mut head = 0;
    while head < order.len() {
        let i = order[head];
        head += 1;
        for &j in &readers[i] {
            indegree[j] -= 1;
            if indegree[j] == 0 {
                order.push(j);
            }
        }
    }
    let cyclic: Vec<usize> = (0..nodes.len()).filter(|&j| indegree[j] > 0).collect();

    let mut dirty = changed.clone();
    for &j in &order {
        let node = &nodes[j];
        let (mode, state, formula_rows) = {
            let c = project.column(node.id).expect("formula column is still present");
            (c.recalc, c.formula_state, c.formula_rows.clone())
        };
        let deps = match &node.deps {
            Ok(d) => d,
            Err(why) => {
                let c = project.column_mut(node.id).expect("formula column is still present");
                let short = c.short_name.clone();
                problems.push(RecalcProblem { column: node.id, message: format!("column {short}: {why}") });
                c.formula_state = FormulaState::Broken;
                continue;
            }
        };
        let forced = force.contains(&node.id);
        let needs = forced
            || state == FormulaState::Broken
            || bindings_changed.contains(&node.id)
            || deps.parameters.iter().any(|d| parameter_rows_changed.contains(d))
            || deps.data.iter().chain(&deps.labels).any(|d| dirty.contains(d));
        if !needs {
            continue;
        }
        if mode == RecalcMode::Manual && !forced {
            project.column_mut(node.id).expect("formula column is still present").formula_state = FormulaState::Pending;
            continue;
        }
        let derived = formula_rows.is_none();
        let range = formula_rows.unwrap_or_else(|| {
            // Headers are scalars, not data ranges. A header-only formula fills the current
            // worksheet rows; header reads do not lengthen a formula over actual data columns.
            let rows =
                deps.data.iter().filter_map(|d| project.column(*d)).map(Column::len).max().unwrap_or_else(|| {
                    if deps.labels.is_empty() {
                        0
                    } else {
                        project.locate(node.id).expect("formula column is still present").1.row_count()
                    }
                });
            0..rows
        });
        if range.is_empty() {
            let c = project.column_mut(node.id).expect("formula column is still present");
            // A formula over the whole column follows its sources, so when they are all empty
            // (for example after a shorter re-import) the results go too.
            if derived && !deps.data.is_empty() && !c.is_empty() {
                c.data = if c.data.is_mixed() { ColumnData::default() } else { ColumnData::numeric(Vec::new()) };
                dirty.insert(node.id);
            }
            c.formula_state = FormulaState::Current;
            continue;
        }
        let expr = node.expr.as_ref().expect("a node with resolved columns has a parsed formula");
        let range_end = range.end;
        let host = SheetHost::at(project, node.id).expect("formula column is still present");
        let target = project.column(node.id).expect("formula column is still present");
        match compute_fill(&host, target, expr, range) {
            Ok(mut data) => {
                let c = project.column_mut(node.id).expect("formula column is still present");
                if derived && data.len() > range_end {
                    // The sources got shorter: rows past their end no longer have results.
                    match &data {
                        ColumnData::Numeric(v) => data = ColumnData::numeric(v[..range_end].to_vec()),
                        ColumnData::Mixed { numbers, texts } => {
                            let texts = texts.iter().filter(|(r, _)| **r < range_end).map(|(r, t)| (*r, t.clone()));
                            data = ColumnData::mixed(numbers[..range_end].to_vec(), texts.collect());
                        }
                        ColumnData::Text(_) => {}
                    }
                }
                if c.data != data {
                    c.data = data;
                }
                c.formula_state = FormulaState::Current;
                dirty.insert(node.id);
            }
            Err(e) => {
                let c = project.column_mut(node.id).expect("formula column is still present");
                c.formula_state = FormulaState::Broken;
                problems.push(RecalcProblem { column: node.id, message: format!("column {}: {e}", c.short_name) });
            }
        }
    }
    for j in cyclic {
        let c = project.column_mut(nodes[j].id).expect("formula column is still present");
        c.formula_state = FormulaState::Broken;
        problems.push(RecalcProblem {
            column: c.id,
            message: format!("column {} is part of a circular chain of formulas", c.short_name),
        });
    }
    problems
}

impl Project {
    /// Fills a column from a formula and remembers the formula on the column. The formula runs
    /// once ([`RecalcMode::None`]); see [`Project::set_column_values_with`] for a column that
    /// follows its inputs.
    ///
    /// `rows` is the 0-based row range to fill; `None` fills every row of
    /// the sheet and any referenced remote columns (the longest of their lengths). Rows of the column
    /// outside the range keep their values. A text column becomes numeric,
    /// which is only allowed when no text lies outside the range (otherwise
    /// that text would be lost). On error nothing is changed.
    pub fn set_column_values(
        &mut self,
        column: ColumnId,
        formula: &str,
        rows: Option<Range<usize>>,
    ) -> Result<(), FormulaError> {
        self.set_column_values_with(column, formula, rows, RecalcMode::None)
    }

    /// Like [`Project::set_column_values`], with the Recalculate choice of Set Column Values.
    ///
    /// With `Auto` or `Manual` the column keeps the formula and the row range, and is locked
    /// against hand edits. A formula that reads its own column runs once, as mode `None` (what
    /// Origin does); one that closes a loop through other recalculating columns (B reads C and
    /// C reads B) is refused with the chain named. Columns that read this one are updated
    /// (Auto) or marked pending (Manual) before the call returns.
    pub fn set_column_values_with(
        &mut self,
        column: ColumnId,
        formula: &str,
        rows: Option<Range<usize>>,
        mode: RecalcMode,
    ) -> Result<(), FormulaError> {
        let expr = Expr::parse(formula)?;
        let (_, sheet, target) = self.locate(column).ok_or(FormulaError::NoSuchColumn(column))?;
        let host = SheetHost::at(self, column).expect("located above");
        let deps = resolve_deps(&expr, &host, column);
        if deps.is_err() {
            expr.validate_columns(&host)?;
        }
        let source_rows = deps
            .as_ref()
            .ok()
            .and_then(|d| d.data.iter().filter_map(|id| self.column(*id)).map(Column::len).max())
            .unwrap_or(0);
        let range = rows.clone().unwrap_or(0..sheet.row_count().max(source_rows));
        let mut mode = mode;
        if mode != RecalcMode::None
            && let Ok(deps) = deps
        {
            if deps.data.contains(&column) {
                // Origin documents this case: a formula that reads its own column runs once.
                mode = RecalcMode::None;
            } else if let Some(path) = find_cycle(self, column, &deps.data) {
                let across_sheets =
                    path.iter().any(|id| self.locate(*id).is_some_and(|(_, s, _)| !std::ptr::eq(s, sheet)));
                let names: Vec<String> = path
                    .iter()
                    .map(|id| {
                        self.locate(*id).map_or_else(
                            || "?".into(),
                            |(b, s, c)| {
                                if across_sheets {
                                    format!("[{}]{}!col({})", b.name, s.name, c.short_name)
                                } else {
                                    c.short_name.clone()
                                }
                            },
                        )
                    })
                    .collect();
                return Err(FormulaError::Cycle(names.join(" -> ")));
            }
        }
        let data = compute_fill(&host, target, &expr, range)?;

        let c = self.column_mut(column).expect("located above");
        c.data = data;
        c.formula = Some(formula.trim().to_string());
        c.recalc = mode;
        c.formula_state = FormulaState::Current;
        c.formula_rows = if mode == RecalcMode::None { None } else { rows };
        self.refresh_formulas(&[column]);
        Ok(())
    }

    /// Updates the recalculating columns that read any of the `changed` columns (directly or
    /// through other recalculating columns): Auto columns are recalculated, Manual ones are
    /// marked pending. Call this after changing column data outside a [`crate::Document`] edit,
    /// which does it by itself. Returns the columns that could not be updated.
    pub fn refresh_formulas(&mut self, changed: &[ColumnId]) -> Vec<RecalcProblem> {
        let changed: HashSet<ColumnId> = changed.iter().copied().collect();
        let none = HashSet::new();
        recalc_project(self, &changed, &none, &none, &none)
    }

    /// Recalculates a Manual (or Auto) column now, then the columns that read it. Clears the
    /// pending mark. Errors when the column has no recalculating formula or cannot run.
    pub fn recalculate(&mut self, column: ColumnId) -> Result<(), FormulaError> {
        let c = self.column(column).ok_or(FormulaError::NoSuchColumn(column))?;
        if !c.is_locked() {
            return Err(FormulaError::NotRecalculating(c.short_name.clone()));
        }
        let force = HashSet::from([column]);
        let problems = recalc_project(self, &HashSet::new(), &force, &HashSet::new(), &HashSet::new());
        match problems.into_iter().find(|p| p.column == column) {
            Some(p) => Err(FormulaError::Broken(p.message)),
            None => Ok(()),
        }
    }

    /// Recalculates every column of a sheet that is marked pending. Returns the problems.
    pub fn recalculate_pending(&mut self, book: crate::WindowId, sheet: usize) -> Vec<RecalcProblem> {
        let Some(ws) = self.workbook(book).and_then(|b| b.sheets.get(sheet)) else {
            return Vec::new();
        };
        let force: HashSet<ColumnId> =
            ws.columns.iter().filter(|c| c.formula_state == FormulaState::Pending).map(|c| c.id).collect();
        if force.is_empty() {
            return Vec::new();
        }
        recalc_project(self, &HashSet::new(), &force, &HashSet::new(), &HashSet::new())
    }

    /// Brings recalculating columns up to date after an edit that turned `before` into `self`:
    /// columns whose data, names, label metadata or formats differ are the changes. Does nothing when no column in the
    /// project recalculates. Used by [`crate::Document`], which calls it inside the undo step of
    /// the edit.
    pub fn recalculate_since(&mut self, before: &Project) -> Vec<RecalcProblem> {
        let any = self.workbooks().any(|b| b.sheets.iter().any(|s| s.columns.iter().any(Column::is_locked)));
        if !any {
            return Vec::new();
        }
        let old: HashMap<ColumnId, &Column> = before
            .workbooks()
            .flat_map(|b| b.sheets.iter().flat_map(|s| s.columns.iter()))
            .map(|c| (c.id, c))
            .collect();
        let mut changed = HashSet::new();
        let mut parameter_rows_changed = HashSet::new();
        let mut bindings_changed = HashSet::new();
        let layout_changed = reference_layout_changed(before, self);
        for b in self.workbooks() {
            for (sheet_index, s) in b.sheets.iter().enumerate() {
                if layout_changed {
                    for c in s.columns.iter().filter(|c| c.is_locked()) {
                        if let Ok(expr) = Expr::parse(c.formula.as_deref().unwrap_or_default()) {
                            let previous =
                                SheetHost::at(before, c.id).and_then(|host| resolve_deps(&expr, &host, c.id).ok());
                            let current =
                                SheetHost::at(self, c.id).and_then(|host| resolve_deps(&expr, &host, c.id).ok());
                            if previous != current {
                                bindings_changed.insert(c.id);
                            }
                        }
                    }
                }
                // Dn aliases follow the displayed row order. Adding/removing a parameter
                // on any column can change the row named by an alias on another column.
                let old_names = before
                    .workbook(b.id)
                    .and_then(|book| book.sheets.get(sheet_index))
                    .map(Worksheet::parameter_names)
                    .unwrap_or_default();
                let new_names = s.parameter_names();
                if old_names != new_names {
                    parameter_rows_changed.extend(s.columns.iter().map(|c| c.id));
                }
                changed.extend(
                    s.columns
                        .iter()
                        .filter(|c| {
                            old.get(&c.id).is_none_or(|o| {
                                o.data != c.data
                                    || o.short_name != c.short_name
                                    || o.long_name != c.long_name
                                    || o.units != c.units
                                    || o.comments != c.comments
                                    || o.formula != c.formula
                                    || o.format != c.format
                                    || o.params != c.params
                            })
                        })
                        .map(|c| c.id),
                );
            }
        }
        recalc_project(self, &changed, &HashSet::new(), &parameter_rows_changed, &bindings_changed)
    }
}

/// Positional references and names are resolved anew after changes to the book/sheet/column
/// layout. Data alone cannot reveal that an unchanged column has become a different source.
fn reference_layout_changed(before: &Project, after: &Project) -> bool {
    if before.workbooks().count() != after.workbooks().count() {
        return true;
    }
    before.workbooks().zip(after.workbooks()).any(|(a, b)| {
        a.id != b.id
            || a.name != b.name
            || a.long_name != b.long_name
            || a.sheets.len() != b.sheets.len()
            || a.sheets.iter().zip(&b.sheets).any(|(a, b)| {
                a.name != b.name
                    || a.columns
                        .iter()
                        .map(|c| (c.id, &c.short_name, &c.long_name))
                        .ne(b.columns.iter().map(|c| (c.id, &c.short_name, &c.long_name)))
            })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::Designation;

    fn sample() -> (Project, ColumnId, ColumnId, ColumnId) {
        let mut p = Project::new();
        let book = p.add_workbook("");
        let a = p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0, 3.0, 4.0])).unwrap();
        let b = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![0.5, 0.25, f64::NAN])).unwrap();
        let c = p.add_column(book, 0, Designation::Y, ColumnData::numeric(Vec::new())).unwrap();
        p.column_mut(b).unwrap().long_name = "Current".into();
        (p, a, b, c)
    }

    #[test]
    fn an_absurd_end_row_is_refused_without_allocating() {
        let (mut p, _, _, c) = sample();
        let err = p.set_column_values(c, "i", Some(0..usize::MAX)).unwrap_err();
        assert!(matches!(err, FormulaError::TooManyRows(_)));
        assert!(p.numeric(c).unwrap().is_empty());
    }

    #[test]
    fn text_outside_the_rows_is_never_thrown_away() {
        let mut p = Project::new();
        let book = p.add_workbook("");
        let names = vec!["S1".to_string(), "S2".to_string(), "S3".to_string(), String::new()];
        let t = p.add_column(book, 0, Designation::Y, ColumnData::text(names.clone())).unwrap();
        // Rows 2 and 3 only: S1 would be lost, so nothing changes.
        let err = p.set_column_values(t, "i", Some(1..3)).unwrap_err();
        assert!(matches!(err, FormulaError::TextOutsideRange(_)), "{err}");
        assert_eq!(p.column(t).unwrap().text().unwrap(), &names[..]);
        // All rows that hold text are inside the range (the last one is empty): allowed.
        p.set_column_values(t, "i * 10", Some(0..3)).unwrap();
        assert_eq!(&p.numeric(t).unwrap()[..3], &[10.0, 20.0, 30.0]);
    }

    #[test]
    fn the_row_limit_is_the_exact_boundary() {
        let (mut p, _, _, c) = sample();
        let err = p.set_column_values(c, "1", Some(0..MAX_FORMULA_ROWS + 1)).unwrap_err();
        assert!(matches!(err, FormulaError::TooManyRows(n) if n == MAX_FORMULA_ROWS + 1));
        // A range of a few rows is still fine.
        p.set_column_values(c, "1", Some(0..3)).unwrap();
        assert_eq!(p.numeric(c).unwrap()[..3], [1.0, 1.0, 1.0]);
    }

    #[test]
    fn fills_every_row_of_the_sheet_and_keeps_the_formula() {
        let (mut p, _, _, c) = sample();
        p.set_column_values(c, " col(a)*1000 + i ", None).unwrap();
        assert_eq!(p.numeric(c).unwrap(), &[1001.0, 2002.0, 3003.0, 4004.0]);
        assert_eq!(p.column(c).unwrap().formula.as_deref(), Some("col(a)*1000 + i"));
    }

    #[test]
    fn finds_columns_by_long_name_and_position_and_passes_missing_values_on() {
        let (mut p, _, _, c) = sample();
        p.set_column_values(c, "col(\"Current\")*2 + col(Current)*0 + col(1)*0", None).unwrap();
        let v = p.numeric(c).unwrap();
        assert_eq!(&v[..2], &[1.0, 0.5]);
        assert!(v[2].is_nan(), "a missing input gives a missing result");
        assert!(v[3].is_nan(), "a row past the end of the input is missing");
    }

    #[test]
    fn now_and_today_are_in_local_time() {
        let (mut p, _, _, c) = sample();
        let utc = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64();
        let want = (utc + crate::localtime::local_utc_offset_seconds() as f64) / 86_400.0 + 2_440_587.0;
        p.set_column_values(c, "now()", None).unwrap();
        let got = p.numeric(c).unwrap()[0];
        assert!((got - want).abs() < 60.0 / 86_400.0, "{got} vs {want}");
        p.set_column_values(c, "today()", None).unwrap();
        let today = p.numeric(c).unwrap()[0];
        assert_eq!(today.fract(), 0.0);
        assert!([-60.0, 0.0, 60.0].iter().any(|d| (want + d / 86_400.0).floor() == today), "{today}");
    }

    #[test]
    fn a_row_range_leaves_other_rows_alone() {
        let (mut p, _, b, _) = sample();
        p.set_column_values(b, "col(A)*10", Some(1..3)).unwrap();
        assert_eq!(p.numeric(b).unwrap(), &[0.5, 20.0, 30.0]);
        // A range past the end grows the column with missing values in between.
        p.set_column_values(b, "7", Some(5..6)).unwrap();
        let v = p.numeric(b).unwrap();
        assert_eq!((v.len(), v[5]), (6, 7.0));
        assert!(v[3].is_nan() && v[4].is_nan());
    }

    #[test]
    fn a_column_can_refer_to_itself_and_text_becomes_numeric() {
        let (mut p, a, _, _) = sample();
        p.set_column_values(a, "col(A)^2", None).unwrap();
        assert_eq!(p.numeric(a).unwrap(), &[1.0, 4.0, 9.0, 16.0]);
        let book = p.workbooks().next().unwrap().id;
        let t = p.add_column(book, 0, Designation::Y, ColumnData::text(vec!["x".into()])).unwrap();
        p.set_column_values(t, "i", None).unwrap();
        assert_eq!(p.numeric(t).unwrap(), &[1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn errors_change_nothing() {
        let (mut p, _, b, c) = sample();
        let before = p.clone();
        assert!(matches!(p.set_column_values(c, "col(Nope)+1", None), Err(FormulaError::Expr(_))));
        assert!(matches!(p.set_column_values(c, "1 +", None), Err(FormulaError::Expr(_))));
        assert!(matches!(p.set_column_values(ColumnId(999), "1", None), Err(FormulaError::NoSuchColumn(_))));
        assert_eq!(p, before);
        let mut empty = Project::new();
        let book = empty.add_workbook("");
        let e = empty.add_column(book, 0, Designation::Y, ColumnData::numeric(Vec::new())).unwrap();
        assert!(matches!(empty.set_column_values(e, "1", None), Err(FormulaError::NoRows)));
        let _ = b;
    }
}
