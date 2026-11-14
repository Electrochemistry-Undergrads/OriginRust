//! Discrete frequency (Origin's `discfreqs`, Statistics > Descriptive Statistics > Discrete Frequency): how
//! often each distinct value of a column occurs, with relative and cumulative frequencies.
//!
//! What was measured on Origin (statistics.md 14): every cell from the first row to the last filled one counts,
//! an empty text and a missing number included (they are categories of their own), so the relative frequencies
//! are of all cells. Texts are compared without regard to case unless asked, and a category is shown with the
//! spelling of its first cell. Origin's order of categories with the same count follows no rule we found; here
//! they are in the order of their values.

use std::collections::HashMap;

/// One cell's value.
#[derive(Debug, Clone, PartialEq)]
pub enum Category {
    /// A missing number.
    Missing,
    Number(f64),
    Text(String),
}

impl Category {
    /// Missing first, then numbers, then texts.
    fn rank(&self) -> u8 {
        match self {
            Category::Missing => 0,
            Category::Number(_) => 1,
            Category::Text(_) => 2,
        }
    }

    /// Ascending value order: numbers by value, texts without regard to case (then by their bytes).
    pub fn cmp_value(&self, other: &Category) -> std::cmp::Ordering {
        match (self, other) {
            (Category::Number(a), Category::Number(b)) => a.total_cmp(b),
            (Category::Text(a), Category::Text(b)) => a.to_lowercase().cmp(&b.to_lowercase()).then_with(|| a.cmp(b)),
            _ => self.rank().cmp(&other.rank()),
        }
    }
}

/// Origin's Sort Data by (Categorical Order is not offered: on columns that are not categorical Origin gives the
/// order of Descending Counts).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiscSort {
    CountAsc,
    #[default]
    CountDesc,
    ValueAsc,
    ValueDesc,
}

impl DiscSort {
    pub const ALL: [DiscSort; 4] = [DiscSort::CountAsc, DiscSort::CountDesc, DiscSort::ValueAsc, DiscSort::ValueDesc];

    /// The text of Origin's list.
    pub fn label(self) -> &'static str {
        match self {
            DiscSort::CountAsc => "Ascending Counts",
            DiscSort::CountDesc => "Descending Counts",
            DiscSort::ValueAsc => "Ascending Values",
            DiscSort::ValueDesc => "Descending Values",
        }
    }

    /// The name of Origin's `sort` option.
    pub fn code(self) -> &'static str {
        match self {
            DiscSort::CountAsc => "casc",
            DiscSort::CountDesc => "cdesc",
            DiscSort::ValueAsc => "vasc",
            DiscSort::ValueDesc => "vdesc",
        }
    }

    pub fn from_code(s: &str) -> Option<Self> {
        DiscSort::ALL.into_iter().find(|k| k.code().eq_ignore_ascii_case(s))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiscreteRow {
    pub value: Category,
    pub count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiscreteFreq {
    pub rows: Vec<DiscreteRow>,
    /// Number of cells counted (the base of the relative frequencies).
    pub total: usize,
}

impl DiscreteFreq {
    /// Count divided by the number of cells.
    pub fn relative(&self) -> Vec<f64> {
        self.rows.iter().map(|r| r.count as f64 / self.total as f64).collect()
    }

    /// Running sum of the relative frequencies in the order of the rows.
    pub fn cumulative(&self) -> Vec<f64> {
        let mut sum = 0usize;
        self.rows
            .iter()
            .map(|r| {
                sum += r.count;
                sum as f64 / self.total as f64
            })
            .collect()
    }
}

#[derive(Hash, PartialEq, Eq)]
enum Key {
    Missing,
    Number(u64),
    Text(String),
}

/// Counts the distinct values of `values`.
pub fn discrete_frequency(values: &[Category], case_sensitive: bool, sort: DiscSort) -> DiscreteFreq {
    let mut index: HashMap<Key, usize> = HashMap::new();
    let mut rows: Vec<DiscreteRow> = Vec::new();
    for v in values {
        let key = match v {
            Category::Missing => Key::Missing,
            Category::Number(x) if x.is_nan() => Key::Missing,
            Category::Number(x) => Key::Number(if *x == 0.0 { 0f64.to_bits() } else { x.to_bits() }),
            Category::Text(s) if case_sensitive => Key::Text(s.clone()),
            Category::Text(s) => Key::Text(s.to_lowercase()),
        };
        let i = *index.entry(key).or_insert_with(|| {
            let value = match v {
                Category::Number(x) if x.is_nan() => Category::Missing,
                Category::Number(x) if *x == 0.0 => Category::Number(0.0),
                other => other.clone(),
            };
            rows.push(DiscreteRow { value, count: 0 });
            rows.len() - 1
        });
        rows[i].count += 1;
    }
    rows.sort_by(|a, b| a.value.cmp_value(&b.value));
    match sort {
        DiscSort::ValueAsc => {}
        DiscSort::ValueDesc => rows.reverse(),
        DiscSort::CountAsc => rows.sort_by_key(|r| r.count),
        DiscSort::CountDesc => rows.sort_by_key(|r| std::cmp::Reverse(r.count)),
    }
    DiscreteFreq { rows, total: values.len() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(v: &[&str]) -> Vec<Category> {
        v.iter().map(|s| Category::Text(s.to_string())).collect()
    }

    #[test]
    fn texts_ignore_case_and_keep_the_first_spelling() {
        let f = discrete_frequency(&texts(&["b", "a", "B", "A", "a", ""]), false, DiscSort::ValueAsc);
        let got: Vec<(Category, usize)> = f.rows.iter().map(|r| (r.value.clone(), r.count)).collect();
        assert_eq!(
            got,
            vec![(Category::Text(String::new()), 1), (Category::Text("a".into()), 3), (Category::Text("b".into()), 2)]
        );
        assert_eq!(f.total, 6);
        let cs = discrete_frequency(&texts(&["b", "a", "B", "A", "a", ""]), true, DiscSort::ValueAsc);
        assert_eq!(cs.rows.len(), 5);
        assert_eq!(*cs.cumulative().last().unwrap(), 1.0);
    }

    #[test]
    fn missing_numbers_are_a_category_and_sort_first() {
        let v: Vec<Category> = [3.0, f64::NAN, 1.0, 3.0, -0.0, 0.0].iter().map(|x| Category::Number(*x)).collect();
        let f = discrete_frequency(&v, false, DiscSort::ValueAsc);
        assert_eq!(f.rows[0].value, Category::Missing);
        assert_eq!(f.rows[1], DiscreteRow { value: Category::Number(-0.0), count: 2 });
        let d = discrete_frequency(&v, false, DiscSort::CountDesc);
        assert_eq!(d.rows[0].count, 2);
        assert_eq!(d.rows.last().unwrap().count, 1);
        assert!((d.relative()[0] - 2.0 / 6.0).abs() < 1e-15);
    }
}
