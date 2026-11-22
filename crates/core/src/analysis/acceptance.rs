//! Statistics > Acceptance Sampling (T369, Origin 2026b's Acceptance Sampling app): an attribute single
//! sampling plan designed from AQL, LTPD and the two risks, or a plan given by hand, with its operating
//! characteristic and, with a lot size, the average outgoing quality and total inspection. Needs no
//! worksheet data; the result is a new workbook and a graph of the OC curve. Rules:
//! `docs/spec/statistics.md`, "Acceptance sampling"; the numbers are `sheaf_calc::acceptance`.

use super::{AnalysisError, Report};
use crate::template::Template;
use crate::{ColumnData, Designation, Project, Worksheet};
use sheaf_calc::acceptance::{self as acc, SamplingPlan};

/// What Acceptance Sampling is asked for.
#[derive(Debug, Clone, PartialEq)]
pub struct AcceptanceSpec {
    /// Acceptable quality level: the fraction defective a lot should be accepted at (0.01 is 1 %).
    pub aql: f64,
    /// Lot tolerance percent defective (rejectable quality level), as a fraction.
    pub ltpd: f64,
    /// Producer's risk: the largest chance of rejecting a lot at the AQL.
    pub alpha: f64,
    /// Consumer's risk: the largest chance of accepting a lot at the LTPD.
    pub beta: f64,
    /// A plan given by hand (sample size, acceptance number) instead of a designed one.
    pub plan: Option<(u64, u64)>,
    /// Lot size, for the average outgoing quality (AOQ, AOQL) and the average total inspection (ATI).
    pub lot: Option<u64>,
}

impl Default for AcceptanceSpec {
    fn default() -> Self {
        AcceptanceSpec { aql: 0.01, ltpd: 0.05, alpha: 0.05, beta: 0.10, plan: None, lot: None }
    }
}

/// The plan and its numbers.
#[derive(Debug, Clone, PartialEq)]
pub struct AcceptanceOutput {
    pub plan: SamplingPlan,
    /// Probability of acceptance at the AQL and at the LTPD.
    pub pa_aql: f64,
    pub pa_ltpd: f64,
    /// The AOQL and the fraction defective where it is reached, with a lot size.
    pub aoql: Option<(f64, f64)>,
    /// The OC curve: fractions defective and probabilities of acceptance (and AOQ, ATI with a lot size).
    pub p: Vec<f64>,
    pub pa: Vec<f64>,
    pub aoq: Vec<f64>,
    pub ati: Vec<f64>,
}

fn bad(e: acc::AcceptanceError) -> AnalysisError {
    AnalysisError::BadOption(e.to_string())
}

/// Designs or checks the plan and computes its curves.
pub fn acceptance_sampling(spec: &AcceptanceSpec) -> Result<AcceptanceOutput, AnalysisError> {
    let plan = match spec.plan {
        Some((n, c)) => {
            let plan = SamplingPlan { n, c };
            acc::check_plan(plan).map_err(bad)?;
            plan
        }
        None => acc::design(spec.aql, spec.ltpd, spec.alpha, spec.beta).map_err(bad)?,
    };
    if let Some(lot) = spec.lot
        && lot < plan.n
    {
        return Err(AnalysisError::BadOption(format!("the lot size ({lot}) is smaller than the sample ({})", plan.n)));
    }
    let p = acc::curve_fractions(plan, spec.ltpd.max(spec.aql), 101);
    let pa: Vec<f64> = p.iter().map(|&x| acc::accept_probability(plan, x)).collect();
    let (aoq, ati) = match spec.lot {
        Some(lot) => {
            (p.iter().map(|&x| acc::aoq(plan, lot, x)).collect(), p.iter().map(|&x| acc::ati(plan, lot, x)).collect())
        }
        None => (Vec::new(), Vec::new()),
    };
    Ok(AcceptanceOutput {
        plan,
        pa_aql: acc::accept_probability(plan, spec.aql),
        pa_ltpd: acc::accept_probability(plan, spec.ltpd),
        aoql: spec.lot.map(|lot| acc::aoql(plan, lot)),
        p,
        pa,
        aoq,
        ati,
    })
}

impl Project {
    /// Runs Acceptance Sampling and puts the result in a new workbook: sheet `Plan` (the quantities and their
    /// values) and sheet `OC Curve` (fraction defective, probability of acceptance, and AOQ and ATI with a lot
    /// size), with a line graph of the OC curve.
    pub fn acceptance_sampling(&mut self, spec: &AcceptanceSpec) -> Result<Report, AnalysisError> {
        let out = acceptance_sampling(spec)?;
        let book = self.add_workbook("Acceptance Sampling");
        let mut rows: Vec<(&str, f64)> = vec![
            ("Sample size n", out.plan.n as f64),
            ("Acceptance number c", out.plan.c as f64),
            ("AQL", spec.aql),
            ("LTPD", spec.ltpd),
            ("P(accept) at AQL", out.pa_aql),
            ("P(accept) at LTPD", out.pa_ltpd),
            ("Producer's risk", 1.0 - out.pa_aql),
            ("Consumer's risk", out.pa_ltpd),
        ];
        if let (Some(lot), Some((q, at))) = (spec.lot, out.aoql) {
            rows.extend([("Lot size", lot as f64), ("AOQL", q), ("Fraction defective at AOQL", at)]);
        }
        let labels: Vec<String> = rows.iter().map(|r| r.0.to_string()).collect();
        let values: Vec<f64> = rows.iter().map(|r| r.1).collect();
        let quantity = self.add_column(book, 0, Designation::Label, ColumnData::text(labels)).expect("new book");
        let value = self.add_column(book, 0, Designation::Y, ColumnData::numeric(values)).expect("new book");
        for (id, name) in [(quantity, "Quantity"), (value, "Value")] {
            if let Some(c) = self.column_mut(id) {
                c.long_name = name.to_string();
            }
        }
        if let Some(b) = self.workbook_mut(book) {
            b.sheets[0].name = "Plan".to_string();
            b.sheets.push(Worksheet::new("OC Curve"));
        }
        let mut curve =
            vec![("Fraction Defective", Designation::X, out.p.clone()), ("P(accept)", Designation::Y, out.pa.clone())];
        if spec.lot.is_some() {
            curve.push(("AOQ", Designation::Y, out.aoq.clone()));
            curve.push(("ATI", Designation::Y, out.ati.clone()));
        }
        let mut pa_column = None;
        for (name, designation, data) in curve {
            let id = self.add_column(book, 1, designation, ColumnData::numeric(data)).expect("the sheet was added");
            if let Some(c) = self.column_mut(id) {
                c.long_name = name.to_string();
            }
            if name == "P(accept)" {
                pa_column = Some(id);
            }
        }
        let graph = self.add_graph(Template::Line);
        if let Some(id) = pa_column {
            self.add_plot(graph, 0, id).map_err(|e| AnalysisError::BadOption(e.to_string()))?;
        }
        let mut notes = vec![format!(
            "Plan: sample {} items, accept the lot with at most {} defective. P(accept) {:.4} at AQL {} and {:.4} at LTPD {}.",
            out.plan.n, out.plan.c, out.pa_aql, spec.aql, out.pa_ltpd, spec.ltpd
        )];
        if let Some((q, at)) = out.aoql {
            notes.push(format!("AOQL {q:.6} at a fraction defective of {at:.6}."));
        }
        Ok(Report {
            book,
            sheet: 0,
            new_columns: Vec::new(),
            new_sheet: Some(0),
            new_graph: Some(graph),
            more_graphs: Vec::new(),
            table: None,
            message: format!("Acceptance Sampling: n = {}, c = {}", out.plan.n, out.plan.c),
            notes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_designed_plan_fills_a_book_and_draws_the_oc_curve() {
        let mut p = Project::new();
        let spec = AcceptanceSpec { lot: Some(1000), ..AcceptanceSpec::default() };
        let r = p.acceptance_sampling(&spec).unwrap();
        let b = p.workbook(r.book).unwrap();
        assert_eq!(b.sheets.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), ["Plan", "OC Curve"]);
        // scipy: AQL 1 %, LTPD 5 %, risks 5 % and 10 % give n 132, c 3 (tools/acceptance-ref.py).
        assert_eq!(b.sheets[0].columns[1].numeric().unwrap()[..2], [132.0, 3.0]);
        assert_eq!(b.sheets[0].columns[0].shown_text(8), "Lot size");
        assert_eq!(b.sheets[1].columns.len(), 4, "p, P(accept), AOQ, ATI");
        let g = p.graph(r.new_graph.unwrap()).unwrap();
        assert_eq!(g.layers[0].plots.len(), 1);
        assert!(r.message.contains("n = 132"));
    }

    #[test]
    fn a_plan_by_hand_is_checked_and_a_lot_smaller_than_the_sample_is_refused() {
        let mut p = Project::new();
        let by_hand = AcceptanceSpec { plan: Some((50, 1)), ..AcceptanceSpec::default() };
        let r = p.acceptance_sampling(&by_hand).unwrap();
        assert_eq!(p.workbook(r.book).unwrap().sheets[1].columns.len(), 2, "no lot size: p and P(accept)");
        assert!(p.acceptance_sampling(&AcceptanceSpec { plan: Some((5, 5)), ..by_hand.clone() }).is_err());
        assert!(p.acceptance_sampling(&AcceptanceSpec { lot: Some(10), ..by_hand }).is_err());
    }
}
