//! The two charts of an X bar R chart (T111, graphing spec "QC (X bar R) chart"): the subgroup means as
//! squares with a line to the centre line, the subgroup ranges as bars from the centre line, 0.8 of the
//! X spacing wide. The centre line and the control limits are the layer's straight lines.

use crate::data::PlotData;
use crate::scene::{self, Item};
use crate::{LayerGeometry, plots::line_stroke};
use sheaf_core::graph::PlotKind;

/// The width of a range bar in X units (one per subgroup; measured 0.8).
const BAR_WIDTH: f64 = 0.8;

pub(crate) fn draw_qc(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry) {
    let Some(center) = pd.plot.decor.qc_center else { return };
    let stroke = line_stroke(&pd.plot.line);
    for i in pd.rows() {
        let (x, y) = (pd.x_at(i), pd.y_at(i));
        if !x.is_finite() || !y.is_finite() {
            continue;
        }
        let (px, py) = geom.data_to_page(x, y);
        let (_, pc) = geom.data_to_page(x, center);
        if pd.plot.kind == PlotKind::QcRanges {
            let (x0, _) = geom.data_to_page(x - BAR_WIDTH / 2.0, y);
            let (x1, _) = geom.data_to_page(x + BAR_WIDTH / 2.0, y);
            let mut d = String::new();
            scene::move_to(&mut d, x0, pc);
            scene::line_to(&mut d, x1, pc);
            scene::line_to(&mut d, x1, py);
            scene::line_to(&mut d, x0, py);
            d.push('Z');
            out.push(Item::Path { d, fill: Some(pd.plot.symbol.color), stroke: Some(stroke.clone()) });
        } else {
            let mut d = String::new();
            scene::move_to(&mut d, px, pc);
            scene::line_to(&mut d, px, py);
            out.push(Item::Path { d, fill: None, stroke: Some(stroke.clone()) });
            let r = pd.plot.symbol.size_pt / 2.0;
            let mut s = String::new();
            scene::move_to(&mut s, px - r, py - r);
            scene::line_to(&mut s, px + r, py - r);
            scene::line_to(&mut s, px + r, py + r);
            scene::line_to(&mut s, px - r, py + r);
            s.push('Z');
            out.push(Item::Path { d: s, fill: Some(pd.plot.symbol.color), stroke: None });
        }
    }
}
