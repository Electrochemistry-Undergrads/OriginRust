//! Wind roses (T113, graphing spec "Wind rose").

use super::*;
use sheaf_core::template::Template;
use sheaf_core::{ColumnData, Designation};

#[test]
fn raw_data_piles_the_counts_of_the_bands_in_each_sector() {
    let mut p = Project::new();
    let b = p.add_workbook("");
    // Four rows north (two slow, two fast), two east, two south.
    p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![0.0, 5.0, 355.0, 10.0, 90.0, 92.0, 180.0, 178.0]))
        .unwrap();
    let s = p
        .add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0, 1.5, 11.0, 11.5, 3.0, 5.0, 7.0, 9.0]))
        .unwrap();
    let g = p.plot_columns(Template::WindRoseRaw, &[s]).unwrap();
    let graph = p.graph(g).unwrap();
    let ld = data::collect(&p, &graph.layers[0]);
    let roses: Vec<&crate::wind_rose::WindRoseData> = ld.plots.iter().filter_map(|pd| pd.wind_rose.as_ref()).collect();
    assert_eq!(roses.len(), 7, "0-2 ... 10-12 and >= 12 (Origin, T324)");
    assert!(roses.iter().all(|w| w.angles.len() == 16 && w.width == 22.5));
    // North: two rows in the first band (0-2) and two in 10-12, piled; counts as Origin's default.
    assert_eq!(roses[0].values[0], 2.0);
    assert_eq!(roses[5].values[0], 2.0);
    assert_eq!(roses[5].base[0], 2.0);
    let top = roses.iter().map(|w| w.base[0] + w.values[0]).fold(0.0, f64::max);
    assert_eq!(top, 4.0);
    // The radius axis reaches the tallest pile; every non-empty band sector is one wedge.
    let geom = &layout_geometry(&p, g).unwrap()[0];
    assert!(geom.y.max >= 4.0, "{:?}", geom.y);
    let s = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let wedges = s.layers[0].plots.iter().filter(|it| matches!(it, scene::Item::Path { fill: Some(_), .. })).count();
    assert_eq!(wedges, 2 + 2 + 2, "north 2 bands, east 2, south 2");
}

#[test]
fn binned_data_wedges_are_a_sixteenth_of_the_circle_as_origin() {
    // T327, measured in Origin 2016: eight directions 45 degrees apart still get 22.5 degree wedges.
    let mut p = Project::new();
    let b = p.add_workbook("");
    let dirs: Vec<f64> = (0..8).map(|k| f64::from(k) * 45.0).collect();
    p.add_column(b, 0, Designation::X, ColumnData::numeric(dirs)).unwrap();
    let f =
        p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![3.0, 1.0, 2.0, 0.0, 4.0, 1.0, 2.0, 1.0])).unwrap();
    let g = p.plot_columns(Template::WindRoseBinned, &[f]).unwrap();
    let ld = data::collect(&p, &p.graph(g).unwrap().layers[0]);
    let rose = ld.plots[0].wind_rose.as_ref().unwrap();
    assert_eq!((rose.angles.len(), rose.width), (8, 22.5));
}
