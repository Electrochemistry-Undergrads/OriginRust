use super::*;
use crate::column::{ColumnData, Designation};

fn project(columns: usize) -> (Project, Vec<ColumnId>) {
    let mut p = Project::new();
    let b = p.add_workbook("");
    let ids = (0..columns)
        .map(|k| {
            let values: Vec<f64> = (0..6).map(|i| (i * (k + 1)) as f64 + 0.5 * k as f64).collect();
            let d = if k == 0 { Designation::X } else { Designation::Y };
            p.add_column(b, 0, d, ColumnData::numeric(values)).unwrap()
        })
        .collect();
    (p, ids)
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn a_square_matrix_has_a_layer_per_cell_with_column_c_on_x_and_row_r_on_y() {
    let (mut p, cols) = project(3);
    let g = p.plot_scatter_matrix(&cols, &ScatterMatrixOptions::default()).unwrap();
    let graph = p.graph(g).unwrap();
    assert_eq!(graph.layers.len(), 9);
    assert!(close(graph.page.width_pt, graph.page.height_pt), "the page is square");
    for r in 0..3 {
        for c in 0..3 {
            let layer = &graph.layers[r * 3 + c];
            assert!(layer.link.is_none());
            assert!(!layer.legend.visible);
            if r == c {
                assert!(layer.plots.is_empty(), "no plot on the diagonal by default");
                assert_eq!(layer.labels.len(), 1);
                assert_eq!(layer.labels[0].text, p.column(cols[r]).unwrap().display_name());
            } else {
                let plot = &layer.plots[0];
                assert_eq!((plot.kind, plot.x, plot.y), (PlotKind::Scatter, Some(cols[c]), cols[r]));
                assert!(layer.labels.is_empty());
            }
            // Tick labels on the outer cells only: X along the bottom row, Y down the left column.
            assert_eq!(layer.x_axis.show_tick_labels, r == 2, "X labels of cell ({r}, {c})");
            assert_eq!(layer.y_axis.show_tick_labels, c == 0, "Y labels of cell ({r}, {c})");
            assert!(!layer.x_axis.show_title && !layer.y_axis.show_title);
        }
    }
    // Gap 0: the cells touch, row by row from the top left, inside the margins.
    let (f0, f1, f3) = (graph.layers[0].frame, graph.layers[1].frame, graph.layers[3].frame);
    assert!(close(f0.left, 0.14) && close(f0.top, 0.04));
    assert!(close(f0.left + f0.width, f1.left) && close(f0.top, f1.top));
    assert!(close(f0.top + f0.height, f3.top) && close(f0.left, f3.left));
    let last = graph.layers[8].frame;
    assert!(close(last.left + last.width, 0.96) && close(last.top + last.height, 0.88));
}

#[test]
fn every_cell_of_a_grid_column_or_row_has_its_variables_limits_and_step() {
    let (mut p, cols) = project(3);
    let g = p.plot_scatter_matrix(&cols, &ScatterMatrixOptions::default()).unwrap();
    let graph = p.graph(g).unwrap();
    let margin = graph.layers[0].x_axis.rescale_margin_pct;
    for (k, &col) in cols.iter().enumerate() {
        let (lo, hi, step) = variable_range(p.numeric(col).unwrap(), margin).unwrap();
        for i in 0..3 {
            let x = &graph.layers[i * 3 + k].x_axis;
            assert_eq!((x.min, x.max, x.major_step), (Some(lo), Some(hi), Some(step)), "X of cell ({i}, {k})");
            let y = &graph.layers[k * 3 + i].y_axis;
            assert_eq!((y.min, y.max, y.major_step), (Some(lo), Some(hi), Some(step)), "Y of cell ({k}, {i})");
        }
    }
}

#[test]
fn a_variables_range_is_the_automatic_one_with_at_most_five_steps() {
    // 0 to 19.5: the automatic range -2..22 by 2 has twelve steps; by 5 it has under five.
    let values: Vec<f64> = (0..40).map(|i| f64::from(i) * 0.5).collect();
    let (lo, hi, auto) = crate::autorange::linear_range(0.0, 19.5, 8.0);
    assert!((hi - lo) / auto > MAX_STEPS);
    let (from, to, step) = variable_range(&values, 8.0).unwrap();
    assert_eq!((from, to), (lo, hi));
    assert!(close(step, 5.0));
    assert_eq!(variable_range(&[3.0, 3.0], 8.0), Some((2.0, 4.0, 0.5)));
    assert_eq!(variable_range(&[f64::NAN], 8.0), None);
    assert!(close(wider_step(0.2), 0.5) && close(wider_step(5.0), 10.0) && close(wider_step(10.0), 20.0));
}

#[test]
fn triangular_matrices_leave_out_the_other_half_and_label_their_outer_cells() {
    let (mut p, cols) = project(3);
    let lower = ScatterMatrixOptions { display: MatrixDisplay::Lower, ..Default::default() };
    let g = p.plot_scatter_matrix(&cols, &lower).unwrap();
    let graph = p.graph(g).unwrap();
    assert_eq!(graph.layers.len(), 6);
    // (0,0) (1,0) (1,1) (2,0) (2,1) (2,2): the bottom row labels X, the left column Y.
    let xs: Vec<bool> = graph.layers.iter().map(|l| l.x_axis.show_tick_labels).collect();
    let ys: Vec<bool> = graph.layers.iter().map(|l| l.y_axis.show_tick_labels).collect();
    assert_eq!(xs, [false, false, false, true, true, true]);
    assert_eq!(ys, [true, true, false, true, false, false]);
    assert_eq!(graph.layers[3].plots[0].x, Some(cols[0]));
    assert_eq!(graph.layers[3].plots[0].y, cols[2]);

    let upper = ScatterMatrixOptions { display: MatrixDisplay::Upper, ..Default::default() };
    let g = p.plot_scatter_matrix(&cols, &upper).unwrap();
    let graph = p.graph(g).unwrap();
    assert_eq!(graph.layers.len(), 6);
    // (0,0) (0,1) (0,2) (1,1) (1,2) (2,2): the top row labels X and the right column Y, on those sides.
    let xs: Vec<bool> = graph.layers.iter().map(|l| l.x_axis.show_tick_labels).collect();
    let ys: Vec<bool> = graph.layers.iter().map(|l| l.y_axis.show_tick_labels).collect();
    assert_eq!(xs, [true, true, true, false, false, false]);
    assert_eq!(ys, [false, false, true, false, true, true]);
    assert!(graph.layers.iter().all(|l| l.x_axis.side == AxisSide::Opposite && l.y_axis.side == AxisSide::Opposite));
    assert_eq!(graph.layers[1].plots[0].x, Some(cols[1]));
    assert_eq!(graph.layers[1].plots[0].y, cols[0]);
    assert!(close(graph.layers[5].frame.left, graph.layers[2].frame.left));
}

#[test]
fn histograms_on_the_diagonal_and_gaps_between_cells() {
    let (mut p, cols) = project(2);
    let o =
        ScatterMatrixOptions { diagonal: MatrixDiagonal::Histogram, names: false, gap_pct: 2.0, ..Default::default() };
    let g = p.plot_scatter_matrix(&cols, &o).unwrap();
    let graph = p.graph(g).unwrap();
    for k in [0, 3] {
        let d = &graph.layers[k];
        assert_eq!(d.plots[0].kind, PlotKind::Histogram);
        assert!(d.labels.is_empty());
        assert!(!d.y_axis.show_tick_labels, "the counts get no tick labels");
        assert_eq!(d.y_axis.min, Some(0.0));
        assert!(d.x_axis.min.is_some());
        assert_eq!((d.y_axis.max, d.y_axis.major_step), (None, None), "the counts keep an automatic scale");
    }
    assert_eq!(graph.layers[0].plots[0].y, cols[0]);
    assert_eq!(graph.layers[3].plots[0].y, cols[1]);
    let (f0, f1) = (graph.layers[0].frame, graph.layers[1].frame);
    assert!(close(f1.left - (f0.left + f0.width), 0.02));
}

#[test]
fn wrong_selections_are_refused() {
    let (mut p, cols) = project(9);
    let o = ScatterMatrixOptions::default();
    assert!(matches!(p.plot_scatter_matrix(&cols[..1], &o), Err(ProjectError::ScatterMatrixColumns(1))));
    assert!(matches!(p.plot_scatter_matrix(&cols, &o), Err(ProjectError::ScatterMatrixColumns(9))));
    let wide = ScatterMatrixOptions { gap_pct: 11.0, ..o };
    assert!(matches!(p.plot_scatter_matrix(&cols[..2], &wide), Err(ProjectError::ScatterMatrixGap)));
    let b = p.add_workbook("");
    let text = p.add_column(b, 0, Designation::Y, ColumnData::text(vec!["a".into(), "b".into()])).unwrap();
    assert!(matches!(p.plot_scatter_matrix(&[cols[0], text], &o), Err(ProjectError::ScatterMatrixText(_))));
    let graphs = p.graphs().count();
    assert_eq!(graphs, 0, "nothing is made from a wrong selection");
}

#[test]
fn the_template_route_makes_the_default_matrix() {
    let (mut p, cols) = project(4);
    let g = p.plot_columns(Template::ScatterMatrix, &cols).unwrap();
    let graph = p.graph(g).unwrap();
    assert_eq!(graph.layers.len(), 16);
    // Smaller marks and tick labels for more variables.
    assert!(graph.layers[1].plots[0].symbol.size_pt < 6.0);
    assert!(graph.layers[12].x_axis.tick_font.size_pt <= 18.0);
    let mut fit = Frame { left: 0.0, top: 0.0, width: 0.0, height: 0.0 };
    for l in &graph.layers {
        fit.width = fit.width.max(l.frame.left + l.frame.width);
    }
    assert!(fit.width <= 1.0);
}

#[test]
fn with_no_selection_every_numeric_y_column_is_a_variable_whatever_its_unit() {
    let mut p = Project::new();
    let b = p.add_workbook("");
    p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
    let a = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0, 3.0])).unwrap();
    let c = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![2.0, 5.0])).unwrap();
    p.column_mut(a).unwrap().units = "V".into();
    p.column_mut(c).unwrap().units = "mA".into();
    assert_eq!(p.default_plot_columns(b, 0, Template::ScatterMatrix), vec![a, c]);
}

#[test]
fn an_upper_matrix_mirrors_the_margins_and_names_fit_their_cells() {
    let (mut p, cols) = project(8);
    let upper = ScatterMatrixOptions { display: MatrixDisplay::Upper, ..Default::default() };
    let g = p.plot_scatter_matrix(&cols, &upper).unwrap();
    let graph = p.graph(g).unwrap();
    // The first cell is (0, 0) at the top left, the last (7, 7) at the bottom right.
    let (first, last) = (graph.layers[0].frame, graph.layers.last().unwrap().frame);
    assert!(close(first.left, 0.04) && close(first.top, 0.12));
    assert!(close(last.left + last.width, 0.86) && close(last.top + last.height, 0.96));
    assert_eq!(margins(MatrixDisplay::Lower), (14.0, 4.0, 4.0, 12.0));
    // Eight variables: 9 pt tick labels; a name is 1.25 times that unless it must shrink to fit its cell.
    let page_w = graph.page.width_pt;
    let d = &graph.layers[0];
    assert!(close(d.x_axis.tick_font.size_pt, 9.0));
    let label = name_label(d, "a rather long column name here", page_w, MatrixDiagonal::None);
    let cell_w = d.frame.width * page_w;
    assert!(label.font.size_pt < 1.25 * 9.0);
    assert!(close(30.0 * 0.55 * label.font.size_pt, 0.8 * cell_w));
    assert!(close(label.x, 0.1));
    let short = name_label(d, "A", page_w, MatrixDiagonal::None);
    assert!(close(short.font.size_pt, 1.25 * 9.0));
}
