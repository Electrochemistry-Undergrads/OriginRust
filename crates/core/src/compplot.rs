//! Multiple Comparisons Plot (T58, Origin 2027): a graph of the groups of a one-way ANOVA with a means comparison, or
//! of a two-sample t-test, with significance brackets between the compared groups. What it draws is written down in
//! `docs/spec/statistics.md`, section "Multiple comparisons plot (T58)".
//!
//! The report sheet of the two tests carries a [`ComparisonSource`], a copy of the groups' numbers as the test used
//! them. The p-value of each pair is read from the report sheet itself (the "Prob" column of the means comparison,
//! or of the t-test table), never computed again. The graph's data go to a new sheet of the workbook, the bracket
//! pairs and the options to the graph's layer, so the labels can be restyled without the report.

use serde::{Deserialize, Serialize};

use crate::column::{Column, ColumnData, Designation};
use crate::graph::{PlotKind, SymbolShape, TickLabelSource};
use crate::template::Template;
use crate::{Color, ColumnId, Project, WindowId, Worksheet};

/// Which test a report sheet belongs to.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComparisonKind {
    OneWayAnova,
    TwoSampleTTest,
}

/// One group of a report: its name and the numbers the test used (missing values already left out).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComparisonGroup {
    pub name: String,
    pub values: Vec<f64>,
}

/// A step-down method of the means comparison (Holm): its report lists the plain pairwise p-values and judges each
/// pair at the level of its rank, so the p-values the brackets show are adjusted from the report's (see
/// [`holm_adjust`]).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepDown {
    Bonferroni,
    Sidak,
}

/// The groups behind a report sheet, in the order of the report.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComparisonSource {
    pub kind: ComparisonKind,
    pub groups: Vec<ComparisonGroup>,
    /// The groups `(i, j)` (0-based indices into `groups`) of each row of the report's means comparison, in the order
    /// of the report. Empty in a report that stored none (the pairs are then found by the names in its labels).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pairs: Vec<(usize, usize)>,
    /// The significance level of the analysis.
    #[serde(default = "default_threshold")]
    pub alpha: f64,
    /// Set for the Holm methods.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step_down: Option<StepDown>,
    /// The unit all groups share (empty when they differ).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub units: String,
}

/// What the groups are drawn as.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonPlotType {
    /// Columns of the group means.
    #[default]
    Bar,
    Box,
    Violin,
    /// The raw points, spread sideways deterministically, with the mean and its error bar.
    Scatter,
}

impl ComparisonPlotType {
    pub const ALL: [ComparisonPlotType; 4] =
        [ComparisonPlotType::Bar, ComparisonPlotType::Box, ComparisonPlotType::Violin, ComparisonPlotType::Scatter];

    pub fn label(self) -> &'static str {
        match self {
            ComparisonPlotType::Bar => "Bar",
            ComparisonPlotType::Box => "Box",
            ComparisonPlotType::Violin => "Violin",
            ComparisonPlotType::Scatter => "Scatter",
        }
    }

    pub fn from_word(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.label().eq_ignore_ascii_case(word.trim()))
    }

    /// Whether the error bar setting applies (bar and scatter).
    pub fn has_error_bars(self) -> bool {
        matches!(self, ComparisonPlotType::Bar | ComparisonPlotType::Scatter)
    }
}

/// The error bar of a bar or scatter plot.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonErrorBar {
    /// One standard deviation.
    #[default]
    Sd,
    /// One standard error of the mean.
    Sem,
    /// Half the width of the 95 % confidence interval of the mean (Student's t).
    Ci95,
}

impl ComparisonErrorBar {
    pub const ALL: [ComparisonErrorBar; 3] =
        [ComparisonErrorBar::Sd, ComparisonErrorBar::Sem, ComparisonErrorBar::Ci95];

    pub fn label(self) -> &'static str {
        match self {
            ComparisonErrorBar::Sd => "SD",
            ComparisonErrorBar::Sem => "SEM",
            ComparisonErrorBar::Ci95 => "95% CI",
        }
    }

    pub fn from_word(word: &str) -> Option<Self> {
        match word.trim().to_ascii_lowercase().replace([' ', '_', '%'], "").as_str() {
            "sd" => Some(ComparisonErrorBar::Sd),
            "sem" | "se" => Some(ComparisonErrorBar::Sem),
            "ci95" | "ci" | "95ci" => Some(ComparisonErrorBar::Ci95),
            _ => None,
        }
    }
}

/// How a bracket is labelled.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BracketLabels {
    #[default]
    Asterisks,
    PValues,
}

impl BracketLabels {
    pub fn label(self) -> &'static str {
        match self {
            BracketLabels::Asterisks => "Asterisks",
            BracketLabels::PValues => "P-values",
        }
    }

    pub fn from_word(word: &str) -> Option<Self> {
        match word.trim().to_ascii_lowercase().replace([' ', '_', '-'], "").as_str() {
            "asterisks" | "asterisk" | "stars" => Some(BracketLabels::Asterisks),
            "pvalues" | "pvalue" | "p" => Some(BracketLabels::PValues),
            _ => None,
        }
    }
}

/// The text of a bracket: `*` for p < 0.05, `**` < 0.01, `***` < 0.001, `****` < 0.0001 and `ns` otherwise; or
/// `p = 0.0123` (four decimals, trailing zeros dropped down to two) and `p < 0.0001` below that.
pub fn bracket_label(p: f64, style: BracketLabels) -> String {
    match style {
        BracketLabels::Asterisks => if p < 0.0001 {
            "****"
        } else if p < 0.001 {
            "***"
        } else if p < 0.01 {
            "**"
        } else if p < 0.05 {
            "*"
        } else {
            "ns"
        }
        .to_string(),
        BracketLabels::PValues => {
            if p < 0.0001 {
                return "p < 0.0001".to_string();
            }
            let mut t = format!("{:.4}", p.min(1.0));
            while t.ends_with('0') && t.split('.').nth(1).is_some_and(|d| d.len() > 2) {
                t.pop();
            }
            format!("p = {t}")
        }
    }
}

/// One compared pair of groups (0-based, `a < b`) with the p-value of the report. For the Holm methods `p` is the
/// step-down adjusted p-value derived from the report's plain one (see [`holm_adjust`]); for every other method it is
/// the report's "Prob" itself.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComparisonPair {
    pub a: usize,
    pub b: usize,
    pub p: f64,
    /// The report's own decision for this pair at the analysis' alpha (its "Significant" column); `None` when the
    /// pair was stored without it (then `p` below the threshold decides).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub significant: Option<bool>,
}

impl ComparisonPair {
    fn new(i: usize, j: usize, p: f64, significant: Option<bool>) -> Self {
        ComparisonPair { a: i.min(j), b: i.max(j), p, significant }
    }
}

/// Holm's step-down adjusted p-values of the plain pairwise p-values `p`: with the p-values sorted ascending
/// (`p_(1) <= ... <= p_(m)`, equal ones in the order given), `adj_(r) = max over k <= r of min(1, (m - k + 1) p_(k))`
/// for Holm-Bonferroni and `max over k <= r of 1 - (1 - p_(k))^(m - k + 1)` for Holm-Sidak. A pair is significant at
/// alpha exactly when its adjusted p-value is below alpha, which is the decision the report's step-down makes.
pub fn holm_adjust(p: &[f64], method: StepDown) -> Vec<f64> {
    let m = p.len();
    let mut order: Vec<usize> = (0..m).collect();
    order.sort_by(|&a, &b| p[a].partial_cmp(&p[b]).unwrap_or(std::cmp::Ordering::Equal));
    let mut out = vec![f64::NAN; m];
    let mut running = 0.0f64;
    for (k, &i) in order.iter().enumerate() {
        let left = (m - k) as f64;
        let adj = match method {
            StepDown::Bonferroni => (left * p[i]).min(1.0),
            StepDown::Sidak => 1.0 - (1.0 - p[i]).powf(left),
        };
        running = running.max(adj);
        out[i] = running;
    }
    out
}

/// A bracket between two groups, drawn above them (group indices from 0; the groups stand at 1, 2, 3, ...).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SignificanceBracket {
    pub group_a: usize,
    pub group_b: usize,
    pub p: f64,
    pub label: String,
}

/// The settings of a multiple comparisons plot, stored with its layer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComparisonPlotOptions {
    #[serde(default)]
    pub plot_type: ComparisonPlotType,
    #[serde(default)]
    pub error_bar: ComparisonErrorBar,
    #[serde(default)]
    pub labels: BracketLabels,
    /// Show only the significant pairs: those with p below [`ComparisonPlotOptions::threshold`], or, when the threshold
    /// is the analysis' own alpha, those the report marks significant.
    #[serde(default = "yes")]
    pub significant_only: bool,
    #[serde(default = "default_threshold")]
    pub threshold: f64,
    /// The significance level of the analysis (filled from the report).
    #[serde(default = "default_threshold")]
    pub alpha: f64,
    /// Every compared pair of the report, shown or not.
    #[serde(default)]
    pub pairs: Vec<ComparisonPair>,
}

fn yes() -> bool {
    true
}

fn default_threshold() -> f64 {
    0.05
}

impl Default for ComparisonPlotOptions {
    fn default() -> Self {
        ComparisonPlotOptions {
            plot_type: ComparisonPlotType::Bar,
            error_bar: ComparisonErrorBar::Sd,
            labels: BracketLabels::Asterisks,
            significant_only: true,
            threshold: 0.05,
            alpha: 0.05,
            pairs: Vec::new(),
        }
    }
}

impl ComparisonPlotOptions {
    /// Whether the report's decision (rather than `p < threshold`) rules the pair: the threshold is the analysis' alpha.
    fn follows_report(&self) -> bool {
        (self.threshold - self.alpha).abs() <= 1e-12
    }

    /// Whether a pair is shown when only the significant ones are.
    fn passes(&self, q: &ComparisonPair) -> bool {
        match q.significant {
            Some(sig) if self.follows_report() => sig,
            _ => q.p < self.threshold,
        }
    }

    /// The text of a pair's bracket. Asterisks follow the report's decision at the analysis' alpha: a pair the report
    /// marks not significant reads `ns` whatever its p, a significant one has at least one `*`.
    fn label(&self, q: &ComparisonPair) -> String {
        let stars = bracket_label(q.p, self.labels);
        if self.labels != BracketLabels::Asterisks {
            return stars;
        }
        match q.significant {
            Some(false) => "ns".to_string(),
            Some(true) if stars == "ns" => "*".to_string(),
            _ => stars,
        }
    }

    /// The brackets these settings show: every pair (or the significant ones), in report order.
    pub fn brackets(&self) -> Vec<SignificanceBracket> {
        self.pairs
            .iter()
            .filter(|q| q.p.is_finite() && (!self.significant_only || self.passes(q)))
            .map(|q| SignificanceBracket { group_a: q.a.min(q.b), group_b: q.a.max(q.b), p: q.p, label: self.label(q) })
            .collect()
    }
}

/// The error bar value of one group.
pub fn error_value(values: &[f64], kind: ComparisonErrorBar) -> f64 {
    let s = sheaf_calc::stats(values);
    match kind {
        ComparisonErrorBar::Sd => s.sd,
        ComparisonErrorBar::Sem => s.sem,
        ComparisonErrorBar::Ci95 => {
            let (lo, hi) = sheaf_calc::mean_confidence_limits(&s, 0.95);
            (hi - lo) / 2.0
        }
    }
}

/// The compared pairs of a report sheet with their p-values, read from the sheet: for a one-way ANOVA the rows of the
/// means comparison ("<method> comparison", its "Prob" and "Significant" columns), for a t-test its "Prob".
///
/// The groups of each row come from the pair indices stored with the report. A report without them (an older one) is
/// read by the names in the labels `"{name_j} - {name_i}"`, which is refused when the names do not tell the groups
/// apart (two groups of one name, or a name that contains " - ").
///
/// Holm: the report lists the plain pairwise p-values and judges them by the step-down; the p of each pair returned
/// here is the adjusted p derived from the report's raw p ([`holm_adjust`]), matching its step-down decisions.
pub fn read_pairs(sheet: &Worksheet) -> Result<Vec<ComparisonPair>, String> {
    let src = sheet.comparison_source.as_ref().ok_or("This sheet is not the report of a one-way ANOVA or t-test")?;
    let numeric_after = |name: &str, from: usize| {
        sheet.columns.iter().skip(from).find(|c| c.long_name == name).and_then(Column::numeric).map(<[f64]>::to_vec)
    };
    match src.kind {
        ComparisonKind::TwoSampleTTest => {
            let p = numeric_after("Prob", 0).and_then(|v| v.first().copied()).ok_or("The report has no Prob column")?;
            Ok(vec![ComparisonPair::new(0, 1, p, Some(p < src.alpha))])
        }
        ComparisonKind::OneWayAnova => {
            let at = sheet
                .columns
                .iter()
                .position(|c| c.long_name.ends_with(" comparison"))
                .ok_or("The ANOVA has no means comparison")?;
            let raw = numeric_after("Prob", at + 1).ok_or("The means comparison has no Prob column")?;
            let sig = numeric_after("Significant", at + 1);
            let n = src.groups.len();
            let pairs: Vec<(usize, usize)> =
                if src.pairs.len() == raw.len() && src.pairs.iter().all(|&(i, j)| i < n && j < n) {
                    src.pairs.clone()
                } else {
                    let labels = sheet.columns[at].text().ok_or("The means comparison has no labels")?;
                    labels.iter().map(|l| pair_by_name(src, l)).collect::<Result<_, _>>()?
                };
            let adjusted = src.step_down.map(|m| holm_adjust(&raw, m));
            Ok(pairs
                .into_iter()
                .enumerate()
                .map(|(k, (i, j))| {
                    let p = adjusted.as_ref().map_or(raw[k], |a| a[k]);
                    let significant = sig.as_ref().and_then(|s| s.get(k)).map(|v| *v != 0.0).or(Some(p < src.alpha));
                    ComparisonPair::new(i, j, p, significant)
                })
                .collect())
        }
    }
}

/// The groups named by a label `"{name_j} - {name_i}"`; an error unless exactly one pair of groups fits.
fn pair_by_name(src: &ComparisonSource, label: &str) -> Result<(usize, usize), String> {
    let names: Vec<&str> = src.groups.iter().map(|g| g.name.as_str()).collect();
    let found: Vec<(usize, usize)> = (0..names.len())
        .flat_map(|i| (0..names.len()).map(move |j| (i, j)))
        .filter(|&(i, j)| i != j && label == format!("{} - {}", names[j], names[i]))
        .collect();
    match found.as_slice() {
        [one] => Ok(*one),
        [] => Err(format!("The comparison {label:?} names no pair of groups")),
        _ => Err(format!(
            "The comparison {label:?} fits more than one pair of groups (the group names are not distinct, or contain \" - \"); run the analysis again to make the plot"
        )),
    }
}

impl Project {
    /// Whether the sheet is the report of a test a multiple comparisons plot can be made from.
    pub fn can_comparison_plot(&self, book: WindowId, sheet: usize) -> bool {
        self.workbook(book)
            .and_then(|b| b.sheets.get(sheet))
            .is_some_and(|s| s.comparison_source.is_some() && read_pairs(s).is_ok_and(|p| !p.is_empty()))
    }

    /// Makes the graph of a report sheet (one undoable change of the project): the groups on a new sheet of the
    /// workbook and a graph of them with the brackets. `options.pairs` is filled from the report.
    pub fn comparison_plot(
        &mut self,
        book: WindowId,
        sheet: usize,
        options: &ComparisonPlotOptions,
    ) -> Result<WindowId, String> {
        if !(options.threshold > 0.0 && options.threshold <= 1.0) {
            return Err(format!("The threshold must lie between 0 and 1, not {}", options.threshold));
        }
        let report =
            self.workbook(book).and_then(|b| b.sheets.get(sheet)).ok_or("The report sheet no longer exists")?;
        let pairs = read_pairs(report)?;
        let src = report.comparison_source.clone().expect("read_pairs checked it");
        let options = ComparisonPlotOptions { pairs, alpha: src.alpha, ..options.clone() };
        let groups = &src.groups;
        let n = groups.len();
        if n < 2 {
            return Err("A comparison needs at least two groups".into());
        }

        // The data of the graph, on a sheet of its own.
        let mut ws = Worksheet::new(crate::analysis::free_sheet_name(
            &self.workbook(book).expect("checked above").sheets,
            "MultComp",
        ));
        let add = |this: &mut Project, ws: &mut Worksheet, d: Designation, data: ColumnData, name: &str, u: &str| {
            let id = this.new_column_id();
            let short = ws.next_short_name();
            ws.columns.push(Column::new(id, short, d, data).with_labels(name, u, ""));
            id
        };
        let units = src.units.clone();
        let group_names: Vec<String> = groups.iter().map(|g| g.name.clone()).collect();
        let group_col = add(self, &mut ws, Designation::Label, ColumnData::text(group_names), "Group", "");
        let means: Vec<f64> = groups.iter().map(|g| sheaf_calc::stats(&g.values).mean).collect();
        let errors: Vec<f64> = groups.iter().map(|g| error_value(&g.values, options.error_bar)).collect();
        let mean_col = add(self, &mut ws, Designation::Y, ColumnData::numeric(means), "Mean", &units);
        let err_col =
            add(self, &mut ws, Designation::YError, ColumnData::numeric(errors), options.error_bar.label(), &units);
        let mut value_cols = Vec::new();
        if matches!(options.plot_type, ComparisonPlotType::Box | ComparisonPlotType::Violin) {
            for g in groups {
                value_cols.push(add(
                    self,
                    &mut ws,
                    Designation::Y,
                    ColumnData::numeric(g.values.clone()),
                    &g.name,
                    &units,
                ));
            }
        }
        let mut points: Option<(ColumnId, ColumnId)> = None;
        if options.plot_type == ComparisonPlotType::Scatter {
            let (mut xs, mut ys) = (Vec::new(), Vec::new());
            for (gi, g) in groups.iter().enumerate() {
                for (k, &v) in g.values.iter().enumerate() {
                    xs.push(jittered(gi, k));
                    ys.push(v);
                }
            }
            let x = add(self, &mut ws, Designation::X, ColumnData::numeric(xs), "Position", "");
            let y = add(self, &mut ws, Designation::Y, ColumnData::numeric(ys), "Value", &units);
            points = Some((x, y));
        }
        let b = self.workbook_mut(book).expect("checked above");
        b.sheets.push(ws);

        let graph = self.add_graph(Template::Scatter);
        let err_name = options.error_bar.label();
        let y_title = match options.plot_type {
            ComparisonPlotType::Bar | ComparisonPlotType::Scatter => format!("Mean \u{b1} {err_name}"),
            _ => "Value".to_string(),
        };
        let y_title = if units.is_empty() { y_title } else { format!("{y_title} ({units})") };
        match options.plot_type {
            ComparisonPlotType::Bar => {
                self.add_plot_xy(graph, 0, None, mean_col, Some(err_col)).map_err(|e| e.to_string())?;
            }
            ComparisonPlotType::Box | ComparisonPlotType::Violin => {
                for &c in &value_cols {
                    self.add_plot_xy(graph, 0, None, c, None).map_err(|e| e.to_string())?;
                }
            }
            ComparisonPlotType::Scatter => {
                self.add_plot_xy(graph, 0, None, mean_col, Some(err_col)).map_err(|e| e.to_string())?;
                let (x, y) = points.expect("made above");
                self.add_plot_xy(graph, 0, Some(x), y, None).map_err(|e| e.to_string())?;
            }
        }
        let g = self.graph_mut(graph).expect("just added");
        g.long_name = "Multiple Comparisons Plot".to_string();
        let l = &mut g.layers[0];
        match options.plot_type {
            ComparisonPlotType::Bar => l.plots[0].kind = PlotKind::Column,
            ComparisonPlotType::Box => l.plots.iter_mut().for_each(|p| p.kind = PlotKind::Box),
            ComparisonPlotType::Violin => l.plots.iter_mut().for_each(|p| p.kind = PlotKind::Violin),
            ComparisonPlotType::Scatter => {
                for p in &mut l.plots {
                    p.kind = PlotKind::Scatter;
                }
            }
        }
        l.apply_palette();
        if options.plot_type == ComparisonPlotType::Scatter {
            // The mean stands first (it makes the X axis a category axis) and is drawn in black over nothing; the
            // raw points are the second plot.
            let mean = &mut l.plots[0];
            mean.symbol.shape = SymbolShape::Diamond;
            mean.symbol.size_pt = 9.0;
            mean.symbol.color = Color::BLACK;
            mean.line.color = Color::BLACK;
            mean.auto_color = false;
            l.plots[1].symbol.size_pt = 6.0;
            l.plots[0].label = Some("Mean".to_string());
            l.plots[1].label = Some("Data".to_string());
        }
        if options.plot_type != ComparisonPlotType::Box && options.plot_type != ComparisonPlotType::Violin {
            l.x_axis.tick_label_source = TickLabelSource::Column { column: group_col };
        }
        l.x_axis.title = Some(String::new());
        l.y_axis.title = Some(y_title);
        l.legend.visible = false;
        l.brackets = options.brackets();
        l.comparison = Some(options);
        Ok(graph)
    }
}

/// The X position of the k-th point of group `g` (0-based): the group's position 1, 2, 3, ... plus a fixed pseudo-random
/// shift of up to 0.18 either way (the golden-ratio sequence, so that the picture is the same every time).
pub fn jittered(g: usize, k: usize) -> f64 {
    let u = ((k as f64 + 1.0) * 0.618_033_988_749_894_9).fract();
    (g + 1) as f64 + (u - 0.5) * 0.36
}
