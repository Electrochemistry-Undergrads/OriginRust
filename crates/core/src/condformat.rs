//! Worksheet > Conditional Formatting: rules that colour cells by their value, live as the values
//! change. A rule holds a range of the sheet, what it looks for (the minimum and maximum of each
//! row or column, or the top or bottom N items or N %) and the colours of what it finds.
//!
//! Evaluation is a pure function ([`evaluate`]): the sheet and its rules give a [`CellStyles`]
//! that the grid reads. Origin's documentation names the two rule kinds, the default colours of
//! Min Max and that the upper rule wins where rules overlap; everything else here (ties, masked
//! cells, how N % rounds, Top Bottom working on each column alone) is Sheaf's own choice and
//! unmeasured, see docs/spec/gui-look.md.

use crate::color::Color;
use crate::ids::ColumnId;
use crate::worksheet::Worksheet;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Origin's Highlight Min Max defaults (`wcolorbycr`, RGB ints 30067455, 20132812, 33085116,
/// 26692113).
pub const MAX_FILL: Color = Color::rgb(255, 202, 202);
pub const MAX_FONT: Color = Color::rgb(204, 51, 51);
pub const MIN_FILL: Color = Color::rgb(188, 214, 248);
pub const MIN_FONT: Color = Color::rgb(17, 74, 151);
/// Sheaf's own defaults for Top Bottom (Origin's are not documented): the Min Max "max" colours
/// for the top, the "min" colours for the bottom.
pub const TOP_FILL: Color = MAX_FILL;
pub const TOP_FONT: Color = MAX_FONT;
pub const BOTTOM_FILL: Color = MIN_FILL;
pub const BOTTOM_FONT: Color = MIN_FONT;
/// Sheaf's default N of Top Bottom ("Top 10 Items").
pub const DEFAULT_N: f64 = 10.0;

/// What one class of highlighted cells looks like; `None` leaves that part as it was.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Colours {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<Color>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font: Option<Color>,
}

impl Colours {
    pub fn is_none(&self) -> bool {
        self.fill.is_none() && self.font.is_none()
    }
}

/// Origin's `dir` of Highlight Min Max: 0 row-wise (the default), 1 column-wise.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    #[default]
    RowWise,
    ColumnWise,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Rule {
    /// The largest and smallest number of every row or every column of the range, each row or
    /// column on its own.
    MinMax { dir: Direction, max: Colours, min: Colours },
    /// The top or bottom `n` items (or `n` percent of the numbers) of every column of the range.
    TopBottom { top: bool, n: f64, percent: bool, colours: Colours },
}

impl Rule {
    /// Highlight Min Max with Origin's default direction and colours.
    pub fn min_max() -> Rule {
        Rule::MinMax {
            dir: Direction::RowWise,
            max: Colours { fill: Some(MAX_FILL), font: Some(MAX_FONT) },
            min: Colours { fill: Some(MIN_FILL), font: Some(MIN_FONT) },
        }
    }

    /// Highlight Top Bottom with Sheaf's defaults: Top 10 Items.
    pub fn top_bottom() -> Rule {
        Rule::TopBottom {
            top: true,
            n: DEFAULT_N,
            percent: false,
            colours: Colours { fill: Some(TOP_FILL), font: Some(TOP_FONT) },
        }
    }
}

/// The cells a rule works on: columns by id (taken in sheet order) and a block of rows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CondRange {
    pub columns: Vec<ColumnId>,
    /// First row, 0-based.
    pub first_row: usize,
    /// Last row, 0-based; `None` runs to the end of the data (Origin's `R0`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_row: Option<usize>,
}

impl CondRange {
    pub fn whole_columns(columns: Vec<ColumnId>) -> Self {
        CondRange { columns, first_row: 0, last_row: None }
    }

    fn rows(&self, row_count: usize) -> std::ops::Range<usize> {
        self.first_row.min(row_count)..self.last_row.map_or(row_count, |l| l.saturating_add(1)).min(row_count)
    }

    /// True when the two ranges share a cell (a rule that runs to the end reaches any later row).
    pub fn intersects(&self, other: &CondRange) -> bool {
        let last = |r: &CondRange| r.last_row.unwrap_or(usize::MAX);
        self.columns.iter().any(|c| other.columns.contains(c))
            && self.first_row <= last(other)
            && other.first_row <= last(self)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CondFormat {
    pub name: String,
    pub range: CondRange,
    pub rule: Rule,
}

impl CondFormat {
    pub fn new(sheet: &Worksheet, range: CondRange, rule: Rule) -> Self {
        CondFormat { name: default_name(sheet, &range), range, rule }
    }
}

/// Origin's default rule name: the first and last cell of the range, `C<c1>R<r1>-C<c2>R<r2>`
/// (1-based column positions and rows; the row 0 stands for "to the end").
pub fn default_name(sheet: &Worksheet, range: &CondRange) -> String {
    let positions: Vec<usize> = range.columns.iter().filter_map(|id| sheet.column_index(*id)).collect();
    let c1 = positions.iter().min().map_or(0, |p| p + 1);
    let c2 = positions.iter().max().map_or(0, |p| p + 1);
    let last = range.last_row.map_or(0, |l| l + 1);
    format!("C{c1}R{}-C{c2}R{last}", range.first_row + 1)
}

/// What a rule does to one cell.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CellStyle {
    pub fill: Option<Color>,
    pub font: Option<Color>,
}

impl From<Colours> for CellStyle {
    fn from(c: Colours) -> Self {
        CellStyle { fill: c.fill, font: c.font }
    }
}

/// The styles the rules of a sheet give its cells: one small index per row of each touched column.
#[derive(Clone, Debug, Default)]
pub struct CellStyles {
    /// Entry 0 of `palette` is unused (no style).
    palette: Vec<CellStyle>,
    columns: HashMap<ColumnId, Vec<u16>>,
}

impl CellStyles {
    pub fn is_empty(&self) -> bool {
        self.columns.is_empty()
    }

    /// The style of a cell, if a rule highlights it.
    pub fn get(&self, column: ColumnId, row: usize) -> Option<CellStyle> {
        let slot = *self.columns.get(&column)?.get(row)?;
        (slot != 0).then(|| self.palette[usize::from(slot)])
    }
}

struct Painter<'a> {
    styles: &'a mut CellStyles,
    rows: usize,
    palette_of: HashMap<(usize, u8), u16>,
}

impl Painter<'_> {
    /// Gives the cell the style of (`rule`, `class`) unless an earlier rule already styled it.
    fn paint(&mut self, column: ColumnId, row: usize, rule: usize, class: u8, colours: Colours) {
        if colours.is_none() {
            return;
        }
        let slot = *self.palette_of.entry((rule, class)).or_insert_with(|| {
            self.styles.palette.push(colours.into());
            (self.styles.palette.len() - 1) as u16
        });
        let rows = self.rows;
        let cells = self.styles.columns.entry(column).or_insert_with(|| vec![0; rows]);
        if let Some(cell) = cells.get_mut(row)
            && *cell == 0
        {
            *cell = slot;
        }
    }
}

/// The most rules evaluated (the palette index is 16 bits wide; a sheet never gets near it).
const MAX_RULES: usize = 10_000;

/// The number of top or bottom items for `n` and `percent` among `count` numbers: N % rounds up,
/// so that any non-empty selection of a non-empty column highlights at least one cell.
pub fn item_count(n: f64, percent: bool, count: usize) -> usize {
    if n.is_nan() || n <= 0.0 || count == 0 {
        return 0;
    }
    let k = if percent { (n / 100.0 * count as f64).ceil() } else { n.floor() };
    (k.max(1.0) as usize).min(count)
}

/// Evaluates the rules on the sheet's current values. Only numbers count: text, empty, NaN and
/// masked cells are ignored (a masked cell takes no part in ranking and is never coloured).
/// Every cell equal to an extreme is highlighted (ties); where a row or column has one number it
/// is both its maximum and minimum and counts as the maximum. Where rules overlap the earlier
/// rule in the list wins the whole cell.
pub fn evaluate(sheet: &Worksheet, rules: &[CondFormat]) -> CellStyles {
    let mut styles = CellStyles::default();
    if rules.is_empty() {
        return styles;
    }
    styles.palette.push(CellStyle::default());
    let row_count = sheet.row_count();
    let mut painter = Painter { styles: &mut styles, rows: row_count, palette_of: HashMap::new() };
    for (index, rule) in rules.iter().enumerate().take(MAX_RULES) {
        // The range's columns in sheet order, numeric ones only.
        let columns: Vec<(ColumnId, &[f64], &std::collections::BTreeSet<usize>)> = sheet
            .columns
            .iter()
            .filter(|c| rule.range.columns.contains(&c.id))
            .filter_map(|c| c.numeric().map(|v| (c.id, v, &c.mask)))
            .collect();
        let rows = rule.range.rows(row_count);
        let value = |col: &(ColumnId, &[f64], &std::collections::BTreeSet<usize>), r: usize| -> Option<f64> {
            let v = *col.1.get(r)?;
            (!v.is_nan() && !col.2.contains(&r)).then_some(v)
        };
        match &rule.rule {
            Rule::MinMax { dir: Direction::ColumnWise, max, min } => {
                for col in &columns {
                    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
                    for r in rows.clone() {
                        if let Some(v) = value(col, r) {
                            lo = lo.min(v);
                            hi = hi.max(v);
                        }
                    }
                    if lo > hi {
                        continue;
                    }
                    for r in rows.clone() {
                        match value(col, r) {
                            Some(v) if v == hi => painter.paint(col.0, r, index, 0, *max),
                            Some(v) if v == lo => painter.paint(col.0, r, index, 1, *min),
                            _ => {}
                        }
                    }
                }
            }
            Rule::MinMax { dir: Direction::RowWise, max, min } => {
                for r in rows.clone() {
                    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
                    for col in &columns {
                        if let Some(v) = value(col, r) {
                            lo = lo.min(v);
                            hi = hi.max(v);
                        }
                    }
                    if lo > hi {
                        continue;
                    }
                    for col in &columns {
                        match value(col, r) {
                            Some(v) if v == hi => painter.paint(col.0, r, index, 0, *max),
                            Some(v) if v == lo => painter.paint(col.0, r, index, 1, *min),
                            _ => {}
                        }
                    }
                }
            }
            Rule::TopBottom { top, n, percent, colours } => {
                for col in &columns {
                    let mut values: Vec<f64> = rows.clone().filter_map(|r| value(col, r)).collect();
                    let k = item_count(*n, *percent, values.len());
                    if k == 0 {
                        continue;
                    }
                    // The k-th largest (or smallest) value is the threshold; ties with it count.
                    let threshold = if *top {
                        let (_, kth, _) = values.select_nth_unstable_by(k - 1, |a, b| b.total_cmp(a));
                        *kth
                    } else {
                        let (_, kth, _) = values.select_nth_unstable_by(k - 1, |a, b| a.total_cmp(b));
                        *kth
                    };
                    for r in rows.clone() {
                        if let Some(v) = value(col, r)
                            && (if *top { v >= threshold } else { v <= threshold })
                        {
                            painter.paint(col.0, r, index, 0, *colours);
                        }
                    }
                }
            }
        }
    }
    styles
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::{Column, ColumnData, Designation};

    fn sheet(cols: &[&[f64]]) -> Worksheet {
        let mut s = Worksheet::new("Sheet1");
        for (i, v) in cols.iter().enumerate() {
            s.columns.push(Column::new(
                ColumnId(i as u64 + 1),
                crate::column::default_short_name(i),
                Designation::Y,
                ColumnData::numeric(v.to_vec()),
            ));
        }
        s
    }

    fn all(s: &Worksheet) -> CondRange {
        CondRange::whole_columns(s.columns.iter().map(|c| c.id).collect())
    }

    fn rule(s: &Worksheet, rule: Rule) -> CondFormat {
        CondFormat::new(s, all(s), rule)
    }

    fn fill_of(st: &CellStyles, col: u64, row: usize) -> Option<Color> {
        st.get(ColumnId(col), row).and_then(|c| c.fill)
    }

    fn column_wise() -> Rule {
        match Rule::min_max() {
            Rule::MinMax { max, min, .. } => Rule::MinMax { dir: Direction::ColumnWise, max, min },
            _ => unreachable!(),
        }
    }

    fn top(top: bool, n: f64, percent: bool) -> Rule {
        Rule::TopBottom {
            top,
            n,
            percent,
            colours: Colours { fill: Some(if top { TOP_FILL } else { BOTTOM_FILL }), font: None },
        }
    }

    #[test]
    fn defaults_are_origins_min_max_colours() {
        assert_eq!((MAX_FILL, MAX_FONT), (Color::rgb(255, 202, 202), Color::rgb(204, 51, 51)));
        assert_eq!((MIN_FILL, MIN_FONT), (Color::rgb(188, 214, 248), Color::rgb(17, 74, 151)));
        assert!(matches!(Rule::min_max(), Rule::MinMax { dir: Direction::RowWise, .. }));
    }

    #[test]
    fn row_wise_marks_each_rows_extremes() {
        let s = sheet(&[&[1.0, 9.0], &[5.0, 2.0], &[3.0, 4.0]]);
        let st = evaluate(&s, &[rule(&s, Rule::min_max())]);
        // Row 0: 1, 5, 3 -> min in A, max in B. Row 1: 9, 2, 4 -> max in A, min in B.
        assert_eq!(fill_of(&st, 1, 0), Some(MIN_FILL));
        assert_eq!(fill_of(&st, 2, 0), Some(MAX_FILL));
        assert_eq!(fill_of(&st, 3, 0), None);
        assert_eq!(fill_of(&st, 1, 1), Some(MAX_FILL));
        assert_eq!(fill_of(&st, 2, 1), Some(MIN_FILL));
        assert_eq!(st.get(ColumnId(1), 1).unwrap().font, Some(MAX_FONT));
    }

    #[test]
    fn column_wise_marks_each_columns_extremes() {
        let s = sheet(&[&[3.0, 1.0, 2.0], &[10.0, 30.0, 20.0]]);
        let st = evaluate(&s, &[rule(&s, column_wise())]);
        assert_eq!(fill_of(&st, 1, 0), Some(MAX_FILL));
        assert_eq!(fill_of(&st, 1, 1), Some(MIN_FILL));
        assert_eq!(fill_of(&st, 1, 2), None);
        assert_eq!(fill_of(&st, 2, 0), Some(MIN_FILL));
        assert_eq!(fill_of(&st, 2, 1), Some(MAX_FILL));
    }

    #[test]
    fn ties_highlight_every_equal_cell() {
        let s = sheet(&[&[5.0, 1.0, 5.0, 1.0, 3.0]]);
        let st = evaluate(&s, &[rule(&s, column_wise())]);
        assert_eq!(fill_of(&st, 1, 0), Some(MAX_FILL));
        assert_eq!(fill_of(&st, 1, 2), Some(MAX_FILL));
        assert_eq!(fill_of(&st, 1, 1), Some(MIN_FILL));
        assert_eq!(fill_of(&st, 1, 3), Some(MIN_FILL));
        assert_eq!(fill_of(&st, 1, 4), None);
        // One number is both extremes and counts as the maximum.
        let one = sheet(&[&[7.0]]);
        let st = evaluate(&one, &[rule(&one, column_wise())]);
        assert_eq!(fill_of(&st, 1, 0), Some(MAX_FILL));
    }

    #[test]
    fn text_nan_and_masked_cells_are_ignored() {
        let mut s = sheet(&[&[f64::NAN, 4.0, 100.0, 2.0, 50.0]]);
        s.columns[0].data =
            ColumnData::mixed(vec![f64::NAN, 4.0, 100.0, 2.0, 50.0], [(0, "big".to_string())].into_iter().collect());
        s.columns[0].mask.insert(2);
        let st = evaluate(&s, &[rule(&s, column_wise())]);
        // The masked 100 takes no part: 50 is the maximum, 2 the minimum; text and masked get nothing.
        assert_eq!(fill_of(&st, 1, 4), Some(MAX_FILL));
        assert_eq!(fill_of(&st, 1, 3), Some(MIN_FILL));
        assert_eq!(fill_of(&st, 1, 0), None);
        assert_eq!(fill_of(&st, 1, 2), None);
        // A text column is skipped altogether.
        let mut t = sheet(&[&[1.0, 2.0]]);
        t.columns[0].data = ColumnData::text(vec!["a".into(), "b".into()]);
        assert!(evaluate(&t, &[rule(&t, column_wise())]).is_empty());
    }

    #[test]
    fn top_and_bottom_items() {
        let s = sheet(&[&[5.0, 9.0, 1.0, 7.0, 3.0]]);
        let st = evaluate(&s, &[rule(&s, top(true, 2.0, false))]);
        let marked: Vec<usize> = (0..5).filter(|r| fill_of(&st, 1, *r).is_some()).collect();
        assert_eq!(marked, vec![1, 3]);
        let st = evaluate(&s, &[rule(&s, top(false, 2.0, false))]);
        let marked: Vec<usize> = (0..5).filter(|r| fill_of(&st, 1, *r).is_some()).collect();
        assert_eq!(marked, vec![2, 4]);
        // N larger than the column marks everything; N of 0 or less marks nothing.
        let st = evaluate(&s, &[rule(&s, top(true, 99.0, false))]);
        assert!((0..5).all(|r| fill_of(&st, 1, r).is_some()));
        assert!(evaluate(&s, &[rule(&s, top(true, 0.0, false))]).is_empty());
        // A tie at the threshold marks every tied cell.
        let t = sheet(&[&[5.0, 9.0, 5.0, 1.0]]);
        let st = evaluate(&t, &[rule(&t, top(true, 2.0, false))]);
        let marked: Vec<usize> = (0..4).filter(|r| fill_of(&st, 1, *r).is_some()).collect();
        assert_eq!(marked, vec![0, 1, 2]);
    }

    #[test]
    fn top_and_bottom_percent_round_up() {
        let s = sheet(&[&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0]]);
        let st = evaluate(&s, &[rule(&s, top(true, 20.0, true))]);
        let marked: Vec<usize> = (0..10).filter(|r| fill_of(&st, 1, *r).is_some()).collect();
        assert_eq!(marked, vec![8, 9]);
        // 11 % of 10 numbers is 1.1 items: two.
        assert_eq!(item_count(11.0, true, 10), 2);
        // A small percent still marks one.
        assert_eq!(item_count(0.5, true, 10), 1);
        let st = evaluate(&s, &[rule(&s, top(false, 30.0, true))]);
        let marked: Vec<usize> = (0..10).filter(|r| fill_of(&st, 1, *r).is_some()).collect();
        assert_eq!(marked, vec![0, 1, 2]);
    }

    #[test]
    fn missing_values_do_not_count_toward_percent() {
        let s = sheet(&[&[1.0, f64::NAN, 2.0, f64::NAN, 3.0, 4.0]]);
        // 4 numbers, 50 % = 2 items.
        let st = evaluate(&s, &[rule(&s, top(true, 50.0, true))]);
        let marked: Vec<usize> = (0..6).filter(|r| fill_of(&st, 1, *r).is_some()).collect();
        assert_eq!(marked, vec![4, 5]);
    }

    #[test]
    fn the_earlier_rule_wins_where_rules_overlap() {
        let s = sheet(&[&[1.0, 2.0, 3.0]]);
        let first = CondFormat::new(
            &s,
            all(&s),
            Rule::TopBottom {
                top: true,
                n: 1.0,
                percent: false,
                colours: Colours { fill: Some(Color::rgb(1, 2, 3)), font: None },
            },
        );
        let second = rule(&s, column_wise());
        let st = evaluate(&s, &[first.clone(), second.clone()]);
        assert_eq!(fill_of(&st, 1, 2), Some(Color::rgb(1, 2, 3)));
        assert_eq!(fill_of(&st, 1, 0), Some(MIN_FILL));
        let st = evaluate(&s, &[second, first]);
        assert_eq!(fill_of(&st, 1, 2), Some(MAX_FILL));
    }

    #[test]
    fn a_row_range_limits_the_rule_and_none_runs_to_the_end() {
        let s = sheet(&[&[1.0, 50.0, 3.0, 2.0, 99.0]]);
        let mut range = all(&s);
        range.first_row = 2;
        range.last_row = Some(3);
        let st = evaluate(&s, &[CondFormat::new(&s, range.clone(), column_wise())]);
        assert_eq!(fill_of(&st, 1, 2), Some(MAX_FILL));
        assert_eq!(fill_of(&st, 1, 3), Some(MIN_FILL));
        assert_eq!(fill_of(&st, 1, 1), None);
        assert_eq!(fill_of(&st, 1, 4), None);
        // R0: from row 3 (index 2) to the end.
        range.last_row = None;
        let st = evaluate(&s, &[CondFormat::new(&s, range, column_wise())]);
        assert_eq!(fill_of(&st, 1, 4), Some(MAX_FILL));
        assert_eq!(fill_of(&st, 1, 3), Some(MIN_FILL));
        assert_eq!(fill_of(&st, 1, 0), None);
    }

    #[test]
    fn a_rule_without_colours_and_deleted_columns_do_nothing() {
        let s = sheet(&[&[1.0, 2.0]]);
        let none = Rule::MinMax { dir: Direction::ColumnWise, max: Colours::default(), min: Colours::default() };
        assert!(evaluate(&s, &[rule(&s, none)]).is_empty());
        let gone = CondFormat::new(&s, CondRange::whole_columns(vec![ColumnId(99)]), column_wise());
        assert!(evaluate(&s, &[gone]).is_empty());
    }

    #[test]
    fn default_names_follow_origins_form() {
        let s = sheet(&[&[1.0], &[2.0], &[3.0], &[4.0]]);
        assert_eq!(default_name(&s, &all(&s)), "C1R1-C4R0");
        let range = CondRange { columns: vec![ColumnId(2), ColumnId(3)], first_row: 4, last_row: Some(9) };
        assert_eq!(default_name(&s, &range), "C2R5-C3R10");
    }

    #[test]
    fn ranges_intersect_by_column_and_row() {
        let a = CondRange { columns: vec![ColumnId(1), ColumnId(2)], first_row: 0, last_row: Some(4) };
        let b = CondRange { columns: vec![ColumnId(2)], first_row: 4, last_row: None };
        let c = CondRange { columns: vec![ColumnId(2)], first_row: 5, last_row: None };
        let d = CondRange { columns: vec![ColumnId(3)], first_row: 0, last_row: None };
        assert!(a.intersects(&b));
        assert!(!a.intersects(&c));
        assert!(!a.intersects(&d));
    }

    #[test]
    fn a_sheet_saved_before_conditional_formatting_loads_without_rules() {
        let old = r#"{"name":"Sheet1","columns":[]}"#;
        let s: Worksheet = serde_json::from_str(old).unwrap();
        assert!(s.cond_formats.is_empty());
        // And an empty list is not written.
        assert!(!serde_json::to_string(&s).unwrap().contains("cond_formats"));
    }

    #[test]
    fn rules_round_trip_through_json() {
        let s = sheet(&[&[1.0]]);
        let rules = vec![rule(&s, Rule::min_max()), rule(&s, top(false, 5.0, true))];
        let text = serde_json::to_string(&rules).unwrap();
        let back: Vec<CondFormat> = serde_json::from_str(&text).unwrap();
        assert_eq!(back, rules);
    }

    #[test]
    #[ignore = "timing: run with --ignored --nocapture"]
    fn a_million_row_column_evaluates_quickly() {
        let v: Vec<f64> = (0..1_000_000).map(|i| ((i * 7919) % 1_000_003) as f64).collect();
        let s = sheet(&[&v]);
        for r in [column_wise(), top(true, 10.0, true)] {
            let t = std::time::Instant::now();
            let st = evaluate(&s, &[rule(&s, r)]);
            eprintln!("evaluated in {:?} (empty: {})", t.elapsed(), st.is_empty());
        }
    }
}
