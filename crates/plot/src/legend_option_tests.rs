//! `%(n, @option)` in a legend, as Origin 2016 substitutes it on a graph (T339).
use super::*;
use sheaf_core::template::Template;
use sheaf_core::{ColumnData, Designation};

#[test]
fn legend_options_give_what_origin_2016_gives() {
    let mut p = Project::new();
    let b = p.add_workbook("BookLong");
    p.add_column(b, 0, Designation::X, ColumnData::numeric((1..=5).map(f64::from).collect())).unwrap();
    let y =
        p.add_column(b, 0, Designation::Y, ColumnData::numeric((1..=5).map(|i| f64::from(i * i)).collect())).unwrap();
    let z =
        p.add_column(b, 0, Designation::Y, ColumnData::numeric((1..=5).map(|i| f64::from(3 * i)).collect())).unwrap();
    {
        let book = p.workbook_mut(b).unwrap();
        book.name = "Book1".into();
        book.sheets[0].name = "Sheet1".into();
        let c = book.sheets[0].column_mut(y).unwrap();
        (c.short_name, c.long_name, c.units, c.comments) = ("B".into(), "Volt".into(), "mV".into(), "cmt".into());
        book.sheets[0].column_mut(z).unwrap().short_name = "C".into();
    }
    let g = p.add_graph(Template::LineSymbol);
    p.add_plot(g, 0, y).unwrap();
    p.add_plot(g, 0, z).unwrap();
    let plots = p.graph(g).unwrap().layers[0].plots.clone();
    // Origin 2016's answers (re-workspace/scratch/t338/probe3): column B with long name, units and
    // comment, column C with none, the book's long name BookLong.
    let cases: [(&str, &str, &str); 21] = [
        ("", "cmt", "C"),
        ("@LL", "Volt", ""),
        ("@LU", "mV", ""),
        ("@LC", "cmt", ""),
        ("@LS", "B", "C"),
        ("@LG", "Volt (mV)", "C"),
        ("@LA", "Volt", "C"),
        ("@LN", "cmt (mV)", "C"),
        ("@L", "Volt", ""),
        ("@U", "Volt (mV)", "C"),
        ("@C", "B", "C"),
        ("@D", "Book1_B", "Book1_C"),
        ("@W", "Book1", "Book1"),
        ("@WS", "Sheet1", "Sheet1"),
        ("@WL", "BookLong", "BookLong"),
        ("@LP", "", ""),
        ("@X", "###", "###"),
        ("@LD", "###", "###"),
        ("@LM", "cmt", "C"),
        ("@N", "###", "###"),
        ("@R", "[Book1]Sheet1!Col(\"Volt\")[1:5]", "[Book1]Sheet1!Col(C)[1:5]"),
    ];
    for (option, first, second) in cases {
        assert_eq!(layout::legend_name(&p, &plots[0], option), first, "{option} on B");
        assert_eq!(layout::legend_name(&p, &plots[1], option), second, "{option} on C");
    }
    // In a drawn legend: the option's text replaces the name.
    p.graph_mut(g).unwrap().layers[0].legend.text = Some(r"\l(1) %(1,@LU) and %(2, @D)".into());
    let svg = render_svg(&p, g).unwrap();
    assert!(svg.contains("mV and Book1_C"), "the substituted legend line");
}
