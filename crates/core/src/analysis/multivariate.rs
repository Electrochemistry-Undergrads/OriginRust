//! The multivariate tools of `Project::analyze`. Principal component analysis (section 17) and k-means cluster analysis (section 18).
//! The numerics are in `sheaf_calc` (`pca`); what the tools compute and report is written down in
//! `docs/spec/statistics.md`, sections 17 and 18.

use super::dist::{GraphPlot, Output, XyGraph};
use super::statistics::num;
use super::*;
use crate::graph::{PlotKind, ScaleKind};

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

    // The result sheet: descriptive statistics and the eigenvalue table side by side (both have one row per
    // variable), then the loadings, the correlation matrix and the scores.
    let mut on_sheet = vec![
        label_column("Variable", names.clone()),
        number_column("N", Designation::Y, r.count.iter().map(|&n| n as f64).collect()),
        number_column("Missing", Designation::Y, r.missing.iter().map(|&n| n as f64).collect()),
        number_column("Mean", Designation::Y, r.mean.clone()),
        number_column("SD", Designation::Y, r.sd.clone()),
        label_column("Component", (0..p).map(pc).collect()),
        number_column("Component number", Designation::X, (1..=p).map(|k| k as f64).collect()),
        number_column("Eigenvalue", Designation::Y, r.eigenvalues.clone()),
        number_column("Percent of variance", Designation::Y, r.percent.clone()),
        number_column("Cumulative percent", Designation::Y, r.cumulative.clone()),
    ];
    let eigen_at = on_sheet.len() - 3;
    if let Some(t) = &r.tests {
        on_sheet.push(number_column("Chi-square", Designation::Y, t.iter().map(|x| x.chi_square).collect()));
        on_sheet.push(number_column("DF", Designation::Y, t.iter().map(|x| x.df).collect()));
        on_sheet.push(number_column("Prob>Chi-square", Designation::Y, t.iter().map(|x| x.p).collect()));
    }
    for (k, v) in r.loadings.iter().enumerate() {
        on_sheet.push(number_column(&format!("Loading {}", pc(k)), Designation::Y, v.clone()));
    }
    for (j, row) in r.correlation.iter().enumerate() {
        on_sheet.push(number_column(&format!("Correlation with {}", inputs[j].short), Designation::Y, row.clone()));
    }
    if spec.scores {
        for (k, s) in r.scores.iter().enumerate() {
            on_sheet.push(number_column(&format!("Score {}", pc(k)), Designation::Y, s.clone()));
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

    let graph = spec.scree.then(|| XyGraph {
        plots: vec![GraphPlot {
            x: eigen_at - 1,
            y: eigen_at,
            kind: PlotKind::LineSymbol,
            label: "Eigenvalue".into(),
            line_of: None,
        }],
        x_scale: ScaleKind::Linear,
        y_scale: ScaleKind::Linear,
        x_title: "Component number".into(),
        y_title: "Eigenvalue".into(),
    });
    Ok(Output { table: Some(table), on_sheet, notes, graph })
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

    // The result sheet: statistics of the variables, the clusters (initial and final centres, sizes, distances),
    // the analysis of variance, then the cluster of every row.
    let mut on_sheet = vec![
        label_column("Variable", names.clone()),
        number_column("N", Designation::Y, r.count.iter().map(|&n| n as f64).collect()),
        number_column("Missing", Designation::Y, r.missing.iter().map(|&n| n as f64).collect()),
        number_column("Mean", Designation::Y, r.mean.clone()),
        number_column("SD", Designation::Y, r.sd.clone()),
        label_column("Cluster", (0..k).map(cl).collect()),
    ];
    for (j, input) in inputs.iter().enumerate() {
        on_sheet.push(number_column(
            &format!("Initial centre {}", input.short),
            Designation::Y,
            r.clusters.iter().map(|c| c.initial[j]).collect(),
        ));
    }
    for (j, input) in inputs.iter().enumerate() {
        on_sheet.push(number_column(
            &format!("Centre {}", input.short),
            Designation::Y,
            r.clusters.iter().map(|c| c.centre[j]).collect(),
        ));
    }
    on_sheet.push(number_column("Cluster N", Designation::Y, r.clusters.iter().map(|c| c.count as f64).collect()));
    on_sheet.push(number_column("Sum of squares", Designation::Y, r.clusters.iter().map(|c| c.sum_squares).collect()));
    on_sheet.push(number_column("Mean distance", Designation::Y, r.clusters.iter().map(|c| c.mean_distance).collect()));
    on_sheet.push(number_column("Max distance", Designation::Y, r.clusters.iter().map(|c| c.max_distance).collect()));
    for q in 0..k {
        on_sheet.push(number_column(
            &format!("Distance to {}", cl(q)),
            Designation::Y,
            r.centre_distances.iter().map(|row| row[q]).collect(),
        ));
    }
    if spec.anova {
        on_sheet.push(label_column("ANOVA variable", names.clone()));
        let pick = |f: &dyn Fn(&calc::VariableAnova) -> f64| r.anova.iter().map(f).collect::<Vec<f64>>();
        on_sheet.push(number_column("DF between", Designation::Y, pick(&|a| a.df_between)));
        on_sheet.push(number_column("MS between", Designation::Y, pick(&|a| a.ms_between)));
        on_sheet.push(number_column("DF error", Designation::Y, pick(&|a| a.df_error)));
        on_sheet.push(number_column("MS error", Designation::Y, pick(&|a| a.ms_error)));
        on_sheet.push(number_column("F", Designation::Y, pick(&|a| a.f)));
        on_sheet.push(number_column("Prob>F", Designation::Y, pick(&|a| a.p)));
    }
    if spec.membership {
        let nan = f64::NAN;
        on_sheet.push(number_column(
            "Cluster number",
            Designation::Y,
            r.membership.iter().map(|m| m.map_or(nan, |(q, _)| (q + 1) as f64)).collect(),
        ));
        if spec.member_distance {
            on_sheet.push(number_column(
                "Distance to centre",
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
        });
    }
    Ok(Output { table: Some(table), on_sheet, notes, graph })
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
            long.contains(&"Loading PC2") && long.contains(&"Score PC1") && long.contains(&"Correlation with C"),
            "{long:?}"
        );
        assert!(!long.contains(&"Chi-square"), "a correlation analysis has no equality test");
        assert!(r.new_graph.is_some());
        let cov = p
            .analyze(&Request {
                columns: ids,
                rows: RowRange::All,
                tool: Tool::Pca(PcaSpec { correlation: false, scree: false, components: 3, ..Default::default() }),
            })
            .unwrap();
        assert!(cov.new_graph.is_none());
        assert_eq!(cov.table.unwrap().headers.len(), 6);
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
        assert!(long.contains(&"Cluster number") && long.contains(&"Prob>F") && long.contains(&"Centre B"), "{long:?}");
        assert_eq!(p.graph(r.new_graph.unwrap()).unwrap().layers[0].plots.len(), 2);
        let bare = KmeansSpec { clusters: 2, anova: false, membership: false, graph: false, ..Default::default() };
        let r = p.analyze(&Request { columns: ids.clone(), rows: RowRange::All, tool: Tool::Kmeans(bare) }).unwrap();
        assert!(r.new_graph.is_none());
        let too_many = KmeansSpec { clusters: 7, ..Default::default() };
        assert!(p.analyze(&Request { columns: ids, rows: RowRange::All, tool: Tool::Kmeans(too_many) }).is_err());
    }
}
