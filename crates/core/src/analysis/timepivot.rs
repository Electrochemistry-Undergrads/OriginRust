//! Worksheet > Time Series Pivot as a tool of `Project::analyze` (T358; Origin 2025's Time Series Pivot, not in
//! Origin 2016, so the rules and layout are Sheaf's own; `docs/spec/statistics.md` section 32).
//!
//! The X column holds date-time numbers (Origin's day numbers, [`crate::datetime`]); every chosen Y column is
//! summarized per calendar period by one statistic. Periods without a value are left out.

use super::*;
use crate::datetime;

/// The calendar period rows are grouped by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PivotPeriod {
    Minute,
    Hour,
    #[default]
    Day,
    /// Weeks start on Monday.
    Week,
    Month,
    Quarter,
    Year,
}

impl PivotPeriod {
    pub const ALL: [PivotPeriod; 7] = [
        PivotPeriod::Minute,
        PivotPeriod::Hour,
        PivotPeriod::Day,
        PivotPeriod::Week,
        PivotPeriod::Month,
        PivotPeriod::Quarter,
        PivotPeriod::Year,
    ];

    pub fn name(self) -> &'static str {
        match self {
            PivotPeriod::Minute => "Minute",
            PivotPeriod::Hour => "Hour",
            PivotPeriod::Day => "Day",
            PivotPeriod::Week => "Week",
            PivotPeriod::Month => "Month",
            PivotPeriod::Quarter => "Quarter",
            PivotPeriod::Year => "Year",
        }
    }

    /// The date number where the period holding `v` starts.
    pub fn start(self, v: f64) -> f64 {
        let p = datetime::parts(v);
        let day = datetime::date_value(p.year, p.month, p.day).unwrap_or(v.floor());
        match self {
            PivotPeriod::Minute => day + (f64::from(p.hour) * 60.0 + f64::from(p.minute)) / 1440.0,
            PivotPeriod::Hour => day + f64::from(p.hour) / 24.0,
            PivotPeriod::Day => day,
            PivotPeriod::Week => day - f64::from(datetime::weekday(day)),
            PivotPeriod::Month => datetime::date_value(p.year, p.month, 1).unwrap_or(day),
            PivotPeriod::Quarter => datetime::date_value(p.year, 3 * ((p.month - 1) / 3) + 1, 1).unwrap_or(day),
            PivotPeriod::Year => datetime::date_value(p.year, 1, 1).unwrap_or(day),
        }
    }

    /// How a period starting at `start` is written: `2026-11-22 14:05`, `2026-11-22 14:00`, `2026-11-22`,
    /// `Week of 2026-11-16`, `2026-11`, `2026 Q4`, `2026`.
    pub fn label(self, start: f64) -> String {
        let p = datetime::parts(start);
        match self {
            PivotPeriod::Minute | PivotPeriod::Hour => {
                format!("{:04}-{:02}-{:02} {:02}:{:02}", p.year, p.month, p.day, p.hour, p.minute)
            }
            PivotPeriod::Day => format!("{:04}-{:02}-{:02}", p.year, p.month, p.day),
            PivotPeriod::Week => format!("Week of {:04}-{:02}-{:02}", p.year, p.month, p.day),
            PivotPeriod::Month => format!("{:04}-{:02}", p.year, p.month),
            PivotPeriod::Quarter => format!("{:04} Q{}", p.year, (p.month - 1) / 3 + 1),
            PivotPeriod::Year => format!("{:04}", p.year),
        }
    }
}

/// The statistic of each period.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PivotStat {
    Sum,
    #[default]
    Mean,
    Count,
    Min,
    Max,
    Median,
}

impl PivotStat {
    pub const ALL: [PivotStat; 6] =
        [PivotStat::Sum, PivotStat::Mean, PivotStat::Count, PivotStat::Min, PivotStat::Max, PivotStat::Median];

    pub fn name(self) -> &'static str {
        match self {
            PivotStat::Sum => "Sum",
            PivotStat::Mean => "Mean",
            PivotStat::Count => "Count",
            PivotStat::Min => "Minimum",
            PivotStat::Max => "Maximum",
            PivotStat::Median => "Median",
        }
    }

    fn of(self, v: &mut [f64]) -> f64 {
        let n = v.len() as f64;
        match self {
            PivotStat::Sum => v.iter().sum(),
            PivotStat::Mean => v.iter().sum::<f64>() / n,
            PivotStat::Count => n,
            PivotStat::Min => v.iter().copied().fold(f64::INFINITY, f64::min),
            PivotStat::Max => v.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            PivotStat::Median => {
                v.sort_by(f64::total_cmp);
                let m = v.len();
                if m % 2 == 1 { v[m / 2] } else { 0.5 * (v[m / 2 - 1] + v[m / 2]) }
            }
        }
    }
}

/// What [`Tool::TimePivot`] does.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TimePivotSpec {
    pub period: PivotPeriod,
    pub stat: PivotStat,
}

pub(super) struct PivotOut {
    pub table: ResultTable,
    pub on_sheet: Vec<SheetColumn>,
    pub notes: Vec<String>,
}

pub(super) fn run(spec: &TimePivotSpec, inputs: &[Input<'_>]) -> Result<PivotOut, AnalysisError> {
    // Period start -> one list of values per input.
    let mut groups: std::collections::BTreeMap<i64, (f64, Vec<Vec<f64>>)> = std::collections::BTreeMap::new();
    for (k, input) in inputs.iter().enumerate() {
        for (x, y) in input.xs().iter().zip(input.ys()) {
            if !(datetime::in_range(*x) && y.is_finite()) {
                continue;
            }
            let start = spec.period.start(*x);
            // Keys in whole milliseconds, so equal starts meet exactly.
            let key = (start * 86_400_000.0).round() as i64;
            let entry = groups.entry(key).or_insert_with(|| (start, vec![Vec::new(); inputs.len()]));
            entry.1[k].push(*y);
        }
    }
    if groups.is_empty() {
        return Err(AnalysisError::BadOption(
            "no row has a date-time X and a value: the X column must hold dates or date-times".into(),
        ));
    }
    let mut headers = vec!["Period start".to_string()];
    for input in inputs {
        headers.push(format!("{} of {}", spec.stat.name(), input.name));
        headers.push(format!("N of {}", input.name));
    }
    let header_refs: Vec<&str> = headers.iter().map(String::as_str).collect();
    let mut table = ResultTable::new(&header_refs);
    for (start, values) in groups.values_mut() {
        let mut row = vec![*start];
        for v in values.iter_mut() {
            row.push(if v.is_empty() { f64::NAN } else { spec.stat.of(v) });
            row.push(v.len() as f64);
        }
        table.push_row(spec.period.label(*start), row, vec![String::new(); headers.len()]);
    }
    let notes = vec![format!(
        "{} per {} of {} column(s): {} period(s), periods without a value left out.",
        spec.stat.name(),
        spec.period.name().to_lowercase(),
        inputs.len(),
        table.rows.len()
    )];
    let mut on_sheet = vec![SheetColumn {
        designation: Designation::Label,
        long_name: "Period".into(),
        units: String::new(),
        comments: spec.period.name().into(),
        data: ColumnData::text(table.rows.iter().map(|(l, _)| l.clone()).collect()),
    }];
    for (i, h) in table.headers.iter().enumerate() {
        on_sheet.push(SheetColumn {
            designation: if i == 0 { Designation::X } else { Designation::Y },
            long_name: h.clone(),
            units: if i == 0 { String::new() } else { inputs[(i - 1) / 2].units.clone() },
            comments: String::new(),
            data: ColumnData::numeric(table.rows.iter().map(|(_, v)| v[i]).collect()),
        });
    }
    Ok(PivotOut { table, on_sheet, notes })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(y: i64, m: u32, d: u32, h: u32, min: u32) -> f64 {
        datetime::date_value(y, m, d).unwrap() + datetime::time_value(h, min, 0.0).unwrap()
    }

    #[test]
    fn periods_start_where_the_calendar_says() {
        let v = at(2026, 11, 22, 14, 37); // a Sunday
        assert_eq!(PivotPeriod::Minute.start(v), at(2026, 11, 22, 14, 37));
        assert_eq!(PivotPeriod::Hour.start(v), at(2026, 11, 22, 14, 0));
        assert_eq!(PivotPeriod::Day.start(v), at(2026, 11, 22, 0, 0));
        assert_eq!(PivotPeriod::Week.start(v), at(2026, 11, 16, 0, 0), "the Monday before");
        assert_eq!(PivotPeriod::Month.start(v), at(2026, 11, 1, 0, 0));
        assert_eq!(PivotPeriod::Quarter.start(v), at(2026, 10, 1, 0, 0));
        assert_eq!(PivotPeriod::Year.start(v), at(2026, 1, 1, 0, 0));
        assert_eq!(PivotPeriod::Quarter.label(at(2026, 10, 1, 0, 0)), "2026 Q4");
        assert_eq!(PivotPeriod::Week.label(at(2026, 11, 16, 0, 0)), "Week of 2026-11-16");
        assert_eq!(PivotPeriod::Hour.label(at(2026, 11, 22, 14, 0)), "2026-11-22 14:00");
    }

    #[test]
    fn values_are_summarized_per_period() {
        let mut p = Project::new();
        let book = p.add_workbook("tp");
        let xs =
            vec![at(2026, 1, 5, 9, 0), at(2026, 1, 5, 17, 0), at(2026, 1, 6, 8, 0), at(2026, 2, 1, 0, 0), f64::NAN];
        let ys = vec![1.0, 3.0, 10.0, 7.0, 99.0];
        p.add_column(book, 0, Designation::X, ColumnData::numeric(xs)).unwrap();
        let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(ys)).unwrap();
        let request = |spec| Request { columns: vec![y], rows: RowRange::All, tool: Tool::TimePivot(spec) };
        let out = p.analyze(&request(TimePivotSpec { period: PivotPeriod::Day, stat: PivotStat::Mean })).unwrap();
        let t = out.table.as_ref().unwrap();
        let labels: Vec<&str> = t.rows.iter().map(|(l, _)| l.as_str()).collect();
        assert_eq!(labels, ["2026-01-05", "2026-01-06", "2026-02-01"]);
        assert_eq!((t.rows[0].1[1], t.rows[0].1[2]), (2.0, 2.0));
        let out = p.analyze(&request(TimePivotSpec { period: PivotPeriod::Month, stat: PivotStat::Sum })).unwrap();
        let t = out.table.as_ref().unwrap();
        assert_eq!(t.rows.iter().map(|(_, v)| v[1]).collect::<Vec<_>>(), [14.0, 7.0]);
        assert_eq!(t.headers, ["Period start", "Sum of B", "N of B"]);
    }
}
