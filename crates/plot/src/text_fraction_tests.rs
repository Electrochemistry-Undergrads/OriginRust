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

#[test]
fn a_large_operator_takes_its_limits_below_and_above_centred() {
    // T481: ∑ at 140 % (28 pt), its limits at 70 % (14 pt) centred under and over it
    let items = placed(r"$\sum_{i=1}^{n}$");
    let ts = texts(&items);
    let sum = ts.iter().find(|t| t.text == "∑").unwrap();
    near(sum.size, 28.0);
    near(sum.y, 30.0);
    let n = ts.iter().find(|t| t.text == "n").unwrap();
    let one = ts.iter().find(|t| t.text == "=1").unwrap();
    near(n.size, 14.0);
    assert!(n.y < 30.0 - 0.72 * 28.0, "above the operator's ink: {}", n.y);
    assert!(one.y > 30.0 + 0.21 * 28.0, "below the operator's ink: {}", one.y);
    // the column is as wide as the widest part, here the lower limit "i=1" (23.1); "n" (7.7) centred in it
    near(n.x, 10.0 + (23.1 - 7.7) / 2.0);
    near(crate::text::width(&FixedMetrics, r"$\sum_{i=1}^{n}$", &Font::new("Arial", 20.0), 20.0), 23.1);
    // a limit function keeps its size; its limit is centred under it
    let items = placed(r"$\lim_{x\to 0}$");
    let ts = texts(&items);
    assert_eq!(ts[0].text, "lim");
    assert!(ts.iter().skip(1).all(|t| t.y > 30.0), "the limit sits below");
}

#[test]
fn stretchy_delimiters_span_what_they_enclose() {
    // T482: round brackets around a stacked fraction reach above its numerator and below its denominator
    let items = placed(r"$\left(\frac{a}{b}\right)$");
    let ts = texts(&items);
    assert_eq!(ts.iter().map(|t| t.text.as_str()).collect::<Vec<_>>(), ["a", "b"]);
    let paths: Vec<&String> = items
        .iter()
        .filter_map(|i| if let Item::Path { d, stroke: Some(_), .. } = i { Some(d) } else { None })
        .collect();
    assert_eq!(paths.len(), 3, "two delimiters and the bar");
    let ys = |d: &str| -> Vec<f64> {
        let n: Vec<f64> =
            d.split(|c: char| c.is_ascii_alphabetic() || c == ' ').filter_map(|s| s.parse().ok()).collect();
        n.chunks(2).map(|p| p[1]).collect()
    };
    let left = ys(paths[0]);
    let (top, bottom) = (left.iter().copied().fold(f64::MAX, f64::min), left.iter().copied().fold(f64::MIN, f64::max));
    // the numerator's glyph top (at 70 %, ascent 0.72) and the denominator's bottom (descent 0.21)
    assert!(top < ts[0].y - 0.72 * 14.0, "{top}");
    assert!(bottom > ts[1].y + 0.21 * 14.0, "{bottom}");
    // the fraction starts after the left delimiter (0.36 em of 20 pt)
    assert!(ts[0].x > 10.0 + 0.36 * 20.0);
    // plain text keeps the brackets
    let runs = crate::markup::parse(r"\dl(pp,x)");
    assert_eq!(runs.iter().map(|r| r.text.as_str()).collect::<String>(), "(x)");
}

#[test]
fn a_matrix_lays_its_cells_out_in_centred_columns_between_its_delimiters() {
    // T483: a 2 x 2 matrix in round brackets; each column as wide as its widest cell, 0.8 em between columns
    let items = placed(r"$\begin{pmatrix} 1 & 22 \\ 333 & 4 \end{pmatrix}$");
    let ts = texts(&items);
    assert_eq!(ts.iter().map(|t| t.text.as_str()).collect::<Vec<_>>(), ["1", "22", "333", "4"]);
    // the rows: the first above the second, 1.25 em apart
    near(ts[0].y, ts[1].y);
    near(ts[2].y - ts[0].y, 1.25 * 20.0);
    // column 1 is "333" wide (33 pt), column 2 "22" (22 pt); "1" and "4" centred in theirs
    let start = ts[2].x;
    near(ts[0].x, start + (33.0 - 11.0) / 2.0);
    near(ts[1].x, start + 33.0 + 16.0);
    near(ts[3].x, start + 33.0 + 16.0 + (22.0 - 11.0) / 2.0);
    // two delimiters around it
    let paths = items.iter().filter(|i| matches!(i, Item::Path { stroke: Some(_), .. })).count();
    assert_eq!(paths, 2);
    let runs = crate::markup::parse(r"\mx(2,a,b,c,d)");
    assert_eq!(runs.iter().map(|r| r.text.as_str()).collect::<String>(), "a, b; c, d");
}

#[test]
fn a_title_takes_more_room_only_for_tall_math() {
    // T484: plain text and scripts keep a letter's ascent and descent (Origin's measured title gap)
    near(crate::text::tall_extent("Time (s)").0, 0.72);
    near(crate::text::tall_extent("Time (s)").1, 0.21);
    near(crate::text::tall_extent(r"x\+(2)").0, 0.72);
    // a stacked fraction reaches above and below them
    let (top, bottom) = crate::text::tall_extent(r"Rate $\frac{dN}{dt}$");
    assert!(top > 0.72 && bottom > 0.21, "{top} {bottom}");
}
