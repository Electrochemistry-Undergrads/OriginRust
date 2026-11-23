//! EC50 marks on dose-response fit graphs (T398; Origin 2027's "mark IC50/EC50 on dose-response fits", not in Origin
//! 2016, so the look is Sheaf's own; `docs/spec/fitting.md`, "EC50 marks").
//!
//! For a fit whose function has an `EC50` derived value and asymptote parameters `A1` and `A2` (DoseResp, Logistic,
//! Logistic5 and the others of the library), two dashed straight lines in the fitted curve's colour cross at the half
//! response `(A1 + A2) / 2`: a vertical one at the EC50 on the plot's X scale (`log10 EC50` when the function has a
//! `LOGx0` parameter, whose X is the log dose) and a horizontal one at the half response, each labelled with its value.
//! The marks are opt-in: Origin 2016's fit graphs, which Sheaf copies, have none.

use crate::fitting::FitOutcome;
use crate::graph::Dash;
use crate::ids::WindowId;
use crate::project::Project;
use crate::straight_line::{StraightLine, StraightLineLabel};

/// Where the EC50 marks were drawn: the X position and the half response.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ec50Mark {
    pub x: f64,
    pub y: f64,
}

impl Project {
    /// Marks the EC50 of `outcome` on `graph` (the layer that draws the fitted curve, else the first).
    pub fn mark_ec50(&mut self, graph: WindowId, outcome: &FitOutcome) -> Result<Ec50Mark, String> {
        let ec50 = outcome
            .derived
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case("EC50"))
            .map(|(_, v)| *v)
            .ok_or_else(|| format!("the {} fit has no EC50 to mark", outcome.report.model))?;
        let param =
            |name: &str| outcome.report.params.iter().find(|p| p.name.eq_ignore_ascii_case(name)).map(|p| p.value);
        let (Some(a1), Some(a2)) = (param("A1"), param("A2")) else {
            return Err(format!("the {} fit has no A1 and A2 asymptotes", outcome.report.model));
        };
        let x = if param("LOGx0").is_some() { ec50.log10() } else { ec50 };
        let y = 0.5 * (a1 + a2);
        if !(x.is_finite() && y.is_finite()) {
            return Err(format!("the EC50 ({ec50}) cannot be placed on the graph"));
        }
        let g = self.graph_mut(graph).ok_or_else(|| "the graph does not exist".to_string())?;
        let li = g.layers.iter().position(|l| l.plots.iter().any(|p| p.y == outcome.curve_y)).unwrap_or(0);
        let layer = g.layers.get_mut(li).ok_or_else(|| "the graph has no layer".to_string())?;
        let color =
            layer.plots.iter().find(|p| p.y == outcome.curve_y).map(|p| p.line.color).unwrap_or(crate::Color::BLACK);
        for (vertical, value, location) in [(true, x, StraightLineLabel::Top), (false, y, StraightLineLabel::Right)] {
            let name = StraightLine::next_name(&layer.straight_lines, vertical);
            layer.straight_lines.push(StraightLine {
                color,
                width_pt: 1.0,
                dash: Dash::Dash,
                location,
                name,
                ..StraightLine::new(vertical, value)
            });
        }
        Ok(Ec50Mark { x, y })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::{ColumnData, Designation};
    use crate::fitting::{FitModel, FitRequest};

    #[test]
    fn a_dose_response_fit_is_marked_at_log_ec50_and_the_half_response() {
        let mut p = Project::new();
        let book = p.add_workbook("dr");
        // log10 dose from -3 to 1; a DoseResp curve with A1 = 5, A2 = 95, LOGx0 = -1, p = 1.2 and a little wobble.
        let xs: Vec<f64> = (0..21).map(|i| -3.0 + 0.2 * f64::from(i)).collect();
        let ys: Vec<f64> = xs
            .iter()
            .enumerate()
            .map(|(i, x)| 5.0 + 90.0 / (1.0 + 10f64.powf((-1.0 - x) * 1.2)) + 0.3 * (i as f64 * 1.7).sin())
            .collect();
        p.add_column(book, 0, Designation::X, ColumnData::numeric(xs)).unwrap();
        let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(ys)).unwrap();
        let out = p.fit(&FitRequest::new(y, FitModel::BuiltIn("DoseResp".into()))).unwrap();
        let g = p.plot_fit(&out, y).unwrap();
        assert!(p.graph(g).unwrap().layers[0].straight_lines.is_empty(), "plot_fit alone draws no marks");
        let m = p.mark_ec50(g, &out).unwrap();
        let logx0 = out.report.params.iter().find(|q| q.name == "LOGx0").unwrap().value;
        assert!((m.x - logx0).abs() < 1e-9, "{} vs {logx0}", m.x);
        assert!((m.x + 1.0).abs() < 0.05 && (m.y - 50.0).abs() < 1.0, "{m:?}");
        let lines = &p.graph(g).unwrap().layers[0].straight_lines;
        assert_eq!(lines.len(), 2);
        assert!(lines[0].vertical && !lines[1].vertical);
        assert_eq!((lines[0].name.as_str(), lines[1].name.as_str()), ("vline", "hline"));
        // A straight line has no EC50.
        let lin = p.fit(&FitRequest::new(y, FitModel::Linear)).unwrap();
        assert!(p.mark_ec50(g, &lin).is_err());
    }
}
