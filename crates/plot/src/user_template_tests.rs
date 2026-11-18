//! Tests of graphs made from user templates: the text objects and the legend are read against the new data.

use super::*;
use sheaf_core::graph::{Font, TextLabel};
use sheaf_core::template::Template;
use sheaf_core::{ColumnData, ColumnId, Designation, UserTemplate};

fn book(p: &mut Project, long: &str, names: &[&str]) -> Vec<ColumnId> {
    let b = p.add_workbook(long);
    let x: Vec<f64> = (0..=10).map(f64::from).collect();
    p.add_column(b, 0, Designation::X, ColumnData::numeric(x.clone())).unwrap();
    names
        .iter()
        .enumerate()
        .map(|(k, name)| {
            let y = p
                .add_column(b, 0, Designation::Y, ColumnData::numeric(x.iter().map(|v| v * (k + 1) as f64).collect()))
                .unwrap();
            p.column_mut(y).unwrap().long_name = (*name).to_string();
            y
        })
        .collect()
}

#[test]
fn the_legend_and_labels_of_a_template_follow_the_new_data() {
    let mut p = Project::new();
    let old = book(&mut p, "old", &["Old one", "Old two"]);
    let g = p.plot_columns(Template::Line, &old).unwrap();
    {
        let l = &mut p.graph_mut(g).unwrap().layers[0];
        l.legend.text = Some("\\l(1) %(1)\n\\l(2) %(2)".into());
        l.labels.push(TextLabel::new("scan rate 50 mV/s", 0.5, 0.2, Font::new("Arial", 12.0)));
    }
    let template = UserTemplate::from_graph("look", p.graph(g).unwrap()).unwrap();

    let new = book(&mut p, "new", &["Fresh A", "Fresh B", "Fresh C"]);
    let h = p.plot_with_user_template(&template, &new).unwrap();
    // The legend text is the template's; it names whichever plots the layer has now.
    assert_eq!(legend_text(&p, h, 0).unwrap(), "\\l(1) %(1)\n\\l(2) %(2)");
    let svg = render_svg(&p, h).unwrap();
    assert!(svg.contains("Fresh A") && svg.contains("Fresh B"), "the legend names the new columns");
    assert!(!svg.contains("Old one") && !svg.contains("Old two"));
    assert!(svg.contains("scan rate 50 mV/s"), "text objects are kept");
}

#[test]
fn an_automatic_legend_lists_all_the_new_plots() {
    let mut p = Project::new();
    let old = book(&mut p, "old", &["Old one", "Old two"]);
    let g = p.plot_columns(Template::Line, &old).unwrap();
    let template = UserTemplate::from_graph("look", p.graph(g).unwrap()).unwrap();
    let new = book(&mut p, "new", &["A", "B", "C", "D", "E"]);
    let h = p.plot_with_user_template(&template, &new).unwrap();
    assert_eq!(legend_automatic_text(&p, h, 0).unwrap().lines().count(), 5);
    assert_eq!(legend_text(&p, h, 0).unwrap(), legend_automatic_text(&p, h, 0).unwrap());
}
