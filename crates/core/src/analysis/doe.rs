//! Statistics > Design of Experiments: Two-Level Factorial Design and Factorial Effects as tools of
//! `Project::analyze` (T380; the base of Origin 2027's two-level split-plot designs, not in Origin 2016, so the layout
//! is Sheaf's own). The numerics are in `sheaf_calc::doe`; `docs/spec/statistics.md` section 45 says what is computed
//! and written.

use super::dist::{GraphPlot, Output, XyGraph};
use super::statistics::num;
use super::*;
use crate::graph::{PlotKind, ScaleKind};
use calc::doe::{design, effects, letter, split_plot_design, split_plot_effects};

/// What [`Tool::FactorialDesign`] makes (it reads no data; the results go next to the chosen sheet).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactorialDesignSpec {
    pub factors: usize,
    /// `p` of a `2^(k-p)` fraction (0: full design).
    pub fraction: usize,
    pub replicates: usize,
    pub center_points: usize,
    pub randomize: bool,
    pub seed: u64,
}

impl Default for FactorialDesignSpec {
    fn default() -> Self {
        FactorialDesignSpec { factors: 3, fraction: 0, replicates: 1, center_points: 0, randomize: true, seed: 1 }
    }
}

/// What [`Tool::FactorialEffects`] does: the chosen columns are the factors, then the response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactorialEffectsSpec {
    pub graph: bool,
}

impl Default for FactorialEffectsSpec {
    fn default() -> Self {
        FactorialEffectsSpec { graph: true }
    }
}

/// What [`Tool::SplitPlotDesign`] makes (it reads no data; the results go next to the chosen sheet).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplitPlotDesignSpec {
    pub whole_plot_factors: usize,
    pub subplot_factors: usize,
    /// Replicates of the whole plots.
    pub replicates: usize,
    pub randomize: bool,
    pub seed: u64,
}

impl Default for SplitPlotDesignSpec {
    fn default() -> Self {
        SplitPlotDesignSpec { whole_plot_factors: 1, subplot_factors: 2, replicates: 2, randomize: true, seed: 1 }
    }
}

/// What [`Tool::SplitPlotEffects`] does: the chosen columns are the whole plot, the factors (whole-plot factors
/// first), then the response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplitPlotEffectsSpec {
    pub whole_plot_factors: usize,
    pub graph: bool,
}

impl Default for SplitPlotEffectsSpec {
    fn default() -> Self {
        SplitPlotEffectsSpec { whole_plot_factors: 1, graph: true }
    }
}

fn column_of(name: &str, designation: Designation, values: Vec<f64>) -> SheetColumn {
    SheetColumn {
        designation,
        long_name: name.to_string(),
        units: String::new(),
        comments: String::new(),
        data: ColumnData::numeric(values),
    }
}

pub(super) fn run_design(spec: &FactorialDesignSpec) -> Result<Output, AnalysisError> {
    let d = design(spec.factors, spec.fraction, spec.replicates, spec.center_points, spec.randomize, spec.seed)
        .map_err(|e| AnalysisError::BadOption(e.to_string()))?;
    let n = d.runs.len();
    let mut table = ResultTable::new(&["Factors", "Fraction p", "Runs", "Resolution"]);
    let label = if spec.fraction == 0 {
        format!("2^{} full factorial", spec.factors)
    } else {
        format!("2^({}-{}) fractional factorial", spec.factors, spec.fraction)
    };
    table.push_row(
        label.clone(),
        vec![spec.factors as f64, spec.fraction as f64, n as f64, d.resolution.map_or(f64::NAN, |r| r as f64)],
        vec![String::new(); 4],
    );
    let mut notes = vec![format!(
        "{label}: {n} runs ({} replicate(s), {} centre point(s)){}; {}.",
        spec.replicates,
        spec.center_points,
        match d.resolution {
            Some(r) => format!(", resolution {r}, generators {}", d.generator_text.join(", ")),
            None => String::new(),
        },
        if spec.randomize { format!("run order shuffled with seed {}", spec.seed) } else { "standard order".into() }
    )];
    notes.push(
        "Levels are coded -1 (low) and +1 (high), 0 at centre points; replace them with real settings as needed."
            .into(),
    );
    let mut on_sheet = vec![
        column_of("Run order", Designation::X, (1..=n).map(|i| i as f64).collect()),
        column_of("Standard order", Designation::Y, d.standard.iter().map(|s| *s as f64).collect()),
    ];
    for j in 0..spec.factors {
        on_sheet.push(column_of(&letter(j).to_string(), Designation::Y, d.runs.iter().map(|r| r[j]).collect()));
    }
    on_sheet.push(column_of("Response", Designation::Y, vec![f64::NAN; n]));
    Ok(Output { table: Some(table), on_sheet, notes, graph: None, more_graphs: Vec::new() })
}

pub(super) fn run_effects(
    spec: &FactorialEffectsSpec,
    sheet: &Worksheet,
    columns: &[ColumnId],
    rows: RowRange,
) -> Result<Output, AnalysisError> {
    if columns.len() < 2 {
        return Err(AnalysisError::BadOption(
            "factorial effects need the factor column(s) first and the response column last".into(),
        ));
    }
    let column = |id: ColumnId| sheet.column(id).ok_or(AnalysisError::NoSuchColumn(id));
    let read = |c: &Column| -> Result<Vec<f64>, AnalysisError> {
        sheet.values_of(c).map(|v| v.into_owned()).ok_or_else(|| AnalysisError::NotNumeric(c.short_name.clone()))
    };
    let (y_id, x_ids) = columns.split_last().expect("two columns");
    let yc = column(*y_id)?;
    let y_all = read(yc)?;
    let xcs: Vec<&Column> = x_ids.iter().map(|id| column(*id)).collect::<Result<_, _>>()?;
    let x_all: Vec<Vec<f64>> = xcs.iter().map(|c| read(c)).collect::<Result<_, _>>()?;
    let n = x_all.iter().map(Vec::len).chain([y_all.len()]).max().unwrap_or(0);
    if n == 0 {
        return Err(AnalysisError::Empty(yc.short_name.clone()));
    }
    let (first, last) = resolve_rows(rows, &yc.short_name, &[], n)?;
    let mut x = Vec::new();
    let mut y = Vec::new();
    for r in first..=last {
        let row: Vec<f64> = x_all.iter().map(|c| c.get(r).copied().unwrap_or(f64::NAN)).collect();
        let v = y_all.get(r).copied().unwrap_or(f64::NAN);
        if v.is_finite() && row.iter().all(|a| a.is_finite()) {
            x.push(row);
            y.push(v);
        }
    }
    if y.is_empty() {
        return Err(AnalysisError::BadOption("no row has every factor and the response".into()));
    }
    let e = effects(&x, &y).map_err(|e| AnalysisError::BadOption(e.to_string()))?;
    // Term letters are factor positions; name them by the factor columns.
    let names: Vec<String> = xcs.iter().map(|c| column_label(c)).collect();
    let term_name = |t: &str| -> String {
        t.chars().map(|ch| names[(ch as u8 - b'A') as usize].clone()).collect::<Vec<_>>().join(" x ")
    };
    let mut table = ResultTable::new(&["Coefficient", "Effect", "SE of effect", "t", "Prob>|t|", "Active (Lenth)"]);
    table.push_row(
        "Intercept".into(),
        vec![e.intercept, f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN],
        vec![String::new(); 6],
    );
    for f in &e.effects {
        let active = if f.effect.abs() > e.margin { 1.0 } else { 0.0 };
        table.push_row(
            term_name(&f.term),
            vec![f.coefficient, f.effect, f.se, f.t, f.p, active],
            vec![String::new(); 6],
        );
    }
    let mut notes = vec![format!(
        "{} runs, {} term(s), {} residual df; Lenth PSE {}, margin of error {}{}.",
        y.len(),
        e.effects.len(),
        e.df,
        num(e.pse),
        num(e.margin),
        if e.df > 0 {
            format!(", residual mean square {}", num(e.mse))
        } else {
            " (no error estimate: judge effects by Lenth's margin)".into()
        }
    )];
    if !e.aliased.is_empty() {
        let al: Vec<String> = e.aliased.iter().map(|(a, b)| format!("{} = {}", term_name(a), term_name(b))).collect();
        notes.push(format!("Aliased terms left out: {}.", al.join(", ")));
    }
    let mut sheet_cols = vec![SheetColumn {
        designation: Designation::Label,
        long_name: "Term".into(),
        units: String::new(),
        comments: String::new(),
        data: ColumnData::text(table.rows.iter().map(|(l, _)| l.clone()).collect()),
    }];
    for (i, h) in table.headers.iter().enumerate() {
        sheet_cols.push(column_of(h, Designation::Y, table.rows.iter().map(|(_, v)| v[i]).collect()));
    }
    let shift = sheet_cols.len();
    let m = e.effects.len();
    let on_sheet = vec![
        column_of("Term index", Designation::X, (1..=m).map(|i| i as f64).collect()),
        column_of("|Effect|", Designation::Y, e.effects.iter().map(|f| f.effect.abs()).collect()),
        column_of("Margin X", Designation::X, vec![0.5, m as f64 + 0.5]),
        column_of("Lenth margin of error", Designation::Y, vec![e.margin, e.margin]),
    ];
    let graph = spec.graph.then(|| XyGraph {
        plots: vec![
            GraphPlot { x: 0, y: 1, kind: PlotKind::Column, label: "|Effect|".into(), line_of: None },
            GraphPlot {
                x: 2,
                y: 3,
                kind: PlotKind::Line,
                label: format!("Margin of error {}", num(e.margin)),
                line_of: None,
            },
        ],
        x_scale: ScaleKind::Linear,
        y_scale: ScaleKind::Linear,
        x_title: "Term (row order of the table)".into(),
        y_title: "|Effect|".into(),
        look: Default::default(),
    });
    sheet_cols.extend(on_sheet);
    Ok(Output { table: Some(table), on_sheet: sheet_cols, notes, graph, more_graphs: Vec::new() }.shift_graphs(shift))
}

pub(super) fn run_split_design(spec: &SplitPlotDesignSpec) -> Result<Output, AnalysisError> {
    let d =
        split_plot_design(spec.whole_plot_factors, spec.subplot_factors, spec.replicates, spec.randomize, spec.seed)
            .map_err(|e| AnalysisError::BadOption(e.to_string()))?;
    let n = d.runs.len();
    let plots = d.plot.last().copied().unwrap_or(0);
    let k = spec.whole_plot_factors + spec.subplot_factors;
    let mut table = ResultTable::new(&["Whole-plot factors", "Subplot factors", "Whole plots", "Runs"]);
    table.push_row(
        "Split-plot design".into(),
        vec![spec.whole_plot_factors as f64, spec.subplot_factors as f64, plots as f64, n as f64],
        vec![String::new(); 4],
    );
    let wp_names: String = (0..spec.whole_plot_factors).map(letter).collect();
    let notes = vec![
        format!(
            "Split-plot design: hard-to-change factor(s) {wp_names} set once per whole plot ({plots} whole plots, {} replicate(s)), {} subplot runs in each; {n} runs; {}.",
            spec.replicates,
            n / plots.max(1),
            if spec.randomize {
                format!("plots and subplots shuffled with seed {}", spec.seed)
            } else {
                "standard order".into()
            }
        ),
        "Levels are coded -1 and +1; analyse with Split-Plot Effects (whole plot, factors, response).".into(),
    ];
    let mut on_sheet = vec![
        column_of("Run order", Designation::X, (1..=n).map(|i| i as f64).collect()),
        column_of("Whole plot", Designation::Y, d.plot.iter().map(|p| *p as f64).collect()),
    ];
    for j in 0..k {
        on_sheet.push(column_of(&letter(j).to_string(), Designation::Y, d.runs.iter().map(|r| r[j]).collect()));
    }
    on_sheet.push(column_of("Response", Designation::Y, vec![f64::NAN; n]));
    Ok(Output { table: Some(table), on_sheet, notes, graph: None, more_graphs: Vec::new() })
}

pub(super) fn run_split_effects(
    spec: &SplitPlotEffectsSpec,
    sheet: &Worksheet,
    columns: &[ColumnId],
    rows: RowRange,
) -> Result<Output, AnalysisError> {
    if columns.len() < 4 {
        return Err(AnalysisError::BadOption(
            "split-plot effects need the whole-plot column, the factor columns (whole-plot factors first) and the response".into(),
        ));
    }
    let column = |id: ColumnId| sheet.column(id).ok_or(AnalysisError::NoSuchColumn(id));
    let read = |c: &Column| -> Result<Vec<f64>, AnalysisError> {
        sheet.values_of(c).map(|v| v.into_owned()).ok_or_else(|| AnalysisError::NotNumeric(c.short_name.clone()))
    };
    let pc = column(columns[0])?;
    let (y_id, x_ids) = columns[1..].split_last().expect("factors and response");
    let yc = column(*y_id)?;
    let plot_all = read(pc)?;
    let y_all = read(yc)?;
    let xcs: Vec<&Column> = x_ids.iter().map(|id| column(*id)).collect::<Result<_, _>>()?;
    let x_all: Vec<Vec<f64>> = xcs.iter().map(|c| read(c)).collect::<Result<_, _>>()?;
    let n = x_all.iter().map(Vec::len).chain([y_all.len(), plot_all.len()]).max().unwrap_or(0);
    if n == 0 {
        return Err(AnalysisError::Empty(yc.short_name.clone()));
    }
    let (first, last) = resolve_rows(rows, &yc.short_name, &[], n)?;
    let (mut x, mut y, mut plot) = (Vec::new(), Vec::new(), Vec::new());
    for r in first..=last {
        let row: Vec<f64> = x_all.iter().map(|c| c.get(r).copied().unwrap_or(f64::NAN)).collect();
        let v = y_all.get(r).copied().unwrap_or(f64::NAN);
        let w = plot_all.get(r).copied().unwrap_or(f64::NAN);
        if v.is_finite() && w.is_finite() && w >= 0.0 && row.iter().all(|a| a.is_finite()) {
            x.push(row);
            y.push(v);
            plot.push(w as usize);
        }
    }
    if y.is_empty() {
        return Err(AnalysisError::BadOption("no row has the whole plot, every factor and the response".into()));
    }
    let s = split_plot_effects(&x, &y, &plot, spec.whole_plot_factors)
        .map_err(|e| AnalysisError::BadOption(e.to_string()))?;
    let names: Vec<String> = xcs.iter().map(|c| column_label(c)).collect();
    let term_name = |t: &str| -> String {
        t.chars().map(|ch| names[(ch as u8 - b'A') as usize].clone()).collect::<Vec<_>>().join(" x ")
    };
    let mut table = ResultTable::new(&["Whole-plot term", "Effect", "SE of effect", "t", "Prob>|t|", "Active (Lenth)"]);
    for (e, w) in s.fit.effects.iter().zip(&s.whole) {
        let st = if *w { s.whole_plot } else { s.subplot };
        let active = if e.effect.abs() > st.margin { 1.0 } else { 0.0 };
        table.push_row(
            term_name(&e.term),
            vec![if *w { 1.0 } else { 0.0 }, e.effect, e.se, e.t, e.p, active],
            vec![String::new(); 6],
        );
    }
    let stratum = |name: &str, st: calc::doe::Stratum| {
        if st.df > 0 {
            format!("{name} error: SS {}, {} df, mean square {}", num(st.ss), st.df, num(st.ms))
        } else {
            format!("{name} error: no degrees of freedom (Lenth margin {} over its effects)", num(st.margin))
        }
    };
    let notes = vec![
        format!(
            "{} runs in {} whole plots; whole-plot terms are tested against the whole-plot error, the others against the subplot error.",
            y.len(),
            {
                let mut ids = plot.clone();
                ids.sort_unstable();
                ids.dedup();
                ids.len()
            }
        ),
        format!("{}; {}.", stratum("Whole-plot", s.whole_plot), stratum("Subplot", s.subplot)),
    ];
    let mut sheet_cols = vec![SheetColumn {
        designation: Designation::Label,
        long_name: "Term".into(),
        units: String::new(),
        comments: String::new(),
        data: ColumnData::text(table.rows.iter().map(|(l, _)| l.clone()).collect()),
    }];
    for (i, h) in table.headers.iter().enumerate() {
        sheet_cols.push(column_of(h, Designation::Y, table.rows.iter().map(|(_, v)| v[i]).collect()));
    }
    let shift = sheet_cols.len();
    let m = s.fit.effects.len();
    let on_sheet = vec![
        column_of("Term index", Designation::X, (1..=m).map(|i| i as f64).collect()),
        column_of("|Effect|", Designation::Y, s.fit.effects.iter().map(|e| e.effect.abs()).collect()),
    ];
    let graph = spec.graph.then(|| XyGraph {
        plots: vec![GraphPlot { x: 0, y: 1, kind: PlotKind::Column, label: "|Effect|".into(), line_of: None }],
        x_scale: ScaleKind::Linear,
        y_scale: ScaleKind::Linear,
        x_title: "Term (row order of the table)".into(),
        y_title: "|Effect|".into(),
        look: Default::default(),
    });
    sheet_cols.extend(on_sheet);
    Ok(Output { table: Some(table), on_sheet: sheet_cols, notes, graph, more_graphs: Vec::new() }.shift_graphs(shift))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_design_sheet_and_its_effects() {
        let mut p = Project::new();
        let book = p.add_workbook("doe");
        let host = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![0.0])).unwrap();
        let spec = FactorialDesignSpec { factors: 4, fraction: 1, randomize: false, ..Default::default() };
        let out = p
            .analyze(&Request { columns: vec![host], rows: RowRange::All, tool: Tool::FactorialDesign(spec) })
            .unwrap();
        assert!(out.notes[0].contains("resolution 4, generators D = ABC"), "{:?}", out.notes);
        assert_eq!(out.table.as_ref().unwrap().rows[0].1[2], 8.0);

        let a = vec![-1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0];
        let b = vec![-1.0, -1.0, 1.0, 1.0, -1.0, -1.0, 1.0, 1.0];
        let c = vec![-1.0, -1.0, -1.0, -1.0, 1.0, 1.0, 1.0, 1.0];
        let y: Vec<f64> = (0..8).map(|i| 10.0 + 3.0 * a[i] + 0.1 * (i as f64 * 1.3).sin()).collect();
        let cols: Vec<ColumnId> = [a, b, c, y]
            .into_iter()
            .map(|v| p.add_column(book, 0, Designation::Y, ColumnData::numeric(v)).unwrap())
            .collect();
        let out = p
            .analyze(&Request { columns: cols, rows: RowRange::All, tool: Tool::FactorialEffects(Default::default()) })
            .unwrap();
        let t = out.table.as_ref().unwrap();
        assert_eq!(t.rows[1].0, "B");
        assert!((t.rows[1].1[1] - 6.0).abs() < 0.2, "{:?}", t.rows[1]);
        assert_eq!(t.rows[1].1[5], 1.0, "A is active");
        let gc = super::super::dist::plot_columns(&p, out.new_graph.unwrap());
        assert_eq!(gc[0].1, "|Effect|");
    }

    #[test]
    fn a_split_plot_design_and_its_two_strata() {
        let mut p = Project::new();
        let book = p.add_workbook("sp");
        let host = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![0.0])).unwrap();
        let spec = SplitPlotDesignSpec { randomize: false, ..Default::default() };
        let out = p
            .analyze(&Request { columns: vec![host], rows: RowRange::All, tool: Tool::SplitPlotDesign(spec) })
            .unwrap();
        assert!(
            out.notes[0].contains("(4 whole plots, 2 replicate(s)), 4 subplot runs in each; 16 runs"),
            "{:?}",
            out.notes
        );
        let d = calc::doe::split_plot_design(1, 2, 2, false, 1).unwrap();
        let bump = [0.3, -0.2, 0.1, -0.2];
        let y: Vec<f64> = d
            .runs
            .iter()
            .zip(&d.plot)
            .enumerate()
            .map(|(i, (r, w))| 5.0 + 2.0 * r[0] + r[1] + bump[w - 1] + 0.05 * (i as f64 * 1.7).sin())
            .collect();
        let mut cols = vec![
            p.add_column(book, 0, Designation::Y, ColumnData::numeric(d.plot.iter().map(|w| *w as f64).collect()))
                .unwrap(),
        ];
        for j in 0..3 {
            cols.push(
                p.add_column(book, 0, Designation::Y, ColumnData::numeric(d.runs.iter().map(|r| r[j]).collect()))
                    .unwrap(),
            );
        }
        cols.push(p.add_column(book, 0, Designation::Y, ColumnData::numeric(y)).unwrap());
        let out = p
            .analyze(&Request { columns: cols, rows: RowRange::All, tool: Tool::SplitPlotEffects(Default::default()) })
            .unwrap();
        let t = out.table.as_ref().unwrap();
        assert_eq!(t.rows[0].1[0], 1.0, "the first factor is a whole-plot term");
        assert_eq!(t.rows[1].1[0], 0.0);
        assert!(out.notes[1].contains("Whole-plot error: SS") && out.notes[1].contains("2 df"), "{:?}", out.notes);
    }
}
