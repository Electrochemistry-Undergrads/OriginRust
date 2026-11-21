//! 2D kernel density plots (T116, graphing spec "2D kernel density"): the bivariate Gaussian density of the
//! plot's X and Y columns (`sheaf_calc::kde::density_2d`) drawn as a grid of cells in the shared colour
//! map, the densest cell at the top of the map and zero at its bottom.

use crate::LayerGeometry;
use crate::Rect;
use crate::data::PlotData;
use crate::scene::Item;
use sheaf_calc::kde::Density2d;

pub(crate) struct Kde2dData {
    pub d: Density2d,
    pub max: f64,
}

impl Kde2dData {
    pub(crate) fn new(pd: &PlotData) -> Option<Kde2dData> {
        let rows: Vec<usize> = pd.rows().collect();
        let xs: Vec<f64> = rows.iter().map(|&r| pd.x_at(r)).collect();
        let ys: Vec<f64> = rows.iter().map(|&r| pd.y_at(r)).collect();
        let d = sheaf_calc::kde::density_2d(&xs, &ys)?;
        let max = d.z.iter().copied().fold(0.0, f64::max);
        Some(Kde2dData { d, max })
    }

    /// The grid's span on X (`x`) or Y.
    pub(crate) fn extent(&self, x: bool) -> (f64, f64) {
        let v = if x { &self.d.x } else { &self.d.y };
        (v[0], v[v.len() - 1])
    }
}

/// The cells: each grid point's cell reaches halfway to its neighbours.
pub(crate) fn draw(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry) {
    let Some(k) = &pd.kde2d else { return };
    let edges = |v: &[f64]| -> Vec<f64> {
        let n = v.len();
        (0..=n)
            .map(|i| match i {
                0 => v[0] - (v[1] - v[0]) / 2.0,
                i if i == n => v[n - 1] + (v[n - 1] - v[n - 2]) / 2.0,
                i => (v[i - 1] + v[i]) / 2.0,
            })
            .collect()
    };
    let (ex, ey) = (edges(&k.d.x), edges(&k.d.y));
    let nx = k.d.x.len();
    for (j, w) in ey.windows(2).enumerate() {
        for (i, u) in ex.windows(2).enumerate() {
            let z = k.d.z[j * nx + i];
            let (x0, y0) = geom.data_to_page(u[0], w[1]);
            let (x1, y1) = geom.data_to_page(u[1], w[0]);
            let fill = crate::plots::map_color(if k.max > 0.0 { z / k.max } else { 0.0 });
            out.push(Item::Rect {
                rect: Rect { x: x0.min(x1), y: y0.min(y1), w: (x1 - x0).abs(), h: (y1 - y0).abs() },
                fill: Some(fill),
                stroke: None,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    use sheaf_core::template::Template;
    use sheaf_core::{ColumnData, Designation, Project};

    #[test]
    fn the_density_is_a_grid_of_cells_under_the_points() {
        let mut p = Project::new();
        let b = p.add_workbook("");
        p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0, 3.0, 2.5])).unwrap();
        let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![2.0, 1.0, 3.0, 2.2])).unwrap();
        let g = p.plot_columns(Template::Kde2d, &[y]).unwrap();
        let s = layout::build_scene(&p, g, &FixedMetrics).unwrap();
        let n = sheaf_calc::kde::GRID_2D;
        let cells = s.layers[0].plots.iter().filter(|it| matches!(it, scene::Item::Rect { stroke: None, .. })).count();
        assert!(cells >= n * n, "{cells}");
        // The axes take in the grid, which reaches a quarter of the data range beyond the data (Origin).
        let geom = &layout_geometry(&p, g).unwrap()[0];
        assert!(geom.x.min < 1.0 && geom.x.max > 3.0 && geom.y.min < 1.0 && geom.y.max > 3.0);
    }
}
