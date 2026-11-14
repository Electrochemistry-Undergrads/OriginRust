//! The "Set Values" window (Origin's Set Column Values): fill a column from a formula over the
//! other columns of its sheet. Layout as Origin: a text menu line, the row range, the
//! `Col(X)=` label with column navigation, the formula box, Recalculate with OK / Cancel / Apply,
//! and a collapsible "Before Formula Scripts" box.

use eframe::egui::{self, RichText};
use sheaf_core::{ColumnId, Document, Project, RecalcMode};

/// One entry of the Function menu: the name, the arguments as typed, a one-line description in
/// Sheaf's own words, and an example that the tests evaluate.
pub struct FuncInfo {
    pub name: &'static str,
    pub args: &'static str,
    pub about: &'static str,
    pub example: &'static str,
}

const fn f(name: &'static str, args: &'static str, about: &'static str, example: &'static str) -> FuncInfo {
    FuncInfo { name, args, about, example }
}

/// The functions of `sheaf-expr`, grouped as the Function menu shows them.
pub const FUNCTIONS: &[(&str, &[FuncInfo])] = &[
    (
        "Math",
        &[
            f("abs", "x", "Absolute value.", "abs(-2)"),
            f("sqrt", "x", "Square root; negative input gives a missing value.", "sqrt(4)"),
            f("sqr", "x", "Square of x.", "sqr(3)"),
            f("exp", "x", "e raised to x.", "exp(1)"),
            f("ln", "x", "Natural logarithm.", "ln(2)"),
            f("log", "x", "Logarithm to base 10.", "log(100)"),
            f("log2", "x", "Logarithm to base 2.", "log2(8)"),
            f("pow", "a, b", "a to the power b, the same as a^b.", "pow(2, 3)"),
            f("mod", "a, b", "Remainder of a / b with the sign of a.", "mod(7, 3)"),
            f("rmod", "a, b", "Remainder of a / b with the sign of b.", "rmod(-7, 3)"),
            f("fact", "n", "Factorial of the whole part of n.", "fact(5)"),
            f("distance", "x1, y1, x2, y2", "Euclidean distance between two points.", "distance(0, 0, 3, 4)"),
        ],
    ),
    (
        "Trigonometric",
        &[
            f("sin", "x", "Sine, x in radians.", "sin(1)"),
            f("cos", "x", "Cosine, x in radians.", "cos(1)"),
            f("tan", "x", "Tangent, x in radians.", "tan(1)"),
            f("asin", "x", "Inverse sine, result in radians.", "asin(0.5)"),
            f("acos", "x", "Inverse cosine, result in radians.", "acos(0.5)"),
            f("atan", "x", "Inverse tangent, result in radians.", "atan(1)"),
            f("atan2", "y, x", "Angle of the point (x, y) in radians.", "atan2(1, 1)"),
            f("sinh", "x", "Hyperbolic sine.", "sinh(1)"),
            f("cosh", "x", "Hyperbolic cosine.", "cosh(1)"),
            f("tanh", "x", "Hyperbolic tangent.", "tanh(1)"),
            f("asinh", "x", "Inverse hyperbolic sine.", "asinh(1)"),
            f("acosh", "x", "Inverse hyperbolic cosine.", "acosh(2)"),
            f("atanh", "x", "Inverse hyperbolic tangent.", "atanh(0.5)"),
            f("degrees", "x", "Radians to degrees.", "degrees(1)"),
            f("radians", "x", "Degrees to radians.", "radians(90)"),
        ],
    ),
    (
        "Rounding and sign",
        &[
            f("int", "x", "Whole part, cutting toward zero.", "int(2.7)"),
            f("nint", "x", "Nearest whole number, halves away from zero.", "nint(2.5)"),
            f("round", "x, digits", "Round to a number of decimal digits.", "round(2.345, 2)"),
            f("floor", "x", "Largest whole number not above x.", "floor(2.7)"),
            f("ceil", "x", "Smallest whole number not below x.", "ceil(2.2)"),
            f("sign", "x", "-1, 0 or 1 by the sign of x.", "sign(-3)"),
            f("heaviside", "x", "1 when x is zero or more, else 0.", "heaviside(1)"),
        ],
    ),
    (
        "Special",
        &[
            f("erf", "x", "Error function.", "erf(1)"),
            f("erfc", "x", "Complementary error function.", "erfc(1)"),
            f("gamma", "x", "Gamma function.", "gamma(5)"),
            f("lgamma", "x", "Logarithm of the absolute gamma function.", "lgamma(5)"),
            f("beta", "a, b", "Beta function.", "beta(2, 3)"),
            f("j0", "x", "Bessel function of the first kind, order 0.", "j0(1)"),
            f("j1", "x", "Bessel function of the first kind, order 1.", "j1(1)"),
        ],
    ),
    (
        "Logical and missing",
        &[
            f("ifelse", "c, a, b", "a where the condition c is true, else b.", "ifelse(1, 2, 3)"),
            f("if", "c, a, b", "Same as ifelse.", "if(0, 2, 3)"),
            f("na", "", "A missing value.", "na()"),
            f("isna", "x", "1 when x is missing, else 0.", "isna(1)"),
            f("min", "a, b, ...", "Smallest of two or more numbers.", "min(1, 2)"),
            f("max", "a, b, ...", "Largest of two or more numbers.", "max(1, 2)"),
        ],
    ),
    (
        "Statistics",
        &[
            f("mean", "col(A)", "Mean of a column or a row range, such as col(A)[1:10].", "mean(col(A))"),
            f("sum", "col(A)", "Running sum down the column, as in Origin (same as cumsum).", "sum(col(A))"),
            f("total", "col(A)", "Total of a column or a row range.", "total(col(A))"),
            f("median", "col(A)", "Median of a column or a row range.", "median(col(A))"),
            f("sd", "col(A)", "Sample standard deviation (n - 1).", "sd(col(A))"),
            f("stddev", "col(A)", "Same as sd.", "stddev(col(A))"),
            f("stddevp", "col(A)", "Population standard deviation (n).", "stddevp(col(A))"),
            f("var", "col(A)", "Sample variance.", "var(col(A))"),
            f("sem", "col(A)", "Standard error of the mean.", "sem(col(A))"),
            f("count", "col(A)", "Number of rows of the column or range, missing cells included.", "count(col(A))"),
            f("first", "col(A)", "First value of the column.", "first(col(A))"),
            f("last", "col(A)", "Last value of the column.", "last(col(A))"),
        ],
    ),
    (
        "Column operations",
        &[
            f("cumsum", "col(A)", "Running sum down the column.", "cumsum(col(A))"),
            f("diff", "col(A)", "Difference to the next row; the last row is missing.", "diff(col(A))"),
            f("shift", "col(A), n", "Column moved down by n rows (up when n is negative).", "shift(col(A), 1)"),
            f("rank", "col(A)", "Ascending rank, ties share the average rank.", "rank(col(A))"),
            f("lookup", "x, xcol, ycol", "Value of ycol in the row where xcol equals x.", "lookup(1, col(A), col(A))"),
            f("cell", "row, column", "Value at a row and column position of the sheet.", "cell(1, 1)"),
        ],
    ),
    (
        "Date and time",
        &[
            f("date", "y, m, d", "Date number of a calendar date.", "date(2020, 1, 1)"),
            f("time", "h, m, s", "Fraction of a day.", "time(12, 0, 0)"),
            f("now", "", "Current date and time.", "now()"),
            f("today", "", "Current date.", "today()"),
            f("year", "d", "Year of a date number.", "year(date(2020, 1, 1))"),
            f("month", "d", "Month of a date number.", "month(date(2020, 1, 1))"),
            f("day", "d", "Day of the month of a date number.", "day(date(2020, 1, 1))"),
            f("weekday", "d", "Day of the week, 0 for Sunday.", "weekday(date(2020, 1, 1))"),
        ],
    ),
    (
        "Text",
        &[
            f(
                "text",
                "x, format",
                "Number as text; format \"*3\" gives 3 significant digits, \".2\" 2 decimals.",
                "text(col(A), \"*3\")",
            ),
            f("value", "s", "Number written in a text; missing when there is none.", "value(col(A)$)"),
            f("len", "s", "Number of characters. col(A)$ is the text of a cell.", "len(col(A)$)"),
            f("left", "s, n", "First n characters.", "left(col(A)$, 3)"),
            f("right", "s, n", "Last n characters.", "right(col(A)$, 2)"),
            f("mid", "s, start, n", "n characters from position start (the first is 1).", "mid(col(A)$, 2, 3)"),
            f("upper", "s", "Capital letters.", "upper(col(A)$)"),
            f("lower", "s", "Small letters.", "lower(col(A)$)"),
            f("trim", "s", "Without spaces at the start and the end.", "trim(col(A)$)"),
            f("find", "s, part, start", "Position of part in s, -1 if absent; case matters.", "find(col(A)$, \"1\")"),
            f("search", "s, part, start", "Like find, ignoring case.", "search(col(A)$, \"1\")"),
            f("replace", "s, pos, n, new", "Replaces n characters from pos by new.", "replace(col(A)$, 1, 1, \"x\")"),
            f(
                "substitute",
                "s, new, find, occurrence",
                "Replace matching text; omit occurrence or use 0 for all, 1 for the first, 2 for the second.",
                "substitute(col(A)$, \",\", \".\")$",
            ),
            f(
                "compare",
                "s, t, case",
                "1 when the texts are equal, else 0; case 0 ignores case.",
                "compare(col(A)$, \"1\")",
            ),
            f("exact", "s, t", "1 when the texts are equal with case, else 0.", "exact(col(A)$, \"1\")"),
        ],
    ),
    (
        "Random",
        &[
            f("rnd", "", "Uniform random number from 0 up to 1; new in every row.", "rnd()"),
            f("grnd", "", "Normally distributed random number; new in every row.", "grnd()"),
        ],
    ),
];

/// Formulas offered by the Formula menu; `{}` stands for another column of the sheet.
const SAMPLES: &[(&str, &str)] = &[
    ("Scale by a factor", "{}*2"),
    ("Subtract the mean", "{} - mean({})"),
    ("Running sum", "cumsum({})"),
    ("Difference to next row", "diff({})"),
    ("Keep positive values", "ifelse({} > 0, {}, na())"),
    ("Row number", "i"),
    ("Random numbers", "rnd()"),
];

/// What the dialog edits; two snapshots are compared to decide whether Apply is enabled.
#[derive(Clone, PartialEq)]
struct Fields {
    formula: String,
    /// Row range as typed; empty means automatic (the first or last row of the sheet).
    from: String,
    to: String,
    mode: RecalcMode,
}

pub struct SetValuesDialog {
    column: ColumnId,
    fields: Fields,
    /// The values as last applied (or as opened); Apply is enabled when `fields` differs.
    saved: Fields,
    problem: Option<String>,
    focus: bool,
    /// The function whose description is shown below the formula box.
    described: Option<(usize, usize)>,
    show_scripts: bool,
    scripts: String,
}

impl SetValuesDialog {
    pub fn new(project: &Project, column: ColumnId) -> Option<Self> {
        let fields = Self::read(project, column)?;
        Some(SetValuesDialog {
            column,
            saved: fields.clone(),
            fields,
            problem: None,
            focus: true,
            described: None,
            show_scripts: true,
            scripts: String::new(),
        })
    }

    fn read(project: &Project, column: ColumnId) -> Option<Fields> {
        let (_, _, c) = project.locate(column)?;
        let (from, to) = match &c.formula_rows {
            Some(r) => ((r.start + 1).to_string(), r.end.to_string()),
            None => (String::new(), String::new()),
        };
        Some(Fields {
            formula: c.formula.clone().unwrap_or_default(),
            from,
            to,
            // Origin opens the dialog on Manual for a column that has no recalculating formula.
            mode: if c.is_locked() { c.recalc } else { RecalcMode::Manual },
        })
    }

    /// Switches to another column of the sheet (the arrow buttons next to `Col(X)=`).
    fn go_to(&mut self, project: &Project, column: ColumnId) {
        if let Some(fields) = Self::read(project, column) {
            self.column = column;
            self.saved = fields.clone();
            self.fields = fields;
            self.problem = None;
            self.focus = true;
        }
    }

    /// Types a formula (tests).
    #[cfg(test)]
    pub fn set_formula(&mut self, formula: &str) {
        self.fields.formula = formula.to_string();
    }

    /// Types a row range (tests).
    #[cfg(test)]
    pub fn set_rows(&mut self, from: &str, to: &str) {
        self.fields.from = from.to_string();
        self.fields.to = to.to_string();
    }

    fn is_dirty(&self) -> bool {
        self.fields != self.saved
    }

    /// The row range as the model takes it (0-based, end exclusive); `None` when both ends are
    /// automatic. A blank end is the first or last row of the sheet.
    fn rows(&self, row_count: usize) -> Result<Option<std::ops::Range<usize>>, String> {
        let (from, to) = (self.fields.from.trim(), self.fields.to.trim());
        if from.is_empty() && to.is_empty() {
            return Ok(None);
        }
        let number = |s: &str, default: usize| -> Result<usize, String> {
            if s.is_empty() {
                return Ok(default);
            }
            match s.parse::<usize>() {
                Ok(n) if n >= 1 => Ok(n.min(sheaf_core::formula::MAX_FORMULA_ROWS)),
                _ => Err(format!("\"{s}\" is not a row number")),
            }
        };
        let (a, b) = (number(from, 1)?, number(to, row_count.max(1))?);
        Ok(Some((a - 1)..b.max(a)))
    }

    /// Puts text into the formula at the cursor.
    fn insert(&mut self, ctx: &egui::Context, text: &str) {
        let id = egui::Id::new("set_values_formula");
        let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
        let chars = self.fields.formula.chars().count();
        let at = state.cursor.char_range().map_or(chars, |r| r.primary.index.min(chars));
        let byte = self.fields.formula.char_indices().nth(at).map_or(self.fields.formula.len(), |(b, _)| b);
        self.fields.formula.insert_str(byte, text);
        let end = egui::text::CCursor::new(at + text.chars().count());
        state.cursor.set_char_range(Some(egui::text::CCursorRange::one(end)));
        state.store(ctx, id);
        ctx.memory_mut(|m| m.request_focus(id));
    }

    /// Draws the window. Returns `false` when it was closed.
    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> bool {
        let Some((book, sheet, column)) = doc.project().locate(self.column) else {
            return false;
        };
        let target = column.short_name.clone();
        let title = format!(
            "Set Values - [{}]{}!Col(\"{}\")",
            book.name,
            sheet.name,
            if column.long_name.is_empty() { &target } else { &column.long_name }
        );
        let row_count = sheet.row_count();
        let position = sheet.columns.iter().position(|c| c.id == self.column).unwrap_or(0);
        let ids: Vec<ColumnId> = sheet.columns.iter().map(|c| c.id).collect();
        let others: Vec<(String, String)> = sheet
            .columns
            .iter()
            .map(|c| (c.short_name.clone(), if c.units.is_empty() { c.long_name.clone() } else { c.axis_title() }))
            .collect();
        let sample_column = others.iter().map(|o| o.0.clone()).find(|s| *s != target).unwrap_or_else(|| target.clone());

        let mut open = true;
        let mut close = false;
        let mut apply = false;
        let mut goto = None;
        let mut inserted: Option<String> = None;
        let mut replace: Option<String> = None;
        let mut hover: Option<(usize, usize)> = None;
        let mut picked: Option<(usize, usize)> = None;
        egui::Window::new(title)
            .id(egui::Id::new("set_values"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.set_width(405.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    ui.menu_button("Formula", |ui| {
                        for (label, sample) in SAMPLES {
                            if ui.button(*label).clicked() {
                                replace = Some(sample.replace("{}", &format!("col({sample_column})")));
                                ui.close();
                            }
                        }
                    });
                    ui.menu_button("wcol(1)", |ui| {
                        for n in 1..=ids.len() {
                            if ui.button(format!("wcol({n})")).clicked() {
                                inserted = Some(format!("wcol({n})"));
                                ui.close();
                            }
                        }
                    });
                    ui.menu_button("Col(A)", |ui| {
                        for (short, label) in &others {
                            let button = ui.button(format!("Col({short})"));
                            let button = if label.is_empty() { button } else { button.on_hover_text(label) };
                            if button.clicked() {
                                inserted = Some(format!("col({short})"));
                                ui.close();
                            }
                        }
                    });
                    ui.menu_button("Function", |ui| {
                        for (g, (group, funcs)) in FUNCTIONS.iter().enumerate() {
                            ui.menu_button(*group, |ui| {
                                for (k, func) in funcs.iter().enumerate() {
                                    let r = ui.button(func.name);
                                    if r.hovered() {
                                        hover = Some((g, k));
                                    }
                                    if r.clicked() {
                                        picked = Some((g, k));
                                        ui.close();
                                    }
                                }
                            });
                        }
                    });
                    ui.menu_button("Variables", |ui| {
                        for (text, hint) in
                            [("i", "The row number, counted from 1"), ("pi", "3.14159..."), ("e", "2.71828...")]
                        {
                            if ui.button(text).on_hover_text(hint).clicked() {
                                inserted = Some(text.to_string());
                                ui.close();
                            }
                        }
                    });
                    ui.menu_button("Options", |ui| {
                        ui.checkbox(&mut self.show_scripts, "Before Formula Scripts");
                    });
                });
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.label("Row (i):  From");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.fields.from)
                            .desired_width(60.0)
                            .hint_text("<auto>")
                            .id(egui::Id::new("set_values_from")),
                    );
                    ui.label("To");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.fields.to)
                            .desired_width(60.0)
                            .hint_text("<auto>")
                            .id(egui::Id::new("set_values_to")),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("Col({target})=")).strong());
                    ui.add_space(8.0);
                    // First, previous, next, last column of the sheet.
                    for (text, hint, to) in [
                        ("|<", "First column", Some(0)),
                        ("<", "Previous column", position.checked_sub(1)),
                        (">", "Next column", (position + 1 < ids.len()).then_some(position + 1)),
                        (">|", "Last column", Some(ids.len().saturating_sub(1))),
                    ] {
                        let b = ui.add_enabled(to.is_some_and(|t| t != position), egui::Button::new(text).small());
                        if b.on_hover_text(hint).clicked() {
                            goto = to.map(|t| ids[t]);
                        }
                    }
                });
                let edit = ui.add(
                    egui::TextEdit::multiline(&mut self.fields.formula)
                        .id(egui::Id::new("set_values_formula"))
                        .font(egui::TextStyle::Monospace)
                        .desired_rows(5)
                        .desired_width(f32::INFINITY)
                        .hint_text("Enter a one-line expression, or pick a sample from the Formula menu"),
                );
                if self.focus {
                    edit.request_focus();
                    self.focus = false;
                }
                let shown = hover.or(self.described);
                match shown.and_then(|(g, k)| FUNCTIONS.get(g).and_then(|g| g.1.get(k))) {
                    Some(func) => {
                        ui.label(
                            RichText::new(format!(
                                "{}({}): {}  Example: {}",
                                func.name, func.args, func.about, func.example
                            ))
                            .small(),
                        );
                    }
                    None => {
                        ui.label(
                            RichText::new("Pick a function from the Function menu to see what it does.").small().weak(),
                        );
                    }
                }
                if let Some(p) = &self.problem {
                    ui.colored_label(egui::Color32::from_rgb(170, 30, 30), p);
                }
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Recalculate");
                    egui::ComboBox::from_id_salt("set_values_recalc")
                        .selected_text(self.fields.mode.label())
                        .width(80.0)
                        .show_ui(ui, |ui| {
                            for m in RecalcMode::ALL {
                                let hint = match m {
                                    RecalcMode::None => "Run the formula once; the values are plain data afterwards",
                                    RecalcMode::Auto => "Recalculate whenever a column the formula reads changes",
                                    RecalcMode::Manual => "Mark the column when an input changes; recalculate by hand",
                                };
                                ui.selectable_value(&mut self.fields.mode, m, m.label()).on_hover_text(hint);
                            }
                        });
                    ui.add_space(8.0);
                    if ui.button("OK").clicked() {
                        if self.is_dirty() || !self.fields.formula.trim().is_empty() {
                            apply = true;
                        }
                        close = true;
                    }
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                    if ui.add_enabled(self.is_dirty(), egui::Button::new("Apply")).clicked() {
                        apply = true;
                    }
                });
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    let arrow = if self.show_scripts { "v" } else { ">" };
                    if ui.small_button(arrow).on_hover_text("Show or hide the script box").clicked() {
                        self.show_scripts = !self.show_scripts;
                    }
                    ui.label("Before Formula Scripts");
                });
                if self.show_scripts {
                    // Sheaf runs no scripts, so the box is shown as Origin lays it out but is read-only.
                    ui.add_enabled(
                        false,
                        egui::TextEdit::multiline(&mut self.scripts)
                            .desired_rows(3)
                            .desired_width(f32::INFINITY)
                            .hint_text("Scripts that run before the formula are not supported yet"),
                    );
                }
            });

        if let Some(p) = picked {
            self.described = Some(p);
            if let Some(func) = FUNCTIONS.get(p.0).and_then(|g| g.1.get(p.1)) {
                inserted = Some(format!("{}()", func.name));
            }
        }
        if let Some(text) = replace {
            self.fields.formula = text;
        }
        if let Some(text) = inserted {
            self.insert(ctx, &text);
        }
        if let Some(c) = goto {
            self.go_to(doc.project(), c);
            return open;
        }

        if apply {
            match self.rows(row_count) {
                Err(e) => {
                    self.problem = Some(e);
                    close = false;
                }
                Ok(rows) => {
                    let (id, formula, mode) = (self.column, self.fields.formula.clone(), self.fields.mode);
                    match doc.edit("Set column values", move |p| p.set_column_values_with(id, &formula, rows, mode)) {
                        Ok(()) => {
                            self.problem = None;
                            // Reread: a formula that reads its own column runs once, as "None".
                            if let Some(f) = Self::read(doc.project(), self.column) {
                                self.fields.mode = f.mode;
                            }
                            self.saved = self.fields.clone();
                        }
                        Err(e) => {
                            self.problem = Some(e.to_string());
                            close = false;
                        }
                    }
                }
            }
        }
        open && !close
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sheaf_expr::{Expr, SimpleHost};

    #[test]
    fn every_listed_function_evaluates_in_the_expression_engine() {
        let a = [1.0, 2.0, 3.0];
        let host = SimpleHost::new().short("A", &a).at_row(1);
        for (_, funcs) in FUNCTIONS {
            for func in *funcs {
                let e = Expr::parse(func.example).unwrap_or_else(|e| panic!("{}: {e}", func.example));
                e.eval_rows(&host, 3).unwrap_or_else(|e| panic!("{}: {e}", func.example));
                assert!(func.example.starts_with(func.name), "{}", func.name);
            }
        }
    }

    #[test]
    fn the_samples_parse() {
        for (_, s) in SAMPLES {
            Expr::parse(&s.replace("{}", "col(A)")).unwrap();
        }
    }
}
