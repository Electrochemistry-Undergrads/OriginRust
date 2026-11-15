use super::*;
use crate::scene::{Item, Scene};
use sheaf_core::graph::{LabelForm, Plot, PlotKind};
use sheaf_core::template::Template;
use sheaf_core::{Color, ColumnData, ColumnId, Designation};

const HORIZONTAL: Color = Color::rgb(11, 22, 33);
const VERTICAL: Color = Color::rgb(44, 55, 66);

fn project() -> (Project, WindowId, WindowId, ColumnId) {
    let mut p = Project::new();
    let book = p.add_workbook("Data");
    let x = p.add_column(book, 0, Designation::X, ColumnData::numeric((1..=11).map(f64::from).collect())).unwrap();
    let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![5.0; 11])).unwrap();
    let e = p.add_column(book, 0, Designation::YError, ColumnData::numeric(vec![0.5; 11])).unwrap();
    let g = p.add_graph(Template::LineSymbol);
    p.add_plot_xy(g, 0, Some(x), y, Some(e)).unwrap();
    let plot = plot_mut(&mut p, g);
    plot.x_error = Some(e);
    plot.line.color = Color::BLACK;
    plot.line.width_pt = 1.0;
    plot.symbol.color = Color::RED;
    plot.symbol.filled = true;
    plot.decor.drop.horizontal.on = true;
    plot.decor.drop.horizontal.color = Some(HORIZONTAL);
    plot.decor.drop.vertical.on = true;
    plot.decor.drop.vertical.color = Some(VERTICAL);
    plot.decor.labels.enable = true;
    plot.decor.labels.form = LabelForm::Row;
    (p, g, book, y)
}

fn plot_mut(p: &mut Project, g: WindowId) -> &mut Plot {
    &mut p.graph_mut(g).unwrap().layers[0].plots[0]
}

fn scene(p: &Project, g: WindowId) -> Scene {
    layout::build_scene(p, g, &FixedMetrics).unwrap()
}

fn labels(s: &Scene) -> Vec<&str> {
    s.layers[0]
        .plots
        .iter()
        .filter_map(|item| match item {
            Item::Text(t) => Some(t.text.as_str()),
            _ => None,
        })
        .collect()
}

fn path(s: &Scene, color: Color, width: f64) -> &str {
    s.layers[0]
        .plots
        .iter()
        .find_map(|item| match item {
            Item::Path { d, stroke: Some(st), .. } if st.color == color && st.width == width => Some(d.as_str()),
            _ => None,
        })
        .unwrap()
}

fn symbols(s: &Scene, color: Color) -> usize {
    s.layers[0]
        .plots
        .iter()
        .filter_map(|item| match item {
            Item::Path { d, fill: Some(c), .. } if *c == color => Some(d.matches('Z').count()),
            _ => None,
        })
        .sum()
}

#[test]
fn skip_points_applies_the_origin_2016_frequency_to_all_point_decorations() {
    let (mut p, g, _, y) = project();
    let before = scene(&p, g);
    let geometry = layout_geometry(&p, g).unwrap();
    let source = p.column(y).unwrap().numeric().unwrap().to_vec();
    plot_mut(&mut p, g).symbol.every = 3;
    let after = scene(&p, g);
    assert_eq!(labels(&after), ["1", "4", "7", "10"]);
    assert_eq!(symbols(&after, Color::RED), 4);
    assert_eq!(path(&after, HORIZONTAL, 0.5).matches('M').count(), 4);
    assert_eq!(path(&after, VERTICAL, 0.5).matches('M').count(), 4);
    assert_eq!(path(&after, Color::BLACK, 0.5).matches('M').count(), 24, "X and Y errors with both caps");
    assert_eq!(path(&before, Color::BLACK, 1.0), path(&after, Color::BLACK, 1.0), "the full data line stays");
    assert_eq!(geometry, layout_geometry(&p, g).unwrap(), "the axis range still uses every source value");
    assert_eq!(p.column(y).unwrap().numeric().unwrap(), source);
}

#[test]
fn skip_points_counts_missing_rows_and_starts_at_the_plot_range() {
    let (mut p, g, _, y) = project();
    plot_mut(&mut p, g).symbol.every = 3;
    p.column_mut(y).unwrap().numeric_mut().unwrap()[3] = f64::NAN;
    assert_eq!(labels(&scene(&p, g)), ["1", "7", "10"], "missing row 4 does not shift the frequency");
    plot_mut(&mut p, g).rows = Some((2, 10));
    assert_eq!(labels(&scene(&p, g)), ["3", "6", "9"]);
    plot_mut(&mut p, g).rows = Some((10, 10));
    assert!(scene(&p, g).layers[0].plots.is_empty());
}

#[test]
fn skip_points_has_no_forced_endpoint_and_handles_zero_one_and_large_frequencies() {
    let (mut p, g, _, _) = project();
    for frequency in [0, 1, 2, 3, u32::MAX] {
        plot_mut(&mut p, g).symbol.every = frequency;
        let s = scene(&p, g);
        let expected = match frequency {
            0 | 1 => 11,
            2 => 6,
            3 => 4,
            _ => 1,
        };
        assert_eq!(labels(&s).len(), expected, "frequency {frequency}");
        assert_eq!(symbols(&s, Color::RED), expected);
    }
}

#[test]
fn skip_points_does_not_resurrect_masked_or_filtered_symbols() {
    let (mut p, g, book, y) = project();
    plot_mut(&mut p, g).symbol.every = 3;
    p.column_mut(y).unwrap().mask.extend([1, 3]);
    let s = scene(&p, g);
    // The base symbols are red too: give them another colour to distinguish the mask overlay.
    plot_mut(&mut p, g).symbol.color = Color::BLUE;
    let masked = scene(&p, g);
    assert_eq!(symbols(&masked, Color::RED), 1, "only retained masked row 4 gets an extra red symbol");
    assert_eq!(labels(&s), labels(&masked));
    p.set_row_filter(book, 0, "i != 1").unwrap();
    assert_eq!(labels(&scene(&p, g)), ["2", "5", "8", "11"], "filter compression keeps source-row labels");
}

#[test]
fn skip_points_also_applies_to_bubbles_and_does_not_reduce_bars_or_areas() {
    let (mut p, g, book, _) = project();
    let z = p.add_column(book, 0, Designation::Z, ColumnData::numeric(vec![2.0; 11])).unwrap();
    plot_mut(&mut p, g).size_from = Some(z);
    plot_mut(&mut p, g).symbol.every = 3;
    assert_eq!(symbols(&scene(&p, g), Color::RED), 4, "varied symbols obey the same frequency");
    for kind in [PlotKind::Column, PlotKind::Area] {
        plot_mut(&mut p, g).kind = kind;
        assert_eq!(labels(&scene(&p, g)).len(), 11, "{kind:?} has no Skip Points tab");
    }
}
