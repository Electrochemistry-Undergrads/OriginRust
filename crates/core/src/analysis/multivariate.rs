//! The multivariate tools of `Project::analyze`. Principal component analysis (section 17) and k-means cluster analysis (section 18).
//! The numerics are in `sheaf_calc` (`pca`); what the tools compute and report is written down in
//! `docs/spec/statistics.md`, sections 17 and 18.

use super::dist::{GraphPlot, Look, Output, Overlay, XyGraph};
use super::statistics::num;
use super::*;
use crate::color::Color;
use crate::graph::{PlotKind, ScaleKind, SymbolShape};

pub(super) fn run(tool: &Tool, inputs: &[Input<'_>]) -> Result<Output, AnalysisError> {
    match tool {
        Tool::Pca(spec) => pca(spec, inputs),
        Tool::Kmeans(spec) => kmeans(spec, inputs),
        _ => unreachable!("only the multivariate tools come here"),
    }
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

/// The column with the title of the table of Origin's report that it belongs to written in its comment row.
fn titled(mut column: SheetColumn, table: &str) -> SheetColumn {
    column.comments = table.to_string();
    column
}

fn number_column(name: &str, designation: Designation, values: Vec<f64>) -> SheetColumn {
    SheetColumn {
        designation,
        long_name: name.to_string(),
        units: String::new(),
        comments: String::new(),
        data: ColumnData::numeric(values),
    }
}

// ----- principal component analysis -----------------------------------------------------------------------

/// What [`Tool::Pca`] does (Statistics > Multivariate Analysis > Principal Component Analysis): the chosen
/// columns are the variables, their rows the cases.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PcaSpec {
    /// Analyze the correlation matrix (true, Origin's default) or the covariance matrix.
    pub correlation: bool,
    /// Leave missing values out pairwise instead of dropping a whole row (listwise, Origin's default).
    pub pairwise: bool,
    /// The number of components whose loadings and scores are written (limited to the number of columns).
    pub components: usize,
    /// Divide each column of scores by the square root of its eigenvalue.
    pub standardize_scores: bool,
    /// Also write the scores of every row to the result sheet.
    pub scores: bool,
    /// Make a scree plot (eigenvalue against component number) as a new graph.
    pub scree: bool,
    /// Make a loading plot (the loadings of two components as vectors from the origin, labelled with the
    /// variable names; Origin's default is on).
    pub loading_plot: bool,
    /// Make a score plot (the scores of two components as points; Origin's default is off).
    pub score_plot: bool,
    /// Make a biplot (the scores as points and the loadings as vectors in one graph; Origin's default is on).
    pub biplot: bool,
    /// The components on the horizontal and the vertical axis of those plots, counted from 1 (Origin: 1 and 2).
    pub x_component: usize,
    pub y_component: usize,
}

impl Default for PcaSpec {
    fn default() -> Self {
        PcaSpec {
            correlation: true,
            pairwise: false,
            components: 2,
            standardize_scores: false,
            scores: true,
            scree: true,
            loading_plot: true,
            score_plot: false,
            biplot: true,
            x_component: 1,
            y_component: 2,
        }
    }
}

fn pca(spec: &PcaSpec, inputs: &[Input<'_>]) -> Result<Output, AnalysisError> {
    if spec.components == 0 {
        return Err(AnalysisError::BadOption("the number of components must be at least 1".into()));
    }
    let columns: Vec<Vec<f64>> = inputs.iter().map(|i| i.ys().to_vec()).collect();
    let options = calc::PcaOptions {
        correlation: spec.correlation,
        pairwise: spec.pairwise,
        components: spec.components,
        standardize_scores: spec.standardize_scores,
    };
    let r = calc::pca(&columns, &options).map_err(|e| AnalysisError::BadOption(e.to_string()))?;
    let p = inputs.len();
    let keep = r.loadings.len();
    let names: Vec<String> = inputs.iter().map(|i| i.label.clone()).collect();
    let pc = |k: usize| format!("PC{}", k + 1);

    // The table that is printed: one row per component.
    let mut headers = vec!["Eigenvalue", "Percent of variance", "Cumulative percent"];
    if r.tests.is_some() {
        headers.extend(["Chi-square", "DF", "Prob>Chi-square"]);
    }
    let mut table = ResultTable::new(&headers);
    for k in 0..p {
        let mut v = vec![r.eigenvalues[k], r.percent[k], r.cumulative[k]];
        if let Some(t) = &r.tests {
            v.extend([t[k].chi_square, t[k].df, t[k].p]);
        }
        let units = vec![String::new(); v.len()];
        table.push_row(pc(k), v, units);
    }

    // The result sheet follows the tables of Origin's report (spec section 17.1): descriptive statistics, the
    // correlation matrix, the eigenvalues, the eigenvectors; the title of each table is in the comment row of its
    // columns, and the labels are Origin's. The scores follow (Origin writes them to a sheet of their own).
    let matrix_name = if spec.correlation { "Correlation" } else { "Covariance" };
    let eigen_title = format!("Eigenvalues of the {matrix_name} Matrix");
    let mut on_sheet = vec![
        titled(label_column("Variable", names.clone()), "Descriptive Statistics"),
        titled(
            number_column("N analysis", Designation::Y, r.count.iter().map(|&n| n as f64).collect()),
            "Descriptive Statistics",
        ),
        titled(
            number_column("N missing", Designation::Y, r.missing.iter().map(|&n| n as f64).collect()),
            "Descriptive Statistics",
        ),
        titled(number_column("Mean", Designation::Y, r.mean.clone()), "Descriptive Statistics"),
        titled(number_column("Standard Deviation", Designation::Y, r.sd.clone()), "Descriptive Statistics"),
    ];
    for (j, row) in r.correlation.iter().enumerate() {
        on_sheet.push(titled(number_column(&inputs[j].short, Designation::Y, row.clone()), "Correlation Matrix"));
    }
    on_sheet.push(titled(label_column("Component", (0..p).map(pc).collect()), &eigen_title));
    on_sheet.push(titled(
        number_column("Principal Component Number", Designation::X, (1..=p).map(|k| k as f64).collect()),
        &eigen_title,
    ));
    let eigen_at = on_sheet.len();
    on_sheet.push(titled(number_column("Eigenvalue", Designation::Y, r.eigenvalues.clone()), &eigen_title));
    on_sheet.push(titled(number_column("Percentage of Variance", Designation::Y, r.percent.clone()), &eigen_title));
    on_sheet.push(titled(number_column("Cumulative", Designation::Y, r.cumulative.clone()), &eigen_title));
    if let Some(t) = &r.tests {
        on_sheet.push(titled(
            number_column("Chisq", Designation::Y, t.iter().map(|x| x.chi_square).collect()),
            &eigen_title,
        ));
        on_sheet.push(titled(
            number_column("Degrees of Freedom", Designation::Y, t.iter().map(|x| x.df).collect()),
            &eigen_title,
        ));
        on_sheet.push(titled(
            number_column("Significance Level", Designation::Y, t.iter().map(|x| x.p).collect()),
            &eigen_title,
        ));
    }
    for (k, v) in r.loadings.iter().enumerate() {
        on_sheet.push(titled(
            number_column(&format!("Coefficients of {}", pc(k)), Designation::Y, v.clone()),
            "Extracted Eigenvectors",
        ));
    }
    // The sheet column of the scores of each component, where they are written.
    let mut score_at: Vec<Option<usize>> = vec![None; keep];
    if spec.scores {
        for (k, s) in r.scores.iter().enumerate() {
            score_at[k] = Some(on_sheet.len());
            on_sheet.push(titled(
                number_column(&format!("Principal Component {}", k + 1), Designation::Y, s.clone()),
                "Scores",
            ));
        }
    }

    let mut notes = Vec::new();
    let matrix = if spec.correlation { "correlation" } else { "covariance" };
    let missing = if spec.pairwise { "pairwise" } else { "listwise" };
    let rows_used = r.count.iter().max().copied().unwrap_or(0);
    notes.push(format!(
        "Principal component analysis of the {matrix} matrix of {p} columns, {rows_used} rows used ({missing} exclusion of missing values)."
    ));
    for k in 0..p {
        notes.push(format!(
            "{}: eigenvalue {}, {}% of the variance, {}% cumulative.",
            pc(k),
            num(r.eigenvalues[k]),
            num(r.percent[k]),
            num(r.cumulative[k])
        ));
    }
    for (k, v) in r.loadings.iter().enumerate() {
        let parts: Vec<String> = inputs.iter().zip(v).map(|(i, w)| format!("{} {}", i.short, num(*w))).collect();
        notes.push(format!("Loadings of {}: {}.", pc(k), parts.join(", ")));
    }
    if let Some(t) = &r.tests {
        // Origin's remark under the table of eigenvalues of a covariance analysis (the one measured form).
        if let Some(k) = (0..p).find(|&k| t[k].p > 0.05) {
            notes.push(format!("At the 0.05 level, eigenvalues from {} to {p} are significantly equal.", k + 1));
        }
    }
    notes.push(
        "The signs of the eigenvectors are chosen so that the components of each sum to a positive number.".into(),
    );
    if spec.components > p {
        notes.push(format!("Only {keep} components exist; the number asked for ({}) was lowered.", spec.components));
    }
    if r.missing.iter().any(|&m| m > 0) && !spec.pairwise {
        let dropped = r.scores.first().map_or(0, |s| s.iter().filter(|v| v.is_nan()).count());
        notes.push(format!("{dropped} rows with a missing value were left out; their scores are empty."));
    }

    // The graphs, in Origin's order: scree plot, loading plot, (score plot,) biplot.
    let mut graphs: Vec<XyGraph> = Vec::new();
    if spec.scree {
        let red = Color::RED;
        graphs.push(XyGraph {
            plots: vec![GraphPlot {
                x: eigen_at - 1,
                y: eigen_at,
                kind: PlotKind::LineSymbol,
                label: "Eigenvalues".into(),
                line_of: None,
            }],
            x_scale: ScaleKind::Linear,
            y_scale: ScaleKind::Linear,
            x_title: "Principal Component Number".into(),
            y_title: "Eigenvalues".into(),
            look: Look {
                x_range: Some((0.5, p as f64 + 0.5)),
                x_step: Some(1.0),
                y_min: Some(0.0),
                colors: vec![(0, red)],
                symbols: vec![(0, SymbolShape::Square, 8.0, Some(Color::BLUE))],
                title: Some("Scree Plot".into()),
                no_legend: true,
                ..Default::default()
            },
        });
    }
    if spec.loading_plot || spec.score_plot || spec.biplot {
        let (xc, yc) = (spec.x_component.wrapping_sub(1), spec.y_component.wrapping_sub(1));
        if xc >= keep || yc >= keep || xc == yc {
            notes.push(format!(
                "The loading plot, score plot and biplot need two different components among the {keep} extracted                  (asked for {} and {}); they were not drawn.",
                spec.x_component, spec.y_component
            ));
        } else {
            let (x_title, y_title) =
                (format!("Principal Component {}", xc + 1), format!("Principal Component {}", yc + 1));
            // The loadings as vectors from the origin: (0, 0), the loading, a gap, for every variable.
            let (mut vx, mut vy, mut vl) = (Vec::new(), Vec::new(), Vec::new());
            for (j, input) in inputs.iter().enumerate() {
                vx.extend([0.0, r.loadings[xc][j], f64::NAN]);
                vy.extend([0.0, r.loadings[yc][j], f64::NAN]);
                vl.extend([String::new(), input.short.clone(), String::new()]);
            }
            let vec_at = on_sheet.len();
            on_sheet.push(number_column(&format!("Loading plot {}", pc(xc)), Designation::X, vx));
            on_sheet.push(number_column(&format!("Loading plot {}", pc(yc)), Designation::Y, vy));
            on_sheet.push(label_column("Loading plot label", vl));
            let vector_plot = || GraphPlot {
                x: vec_at,
                y: vec_at + 1,
                kind: PlotKind::Line,
                label: "Loadings".into(),
                line_of: None,
            };
            // The scores of the two components: written already, or added for the plot.
            let mut score_cols = [0usize; 2];
            if spec.score_plot || spec.biplot {
                for (slot, k) in [xc, yc].into_iter().enumerate() {
                    score_cols[slot] = match score_at[k] {
                        Some(at) => at,
                        None => {
                            on_sheet.push(number_column(
                                &format!("Principal Component {}", k + 1),
                                Designation::Y,
                                r.scores[k].clone(),
                            ));
                            on_sheet.len() - 1
                        }
                    };
                }
            }
            if spec.loading_plot {
                graphs.push(XyGraph {
                    plots: vec![vector_plot()],
                    x_scale: ScaleKind::Linear,
                    y_scale: ScaleKind::Linear,
                    x_title: x_title.clone(),
                    y_title: y_title.clone(),
                    look: Look {
                        cross_lines: true,
                        box_frame: true,
                        x_range: Some((-1.0, 1.0)),
                        y_range: Some((-1.0, 1.0)),
                        x_step: Some(0.2),
                        y_step: Some(0.2),
                        colors: vec![(0, Color::BLUE)],
                        labels: vec![(0, vec_at + 2, Color::BLUE)],
                        title: Some("Loading Plot".into()),
                        no_legend: true,
                        ..Default::default()
                    },
                });
            }
            let score_look = |colors: Vec<(usize, Color)>| Look {
                cross_lines: true,
                box_frame: true,
                colors,
                symbols: vec![(0, SymbolShape::Square, 3.0, Some(Color::RED))],
                ..Default::default()
            };
            let score_plot = || GraphPlot {
                x: score_cols[0],
                y: score_cols[1],
                kind: PlotKind::Scatter,
                label: "Scores".into(),
                line_of: None,
            };
            if spec.score_plot {
                graphs.push(XyGraph {
                    plots: vec![score_plot()],
                    x_scale: ScaleKind::Linear,
                    y_scale: ScaleKind::Linear,
                    x_title: x_title.clone(),
                    y_title: y_title.clone(),
                    look: Look {
                        title: Some("Scores Plot".into()),
                        no_legend: true,
                        ..score_look(vec![(0, Color::RED)])
                    },
                });
            }
            if spec.biplot {
                let axes = biplot_axes(&r.scores[xc], &r.scores[yc], &r.loadings[xc], &r.loadings[yc]);
                let mut look = score_look(vec![(0, Color::RED)]);
                // The loadings go against the top and right axes; without usable scores they get -1 to 1.
                let (x_range, y_range) = match &axes {
                    Some(a) => {
                        look.x_range = Some((a.x.0, a.x.1));
                        look.y_range = Some((a.y.0, a.y.1));
                        look.x_step = Some(a.x.2);
                        look.y_step = Some(a.y.2);
                        (a.loading_x, a.loading_y)
                    }
                    None => ((-1.0, 1.0), (-1.0, 1.0)),
                };
                look.overlay = Some(Overlay {
                    plots: vec![vector_plot()],
                    x_range,
                    y_range,
                    color: Color::BLUE,
                    labels: vec![(0, vec_at + 2)],
                });
                look.title = Some("Biplot".into());
                look.no_legend = true;
                graphs.push(XyGraph {
                    plots: vec![score_plot()],
                    x_scale: ScaleKind::Linear,
                    y_scale: ScaleKind::Linear,
                    x_title,
                    y_title,
                    look,
                });
            }
        }
    }
    let mut graphs = graphs.into_iter();
    let graph = graphs.next();
    let more_graphs: Vec<XyGraph> = graphs.collect();
    Ok(Output { table: Some(table), on_sheet, notes, graph, more_graphs })
}

/// The axes of a biplot, measured against Origin (spec section 17.1): the scores are drawn against the bottom and left
/// axes, whose limits are the usual automatic ones for the scores; the loadings are drawn against the top and right
/// axes, whose limits are those of the bottom and left axes times `max |loading| / min(|smallest score|, |largest score|)`
/// of the same component (so the longest loading reaches the nearer end of the scores).
struct BiplotAxes {
    /// Limits and major step of the score axes.
    x: (f64, f64, f64),
    y: (f64, f64, f64),
    /// Limits of the loading axes.
    loading_x: (f64, f64),
    loading_y: (f64, f64),
}

fn biplot_axes(sx: &[f64], sy: &[f64], lx: &[f64], ly: &[f64]) -> Option<BiplotAxes> {
    let axis = |scores: &[f64], loadings: &[f64]| {
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        for v in scores.iter().copied().filter(|v| v.is_finite()) {
            lo = lo.min(v);
            hi = hi.max(v);
        }
        if hi <= lo {
            return None;
        }
        let (from, to, step) = crate::autorange::linear_range(lo, hi, 8.0);
        let nearer = lo.abs().min(hi.abs());
        let longest = loadings.iter().fold(0.0f64, |m, v| m.max(v.abs()));
        if !(nearer > 0.0 && longest > 0.0) {
            return None;
        }
        let f = longest / nearer;
        Some(((from, to, step), (from * f, to * f)))
    };
    let (x, loading_x) = axis(sx, lx)?;
    let (y, loading_y) = axis(sy, ly)?;
    Some(BiplotAxes { x, y, loading_x, loading_y })
}

// ----- k-means cluster analysis -----------------------------------------------------------------------------

/// What [`Tool::Kmeans`] does (Statistics > Multivariate Analysis > K-Means Cluster Analysis): the chosen columns
/// are the variables, their rows the cases.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KmeansSpec {
    /// Number of clusters.
    pub clusters: usize,
    /// Upper limit of the passes of single-case moves.
    pub max_passes: usize,
    /// Analysis of variance of every variable by cluster.
    pub anova: bool,
    /// Write the cluster of every row (and, with `member_distance`, its distance to the cluster centre).
    pub membership: bool,
    pub member_distance: bool,
    /// Make a scatter graph of the first two variables with one plot per cluster.
    pub graph: bool,
}

impl Default for KmeansSpec {
    fn default() -> Self {
        KmeansSpec { clusters: 3, max_passes: 100, anova: true, membership: true, member_distance: true, graph: true }
    }
}

fn kmeans(spec: &KmeansSpec, inputs: &[Input<'_>]) -> Result<Output, AnalysisError> {
    let columns: Vec<Vec<f64>> = inputs.iter().map(|i| i.ys().to_vec()).collect();
    let options = calc::KmeansOptions { clusters: spec.clusters, max_passes: spec.max_passes };
    let r = calc::kmeans(&columns, &options).map_err(|e| AnalysisError::BadOption(e.to_string()))?;
    let p = inputs.len();
    let k = r.clusters.len();
    let names: Vec<String> = inputs.iter().map(|i| i.label.clone()).collect();
    let cl = |q: usize| format!("Cluster {}", q + 1);

    // The table that is printed: one row per cluster.
    let mut table = ResultTable::new(&["N", "Sum of squares", "Mean distance", "Max distance"]);
    for (q, c) in r.clusters.iter().enumerate() {
        table.push_row(
            cl(q),
            vec![c.count as f64, c.sum_squares, c.mean_distance, c.max_distance],
            vec![String::new(); 4],
        );
    }

    // The result sheet follows the tables of Origin's report (spec section 18.1): descriptive statistics, initial and
    // final cluster centres, the cluster summary, the distances between the final centres and the analysis of
    // variance; the title of each table is in the comment row of its columns. Then the cluster of every row.
    let cn = |q: usize| format!("Cluster{}", q + 1);
    let mut on_sheet = vec![
        titled(label_column("Variable", names.clone()), "Descriptive Statistics"),
        titled(
            number_column("N analysis", Designation::Y, r.count.iter().map(|&n| n as f64).collect()),
            "Descriptive Statistics",
        ),
        titled(
            number_column("N missing", Designation::Y, r.missing.iter().map(|&n| n as f64).collect()),
            "Descriptive Statistics",
        ),
        titled(number_column("Mean", Designation::Y, r.mean.clone()), "Descriptive Statistics"),
        titled(number_column("Standard Deviation", Designation::Y, r.sd.clone()), "Descriptive Statistics"),
        titled(label_column("Cluster", (0..k).map(cn).collect()), "Initial Cluster Center"),
    ];
    for (j, input) in inputs.iter().enumerate() {
        on_sheet.push(titled(
            number_column(&input.short, Designation::Y, r.clusters.iter().map(|c| c.initial[j]).collect()),
            "Initial Cluster Center",
        ));
    }
    for (j, input) in inputs.iter().enumerate() {
        on_sheet.push(titled(
            number_column(&input.short, Designation::Y, r.clusters.iter().map(|c| c.centre[j]).collect()),
            "Final Cluster Center",
        ));
    }
    let summary = |name: &str, f: &dyn Fn(&calc::Cluster) -> f64| {
        titled(number_column(name, Designation::Y, r.clusters.iter().map(f).collect()), "Cluster Summary")
    };
    on_sheet.push(summary("Number of Observations", &|c| c.count as f64));
    on_sheet.push(summary("Within Cluster Sum of Square", &|c| c.sum_squares));
    on_sheet.push(summary("Average Distance", &|c| c.mean_distance));
    on_sheet.push(summary("Maximum Distance", &|c| c.max_distance));
    for q in 0..k {
        on_sheet.push(titled(
            number_column(&cn(q), Designation::Y, r.centre_distances.iter().map(|row| row[q]).collect()),
            "Distance between Final Cluster Centers",
        ));
    }
    if spec.anova {
        on_sheet.push(titled(label_column("ANOVA variable", names.clone()), "ANOVA"));
        let pick = |f: &dyn Fn(&calc::VariableAnova) -> f64| r.anova.iter().map(f).collect::<Vec<f64>>();
        // Origin labels the two mean squares "Cluster SS" and "Error SS"; the values are the mean squares (F is
        // their quotient).
        for (name, values) in [
            ("Cluster DF", pick(&|a| a.df_between)),
            ("Cluster SS", pick(&|a| a.ms_between)),
            ("Error DF", pick(&|a| a.df_error)),
            ("Error SS", pick(&|a| a.ms_error)),
            ("F Value", pick(&|a| a.f)),
            ("Prob>F", pick(&|a| a.p)),
        ] {
            on_sheet.push(titled(number_column(name, Designation::Y, values), "ANOVA"));
        }
    }
    if spec.membership {
        let nan = f64::NAN;
        on_sheet.push(number_column(
            "Membership",
            Designation::Y,
            r.membership.iter().map(|m| m.map_or(nan, |(q, _)| (q + 1) as f64)).collect(),
        ));
        if spec.member_distance {
            on_sheet.push(number_column(
                "Distance",
                Designation::Y,
                r.membership.iter().map(|m| m.map_or(nan, |(_, d)| d)).collect(),
            ));
        }
    }

    let mut notes = Vec::new();
    let rows_used = r.membership.iter().filter(|m| m.is_some()).count();
    notes.push(format!(
        "K-means cluster analysis of {p} columns into {k} clusters, {rows_used} of {} rows used.",
        r.membership.len()
    ));
    for (q, c) in r.clusters.iter().enumerate() {
        let centre: Vec<String> =
            inputs.iter().zip(&c.centre).map(|(i, v)| format!("{} {}", i.short, num(*v))).collect();
        notes.push(format!(
            "{}: {} cases, centre ({}), sum of squares {}, mean distance {}, largest distance {}.",
            cl(q),
            c.count,
            centre.join(", "),
            num(c.sum_squares),
            num(c.mean_distance),
            num(c.max_distance)
        ));
    }
    if spec.anova {
        for (a, i) in r.anova.iter().zip(inputs) {
            notes.push(format!(
                "{}: F({}, {}) = {}, Prob>F {}.",
                i.short,
                num(a.df_between),
                num(a.df_error),
                num(a.f),
                num(a.p)
            ));
        }
    }
    notes.push(
        "Distances are Euclidean on the values as they are (not standardized). The initial centres are chosen from \
         the rows by replacement in sheet order, so the cluster numbers depend on the order of the rows."
            .into(),
    );
    if r.missing.iter().any(|&m| m > 0) {
        notes.push("Rows with a missing value in any column were left out (not measured against Origin).".into());
    }

    // The graph: the first two variables, one scatter plot per cluster.
    let mut graph = None;
    if spec.graph && p >= 2 {
        let mut plots = Vec::new();
        for q in 0..k {
            let (mut x, mut y) = (Vec::new(), Vec::new());
            for (row, m) in r.membership.iter().enumerate() {
                if matches!(m, Some((c, _)) if *c == q) {
                    x.push(columns[0].get(row).copied().unwrap_or(f64::NAN));
                    y.push(columns[1].get(row).copied().unwrap_or(f64::NAN));
                }
            }
            let at = on_sheet.len();
            on_sheet.push(number_column(&format!("{} X", cl(q)), Designation::X, x));
            on_sheet.push(number_column(&format!("{} Y", cl(q)), Designation::Y, y));
            plots.push(GraphPlot { x: at, y: at + 1, kind: PlotKind::Scatter, label: cl(q), line_of: None });
        }
        graph = Some(XyGraph {
            plots,
            x_scale: ScaleKind::Linear,
            y_scale: ScaleKind::Linear,
            x_title: names[0].clone(),
            y_title: names[1].clone(),
            look: Default::default(),
        });
    }
    Ok(Output { table: Some(table), on_sheet, notes, graph, more_graphs: Vec::new() })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Three columns of eight rows: the second is the first with a little noise, the third is unrelated.
    fn project() -> (Project, Vec<ColumnId>) {
        let a: Vec<f64> = (1..=8).map(f64::from).collect();
        let b: Vec<f64> = a.iter().enumerate().map(|(i, v)| 2.0 * v + if i % 2 == 0 { 0.3 } else { -0.3 }).collect();
        let c = vec![3.0, 1.0, 4.0, 1.0, 5.0, 9.0, 2.0, 6.0];
        let mut p = Project::new();
        let book = p.add_workbook("pca");
        let ids = [a, b, c]
            .into_iter()
            .map(|v| p.add_column(book, 0, Designation::Y, ColumnData::numeric(v)).unwrap())
            .collect();
        (p, ids)
    }

    #[test]
    fn report_sheet_and_scree_graph() {
        let (mut p, ids) = project();
        let r = p
            .analyze(&Request { columns: ids.clone(), rows: RowRange::All, tool: Tool::Pca(PcaSpec::default()) })
            .unwrap();
        assert!(r.message.contains("Principal component analysis") && r.message.contains("PCA1"), "{}", r.message);
        let t = r.table.unwrap();
        assert_eq!(t.rows.len(), 3);
        assert_eq!(t.headers[0], "Eigenvalue");
        // The eigenvalues of a correlation matrix add up to the number of columns.
        let sum: f64 = t.rows.iter().map(|(_, v)| v[0]).sum();
        assert!((sum - 3.0).abs() < 1e-9, "{sum}");
        assert!(t.rows[0].1[0] > 1.9, "two nearly equal columns give one big component");
        let sheet = &p.workbook(r.book).unwrap().sheets[r.new_sheet.unwrap()];
        let long: Vec<&str> = sheet.columns.iter().map(|c| c.long_name.as_str()).collect();
        assert!(
            long.contains(&"Coefficients of PC2") && long.contains(&"Principal Component 1") && long.contains(&"C"),
            "{long:?}"
        );
        assert!(!long.contains(&"Chi-square"), "a correlation analysis has no equality test");
        assert!(r.new_graph.is_some());
        let cov = p
            .analyze(&Request {
                columns: ids,
                rows: RowRange::All,
                tool: Tool::Pca(PcaSpec {
                    correlation: false,
                    scree: false,
                    loading_plot: false,
                    biplot: false,
                    components: 3,
                    ..Default::default()
                }),
            })
            .unwrap();
        assert!(cov.new_graph.is_none());
        assert_eq!(cov.table.unwrap().headers.len(), 6);
    }

    #[test]
    fn default_run_draws_scree_loading_plot_and_biplot_like_origin() {
        let (mut p, ids) = project();
        let r = p.analyze(&Request { columns: ids, rows: RowRange::All, tool: Tool::Pca(PcaSpec::default()) }).unwrap();
        // Origin's pca draws three graphs by default: scree plot, loading plot, biplot.
        assert_eq!(r.more_graphs.len(), 2);
        let titles = |g: WindowId| {
            let l = &p.graph(g).unwrap().layers[0];
            (l.x_axis.title.clone().unwrap(), l.y_axis.title.clone().unwrap())
        };
        assert_eq!(titles(r.new_graph.unwrap()), ("Principal Component Number".into(), "Eigenvalues".into()));
        assert!(r.message.contains("graphs Graph1"), "{}", r.message);
        let scree = &p.graph(r.new_graph.unwrap()).unwrap().layers[0];
        assert_eq!((scree.x_axis.min, scree.x_axis.max, scree.y_axis.min), (Some(0.5), Some(3.5), Some(0.0)));

        // The loading plot: vectors from the origin to the loadings, labelled with the variable names, on a fixed
        // -1 to 1 frame with lines through zero.
        let loading = p.graph(r.more_graphs[0]).unwrap();
        assert_eq!(titles(r.more_graphs[0]), ("Principal Component 1".into(), "Principal Component 2".into()));
        let l = &loading.layers[0];
        assert_eq!(l.plots.len(), 1);
        assert_eq!(
            (l.x_axis.min, l.x_axis.max, l.y_axis.min, l.y_axis.max),
            (Some(-1.0), Some(1.0), Some(-1.0), Some(1.0))
        );
        assert_eq!((l.x_axis.additional_line, l.y_axis.additional_line), (Some(0.0), Some(0.0)));
        assert!(l.plots[0].decor.labels.enable);
        let sheet = &p.workbook(r.book).unwrap().sheets[r.new_sheet.unwrap()];
        let column = |id| sheet.column(id).unwrap();
        let (vx, vy) = (column(l.plots[0].x.unwrap()), column(l.plots[0].y));
        let (vx, vy) = (vx.numeric().unwrap(), vy.numeric().unwrap());
        assert_eq!(vx.len(), 9, "three variables: origin, loading, gap");
        assert_eq!((vx[0], vy[0], vx[2].is_nan(), vy[5].is_nan()), (0.0, 0.0, true, true));
        // The tip of the vector is the loading of that variable on the two components.
        let load = |name: &str| column_values(sheet, name);
        assert_eq!((vx[1], vy[1]), (load("Coefficients of PC1")[0], load("Coefficients of PC2")[0]));
        let labels = sheet.columns.iter().find(|c| c.long_name == "Loading plot label").unwrap();
        assert_eq!(labels.data.text_cell(1), Some("A"));

        // The biplot: the scores as red points, the loadings in a second layer on the top and right axes.
        let bi = p.graph(r.more_graphs[1]).unwrap();
        assert_eq!(bi.layers.len(), 2);
        assert_eq!(bi.layers[1].x_axis.side, crate::graph::AxisSide::Opposite);
        assert_eq!(bi.layers[1].y_axis.side, crate::graph::AxisSide::Opposite);
        assert_eq!(bi.layers[0].plots.len(), 1);
        assert_eq!(bi.layers[1].plots.len(), 1);
        assert_eq!(bi.layers[0].plots[0].kind, PlotKind::Scatter);
        assert!(bi.layers[1].plots[0].decor.labels.enable);
    }

    /// Origin's biplot axes, `tools/origin-ref/pca-plots3.lines.txt` (raw output `plots/p7.out`, local): seven runs, two
    /// axes each. Per axis: smallest and largest score, the loadings' largest absolute value, then Origin's limits
    /// of the bottom/left axis and of the top/right axis.
    #[test]
    fn biplot_axes_match_origins_seven_runs() {
        #[rustfmt::skip]
        let axes: [(f64, f64, f64, (f64, f64), (f64, f64)); 14] = [
            (-1.62820, 2.96747, 0.58304, (-2.0, 4.0), (-0.71618, 1.43236)),
            (-1.42082, 1.62307, 0.85442, (-2.0, 2.0), (-1.202706, 1.202706)),
            (-9.99387, 17.26543, 0.79224, (-13.0, 20.0), (-1.030536, 1.58544)),
            (-6.32670, 7.33164, 0.69870, (-7.5, 10.0), (-0.82827, 1.10436)),
            (-2.50563, 1.94785, 0.52169, (-3.0, 3.0), (-0.803481, 0.803481)),
            (-1.47267, 1.92305, 0.73981, (-2.0, 2.5), (-1.00472, 1.2559)),
            (-1.12713, 2.05425, 0.58304, (-1.5, 2.5), (-0.7759215, 1.2932025)),
            (-1.45728, 1.66472, 0.85442, (-2.0, 2.0), (-1.17262, 1.17262)),
            (-1.42082, 1.62307, 0.85442, (-2.0, 2.0), (-1.202706, 1.202706)),
            (-1.27306, 1.57478, 0.83077, (-2.0, 2.0), (-1.305154, 1.305154)),
            (-2.42031, 2.33526, 0.58507, (-3.0, 3.0), (-0.751614, 0.751614)),
            (-0.34435, 0.47463, 0.82609, (-0.5, 0.6), (-1.199473, 1.4393676)),
            (-6.18052, 6.03765, 0.62193, (-8.0, 8.0), (-0.824064, 0.824064)),
            (-0.80581, 1.15172, 0.86663, (-1.0, 1.5), (-1.075477, 1.6132155)),
        ];
        let close = |a: f64, b: f64| (a - b).abs() <= 2e-4 * b.abs().max(1.0);
        let mut differ = Vec::new();
        for (run, pair) in axes.chunks(2).enumerate() {
            let (a, b) = (pair[0], pair[1]);
            let got = biplot_axes(&[a.0, a.1], &[b.0, b.1], &[a.2, -0.1], &[b.2, -0.1]).unwrap();
            for (n, (axis, (own, loading))) in
                [(a, (got.x, got.loading_x)), (b, (got.y, got.loading_y))].into_iter().enumerate()
            {
                // The loading axes are the score axes times max |loading| / the nearer end of the scores.
                let f = axis.2 / axis.0.abs().min(axis.1.abs());
                assert!(close(loading.0, own.0 * f) && close(loading.1, own.1 * f), "run {run} axis {n}");
                if !(close(own.0, axis.3.0) && close(own.1, axis.3.1)) {
                    differ.push((run, n, own, axis.3));
                    continue;
                }
                assert!(close(loading.0, axis.4.0) && close(loading.1, axis.4.1), "loadings {:?}: {loading:?}", axis.4);
            }
        }
        // Only the vertical axis of the covariance run of the first data set got other automatic limits (Origin:
        // -7.5 to 10 with step 5, from -6.33 to 7.33; Sheaf's automatic range gives -8 to 9): the automatic range of
        // `autorange.rs` was measured on other graphs.
        assert_eq!(differ.len(), 1, "{differ:?}");
        assert_eq!((differ[0].0, differ[0].1), (1, 1));
        // No spread in the scores: no scaling.
        assert!(biplot_axes(&[1.0, 1.0], &[0.0, 1.0], &[0.5], &[0.5]).is_none());
    }

    fn column_values(sheet: &crate::worksheet::Worksheet, name: &str) -> Vec<f64> {
        sheet.columns.iter().find(|c| c.long_name == name).unwrap().numeric().unwrap().to_vec()
    }

    #[test]
    fn score_plot_component_choice_and_plots_that_cannot_be_drawn() {
        let (mut p, ids) = project();
        let spec = PcaSpec {
            scree: false,
            loading_plot: false,
            biplot: false,
            score_plot: true,
            scores: false,
            components: 3,
            x_component: 2,
            y_component: 3,
            ..Default::default()
        };
        let r = p.analyze(&Request { columns: ids.clone(), rows: RowRange::All, tool: Tool::Pca(spec) }).unwrap();
        assert!(r.more_graphs.is_empty());
        let g = p.graph(r.new_graph.unwrap()).unwrap();
        let l = &g.layers[0];
        assert_eq!(l.x_axis.title.as_deref(), Some("Principal Component 2"));
        assert_eq!(l.y_axis.title.as_deref(), Some("Principal Component 3"));
        let sheet = &p.workbook(r.book).unwrap().sheets[r.new_sheet.unwrap()];
        // The scores were not asked for on the sheet, but the plot needs them (the two it shows).
        let x = sheet.column(l.plots[0].x.unwrap()).unwrap();
        assert_eq!(x.long_name, "Principal Component 2");
        assert_eq!(x.numeric().unwrap().len(), 8);
        assert!(!sheet.columns.iter().any(|c| c.long_name == "Principal Component 1"));
        assert_eq!(l.plots[0].kind, PlotKind::Scatter);

        // Components that were not extracted: no plot, and the sheet says so.
        let bad = PcaSpec { scree: false, x_component: 1, y_component: 3, components: 2, ..Default::default() };
        let r = p.analyze(&Request { columns: ids, rows: RowRange::All, tool: Tool::Pca(bad) }).unwrap();
        assert!(r.new_graph.is_none() && r.more_graphs.is_empty());
        assert!(r.notes.iter().any(|n| n.contains("were not drawn")), "{:?}", r.notes);
    }

    #[test]
    fn needs_two_columns_and_a_component() {
        let (mut p, ids) = project();
        let one = Request { columns: ids[..1].to_vec(), rows: RowRange::All, tool: Tool::Pca(PcaSpec::default()) };
        assert!(p.analyze(&one).is_err());
        let zero = PcaSpec { components: 0, ..Default::default() };
        let r = Request { columns: ids, rows: RowRange::All, tool: Tool::Pca(zero) };
        assert!(p.analyze(&r).is_err());
    }

    #[test]
    fn kmeans_report_sheet_and_cluster_graph() {
        // Two groups of three in two variables.
        let a = vec![1.0, 1.5, 2.0, 9.0, 9.5, 10.0];
        let b = vec![1.0, 2.0, 1.5, 9.0, 8.5, 9.5];
        let mut p = Project::new();
        let book = p.add_workbook("km");
        let ids: Vec<ColumnId> = [a, b]
            .into_iter()
            .map(|v| p.add_column(book, 0, Designation::Y, ColumnData::numeric(v)).unwrap())
            .collect();
        let spec = KmeansSpec { clusters: 2, ..Default::default() };
        let r = p.analyze(&Request { columns: ids.clone(), rows: RowRange::All, tool: Tool::Kmeans(spec) }).unwrap();
        assert!(r.message.contains("K-means") && r.message.contains("KMeans1"), "{}", r.message);
        let t = r.table.unwrap();
        assert_eq!(t.rows.len(), 2);
        assert_eq!((t.rows[0].1[0], t.rows[1].1[0]), (3.0, 3.0));
        let sheet = &p.workbook(r.book).unwrap().sheets[r.new_sheet.unwrap()];
        let long: Vec<&str> = sheet.columns.iter().map(|c| c.long_name.as_str()).collect();
        assert!(
            long.contains(&"Membership") && long.contains(&"Prob>F") && long.contains(&"Within Cluster Sum of Square"),
            "{long:?}"
        );
        // Origin's tables: the title of each is in the comment row of its columns, in Origin's order.
        let titles: Vec<&str> = sheet.columns.iter().map(|c| c.comments.as_str()).collect();
        let first = |t: &str| titles.iter().position(|x| *x == t).unwrap_or_else(|| panic!("no table {t}"));
        let order = [
            "Descriptive Statistics",
            "Initial Cluster Center",
            "Final Cluster Center",
            "Cluster Summary",
            "Distance between Final Cluster Centers",
            "ANOVA",
        ];
        assert!(order.windows(2).all(|w| first(w[0]) < first(w[1])), "{titles:?}");
        assert_eq!(p.graph(r.new_graph.unwrap()).unwrap().layers[0].plots.len(), 2);
        let bare = KmeansSpec { clusters: 2, anova: false, membership: false, graph: false, ..Default::default() };
        let r = p.analyze(&Request { columns: ids.clone(), rows: RowRange::All, tool: Tool::Kmeans(bare) }).unwrap();
        assert!(r.new_graph.is_none());
        let too_many = KmeansSpec { clusters: 7, ..Default::default() };
        assert!(p.analyze(&Request { columns: ids, rows: RowRange::All, tool: Tool::Kmeans(too_many) }).is_err());
    }
}
