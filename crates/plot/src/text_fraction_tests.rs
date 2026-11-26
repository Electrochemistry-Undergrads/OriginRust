//! Stacked fractions in label markup (T480): `\fr(num,den)`, which LaTeX `\frac` becomes.

use crate::fonts::FixedMetrics;
use crate::scene::{Anchor, Item, TextItem};
use sheaf_core::graph::Font;

fn placed(source: &str) -> Vec<Item> {
    let mut items = Vec::new();
    crate::text::place(
        &mut items,
        &FixedMetrics,
        source,
        &Font::new("Arial", 20.0),
        20.0,
        (10.0, 30.0),
        Anchor::Start,
        0.0,
    );
    items
}

fn texts(items: &[Item]) -> Vec<&TextItem> {
    items.iter().filter_map(|i| if let Item::Text(t) = i { Some(t) } else { None }).collect()
}

fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}

#[test]
fn a_fraction_stacks_its_parts_centred_over_a_bar() {
    // FixedMetrics: every character is 0.55 of the size wide. The parts are at 70 % (14 pt, 7.7 pt a
    // character); the column is the longer part plus 0.08 em on each side (18.6 pt for "12" over "3").
    let items = placed(r"P\fr(12,3)Q");
    let ts = texts(&items);
    assert_eq!(ts.iter().map(|t| t.text.as_str()).collect::<Vec<_>>(), ["P", "12", "3", "Q"]);
    near(ts[1].size, 14.0);
    near(ts[2].size, 14.0);
    // centred in the column that starts after P (x 21)
    near(ts[1].x, 21.0 + (18.6 - 15.4) / 2.0);
    near(ts[2].x, 21.0 + (18.6 - 7.7) / 2.0);
    // the numerator above the baseline, the denominator below it
    near(ts[1].y, 30.0 - 0.55 * 20.0);
    near(ts[2].y, 30.0 + 0.36 * 20.0);
    near(ts[3].x, 21.0 + 18.6);
    near(crate::text::width(&FixedMetrics, r"P\fr(12,3)Q", &Font::new("Arial", 20.0), 20.0), 11.0 + 18.6 + 11.0);
    // one bar across the column, between the two parts
    let bars: Vec<&Item> = items.iter().filter(|i| matches!(i, Item::Path { stroke: Some(_), .. })).collect();
    assert_eq!(bars.len(), 1);
    let Item::Path { d, .. } = bars[0] else { unreachable!() };
    let numbers: Vec<f64> =
        d.split(|c: char| c.is_ascii_alphabetic() || c == ' ').filter_map(|s| s.parse().ok()).collect();
    near(numbers[0], 21.0);
    near(numbers[2], 21.0 + 18.6);
    assert!(numbers[1] > ts[1].y && numbers[1] < ts[2].y, "{d}");
}

#[test]
fn latex_frac_draws_stacked_and_reads_as_a_slash_in_plain_text() {
    let items = placed(r"$\frac{a}{2}$");
    let ts = texts(&items);
    assert_eq!(ts.iter().map(|t| t.text.as_str()).collect::<Vec<_>>(), ["a", "2"]);
    assert!(ts[0].italic && ts[0].y < ts[1].y);
    let runs = crate::markup::parse(r"\fr(1,2)");
    assert_eq!(runs.iter().map(|r| r.text.as_str()).collect::<String>(), "1/2");
    // a part with a comma of its own, and a fraction inside a fraction
    let items = placed(r"$\frac{1,5}{\frac{1}{2}}$");
    assert_eq!(texts(&items).iter().map(|t| t.text.as_str()).collect::<Vec<_>>(), ["1,5", "1", "2"]);
}
