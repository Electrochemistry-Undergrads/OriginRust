//! Statistics > Survival Analysis > Kaplan-Meier Estimator as a tool of `Project::analyze`. The numerics are in
//! `sheaf_calc` (`survival`); what the tool computes and reports is written down in `docs/spec/statistics.md`,
//! section 20.
//!
//! The chosen columns are the time column, the censor column and, optionally, a grouping column (numbers or text),
//! in that order, as Origin's dialog takes them. A case counts as censored when its censor cell equals the
//! censoring value.

use super::dist::{GraphPlot, Output, XyGraph};
use super::roc::is_positive as cell_equals;
use super::statistics::num;
use super::*;
use crate::graph::{PlotKind, ScaleKind};

/// What [`Tool::KaplanMeier`] does.
#[derive(Debug, Clone, PartialEq)]
pub struct KaplanMeierSpec {
    /// The value of the censor column that marks a censored case (a number or a text). Cells are compared by
    /// value; Origin's default censoring value is 1.
    pub censor: String,
    /// Confidence level of the quartile and mean limits (and of the limits of the curve), in percent.
    pub confidence: f64,
    /// Write the quartile survival times with their limits.
    pub quartiles: bool,
    /// Write the mean survival time with its standard error and limits.
    pub mean: bool,
    /// Rank tests of the groups (need a grouping column with at least two groups).
    pub log_rank: bool,
    pub breslow: bool,
    pub tarone_ware: bool,
    /// Make a graph of the survival curves with a mark at every censored case.
    pub curve: bool,
    /// Add the confidence limits of the curve to the graph.
    pub band: bool,
}

impl Default for KaplanMeierSpec {
    fn default() -> Self {
        KaplanMeierSpec {
            censor: "1".into(),
            confidence: 95.0,
            quartiles: true,
            mean: true,
            log_rank: false,
            breslow: false,
            tarone_ware: false,
            curve: true,
            band: false,
        }
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

fn number_column(name: &str, comments: &str, designation: Designation, values: Vec<f64>) -> SheetColumn {
    SheetColumn {
        designation,
        long_name: name.to_string(),
        units: String::new(),
        comments: comments.to_string(),
        data: ColumnData::numeric(values),
    }
}

/// The group a case belongs to, as text; `None` for a missing cell.
fn group_cell(c: &Column, row: usize) -> Option<String> {
    if let Some(t) = c.data.text_cell(row) {
        let t = t.trim();
        return (!t.is_empty()).then(|| t.to_string());
    }
    let v = c.numeric()?.get(row).copied().filter(|v| !v.is_nan())?;
    Some(num(v))
}

pub(super) fn run(
    spec: &KaplanMeierSpec,
    sheet: &Worksheet,
    columns: &[ColumnId],
    rows: RowRange,
) -> Result<Output, AnalysisError> {
    let censor = spec.censor.trim();
    if censor.is_empty() {
        return Err(AnalysisError::BadOption("give the value of the censor column that marks a censored case".into()));
    }
    if !(spec.confidence > 0.0 && spec.confidence < 100.0) {
        return Err(AnalysisError::BadOption("the confidence level must lie between 0 and 100 percent".into()));
    }
    if columns.len() < 2 || columns.len() > 3 {
        return Err(AnalysisError::BadOption(
            "the Kaplan-Meier estimator takes two or three columns: time, censor and, optionally, group".into(),
        ));
    }
    if matches!(rows, RowRange::XBetween(..)) {
        return Err(AnalysisError::BadOption(
            "the Kaplan-Meier estimator has no X range to limit; choose rows with a row range".into(),
        ));
    }
    let column = |id: ColumnId| sheet.column(id).ok_or(AnalysisError::NoSuchColumn(id));
    let time_col = column(columns[0])?;
    let censor_col = column(columns[1])?;
    let group_col = columns.get(2).map(|id| column(*id)).transpose()?;
    let times = sheet.values_of(time_col).ok_or_else(|| AnalysisError::NotNumeric(time_col.short_name.clone()))?;
    let n = [time_col.data.len(), censor_col.data.len()].into_iter().chain(group_col.map(|g| g.data.len())).max();
    let n = n.filter(|n| *n > 0).ok_or_else(|| AnalysisError::Empty(time_col.short_name.clone()))?;
    let (first, last) = resolve_rows(rows, &time_col.short_name, &[], n)?;
    let censor_number = censor.parse::<f64>().ok();

    // Cases per group, in the order of the group labels (numbers by value, otherwise as text).
    let mut cases: Vec<(String, Vec<f64>, Vec<bool>)> = Vec::new();
    for row in first..=last {
        let t = times.get(row).copied().unwrap_or(f64::NAN);
        let Some(is_censored) = cell_equals(censor_col, row, censor, censor_number) else { continue };
        if t.is_nan() {
            continue;
        }
        let label = match group_col {
            Some(g) => match group_cell(g, row) {
                Some(l) => l,
                None => continue,
            },
            None => column_label(time_col),
        };
        match cases.iter_mut().find(|c| c.0 == label) {
            Some(c) => {
                c.1.push(t);
                c.2.push(is_censored);
            }
            None => cases.push((label, vec![t], vec![is_censored])),
        }
    }
    if cases.is_empty() {
        return Err(AnalysisError::Empty(time_col.short_name.clone()));
    }
    if group_col.is_some() {
        if cases.iter().all(|c| c.0.parse::<f64>().is_ok()) {
            cases.sort_by(|a, b| a.0.parse::<f64>().unwrap().total_cmp(&b.0.parse::<f64>().unwrap()));
        } else {
            cases.sort_by(|a, b| a.0.cmp(&b.0));
        }
    }

    let mut headers = vec!["N", "Events", "Censored", "Percent censored"];
    if spec.quartiles {
        headers.extend(["25%", "25% lower", "25% upper", "50%", "50% lower", "50% upper", "75%", "75% lower", "75% upper"]);
    }
    if spec.mean {
        headers.extend(["Mean", "Mean SE", "Mean lower", "Mean upper"]);
    }
    let mut table = ResultTable::new(&headers);
    let mut notes = Vec::new();
    let mut fits = Vec::new();
    let opt = |v: Option<f64>| v.unwrap_or(f64::NAN);
    for (label, t, c) in &cases {
        let km = calc::kaplan_meier(t, c, spec.confidence)
            .map_err(|e| AnalysisError::BadOption(format!("{label}: {e}")))?;
        let mut v = vec![km.n as f64, km.events as f64, km.censored as f64, km.censored as f64 / km.n as f64];
        if spec.quartiles {
            for q in &km.quartiles {
                v.extend([opt(q.estimate), opt(q.lower), opt(q.upper)]);
            }
        }
        if spec.mean {
            v.extend([km.mean, km.mean_se, km.mean_lower, km.mean_upper]);
        }
        table.push_row(label.clone(), v.clone(), vec![String::new(); v.len()]);
        let mut line = format!("{label}: {} cases, {} events, {} censored", km.n, km.events, km.censored);
        if spec.quartiles {
            let q = |x: Option<f64>| x.map_or("not reached".to_string(), num);
            let parts: Vec<String> = km
                .quartiles
                .iter()
                .map(|k| format!("{}% {} ({} to {})", num(k.percent), q(k.estimate), q(k.lower), q(k.upper)))
                .collect();
            line.push_str(&format!("; quartile survival times {}", parts.join(", ")));
        }
        if spec.mean {
            line.push_str(&format!(
                "; mean {} (standard error {}, {}% limits {} to {})",
                num(km.mean),
                num(km.mean_se),
                num(spec.confidence),
                num(km.mean_lower),
                num(km.mean_upper)
            ));
        }
        notes.push(format!("{line}."));
        fits.push((label.clone(), km));
    }

    // Rank tests of the groups.
    let wanted: Vec<calc::RankTest> = [
        (spec.log_rank, calc::RankTest::LogRank),
        (spec.breslow, calc::RankTest::Breslow),
        (spec.tarone_ware, calc::RankTest::TaroneWare),
    ]
    .into_iter()
    .filter_map(|(on, t)| on.then_some(t))
    .collect();
    let mut tests: Vec<(calc::RankTest, calc::RankResult)> = Vec::new();
    if !wanted.is_empty() {
        let groups: Vec<(Vec<f64>, Vec<bool>)> = cases.iter().map(|c| (c.1.clone(), c.2.clone())).collect();
        if groups.len() < 2 {
            notes.push("The tests of groups need a grouping column with at least two groups; none was run.".into());
        } else {
            for t in wanted {
                let r = calc::rank_test(&groups, t).map_err(|e| AnalysisError::BadOption(e.to_string()))?;
                notes.push(format!(
                    "{} test: chi-square {}, {} degrees of freedom, Prob>chi-square {}.",
                    t.name(),
                    num(r.chi_square),
                    r.df,
                    num(r.p)
                ));
                tests.push((t, r));
            }
        }
    }
    notes.push(format!(
        "Product-limit estimate with Greenwood standard errors; {} cases censored where the {} column equals {censor}. \
         Quartile limits come from the linear limits of the curve, the mean is cut off at the largest time.",
        cases.iter().map(|c| c.2.iter().filter(|x| **x).count()).sum::<usize>(),
        censor_col.short_name
    ));

    // The result sheet: the summary, the tests, then per group the survival table and the step curve.
    let mut on_sheet = vec![label_column("Group", table.rows.iter().map(|(l, _)| l.clone()).collect())];
    for (i, h) in table.headers.iter().enumerate() {
        on_sheet.push(number_column(h, "", Designation::Y, table.rows.iter().map(|(_, v)| v[i]).collect()));
    }
    if !tests.is_empty() {
        on_sheet.push(label_column("Test", tests.iter().map(|t| t.0.name().to_string()).collect()));
        on_sheet.push(number_column("Chi-square", "", Designation::Y, tests.iter().map(|t| t.1.chi_square).collect()));
        on_sheet.push(number_column("DF", "", Designation::Y, tests.iter().map(|t| t.1.df as f64).collect()));
        on_sheet.push(number_column("Prob>Chi-square", "", Designation::Y, tests.iter().map(|t| t.1.p).collect()));
    }
    let mut plots = Vec::new();
    let mut band_plots = Vec::new();
    for (label, km) in &fits {
        on_sheet.push(number_column("Time", label, Designation::Y, km.rows.iter().map(|r| r.time).collect()));
        on_sheet.push(label_column(
            "Status",
            km.rows.iter().map(|r| if r.censored { "+" } else { "" }.to_string()).collect(),
        ));
        let at = |f: &dyn Fn(f64, f64) -> f64| km.rows.iter().map(|r| r.survival.map_or(f64::NAN, |(s, se)| f(s, se))).collect();
        on_sheet.push(number_column("Survival", label, Designation::Y, at(&|s, _| s)));
        on_sheet.push(number_column("Standard error", label, Designation::Y, at(&|_, se| se)));
        on_sheet.push(number_column("Events", label, Designation::Y, km.rows.iter().map(|r| r.events as f64).collect()));
        on_sheet.push(number_column("At risk", label, Designation::Y, km.rows.iter().map(|r| r.at_risk as f64).collect()));

        // The curve as step vertices, starting at (0, 1) and running on to the largest time.
        let mut x = vec![0.0];
        let mut y = vec![1.0];
        let (mut lo, mut hi) = (vec![1.0], vec![1.0]);
        for s in &km.steps {
            x.extend([s.time, s.time]);
            y.extend([*y.last().unwrap(), s.survival]);
            lo.extend([*lo.last().unwrap(), s.lower]);
            hi.extend([*hi.last().unwrap(), s.upper]);
        }
        if km.last_time > *x.last().unwrap() {
            x.push(km.last_time);
            y.push(*y.last().unwrap());
            lo.push(*lo.last().unwrap());
            hi.push(*hi.last().unwrap());
        }
        let xi = on_sheet.len();
        on_sheet.push(number_column("Curve time", label, Designation::X, x));
        on_sheet.push(number_column("Curve survival", label, Designation::Y, y));
        let (m_t, m_s): (Vec<f64>, Vec<f64>) = km.censor_marks().into_iter().unzip();
        let mi = on_sheet.len();
        on_sheet.push(number_column("Censored time", label, Designation::X, m_t));
        on_sheet.push(number_column("Survival at censored", label, Designation::Y, m_s));
        plots.push(GraphPlot { x: xi, y: xi + 1, kind: PlotKind::Line, label: label.clone(), line_of: None });
        let curve_index = plots.len() - 1;
        if !km.rows.iter().all(|r| !r.censored) {
            plots.push(GraphPlot {
                x: mi,
                y: mi + 1,
                kind: PlotKind::Scatter,
                label: format!("{label} censored"),
                line_of: Some(curve_index),
            });
        }
        if spec.band {
            let li = on_sheet.len();
            on_sheet.push(number_column("Lower limit", label, Designation::Y, lo));
            on_sheet.push(number_column("Upper limit", label, Designation::Y, hi));
            band_plots.push((li, xi, curve_index, label.clone()));
        }
    }
    for (li, xi, curve_index, label) in band_plots {
        for (k, what) in ["lower", "upper"].into_iter().enumerate() {
            plots.push(GraphPlot {
                x: xi,
                y: li + k,
                kind: PlotKind::Line,
                label: format!("{label} {what} limit"),
                line_of: Some(curve_index),
            });
        }
    }

    let graph = spec.curve.then(|| XyGraph {
        plots,
        x_scale: ScaleKind::Linear,
        y_scale: ScaleKind::Linear,
        x_title: if time_col.long_name.is_empty() { time_col.short_name.clone() } else { time_col.long_name.clone() },
        y_title: "Survival probability".into(),
    });
    Ok(Output { table: Some(table), on_sheet, notes, graph })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Time, censor (1 = censored) and a group column of text, ten cases.
    fn project() -> (Project, Vec<ColumnId>) {
        let time = vec![2.0, 3.0, 4.0, 4.0, 6.0, 1.0, 5.0, 7.0, 8.0, 9.0];
        let cens = vec![0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
        let group: Vec<String> = ["a", "a", "a", "a", "a", "b", "b", "b", "b", "b"].iter().map(|s| s.to_string()).collect();
        let mut p = Project::new();
        let book = p.add_workbook("km");
        let t = p.add_column(book, 0, Designation::Y, ColumnData::numeric(time)).unwrap();
        let c = p.add_column(book, 0, Designation::Y, ColumnData::numeric(cens)).unwrap();
        let g = p.add_column(book, 0, Designation::Y, ColumnData::text(group)).unwrap();
        (p, vec![t, c, g])
    }

    fn request(columns: Vec<ColumnId>, spec: KaplanMeierSpec) -> Request {
        Request { columns, rows: RowRange::All, tool: Tool::KaplanMeier(spec) }
    }

    #[test]
    fn one_group_gives_the_hand_worked_numbers_and_a_graph() {
        let (mut p, ids) = project();
        // First five cases: 2, 3+, 4, 4, 6+ : S(2) = 0.8, S(4) = 4/15 (see calc::survival).
        let r = p
            .analyze(&Request {
                columns: ids[..2].to_vec(),
                rows: RowRange::Rows(1, 5),
                tool: Tool::KaplanMeier(KaplanMeierSpec::default()),
            })
            .unwrap();
        assert!(r.message.contains("Kaplan-Meier") && r.message.contains("KaplanMeier1"), "{}", r.message);
        let t = r.table.unwrap();
        assert_eq!(t.rows.len(), 1);
        let v = &t.rows[0].1;
        assert_eq!((v[0], v[1], v[2]), (5.0, 3.0, 2.0));
        assert_eq!(v[4], 4.0, "25% quartile");
        let sheet = &p.workbook(r.book).unwrap().sheets[r.new_sheet.unwrap()];
        let long: Vec<&str> = sheet.columns.iter().map(|c| c.long_name.as_str()).collect();
        assert!(long.contains(&"Survival") && long.contains(&"Curve time"), "{long:?}");
        let g = p.graph(r.new_graph.unwrap()).unwrap();
        assert_eq!(g.layers[0].plots.len(), 2, "the curve and its censor marks");
    }

    #[test]
    fn groups_and_tests_and_errors() {
        let (mut p, ids) = project();
        let spec = KaplanMeierSpec { log_rank: true, breslow: true, tarone_ware: true, curve: false, ..Default::default() };
        let r = p.analyze(&request(ids.clone(), spec.clone())).unwrap();
        assert!(r.new_graph.is_none());
        let t = r.table.unwrap();
        assert_eq!(t.rows.len(), 2);
        assert!(t.rows[0].0 == "a" && t.rows[1].0 == "b");
        assert!(r.notes.iter().any(|n| n.starts_with("Log-rank test")), "{:?}", r.notes);
        assert!(r.notes.iter().any(|n| n.starts_with("Tarone-Ware test")));
        // Tests without groups say so; bad input is refused.
        let one = p.analyze(&request(ids[..2].to_vec(), spec.clone())).unwrap();
        assert!(one.notes.iter().any(|n| n.contains("at least two groups")));
        assert!(p.analyze(&request(ids[..1].to_vec(), spec.clone())).is_err());
        assert!(p.analyze(&request(ids.clone(), KaplanMeierSpec { censor: " ".into(), ..spec.clone() })).is_err());
        assert!(p.analyze(&request(ids, KaplanMeierSpec { confidence: 100.0, ..spec })).is_err());
    }
}
