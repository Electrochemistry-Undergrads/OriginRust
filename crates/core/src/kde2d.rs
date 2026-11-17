//! Plot > Statistics > 2D Kernel Density (T116). Built from Origin's documented behaviour, **not
//! measured**: Origin's COM automation was unregistered on the test machine on 2026-11-16
//! (`docs/spec/graphing.md`, "2D kernel density"). The plot is the bivariate Gaussian density of one Y
//! column against its X column, drawn as a colour map, with the points as small black dots on top.

use crate::Color;
use crate::graph::{PlotKind, SymbolShape};
use crate::ids::{ColumnId, WindowId};
use crate::project::{Project, ProjectError};
use crate::template::Template;

impl Project {
    /// The 2D kernel density of `ys[0]` against its X column: the density plot and a scatter of the points.
    pub fn plot_kde2d(&mut self, ys: &[ColumnId]) -> Result<WindowId, ProjectError> {
        let [y] = ys else { return Err(ProjectError::Kde2dColumns) };
        let y = *y;
        let (book, sheet, _) = self.locate(y).ok_or(ProjectError::NoSuchColumn(y))?;
        let Some(x) = sheet.column_index(y).and_then(|i| sheet.x_for(i)).map(|c| c.id) else {
            return Err(ProjectError::Kde2dColumns);
        };
        let name = book.title_for_graph();
        let id = self.add_graph(Template::Kde2d);
        self.add_plot_xy(id, 0, Some(x), y, None)?;
        self.add_plot_xy(id, 0, Some(x), y, None)?;
        let g = self.graph_mut(id).expect("the graph was just added");
        let points = &mut g.layers[0].plots[1];
        points.kind = PlotKind::Scatter;
        points.auto_color = false;
        points.symbol.shape = SymbolShape::Circle;
        points.symbol.size_pt = 3.0;
        points.symbol.color = Color::BLACK;
        points.symbol.filled = true;
        g.layers[0].plots[0].kind = PlotKind::Kde2d;
        g.long_name = name;
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::{ColumnData, Designation};

    #[test]
    fn the_density_plot_comes_with_the_points_on_top() {
        let mut p = Project::new();
        let b = p.add_workbook("");
        let x = p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0, 3.0])).unwrap();
        let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![2.0, 1.0, 3.0])).unwrap();
        let g = p.plot_columns(Template::Kde2d, &[y]).unwrap();
        let plots = &p.graph(g).unwrap().layers[0].plots;
        assert_eq!((plots[0].kind, plots[1].kind), (PlotKind::Kde2d, PlotKind::Scatter));
        assert_eq!((plots[0].x, plots[0].y, plots[1].symbol.color), (Some(x), y, Color::BLACK));
        assert!(p.plot_columns(Template::Kde2d, &[y, x]).is_err());
    }
}
