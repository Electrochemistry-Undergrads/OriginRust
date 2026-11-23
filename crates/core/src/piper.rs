//! Plot > Specialized > Piper (T117). Built from the standard Piper (trilinear) construction and
//! Origin's documented behaviour, **not measured**: Origin's COM automation was unregistered on the test
//! machine on 2026-11-16 (`docs/spec/graphing.md`, "Piper diagram").
//!
//! - Six Y columns in order: Ca, Mg, Na+K (cations) and Cl, SO4, HCO3+CO3 (anions), in any unit (meq/L
//!   or percent); each triple is turned into percentages of its sum.
//! - In one layer whose X and Y units are equal, with sides of length 1: the cation triangle has Ca at
//!   (0, 0), Na+K at (1, 0) and Mg at the top; the anion triangle, `GAP` to its right, has HCO3+CO3 at the
//!   bottom left, Cl at the bottom right and SO4 at the top; the diamond above them takes each sample
//!   where the line from its cation point parallel to the cation triangle's right side meets the line from
//!   its anion point parallel to the anion triangle's left side.
//! - Every sample is drawn as a symbol in the two triangles and the diamond.

use crate::Color;
use crate::graph::PlotKind;
use crate::ids::{ColumnId, WindowId};
use crate::project::{Project, ProjectError};
use crate::template::Template;
use serde::{Deserialize, Serialize};

/// The gap between the two triangles, in side lengths.
pub const GAP: f64 = 0.2;
/// The height of a triangle of side 1.
pub const H: f64 = 0.866_025_403_784_438_6;

/// The five other columns of a Piper plot (the plot's Y column is Ca), in [`crate::graph::PlotDecor::piper`].
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PiperSpec {
    pub mg: ColumnId,
    pub na_k: ColumnId,
    pub cl: ColumnId,
    pub so4: ColumnId,
    pub hco3: ColumnId,
}

/// The three fractions of a triple, `None` when they do not have a positive sum.
fn shares(a: f64, b: f64, c: f64) -> Option<(f64, f64, f64)> {
    let s = a + b + c;
    (s > 0.0 && s.is_finite() && a >= 0.0 && b >= 0.0 && c >= 0.0).then(|| (a / s, b / s, c / s))
}

/// Where a sample sits: its point in the cation triangle, the anion triangle and the diamond.
pub fn place(ca: f64, mg: f64, na_k: f64, cl: f64, so4: f64, hco3: f64) -> Option<[(f64, f64); 3]> {
    let (_, mg, na_k) = shares(ca, mg, na_k)?;
    let (cl, so4, _) = shares(cl, so4, hco3)?;
    let cation = (na_k + 0.5 * mg, H * mg);
    let anion = (1.0 + GAP + cl + 0.5 * so4, H * so4);
    // cation + t (0.5, H) = anion + s (-0.5, H)
    let s = anion.0 - cation.0 - 0.5 * (anion.1 - cation.1) / H;
    let diamond = (anion.0 - 0.5 * s, anion.1 + H * s);
    Some([cation, anion, diamond])
}

/// The outlines of the diagram: the two triangles and the diamond, as closed polygons.
pub fn outlines() -> [Vec<(f64, f64)>; 3] {
    let g = GAP;
    [
        vec![(0.0, 0.0), (1.0, 0.0), (0.5, H)],
        vec![(1.0 + g, 0.0), (2.0 + g, 0.0), (1.5 + g, H)],
        vec![
            (1.0 + g / 2.0, g * H),
            (1.5 + g / 2.0, (1.0 + g) * H),
            (1.0 + g / 2.0, (2.0 + g) * H),
            (0.5 + g / 2.0, (1.0 + g) * H),
        ],
    ]
}

/// The axis ranges of the Piper layer (the diagram with a margin).
pub const X_RANGE: (f64, f64) = (-0.2, 2.0 + GAP + 0.2);
pub const Y_RANGE: (f64, f64) = (-0.2, (2.0 + GAP) * H + 0.15);

impl Project {
    /// Plots a Piper diagram of six Y columns (Ca, Mg, Na+K, Cl, SO4, HCO3+CO3).
    pub fn plot_piper(&mut self, ys: &[ColumnId]) -> Result<WindowId, ProjectError> {
        let &[ca, mg, na_k, cl, so4, hco3] = ys else { return Err(ProjectError::PiperColumns(ys.len())) };
        for y in ys {
            self.column(*y).ok_or(ProjectError::NoSuchColumn(*y))?;
        }
        let name = self.locate(ca).map(|(b, _, _)| b.title_for_graph()).unwrap_or_default();
        let id = self.add_graph(Template::Piper);
        self.add_plot_xy(id, 0, None, ca, None)?;
        let g = self.graph_mut(id).expect("the graph was just added");
        let plot = g.layers[0].plots.last_mut().expect("a plot was just added");
        plot.kind = PlotKind::Piper;
        plot.auto_color = false;
        plot.symbol.color = Color::RED;
        plot.symbol.filled = true;
        plot.decor.piper = Some(PiperSpec { mg, na_k, cl, so4, hco3 });
        g.long_name = name;
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::{ColumnData, Designation};

    #[test]
    fn pure_waters_sit_on_the_corners() {
        // All calcium and all bicarbonate: the cation's Ca corner, the anion's HCO3 corner, the diamond's left.
        let [c, a, d] = place(1.0, 0.0, 0.0, 0.0, 0.0, 1.0).unwrap();
        let close = |p: (f64, f64), q: (f64, f64)| (p.0 - q.0).abs() < 1e-12 && (p.1 - q.1).abs() < 1e-12;
        assert!(close(c, (0.0, 0.0)) && close(a, (1.0 + GAP, 0.0)), "{c:?} {a:?}");
        assert!(close(d, outlines()[2][3]), "{d:?}");
        // All sodium and all chloride: the diamond's right corner; Na+K with HCO3: the bottom corner.
        assert!(close(place(0.0, 0.0, 1.0, 1.0, 0.0, 0.0).unwrap()[2], outlines()[2][1]));
        assert!(close(place(0.0, 0.0, 1.0, 0.0, 0.0, 1.0).unwrap()[2], outlines()[2][0]));
        // Units do not matter: meq values are turned into percentages.
        assert_eq!(place(2.0, 2.0, 0.0, 3.0, 3.0, 0.0), place(50.0, 50.0, 0.0, 10.0, 10.0, 0.0));
        assert!(place(0.0, 0.0, 0.0, 1.0, 1.0, 1.0).is_none());
    }

    #[test]
    fn the_plot_takes_six_columns_in_order() {
        let mut p = Project::new();
        let b = p.add_workbook("");
        let ids: Vec<ColumnId> = (0..6)
            .map(|k| p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![k as f64 + 1.0])).unwrap())
            .collect();
        assert!(p.plot_columns(Template::Piper, &ids[..5]).is_err());
        let g = p.plot_columns(Template::Piper, &ids).unwrap();
        let graph = p.graph(g).unwrap();
        let plot = &graph.layers[0].plots[0];
        assert_eq!((plot.kind, plot.y), (PlotKind::Piper, ids[0]));
        assert_eq!(plot.decor.piper.unwrap().hco3, ids[5]);
        assert!(graph.layers[0].labels.is_empty(), "Origin titles the sides instead of the corners (T389)");
    }
}
