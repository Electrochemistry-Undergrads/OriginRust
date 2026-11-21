//! Statistics > Descriptive Statistics > Graphical Summary (T40) as a tool of `Project::analyze`. The numbers are in
//! `sheaf_calc::graph_summary`; what the tool computes and draws is written down in `docs/spec/statistics.md`,
//! section 22.
//!
//! For each chosen column (and each group of the optional group columns) the tool writes three tables to a new
//! sheet, "Description Statistics", "<level>% Confidence Intervals" and "Normality Test", and makes one graph
//! with a histogram and a fitted normal curve, a confidence interval plot and a normal probability plot. The
//! histogram and the probability plot are the existing Histogram and Probability Plot tools applied to the
//! group's values; their columns are written to the same sheet, behind the report columns.

use super::dist::XyGraph;
use super::statistics::{cell_left_out, num};
use super::*;
use crate::graph::{AxisSide, DataLabels, Font, LabelForm, LabelPosition, PlotKind, ScaleKind, TextLabel};

/// The most groups one run takes; a near-unique grouping column (an ID) is refused.
const MAX_GROUPS: usize = 1000;

/// Origin's "Graph Arrangement". Only [`GraphArrangement::SeparateGraphs`] is made so far; the other three are
/// refused with a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GraphArrangement {
    /// One graph (histogram, interval plot, probability plot) per variable and group.
    #[default]
    SeparateGraphs,
    PerVariable,
    PerGroup,
    AllInOne,
}

impl GraphArrangement {
    pub const ALL: [GraphArrangement; 4] = [
        GraphArrangement::SeparateGraphs,
        GraphArrangement::PerVariable,
        GraphArrangement::PerGroup,
        GraphArrangement::AllInOne,
    ];

    /// The name in Origin's dialog.
    pub fn label(self) -> &'static str {
        match self {
            GraphArrangement::SeparateGraphs => "Separate Graphs",
            GraphArrangement::PerVariable => "Per Variable",
            GraphArrangement::PerGroup => "Per Group",
            GraphArrangement::AllInOne => "All in One",
        }
    }

    /// Whether this arrangement is made.
    pub fn implemented(self) -> bool {
        self == GraphArrangement::SeparateGraphs
    }

    /// A command-line word: `separate`, `per-variable`, `per-group` or `all-in-one`.
    pub fn from_word(word: &str) -> Option<GraphArrangement> {
        match word.trim().to_ascii_lowercase().replace([' ', '_'], "-").as_str() {
            "separate" | "separate-graphs" => Some(GraphArrangement::SeparateGraphs),
            "per-variable" => Some(GraphArrangement::PerVariable),
            "per-group" => Some(GraphArrangement::PerGroup),
            "all-in-one" => Some(GraphArrangement::AllInOne),
            _ => None,
        }
    }
}

/// What [`Tool::GraphSummary`] does. The chosen columns of the request are the variables.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphSummarySpec {
    /// Group columns (numbers or text) that split the rows into groups; with several, a group is one
    /// combination of their values.
    pub groups: Vec<ColumnId>,
    /// Confidence level in percent (default 95).
    pub level: f64,
    pub arrangement: GraphArrangement,
}

impl Default for GraphSummarySpec {
    fn default() -> Self {
        GraphSummarySpec { groups: Vec::new(), level: 95.0, arrangement: GraphArrangement::SeparateGraphs }
    }
}

/// One graph to build: positions in the result sheet's columns.
pub(super) struct SummaryGraph {
    pub title: String,
    /// Histogram: (bin centre, count) and the normal curve (x, y), all on the count axis.
    pub bars: (usize, usize),
    pub curve: (usize, usize),
    /// Mean interval, median interval (x, y of the two end points) and the estimates with their label column.
    pub mean_ci: (usize, usize),
    pub median_ci: (usize, usize),
    pub estimates: (usize, usize, usize),
    pub prob: XyGraph,
    pub x_title: String,
}

pub(super) struct SummaryOutput {
    pub table: ResultTable,
    pub on_sheet: Vec<SheetColumn>,
    pub notes: Vec<String>,
    pub graphs: Vec<SummaryGraph>,
}

fn label_column(name: &str, values: Vec<String>) -> SheetColumn {
    SheetColumn {
        designation: Designation::Label,
        long_name: name.to_string(),
        units: String::new(),
        comments: String::new(),
        data: ColumnData::text(values),
    }
}

fn value_column(name: &str, comments: &str, units: &str, designation: Designation, values: Vec<f64>) -> SheetColumn {
    SheetColumn {
        designation,
        long_name: name.to_string(),
        units: units.to_string(),
        comments: comments.to_string(),
        data: ColumnData::numeric(values),
    }
}

fn numbers(c: &SheetColumn) -> Vec<f64> {
    match &c.data {
        ColumnData::Numeric(v) => v.to_vec(),
        _ => Vec::new(),
    }
}

/// One group-column value of the Graphical Summary: a number kept exactly, or text.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum GroupKey {
    /// The f64 bits, with -0.0 folded into 0.0 (NaN cells never become keys).
    Num(u64),
    Text(String),
}

impl GroupKey {
    fn of(c: &Column, row: usize) -> Option<GroupKey> {
        if let Some(t) = c.data.text_cell(row) {
            let t = t.trim();
            return (!t.is_empty()).then(|| GroupKey::Text(t.to_string()));
        }
        let v = c.numeric()?.get(row).copied().filter(|v| !v.is_nan())?;
        Some(GroupKey::Num((if v == 0.0 { 0.0 } else { v }).to_bits()))
    }

    /// Numbers print with full round-trip precision, so 2023.5 and 2024 stay apart.
    fn label(&self) -> String {
        match self {
            GroupKey::Num(b) => format!("{}", f64::from_bits(*b)),
            GroupKey::Text(t) => t.clone(),
        }
    }

    /// A total order: numbers first (by `f64::total_cmp`), then text (by bytes).
    fn compare(&self, other: &GroupKey) -> Ordering {
        match (self, other) {
            (GroupKey::Num(a), GroupKey::Num(b)) => f64::from_bits(*a).total_cmp(&f64::from_bits(*b)),
            (GroupKey::Num(_), GroupKey::Text(_)) => Ordering::Less,
            (GroupKey::Text(_), GroupKey::Num(_)) => Ordering::Greater,
            (GroupKey::Text(a), GroupKey::Text(b)) => a.cmp(b),
        }
    }
}

/// Group keys compare column by column with `GroupKey::compare`.
fn compare_keys(a: &[GroupKey], b: &[GroupKey]) -> Ordering {
    a.iter().zip(b).map(|(x, y)| x.compare(y)).find(|o| *o != Ordering::Equal).unwrap_or(Ordering::Equal)
}

/// "95" for 95.0, "99.9" for 99.9: the level in the table name.
fn level_text(level: f64) -> String {
    let t = format!("{level:.4}");
    t.trim_end_matches('0').trim_end_matches('.').to_string()
}

pub(super) fn run(
    spec: &GraphSummarySpec,
    sheet: &Worksheet,
    columns: &[ColumnId],
    rows: RowRange,
) -> Result<SummaryOutput, AnalysisError> {
    if !spec.arrangement.implemented() {
        return Err(AnalysisError::BadOption(format!(
            "the graph arrangement \"{}\" is not available yet; use Separate Graphs",
            spec.arrangement.label()
        )));
    }
    if !(spec.level > 0.0 && spec.level < 100.0) {
        return Err(AnalysisError::BadOption("the confidence level must lie between 0 and 100 percent".into()));
    }
    if matches!(rows, RowRange::XBetween(..)) {
        return Err(AnalysisError::BadOption(
            "the graphical summary has no X range to limit; choose rows with a row range".into(),
        ));
    }
    if columns.is_empty() {
        return Err(AnalysisError::NoColumns);
    }
    let level = spec.level / 100.0;
    let level_pct = level_text(spec.level);
    let ci_table = format!("{level_pct}% Confidence Intervals");
    let group_cols: Vec<&Column> = spec
        .groups
        .iter()
        .map(|id| sheet.column(*id).ok_or(AnalysisError::NoSuchColumn(*id)))
        .collect::<Result<_, _>>()?;

    let mut table = ResultTable::new(&[
        "N",
        "Missing",
        "Mean",
        "Standard Deviation",
        "SE of Mean",
        "Variance",
        "Skewness",
        "Kurtosis",
        "Minimum",
        "1st Quartile",
        "Median",
        "3rd Quartile",
        "Maximum",
        "Mean lower",
        "Mean upper",
        "Median lower",
        "Median upper",
        "Standard Deviation lower",
        "Standard Deviation upper",
        "A-Squared",
        "P-Value",
    ]);
    let (mut v_var, mut v_grp, mut v_tab, mut v_stat) = (vec![], vec![], vec![], vec![]);
    let (mut v_val, mut v_lo, mut v_hi) = (vec![], vec![], vec![]);
    let mut plot_columns: Vec<SheetColumn> = Vec::new();
    // Positions of the plot columns are known only after the seven report columns: remember them as offsets.
    struct Pending {
        title: String,
        x_title: String,
        bars: (usize, usize),
        curve: (usize, usize),
        mean_ci: (usize, usize),
        median_ci: (usize, usize),
        estimates: (usize, usize, usize),
        prob: XyGraph,
    }
    let mut pending: Vec<Pending> = Vec::new();
    let mut notes = Vec::new();

    for &vid in columns {
        let c = sheet.column(vid).ok_or(AnalysisError::NoSuchColumn(vid))?;
        let y = sheet.values_of(c).ok_or_else(|| AnalysisError::NotNumeric(c.short_name.clone()))?;
        if y.is_empty() {
            return Err(AnalysisError::Empty(c.short_name.clone()));
        }
        let (first, last) = resolve_rows(rows, &c.short_name, &[], y.len())?;
        let var_label = c.display_name().to_string();
        // Rows per group, in the order of the group keys.
        let mut keys: Vec<(Vec<GroupKey>, Vec<f64>)> = Vec::new();
        let mut index: std::collections::HashMap<Vec<GroupKey>, usize> = std::collections::HashMap::new();
        for row in first..=last {
            // A masked or filtered group cell leaves the row out (T255).
            if group_cols.iter().any(|g| cell_left_out(sheet, g, row)) {
                continue;
            }
            let key: Option<Vec<GroupKey>> = group_cols.iter().map(|g| GroupKey::of(g, row)).collect();
            let Some(key) = key else { continue };
            let v = y[row];
            match index.get(&key) {
                Some(&k) => keys[k].1.push(v),
                None => {
                    if keys.len() >= MAX_GROUPS {
                        return Err(AnalysisError::BadOption(format!(
                            "the group columns have more than {MAX_GROUPS} different groups; choose columns with fewer groups"
                        )));
                    }
                    index.insert(key.clone(), keys.len());
                    keys.push((key, vec![v]));
                }
            }
        }
        keys.sort_by(|a, b| compare_keys(&a.0, &b.0));
        for (key, values) in keys {
            let group = key.iter().map(GroupKey::label).collect::<Vec<_>>().join(", ");
            let title = if group.is_empty() { var_label.clone() } else { format!("{var_label} {group}") };
            let s = calc::graph_summary(&values, level);
            if s.stats.n < 2 {
                notes.push(format!("{title}: fewer than two numbers, left out."));
                continue;
            }
            let st = &s.stats;
            let (ad, ad_p) = s.anderson_darling.map_or((f64::NAN, f64::NAN), |t| (t.statistic, t.p));
            let units = c.units.clone();
            let mut row_units = vec![String::new(); 21];
            for i in [2, 3, 4, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18] {
                row_units[i] = units.clone();
            }
            row_units[5] = if units.is_empty() { String::new() } else { format!("{units}^2") };
            table.push_row(
                title.clone(),
                vec![
                    st.n as f64,
                    st.missing as f64,
                    st.mean,
                    st.sd,
                    st.sem,
                    st.variance,
                    st.skewness,
                    st.kurtosis,
                    st.min,
                    st.q1,
                    st.median,
                    st.q3,
                    st.max,
                    s.mean_ci.0,
                    s.mean_ci.1,
                    s.median_ci.0,
                    s.median_ci.1,
                    s.sd_ci.0,
                    s.sd_ci.1,
                    ad,
                    ad_p,
                ],
                row_units,
            );
            let nan = f64::NAN;
            let mut put = |t: &str, stat: &str, val: f64, lo: f64, hi: f64| {
                v_var.push(var_label.clone());
                v_grp.push(group.clone());
                v_tab.push(t.to_string());
                v_stat.push(stat.to_string());
                v_val.push(val);
                v_lo.push(lo);
                v_hi.push(hi);
            };
            let d = "Description Statistics";
            put(d, "N", st.n as f64, nan, nan);
            put(d, "Missing", st.missing as f64, nan, nan);
            put(d, "Mean", st.mean, nan, nan);
            put(d, "Standard Deviation", st.sd, nan, nan);
            put(d, "SE of Mean", st.sem, nan, nan);
            put(d, "Variance", st.variance, nan, nan);
            put(d, "Skewness", st.skewness, nan, nan);
            put(d, "Kurtosis", st.kurtosis, nan, nan);
            put(d, "Minimum", st.min, nan, nan);
            put(d, "1st Quartile", st.q1, nan, nan);
            put(d, "Median", st.median, nan, nan);
            put(d, "3rd Quartile", st.q3, nan, nan);
            put(d, "Maximum", st.max, nan, nan);
            put(&ci_table, "Mean", st.mean, s.mean_ci.0, s.mean_ci.1);
            put(&ci_table, "Median", st.median, s.median_ci.0, s.median_ci.1);
            put(&ci_table, "Standard Deviation", st.sd, s.sd_ci.0, s.sd_ci.1);
            put("Normality Test", "A-Squared", ad, nan, nan);
            put("Normality Test", "P-Value", ad_p, nan, nan);
            if s.median_coverage < level - 1e-9 {
                notes.push(format!(
                    "{title}: with {} numbers the median interval cannot reach {level_pct}%; it spans the whole range and its confidence is {}%.",
                    st.n,
                    num(s.median_coverage * 100.0)
                ));
            }
            if s.anderson_darling.is_none() {
                notes.push(format!("{title}: the Anderson-Darling test needs at least 8 numbers with some spread."));
            }
            if !(st.sd.is_finite() && st.mean.is_finite() && st.variance.is_finite()) {
                notes.push(format!(
                    "{title}: the mean or spread is too large to compute (not finite), no graph was made."
                ));
                continue;
            }
            if st.sd <= 0.0 {
                notes.push(format!("{title}: all numbers are equal, no graph was made."));
                continue;
            }

            // The graph: the histogram and the probability plot are the existing tools run on this group.
            let inp = Input {
                id: vid,
                short: c.short_name.clone(),
                label: title.clone(),
                name: title.clone(),
                units: units.clone(),
                x_name: "Row".into(),
                x_short: "Row".into(),
                x_units: String::new(),
                x: (1..=values.len()).map(|i| i as f64).collect(),
                y: Cow::Owned(values.clone()),
                first: 0,
                last: values.len() - 1,
            };
            let hist = statistics::histogram(
                &HistogramSpec { freq: FreqOptions::default(), graph: true },
                std::slice::from_ref(&inp),
            )?;
            let prob = dist::prob_plot(&ProbPlotSpec::default(), std::slice::from_ref(&inp))?;
            let base = plot_columns.len();
            let centers: Vec<f64> = numbers(&hist.on_sheet[0]);
            let ends: Vec<f64> = numbers(&hist.on_sheet[1]);
            let width = if centers.len() >= 2 { centers[1] - centers[0] } else { ends[0] - centers[0] };
            let (lo, hi) = (centers[0] - width / 2.0, ends[ends.len() - 1]);
            let (mut cx, mut cy) = (Vec::new(), Vec::new());
            for i in 0..=100 {
                let x = lo + (hi - lo) * f64::from(i) / 100.0;
                let z = (x - st.mean) / st.sd;
                cx.push(x);
                cy.push(st.n as f64 * width * (-0.5_f64 * z * z).exp() / (st.sd * (2.0 * std::f64::consts::PI).sqrt()));
            }
            let mut hist_cols = hist.on_sheet;
            for col in &mut hist_cols {
                col.comments = title.clone();
            }
            plot_columns.extend(hist_cols);
            let at = base + 5;
            plot_columns.push(value_column("Normal curve X", &title, &units, Designation::X, cx));
            plot_columns.push(value_column("Normal curve count", &title, "", Designation::Y, cy));
            plot_columns.push(value_column(
                "Mean interval X",
                &title,
                &units,
                Designation::X,
                vec![s.mean_ci.0, s.mean_ci.1],
            ));
            plot_columns.push(value_column("Mean interval Y", &title, "", Designation::Y, vec![2.0, 2.0]));
            plot_columns.push(value_column(
                "Median interval X",
                &title,
                &units,
                Designation::X,
                vec![s.median_ci.0, s.median_ci.1],
            ));
            plot_columns.push(value_column("Median interval Y", &title, "", Designation::Y, vec![1.0, 1.0]));
            plot_columns.push(value_column("Estimate X", &title, &units, Designation::X, vec![st.mean, st.median]));
            plot_columns.push(value_column("Estimate Y", &title, "", Designation::Y, vec![2.0, 1.0]));
            plot_columns.push(label_column("Estimate", vec!["Mean".into(), "Median".into()]));
            let pb = at + 9;
            let mut pcols = prob.on_sheet;
            for col in &mut pcols {
                col.comments = title.clone();
            }
            plot_columns.extend(pcols);
            let mut pg = prob.graph.expect("a probability plot graph");
            for p in &mut pg.plots {
                p.x += pb;
                p.y += pb;
            }
            pending.push(Pending {
                title: title.clone(),
                x_title: if units.is_empty() { var_label.clone() } else { format!("{var_label} ({units})") },
                bars: (base, base + 2),
                curve: (at, at + 1),
                mean_ci: (at + 2, at + 3),
                median_ci: (at + 4, at + 5),
                estimates: (at + 6, at + 7, at + 8),
                prob: pg,
            });
            notes.push(format!("{title}: {} values, Anderson-Darling A2 = {}, p = {}.", st.n, num(ad), num(ad_p)));
        }
    }
    if v_var.is_empty() {
        return Err(AnalysisError::BadOption(
            "no variable or group has two numbers; the graphical summary needs at least two numbers".into(),
        ));
    }
    let mut on_sheet = vec![
        label_column("Variable", v_var),
        label_column("Group", v_grp),
        label_column("Table", v_tab),
        label_column("Statistic", v_stat),
        value_column("Value", "", "", Designation::Y, v_val),
        value_column("Lower", &format!("{level_pct}% limit"), "", Designation::Y, v_lo),
        value_column("Upper", &format!("{level_pct}% limit"), "", Designation::Y, v_hi),
    ];
    let shift = on_sheet.len();
    on_sheet.extend(plot_columns);
    let graphs = pending
        .into_iter()
        .map(|p| {
            let mut prob = p.prob;
            for pl in &mut prob.plots {
                pl.x += shift;
                pl.y += shift;
            }
            let up = |t: (usize, usize)| (t.0 + shift, t.1 + shift);
            SummaryGraph {
                title: p.title,
                bars: up(p.bars),
                curve: up(p.curve),
                mean_ci: up(p.mean_ci),
                median_ci: up(p.median_ci),
                estimates: (p.estimates.0 + shift, p.estimates.1 + shift, p.estimates.2 + shift),
                prob,
                x_title: p.x_title,
            }
        })
        .collect();
    notes.push(format!(
        "Confidence intervals at {level_pct}%: mean from Student's t, median from order statistics (exact binomial, ends are sample values), standard deviation from chi-square."
    ));
    Ok(SummaryOutput { table, on_sheet, notes, graphs })
}

const BLUE: crate::color::Color = crate::color::Color::rgb(70, 120, 190);
const RED: crate::color::Color = crate::color::Color::rgb(200, 30, 30);

impl Project {
    /// The graph of one variable-group: histogram with normal curve (top left), interval plot (bottom left) and
    /// normal probability plot (right), three layers with frames of their own (Sheaf's layout).
    pub(super) fn summary_graph(&mut self, ids: &[ColumnId], g: &SummaryGraph) -> WindowId {
        use crate::graph::Frame;
        let graph = self.add_graph(crate::template::Template::Scatter);
        let small = |size: f64| Font::new("Arial", size);
        if let Some(gr) = self.graph_mut(graph) {
            gr.long_name = g.title.clone();
            gr.page = crate::graph::Page::inches(11.0, 7.0);
            let base = gr.layers[0].clone();
            gr.layers = (0..3).map(|_| base.clone()).collect();
            let frames = [
                Frame { left: 0.08, top: 0.11, width: 0.42, height: 0.47 },
                Frame { left: 0.08, top: 0.73, width: 0.42, height: 0.14 },
                Frame { left: 0.60, top: 0.11, width: 0.36, height: 0.76 },
            ];
            for (l, f) in gr.layers.iter_mut().zip(frames) {
                l.frame = f;
                l.legend.visible = false;
                for a in [&mut l.x_axis, &mut l.y_axis] {
                    a.title_font = small(13.0);
                    a.tick_font = small(11.0);
                }
            }
            let mut title = TextLabel::new(g.title.clone(), 0.0, -0.14, small(16.0));
            title.font.bold = true;
            gr.layers[0].labels.push(title);
        }
        let pairs = [
            (0, g.bars.0, g.bars.1),
            (0, g.curve.0, g.curve.1),
            (1, g.mean_ci.0, g.mean_ci.1),
            (1, g.median_ci.0, g.median_ci.1),
            (1, g.estimates.0, g.estimates.1),
        ];
        for (layer, x, y) in pairs {
            self.add_plot_xy(graph, layer, Some(ids[x]), ids[y], None).expect("the graph and the new columns exist");
        }
        for p in &g.prob.plots {
            self.add_plot_xy(graph, 2, Some(ids[p.x]), ids[p.y], None).expect("the graph and the new columns exist");
        }
        let label_col = ids[g.estimates.2];
        let Some(gr) = self.graph_mut(graph) else { return graph };
        // Layer 0: bars and the normal curve on the count axis.
        {
            let l = &mut gr.layers[0];
            l.plots[0].kind = PlotKind::Column;
            l.plots[0].label = Some("Histogram".into());
            l.plots[1].kind = PlotKind::Line;
            l.plots[1].label = Some("Normal".into());
            for (p, c) in l.plots.iter_mut().zip([BLUE, RED]) {
                p.line.color = c;
                p.symbol.color = c;
                p.auto_color = false;
            }
            l.plots[1].line.width_pt = 1.5;
            l.x_axis.title = Some(g.x_title.clone());
            l.y_axis.title = Some("Frequency".into());
            l.y_axis.min = Some(0.0);
        }
        // Layer 1: the two intervals as horizontal bars with the estimate marked and named.
        {
            let l = &mut gr.layers[1];
            for (p, name) in l.plots.iter_mut().zip(["Mean interval", "Median interval"]) {
                p.kind = PlotKind::Line;
                p.line.width_pt = 2.5;
                p.label = Some(name.into());
            }
            l.plots[2].kind = PlotKind::Scatter;
            l.plots[2].label = Some("Estimate".into());
            for p in &mut l.plots {
                p.line.color = crate::color::Color::BLACK;
                p.symbol.color = crate::color::Color::BLACK;
                p.auto_color = false;
            }
            let mut font = small(11.0);
            font.color = crate::color::Color::BLACK;
            l.plots[2].decor.labels = DataLabels {
                enable: true,
                form: LabelForm::Column { column: label_col },
                position: LabelPosition::Above,
                font,
                ..Default::default()
            };
            l.x_axis.title = Some(format!("{} (confidence intervals)", g.x_title));
            l.y_axis.title = Some(String::new());
            l.y_axis.show_title = false;
            l.y_axis.show_tick_labels = false;
            l.y_axis.min = Some(0.0);
            l.y_axis.max = Some(3.0);
            l.y_axis.major_step = Some(1.0);
            l.y_axis.minor_ticks = 0;
            l.y_axis.side = AxisSide::Normal;
        }
        // Layer 2: the probability plot as the Probability Plot tool draws it.
        {
            let l = &mut gr.layers[2];
            for (plot, spec) in l.plots.iter_mut().zip(&g.prob.plots) {
                plot.kind = spec.kind;
                plot.label = Some(spec.label.clone());
            }
            l.plots[0].symbol.color = BLUE;
            l.plots[0].symbol.size_pt = 6.0;
            l.plots[0].auto_color = false;
            l.plots[1].line.color = RED;
            l.plots[1].symbol.color = RED;
            l.plots[1].auto_color = false;
            l.x_axis.scale = g.prob.x_scale;
            l.y_axis.scale = ScaleKind::Probability;
            l.x_axis.title = Some(g.x_title.clone());
            l.y_axis.title = Some(g.prob.y_title.clone());
        }
        graph
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A (24 numbers), B (24 numbers), group column G (text, a/b) and a numeric group column H (1/2 in blocks).
    fn project() -> (Project, Vec<ColumnId>) {
        let a: Vec<f64> = (1..=24).map(|i| 10.0 + 2.0 * (f64::from(i) * 2.3).sin() + f64::from(i) * 0.1).collect();
        let b: Vec<f64> = (1..=24).map(|i| 3.0 + (f64::from(i) * 1.7).cos().abs() * 4.0).collect();
        let g: Vec<String> = (0..24).map(|i| if i % 2 == 0 { "a" } else { "b" }.to_string()).collect();
        let h: Vec<f64> = (0..24).map(|i| if i < 12 { 1.0 } else { 2.0 }).collect();
        let mut p = Project::new();
        let book = p.add_workbook("summary");
        let mut ids = Vec::new();
        for d in [ColumnData::numeric(a), ColumnData::numeric(b), ColumnData::text(g), ColumnData::numeric(h)] {
            ids.push(p.add_column(book, 0, Designation::Y, d).unwrap());
        }
        (p, ids)
    }

    fn run_summary(p: &mut Project, vars: &[ColumnId], spec: GraphSummarySpec) -> Result<Report, AnalysisError> {
        p.analyze(&Request { columns: vars.to_vec(), rows: RowRange::All, tool: Tool::GraphSummary(spec) })
    }

    fn column<'a>(p: &'a Project, r: &Report, name: &str) -> &'a Column {
        let ws = &p.workbook(r.book).unwrap().sheets[r.new_sheet.unwrap()];
        ws.columns.iter().find(|c| c.long_name == name).unwrap_or_else(|| panic!("no column {name}"))
    }

    #[test]
    fn ungrouped_run_makes_the_three_tables_and_one_graph() {
        let (mut p, ids) = project();
        let r = run_summary(&mut p, &ids[..1], GraphSummarySpec::default()).unwrap();
        assert!(r.message.contains("Graphical summary") && r.message.contains("GraphSummary1"), "{}", r.message);
        assert!(r.new_graph.is_some() && r.more_graphs.is_empty());
        let tables = column(&p, &r, "Table").text().unwrap();
        let mut seen: Vec<&str> = Vec::new();
        for t in tables {
            if !seen.contains(&t.as_str()) {
                seen.push(t);
            }
        }
        assert_eq!(seen, ["Description Statistics", "95% Confidence Intervals", "Normality Test"]);
        // The values are the calc numbers.
        let a = p.workbook(r.book).unwrap().sheets[0].columns[0].numeric().unwrap().to_vec();
        let s = calc::graph_summary(&a, 0.95);
        let table = r.table.clone().unwrap();
        let row = &table.rows[0].1;
        let h = |name: &str| table.headers.iter().position(|x| x == name).unwrap();
        assert_eq!(row[h("N")], 24.0);
        assert_eq!(row[h("Mean")], s.stats.mean);
        assert_eq!(row[h("Median")], s.stats.median);
        assert_eq!(row[h("Mean lower")], s.mean_ci.0);
        assert_eq!(row[h("Median upper")], s.median_ci.1);
        assert_eq!(row[h("Standard Deviation lower")], s.sd_ci.0);
        assert_eq!(row[h("A-Squared")], s.anderson_darling.unwrap().statistic);
        assert_eq!(row[h("P-Value")], s.anderson_darling.unwrap().p);
        // The same numbers are on the sheet, row by row.
        let stat = column(&p, &r, "Statistic").text().unwrap();
        let val = column(&p, &r, "Value").numeric().unwrap();
        let lo = column(&p, &r, "Lower").numeric().unwrap();
        let i = stat.iter().position(|x| x == "Standard Deviation").unwrap();
        assert_eq!(val[i], s.stats.sd);
        let j = i + stat[i + 1..].iter().position(|x| x == "Standard Deviation").unwrap() + 1;
        assert_eq!((val[j], lo[j]), (s.stats.sd, s.sd_ci.0));
        // The graph: three layers, histogram with curve, intervals, probability plot.
        let g = p.graph(r.new_graph.unwrap()).unwrap();
        assert_eq!(g.long_name, "A");
        assert_eq!(g.layers.len(), 3);
        assert_eq!((g.layers[0].plots.len(), g.layers[1].plots.len(), g.layers[2].plots.len()), (2, 3, 2));
        assert_eq!(g.layers[0].plots[0].kind, crate::graph::PlotKind::Column);
        assert_eq!(g.layers[2].y_axis.scale, ScaleKind::Probability);
    }

    #[test]
    fn graphs_are_variables_times_groups_titled_by_both() {
        let (mut p, ids) = project();
        let spec = GraphSummarySpec { groups: vec![ids[2]], ..Default::default() };
        let r = run_summary(&mut p, &ids[..2], spec).unwrap();
        let all: Vec<WindowId> = r.new_graph.into_iter().chain(r.more_graphs.iter().copied()).collect();
        assert_eq!(all.len(), 4);
        let titles: Vec<String> = all.iter().map(|g| p.graph(*g).unwrap().long_name.clone()).collect();
        assert_eq!(titles, ["A a", "A b", "B a", "B b"]);
        // The group's own numbers: A in group a are the even rows.
        let a = p.workbook(r.book).unwrap().sheets[0].columns[0].numeric().unwrap().to_vec();
        let even: Vec<f64> = a.iter().copied().step_by(2).collect();
        let s = calc::graph_summary(&even, 0.95);
        assert_eq!(r.table.as_ref().unwrap().rows[0].0, "A a");
        assert_eq!(r.table.as_ref().unwrap().rows[0].1[2], s.stats.mean);
        assert_eq!(r.table.as_ref().unwrap().rows.len(), 4);
        // Numeric groups sort by value; two group columns combine.
        let both = GraphSummarySpec { groups: vec![ids[3], ids[2]], level: 90.0, ..Default::default() };
        let r = run_summary(&mut p, &ids[..1], both).unwrap();
        let labels: Vec<&str> = r.table.as_ref().unwrap().rows.iter().map(|r| r.0.as_str()).collect();
        assert_eq!(labels, ["A 1, a", "A 1, b", "A 2, a", "A 2, b"]);
        let tables = column(&p, &r, "Table").text().unwrap();
        assert!(tables.iter().any(|t| t == "90% Confidence Intervals"));
    }

    #[test]
    fn other_arrangements_bad_levels_and_tiny_samples_are_refused_or_noted() {
        let (mut p, ids) = project();
        for a in [GraphArrangement::PerVariable, GraphArrangement::PerGroup, GraphArrangement::AllInOne] {
            let spec = GraphSummarySpec { arrangement: a, ..Default::default() };
            assert!(matches!(run_summary(&mut p, &ids[..1], spec), Err(AnalysisError::BadOption(_))));
        }
        for level in [0.0, 100.0, f64::NAN] {
            let spec = GraphSummarySpec { level, ..Default::default() };
            assert!(run_summary(&mut p, &ids[..1], spec).is_err());
        }
        // Groups of one number are left out with a note; the other groups remain.
        let mut q = Project::new();
        let book = q.add_workbook("tiny");
        let y = q.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0, 4.0, 7.0, 9.0, 5.0])).unwrap();
        let g = q
            .add_column(
                book,
                0,
                Designation::Y,
                ColumnData::text(["x", "x", "x", "x", "x", "z"].map(String::from).to_vec()),
            )
            .unwrap();
        let r = run_summary(&mut q, &[y], GraphSummarySpec { groups: vec![g], ..Default::default() }).unwrap();
        assert_eq!(r.table.unwrap().rows.len(), 1);
        assert!(r.notes.iter().any(|n| n.contains("fewer than two")), "{:?}", r.notes);
        // Five numbers: no Anderson-Darling, noted, p is missing.
        assert!(r.notes.iter().any(|n| n.contains("Anderson-Darling test needs")));
    }

    fn two_columns(y: Vec<f64>, g: ColumnData) -> (Project, Vec<ColumnId>) {
        let mut q = Project::new();
        let book = q.add_workbook("g");
        let y = q.add_column(book, 0, Designation::Y, ColumnData::numeric(y)).unwrap();
        let g = q.add_column(book, 0, Designation::Y, g).unwrap();
        (q, vec![y, g])
    }

    #[test]
    fn close_numeric_groups_stay_apart_with_exact_labels() {
        let y = vec![1.0, 2.0, 3.0, 5.0];
        let (mut q, ids) = two_columns(y.clone(), ColumnData::numeric(vec![2023.5, 2023.5, 2024.0, 2024.0]));
        let r =
            run_summary(&mut q, &ids[..1], GraphSummarySpec { groups: vec![ids[1]], ..Default::default() }).unwrap();
        let t = r.table.unwrap();
        assert!(t.rows[0].0.ends_with(" 2023.5") && t.rows[1].0.ends_with(" 2024"), "{:?}", t.rows);
        let n = t.headers.iter().position(|x| x == "N").unwrap();
        assert_eq!((t.rows[0].1[n], t.rows[1].1[n]), (2.0, 2.0));
        let (mut q, ids) =
            two_columns(y, ColumnData::numeric(vec![0.1234567891, 0.1234567891, 0.1234567892, 0.1234567892]));
        let r =
            run_summary(&mut q, &ids[..1], GraphSummarySpec { groups: vec![ids[1]], ..Default::default() }).unwrap();
        assert_eq!(r.table.unwrap().rows.len(), 2);
    }

    #[test]
    fn mixed_group_keys_sort_totally_and_deterministically() {
        use GroupKey::{Num, Text};
        let mut keys: Vec<Vec<GroupKey>> = Vec::new();
        for i in 0..40 {
            keys.push(vec![if i % 3 == 0 {
                Text(format!("{}a", i))
            } else if i % 3 == 1 {
                Num(f64::from(i * 7 % 13).to_bits())
            } else {
                Text(["nan", "inf", "x"][i as usize % 3].to_string())
            }]);
        }
        keys.push(vec![Num(f64::INFINITY.to_bits())]);
        keys.push(vec![Num(2.0f64.to_bits())]);
        keys.push(vec![Num(10.0f64.to_bits())]);
        keys.push(vec![Text("1a".into())]);
        keys.dedup();
        let mut a = keys.clone();
        a.sort_by(|x, y| compare_keys(x, y));
        let mut b = keys.clone();
        b.reverse();
        b.sort_by(|x, y| compare_keys(x, y));
        assert_eq!(a, b);
        let first_text = a.iter().position(|k| matches!(k[0], Text(_))).unwrap();
        assert!(a[..first_text].iter().all(|k| matches!(k[0], Num(_))));
        assert!(a[first_text..].iter().all(|k| matches!(k[0], Text(_))));
        let nums: Vec<f64> =
            a[..first_text].iter().map(|k| if let Num(b) = k[0] { f64::from_bits(b) } else { 0.0 }).collect();
        assert!(nums.windows(2).all(|w| w[0].total_cmp(&w[1]).is_le()));
        // A real run with a mixed text column does not panic and puts 2 before 10 before 1a.
        let (mut q, ids) = two_columns(
            vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
            ColumnData::text(["10", "10", "2", "2", "1a", "1a"].map(String::from).to_vec()),
        );
        let r =
            run_summary(&mut q, &ids[..1], GraphSummarySpec { groups: vec![ids[1]], ..Default::default() }).unwrap();
        let ends: Vec<String> =
            r.table.unwrap().rows.iter().map(|r| r.0.rsplit(' ').next().unwrap().to_string()).collect();
        assert_eq!(ends.len(), 3);
        let mut sorted = ends.clone();
        sorted.sort();
        assert_eq!(ends, sorted);
    }

    #[test]
    fn infinite_spread_keeps_the_table_but_skips_the_graph() {
        let mut q = Project::new();
        let book = q.add_workbook("big");
        let y = q.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1e300, -1e300, 0.0])).unwrap();
        let r = run_summary(&mut q, &[y], GraphSummarySpec::default()).unwrap();
        assert_eq!(r.table.unwrap().rows.len(), 1);
        assert!(r.new_graph.is_none() && r.more_graphs.is_empty());
        assert!(r.notes.iter().any(|n| n.contains("not finite")), "{:?}", r.notes);
    }

    #[test]
    fn a_masked_group_cell_leaves_the_row_out() {
        // T255: as a masked value does.
        let (mut p, ids) = project();
        let table = |p: &mut Project| {
            let spec = GraphSummarySpec { groups: vec![ids[2]], ..Default::default() };
            // "Missing" (column 1) differs by design: a masked value is missing in its group, a row
            // whose group is masked belongs to no group (as a blank group cell).
            let t = run_summary(p, &ids[..1], spec).unwrap().table.unwrap();
            let rows: Vec<(String, Vec<f64>)> = t
                .rows
                .into_iter()
                .map(|(n, mut v)| {
                    v.remove(1);
                    (n, v)
                })
                .collect();
            format!("{rows:?}")
        };
        let all = table(&mut p);
        let mut gone = p.clone();
        gone.column_mut(ids[0]).unwrap().mask.insert(0);
        let want = table(&mut gone);
        assert_ne!(all, want);
        p.column_mut(ids[2]).unwrap().mask.insert(0);
        assert_eq!(table(&mut p), want);
    }
}
