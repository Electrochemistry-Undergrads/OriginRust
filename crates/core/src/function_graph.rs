//! Graph > Add Function Graph... (T119). Built from Origin's documented behaviour, **not measured**
//! (Origin's COM automation was unregistered on the test machine on 2026-11-16; `docs/spec/graphing.md`,
//! "Add Function Graph"): the curve of y = f(x), written in Sheaf's expression language with the
//! variable `x`, at evenly spaced points over an X range, drawn as a line in a layer of the graph.
//!
//! Origin keeps such a curve as a function dataset; Sheaf writes its points to a new worksheet (`FuncN`,
//! columns x and y, the y column's long name the function) so that the curve is an ordinary plot.

use crate::column::{ColumnData, Designation};
use crate::graph::PlotKind;
use crate::ids::WindowId;
use crate::project::{Project, ProjectError};

/// The points of y = `expr` at `points` evenly spaced X values from `from` to `to`; a point where the
/// function has no value is NaN.
pub fn sample(expr: &str, from: f64, to: f64, points: usize) -> Result<(Vec<f64>, Vec<f64>), ProjectError> {
    if points < 2 || !from.is_finite() || !to.is_finite() || from == to {
        return Err(ProjectError::FunctionGraph("give two or more points over an X range of some width".into()));
    }
    let f = sheaf_expr::Expr::parse(expr).map_err(|e| ProjectError::FunctionGraph(e.to_string()))?;
    let xs: Vec<f64> = (0..points).map(|k| from + (to - from) * k as f64 / (points - 1) as f64).collect();
    let mut ys = Vec::with_capacity(points);
    for &x in &xs {
        let host = sheaf_expr::SimpleHost::new().var("x", x);
        ys.push(f.eval(&host).map_err(|e| ProjectError::FunctionGraph(e.to_string()))?);
    }
    Ok((xs, ys))
}

impl Project {
    /// Adds the curve of y = `expr` from `from` to `to` (`points` points) to layer `layer` of `graph`, its
    /// points in a new worksheet. Returns the worksheet's workbook.
    pub fn add_function_graph(
        &mut self,
        graph: WindowId,
        layer: usize,
        expr: &str,
        (from, to): (f64, f64),
        points: usize,
    ) -> Result<WindowId, ProjectError> {
        let layers = self.graph(graph).ok_or(ProjectError::NoSuchWindow(graph))?.layers.len();
        if layer >= layers {
            return Err(ProjectError::NoSuchLayer(layer));
        }
        let (xs, ys) = sample(expr, from, to, points)?;
        let book = self.add_workbook(&format!("y = {expr}"));
        let name = self.unique_name("Func");
        if let Some(b) = self.workbook_mut(book) {
            b.name = name;
        }
        let x = self.add_column(book, 0, Designation::X, ColumnData::numeric(xs))?;
        let y = self.add_column(book, 0, Designation::Y, ColumnData::numeric(ys))?;
        if let Some(c) = self.column_mut(x) {
            c.long_name = "x".into();
        }
        if let Some(c) = self.column_mut(y) {
            c.long_name = expr.to_string();
        }
        self.add_plot_xy(graph, layer, Some(x), y, None)?;
        if let Some(p) = self.graph_mut(graph).and_then(|g| g.layers[layer].plots.last_mut()) {
            p.kind = PlotKind::Line;
        }
        Ok(book)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::template::Template;

    #[test]
    fn the_function_is_sampled_into_a_worksheet_and_plotted_as_a_line() {
        let (xs, ys) = sample("x^2 + 1", 0.0, 2.0, 5).unwrap();
        assert_eq!(xs, [0.0, 0.5, 1.0, 1.5, 2.0]);
        assert_eq!(ys, [1.0, 1.25, 2.0, 3.25, 5.0]);
        assert!(sample("x +", 0.0, 1.0, 10).is_err(), "a syntax error is reported");
        assert!(sample("x", 1.0, 1.0, 10).is_err());
        let mut p = Project::new();
        let g = p.add_graph(Template::Scatter);
        let book = p.add_function_graph(g, 0, "sin(x)", (0.0, 10.0), 100).unwrap();
        let wb = p.workbook(book).unwrap();
        assert_eq!((wb.name.as_str(), wb.long_name.as_str()), ("Func1", "y = sin(x)"));
        let plot = &p.graph(g).unwrap().layers[0].plots[0];
        assert_eq!(plot.kind, PlotKind::Line);
        assert_eq!(p.numeric(plot.y).unwrap().len(), 100);
        assert!(p.add_function_graph(g, 3, "x", (0.0, 1.0), 10).is_err());
    }
}
