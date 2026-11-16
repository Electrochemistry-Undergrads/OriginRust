//! Indexed data (T76): one data column whose rows are split by the values (levels) of one or two group columns.
//! The grouped column and grouped box plots read their levels from here.

use crate::column::{Column, ColumnData};

/// A group cell as text: the cell itself, trimmed, or a number written without a trailing ".0". `None` when empty
/// (the row is left out).
pub fn level_text(col: &Column, row: usize) -> Option<String> {
    match &col.data {
        ColumnData::Text(v) => v.get(row).map(|s| s.trim()).filter(|s| !s.is_empty()).map(str::to_string),
        ColumnData::Mixed { .. } if col.data.text_cell(row).is_some() => {
            col.data.text_cell(row).map(|s| s.trim().to_string())
        }
        ColumnData::Numeric(v) | ColumnData::Mixed { numbers: v, .. } => v
            .get(row)
            .copied()
            .filter(|x| !x.is_nan())
            .map(|x| if x.fract() == 0.0 && x.abs() < 1e15 { format!("{}", x as i64) } else { format!("{x}") }),
    }
}

/// Orders two level names: numerically when `numeric` (every level is a number), else by plain string order
/// (case-sensitive; Sheaf's choice, Origin sorts alphabetically).
fn compare(numeric: bool, a: &str, b: &str) -> std::cmp::Ordering {
    if numeric && let (Ok(x), Ok(y)) = (a.parse::<f64>(), b.parse::<f64>()) {
        return x.total_cmp(&y);
    }
    a.cmp(b)
}

/// The distinct levels of `col`, sorted: numerically when every level parses as a number, else in string order.
/// Empty cells are no level.
pub fn levels(col: &Column) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for row in 0..col.data.len() {
        if let Some(t) = level_text(col, row)
            && !out.contains(&t)
        {
            out.push(t);
        }
    }
    sort_levels(&mut out);
    out
}

/// Sorts level names the way [`levels`] does: numerically when every one parses as a number, else in string order.
pub fn sort_levels(levels: &mut [String]) {
    let numeric = levels.iter().all(|l| l.parse::<f64>().is_ok_and(f64::is_finite));
    levels.sort_by(|a, b| compare(numeric, a, b));
}

/// The position of `level` in `levels`.
pub fn level_position(levels: &[String], level: &str) -> Option<usize> {
    levels.iter().position(|l| l == level)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::Designation;
    use crate::ids::ColumnId;

    fn text(v: &[&str]) -> Column {
        Column::new(ColumnId(1), "G", Designation::Y, ColumnData::text(v.iter().map(|s| s.to_string()).collect()))
    }

    #[test]
    fn non_finite_words_do_not_make_levels_numeric() {
        let mut l = vec!["NaN".to_string(), "5".to_string()];
        sort_levels(&mut l);
        assert_eq!(l, ["5", "NaN"].map(String::from));
        let mut l = vec!["Inf".to_string(), "10".to_string()];
        sort_levels(&mut l);
        assert_eq!(l, ["10", "Inf"].map(String::from));
    }

    #[test]
    fn text_levels_sort_in_string_order_and_skip_empty_cells() {
        let c = text(&["b", "a", "", "b", " c ", "B"]);
        assert_eq!(levels(&c), ["B", "a", "b", "c"]);
        assert_eq!(level_text(&c, 2), None);
        assert_eq!(level_text(&c, 4).as_deref(), Some("c"));
    }

    #[test]
    fn numeric_levels_sort_numerically() {
        let c = Column::new(ColumnId(1), "G", Designation::Y, ColumnData::numeric(vec![10.0, 2.0, f64::NAN, 2.0, 1.5]));
        assert_eq!(levels(&c), ["1.5", "2", "10"]);
        // Text that is all numbers sorts numerically too.
        assert_eq!(levels(&text(&["10", "9", "100"])), ["9", "10", "100"]);
    }

    #[test]
    fn mixed_levels_use_string_order() {
        assert_eq!(levels(&text(&["10", "9", "a"])), ["10", "9", "a"]);
        assert_eq!(level_position(&levels(&text(&["x", "y"])), "y"), Some(1));
    }
}
