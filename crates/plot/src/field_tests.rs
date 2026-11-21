//! Heat maps and contour plots: golden images and what the pictures must contain.

use super::*;
use sheaf_core::template::FieldTemplate;
use sheaf_core::{ColumnData, Designation};

/// A project with a 14 x 11 matrix whose values use only plain arithmetic (so that the golden
/// images do not depend on the platform's math library).
fn matrix_project() -> (Project, WindowId) {
    let mut p = Project::new();
    let m = p.add_matrix_book("field");
    {
        let s = &mut p.matrix_book_mut(m).unwrap().sheets[0];
        s.set_dimensions(11, 14, Some((-1.0, 2.0, 0.0, 5.0))).unwrap();
        s.set_values("x*x/4 - y*y/3 + x*y/8").unwrap();
        s.x.long_name = "Potential".into();
        s.x.units = "V".into();
        s.y.long_name = "Time".into();
        s.y.units = "s".into();
        s.z.long_name = "Current".into();
        s.z.units = "mA".into();
    }
    (p, m)
}

fn graph_of(template: FieldTemplate) -> (Project, WindowId) {
    let (mut p, m) = matrix_project();
    let g = p.plot_matrix(template, m, 0).unwrap();
    (p, g)
}

fn svg(p: &Project, g: WindowId) -> String {
    render_svg_with(p, g, &FixedMetrics).unwrap()
}

fn texts(svg: &str) -> Vec<String> {
    let d = roxmltree::Document::parse(svg).expect("well-formed XML");
    d.descendants().filter(|n| n.has_tag_name("text")).map(|n| n.text().unwrap_or("").to_string()).collect()
}

fn plots_group(svg: &str) -> Vec<(String, String)> {
    // (tag, fill or stroke) of the children of the clipped plots group
    let d = roxmltree::Document::parse(svg).expect("well-formed XML");
    let g = d.descendants().find(|n| n.attribute("class") == Some("plots")).expect("plots group");
    g.children()
        .filter(|c| c.is_element())
        .map(|c| (c.tag_name().name().to_string(), c.attribute("fill").unwrap_or("").to_string()))
        .collect()
}

#[test]
fn golden_contour_fill_of_a_matrix() {
    let (p, g) = graph_of(FieldTemplate::ContourFill);
    crate::tests::compare_golden("matrix-contour", &svg(&p, g));
}

#[test]
fn golden_contour_lines_of_a_matrix() {
    let (p, g) = graph_of(FieldTemplate::ContourLines);
    crate::tests::compare_golden("matrix-contour-lines", &svg(&p, g));
}

#[test]
fn golden_heat_map_of_a_matrix() {
    let (p, g) = graph_of(FieldTemplate::HeatMap);
    crate::tests::compare_golden("matrix-heatmap", &svg(&p, g));
}

#[test]
fn golden_gray_scale_map_of_a_matrix() {
    let (p, g) = graph_of(FieldTemplate::GrayScale);
    let svg = svg(&p, g);
    // Black for the lowest band, white for the highest (Origin's 32 grays); the colour scale has
    // Origin's blue box for values above it and red for values below.
    assert!(svg.contains("#000000") && svg.contains("#FFFFFF"), "gray ramp");
    assert!(svg.contains("#0000FF") && svg.contains("#FF0000"), "out-of-range boxes");
    // 8 major steps of 6 bands, X tick labels upright (measured).
    let f = p.graph(g).unwrap().layers[0].field.as_ref().unwrap().clone();
    assert_eq!((f.levels.major, f.levels.minor), (8, 6));
    assert_eq!(p.graph(g).unwrap().layers[0].x_axis.tick_label_angle_deg, 0.0);
    crate::tests::compare_golden("matrix-grayscale", &svg);
}

#[test]
fn golden_contour_of_scattered_xyz_columns() {
    // 25 points off a grid, on a smooth surface of plain arithmetic.
    let mut p = Project::new();
    let book = p.add_workbook("scatter");
    let pts: Vec<(f64, f64)> = (0..25)
        .map(|i| {
            let (a, b) = ((i * 7 % 25) as f64, (i * 11 % 25) as f64);
            (a / 5.0 + (b % 3.0) * 0.07, b / 5.0 + (a % 4.0) * 0.05)
        })
        .collect();
    let col = |p: &mut Project, d, v: Vec<f64>| p.add_column(book, 0, d, ColumnData::numeric(v)).unwrap();
    let x = col(&mut p, Designation::X, pts.iter().map(|q| q.0).collect());
    let y = col(&mut p, Designation::Y, pts.iter().map(|q| q.1).collect());
    let z = col(&mut p, Designation::Z, pts.iter().map(|q| q.0 * q.0 - q.1 * q.0 / 2.0 + q.1).collect());
    let g = p.plot_xyz(FieldTemplate::ContourFill, x, y, z).unwrap();
    crate::tests::compare_golden("xyz-contour-scattered", &svg(&p, g));
}

#[test]
fn the_colour_scale_has_a_label_at_every_major_level_and_a_cell_beyond_each_end() {
    let (p, g) = graph_of(FieldTemplate::ContourFill);
    let s = svg(&p, g);
    let t = texts(&s);
    // 8 major steps: 9 labels, written with four significant digits.
    let labels: Vec<&String> =
        t.iter().filter(|x| x.parse::<f64>().is_ok() && x.contains('.') && x.len() >= 5).collect();
    assert!(labels.len() >= 9, "{t:?}");
    // The scale title is the Z label of the matrix.
    assert!(t.contains(&"Current (mA)".to_string()), "{t:?}");
    // Axis titles come from the matrix too.
    assert!(t.contains(&"Potential (V)".to_string()) && t.contains(&"Time (s)".to_string()), "{t:?}");
}

#[test]
fn contour_fill_paints_one_path_per_band_and_one_path_of_lines() {
    let (p, g) = graph_of(FieldTemplate::ContourFill);
    let items = plots_group(&svg(&p, g));
    let filled = items.iter().filter(|(tag, fill)| tag == "path" && !fill.is_empty() && fill != "none").count();
    assert!(filled > 10 && filled <= 40, "at most one path per band (8 x 5 = 40): {filled}");
    let lines = items.iter().filter(|(tag, fill)| tag == "path" && (fill.is_empty() || fill == "none")).count();
    assert_eq!(lines, 1, "all level lines are one path");
}

#[test]
fn contour_lines_have_labels_but_no_fill_and_no_scale() {
    let (p, g) = graph_of(FieldTemplate::ContourLines);
    let s = svg(&p, g);
    let items = plots_group(&s);
    assert!(items.iter().all(|(tag, fill)| tag != "path" || fill.is_empty() || fill == "none"));
    assert!(items.iter().any(|(tag, _)| tag == "rect"), "label backgrounds");
    let t = texts(&s);
    assert!(t.iter().any(|x| x.starts_with('-') && x.len() <= 5), "levels on the lines: {t:?}");
}

#[test]
fn a_heat_map_has_blocks_and_extends_the_axes_by_half_a_cell() {
    let (p, g) = graph_of(FieldTemplate::HeatMap);
    let items = plots_group(&svg(&p, g));
    let blocks = items.iter().filter(|(tag, _)| tag == "rect").count();
    assert!(blocks > 0 && blocks <= 14 * 11, "{blocks}");
    let geoms = layout_geometry(&p, g).unwrap();
    // 14 columns over 3 units: the step is 3/13, so the axis reaches 1/26 beyond both ends.
    let (lo, hi) = (geoms[0].x.min, geoms[0].x.max);
    assert!((lo - (-1.0 - 3.0 / 26.0)).abs() < 1e-9 && (hi - (2.0 + 3.0 / 26.0)).abs() < 1e-9, "{lo} {hi}");
    // A contour plot keeps the axes exactly on the first and last column.
    let (p, g) = graph_of(FieldTemplate::ContourFill);
    let geoms = layout_geometry(&p, g).unwrap();
    assert_eq!((geoms[0].x.min, geoms[0].x.max, geoms[0].y.min, geoms[0].y.max), (-1.0, 2.0, 0.0, 5.0));
}

#[test]
fn a_missing_matrix_value_leaves_a_hole() {
    let (mut p, g) = graph_of(FieldTemplate::HeatMap);
    let full = plots_group(&svg(&p, g)).len();
    let m = p.matrix_books().next().unwrap().id;
    p.matrix_sheet_mut(m, 0).unwrap().set_value(5, 5, f64::NAN).unwrap();
    let with_hole = plots_group(&svg(&p, g)).len();
    assert!(with_hole >= full, "the row is cut in two pieces at the hole");
    let sheet = p.matrix_sheet(m, 0).unwrap();
    assert!(sheet.value(5, 5).is_nan());
}

#[test]
fn a_graph_of_a_matrix_that_is_gone_draws_the_axes_only() {
    let (mut p, g) = graph_of(FieldTemplate::ContourFill);
    let m = p.matrix_books().next().unwrap().id;
    p.remove_window(m);
    let s = svg(&p, g);
    assert!(roxmltree::Document::parse(&s).is_ok());
    assert!(plots_group(&s).is_empty());
}

#[test]
fn xyz_on_a_regular_grid_plots_like_the_matrix() {
    let (pm, gm) = graph_of(FieldTemplate::ContourFill);
    let m = pm.matrix_books().next().unwrap().id;
    let s = pm.matrix_sheet(m, 0).unwrap();
    let mut p = Project::new();
    let book = p.add_workbook("xyz");
    let (mut xs, mut ys, mut zs) = (vec![], vec![], vec![]);
    for r in 0..s.rows {
        for c in 0..s.cols {
            xs.push(s.x_at(c));
            ys.push(s.y_at(r));
            zs.push(s.value(r, c));
        }
    }
    let col = |p: &mut Project, d, v: Vec<f64>| p.add_column(book, 0, d, ColumnData::numeric(v)).unwrap();
    let (x, y, z) = (col(&mut p, Designation::X, xs), col(&mut p, Designation::Y, ys), col(&mut p, Designation::Z, zs));
    let g = p.plot_xyz(FieldTemplate::ContourFill, x, y, z).unwrap();
    // The same colours and lines; only the axis titles differ (the columns have no labels).
    let a = plots_group(&svg(&pm, gm));
    let b = plots_group(&svg(&p, g));
    assert_eq!(a.len(), b.len());
}

#[test]
fn the_levels_can_be_set_by_hand() {
    let (mut p, g) = graph_of(FieldTemplate::ContourFill);
    {
        let f = p.graph_mut(g).unwrap().layers[0].field.as_mut().unwrap();
        f.levels.min = Some(-1.0);
        f.levels.max = Some(3.0);
        f.levels.major = 4;
        f.levels.minor = 1;
    }
    let t = texts(&svg(&p, g));
    for want in ["-1.000", "0.000", "1.000", "2.000", "3.000"] {
        assert!(t.contains(&want.to_string()), "{want} in {t:?}");
    }
}

#[test]
fn a_polar_contour_is_drawn_inside_the_circle() {
    // z = r over a grid of angles 0..330 and radii 1..4: the contour bands are rings.
    let mut p = Project::new();
    let book = p.add_workbook("polar");
    let (mut th, mut r, mut z) = (Vec::new(), Vec::new(), Vec::new());
    for i in 0..12 {
        for k in 1..=4 {
            th.push(f64::from(i) * 30.0);
            r.push(f64::from(k));
            z.push(f64::from(k));
        }
    }
    let t = p.add_column(book, 0, Designation::X, ColumnData::numeric(th)).unwrap();
    let rc = p.add_column(book, 0, Designation::Y, ColumnData::numeric(r)).unwrap();
    let zc = p.add_column(book, 0, Designation::Z, ColumnData::numeric(z)).unwrap();
    let g = p.plot_xyz_polar(t, rc, zc).unwrap();
    let graph = p.graph(g).unwrap();
    assert!(graph.layers[0].coords.polar().is_some(), "a polar layer");
    assert!(graph.layers[0].field.is_some(), "with a contour field");
    let geom = crate::layout_geometry(&p, g).unwrap();
    let f = geom[0].frame;
    let (cx, cy, radius) = (f.x + f.w / 2.0, f.y + f.h / 2.0, f.w.min(f.h) / 2.0);
    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let filled: Vec<&crate::scene::Item> =
        scene.layers[0].plots.iter().filter(|i| matches!(i, crate::scene::Item::Path { fill: Some(_), .. })).collect();
    assert!(!filled.is_empty(), "the contour bands are drawn");
    for item in filled {
        if let crate::scene::Item::Path { d, .. } = item {
            for (x, y) in crate::hit::path_points(d) {
                assert!((x - cx).hypot(y - cy) <= radius * 1.02, "({x}, {y}) lies outside the circle");
            }
        }
    }
}

#[test]
fn a_ternary_contour_is_drawn_inside_the_triangle() {
    // A grid over the triangle in percent; the value is the share of A.
    let mut p = Project::new();
    let book = p.add_workbook("ternary");
    let (mut a, mut b, mut c, mut z) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for i in 0..=10 {
        for j in 0..=(10 - i) {
            a.push(f64::from(i) * 10.0);
            b.push(f64::from(j) * 10.0);
            c.push(f64::from(10 - i - j) * 10.0);
            z.push(f64::from(i));
        }
    }
    let cols: Vec<_> = [a, b, c, z]
        .into_iter()
        .zip([Designation::X, Designation::Y, Designation::Z, Designation::Z])
        .map(|(v, d)| p.add_column(book, 0, d, ColumnData::numeric(v)).unwrap())
        .collect();
    let g = p.plot_ternary_contour(cols[0], cols[1], cols[2], cols[3]).unwrap();
    let graph = p.graph(g).unwrap();
    assert!(graph.layers[0].coords.ternary().is_some() && graph.layers[0].field.is_some());
    let geom = crate::layout_geometry(&p, g).unwrap();
    let map = geom[0].ternary.as_ref().expect("a ternary map");
    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let filled: Vec<&crate::scene::Item> =
        scene.layers[0].plots.iter().filter(|i| matches!(i, crate::scene::Item::Path { fill: Some(_), .. })).collect();
    assert!(!filled.is_empty(), "the contour bands are drawn");
    for item in filled {
        if let crate::scene::Item::Path { d, .. } = item {
            for (x, y) in crate::hit::path_points(d) {
                let (fa, fb) = map.fractions(x, y);
                assert!(fa >= -0.02 && fb >= -0.02 && fa + fb <= 1.02, "({x}, {y}) lies outside the triangle");
            }
        }
    }
}
