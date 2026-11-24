//! Worksheet > Time Series Pivot as a tool of `Project::analyze` (T358; Origin 2025's Time Series Pivot, not in
//! Origin 2016, so the rules and layout are Sheaf's own; `docs/spec/statistics.md` section 32).
//!
//! The X column holds date-time numbers (Origin's day numbers, [`crate::datetime`]); every chosen Y column is
//! summarized per calendar period by one statistic. Periods without a value are left out.
//!
//! Custom intervals and fiscal years (T404): a period can be several units long ([`TimePivotSpec::every`]: 15
//! minutes, 6 hours, 2 weeks, ...) and years and quarters can follow a fiscal year starting in another month
//! ([`TimePivotSpec::fiscal_start`]). Minutes and hours count from midnight, so the periods start again each day;
//! days and weeks count from the first day (Monday) of the data; months and quarters start again with each
//! (fiscal) year, its last group shorter when the count does not divide the year; years count from year 0, so
//! groups of 10 are decades. A fiscal year is named by the calendar year it ends in (FY2027 from October 2026).

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
    /// How many periods make one group (T404); 0 and 1 both mean one.
    pub every: u32,
    /// The month (1 to 12) a fiscal year starts in, for months, quarters and years (T404); 0 and 1 both mean
    /// January, the calendar year.
    pub fiscal_start: u32,
}

impl TimePivotSpec {
    /// The number of periods in a group.
    pub fn n(&self) -> u32 {
        self.every.max(1)
    }

    /// The month the (fiscal) year starts in, 1 to 12.
    pub fn first_month(&self) -> u32 {
        self.fiscal_start.clamp(1, 12)
    }

    fn fiscal(&self) -> bool {
        self.first_month() != 1 && matches!(self.period, PivotPeriod::Month | PivotPeriod::Quarter | PivotPeriod::Year)
    }

    /// The date number where the group holding `v` starts; `first` is the earliest date-time of the data (days
    /// and weeks are counted from it).
    pub fn start(&self, v: f64, first: f64) -> f64 {
        let n = i64::from(self.n());
        if n == 1 && !self.fiscal() {
            return self.period.start(v);
        }
        let p = datetime::parts(v);
        let day = PivotPeriod::Day.start(v);
        // Months since the start of a fiscal year in year 0.
        let fs = i64::from(self.first_month()) - 1;
        let m = p.year * 12 + i64::from(p.month) - 1 - fs;
        let month_start = |k: i64| {
            let k = k + fs;
            datetime::date_value(k.div_euclid(12), k.rem_euclid(12) as u32 + 1, 1).unwrap_or(day)
        };
        // Groups of `len` months start again with each (fiscal) year; the last of a year can be shorter.
        let months = |len: i64| {
            let year_start = m.div_euclid(12) * 12;
            month_start(year_start + (m - year_start).div_euclid(len) * len)
        };
        match self.period {
            PivotPeriod::Minute => {
                let minute = i64::from(p.hour) * 60 + i64::from(p.minute);
                day + (minute.div_euclid(n) * n) as f64 / 1440.0
            }
            PivotPeriod::Hour => day + (i64::from(p.hour).div_euclid(n) * n) as f64 / 24.0,
            PivotPeriod::Day => {
                let d0 = PivotPeriod::Day.start(first);
                d0 + (((day - d0).round() as i64).div_euclid(n) * n) as f64
            }
            PivotPeriod::Week => {
                let w0 = PivotPeriod::Week.start(first);
                let weeks = ((PivotPeriod::Week.start(v) - w0) / 7.0).round() as i64;
                w0 + 7.0 * (weeks.div_euclid(n) * n) as f64
            }
            PivotPeriod::Month => months(n),
            PivotPeriod::Quarter => months(3 * n),
            // Years count from year 0: groups of 10 are decades.
            PivotPeriod::Year => month_start(m.div_euclid(12 * n) * 12 * n),
        }
    }

    /// How a group starting at `start` is written: as [`PivotPeriod::label`], with fiscal quarters and years as
    /// `FY2027 Q1` and `FY2027`.
    pub fn label(&self, start: f64) -> String {
        if !self.fiscal() || self.period == PivotPeriod::Month {
            return self.period.label(start);
        }
        let p = datetime::parts(start);
        let fs = self.first_month();
        let fy = if p.month >= fs { p.year + 1 } else { p.year };
        match self.period {
            PivotPeriod::Quarter => format!("FY{fy:04} Q{}", (p.month + 12 - fs) % 12 / 3 + 1),
            _ => format!("FY{fy:04}"),
        }
    }

    /// The group in words: `day`, `15 minutes`, `fiscal quarter (from October)`.
    pub fn describe(&self) -> String {
        let unit = self.period.name().to_lowercase();
        let unit = if self.n() == 1 { unit } else { format!("{} {unit}s", self.n()) };
        if self.fiscal() {
            const MONTHS: [&str; 12] = [
                "January",
                "February",
                "March",
                "April",
                "May",
                "June",
                "July",
                "August",
                "September",
                "October",
                "November",
                "December",
            ];
            let name = MONTHS[self.first_month() as usize - 1];
            if self.period == PivotPeriod::Month {
                format!("{unit} (fiscal years from {name})")
            } else {
                format!("fiscal {unit} (from {name})")
            }
        } else {
            unit
        }
    }
}

pub(super) struct PivotOut {
    pub table: ResultTable,
    pub on_sheet: Vec<SheetColumn>,
    pub notes: Vec<String>,
}

pub(super) fn run(spec: &TimePivotSpec, inputs: &[Input<'_>]) -> Result<PivotOut, AnalysisError> {
    // Period start -> one list of values per input.
    let mut groups: std::collections::BTreeMap<i64, (f64, Vec<Vec<f64>>)> = std::collections::BTreeMap::new();
    // Days and weeks of several units count from the earliest date-time with a value.
    let first = inputs
        .iter()
        .flat_map(|input| input.xs().iter().zip(input.ys()))
        .filter(|(x, y)| datetime::in_range(**x) && y.is_finite())
        .map(|(x, _)| *x)
        .fold(f64::INFINITY, f64::min);
    for (k, input) in inputs.iter().enumerate() {
        for (x, y) in input.xs().iter().zip(input.ys()) {
            if !(datetime::in_range(*x) && y.is_finite()) {
                continue;
            }
            let start = spec.start(*x, first);
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
        table.push_row(spec.label(*start), row, vec![String::new(); headers.len()]);
    }
    let notes = vec![format!(
        "{} per {} of {} column(s): {} period(s), periods without a value left out.",
        spec.stat.name(),
        spec.describe(),
        inputs.len(),
        table.rows.len()
    )];
    let mut on_sheet = vec![SheetColumn {
        designation: Designation::Label,
        long_name: "Period".into(),
        units: String::new(),
        comments: spec.describe(),
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
        let out = p
            .analyze(&request(TimePivotSpec { period: PivotPeriod::Day, stat: PivotStat::Mean, ..Default::default() }))
            .unwrap();
        let t = out.table.as_ref().unwrap();
        let labels: Vec<&str> = t.rows.iter().map(|(l, _)| l.as_str()).collect();
        assert_eq!(labels, ["2026-01-05", "2026-01-06", "2026-02-01"]);
        assert_eq!((t.rows[0].1[1], t.rows[0].1[2]), (2.0, 2.0));
        let out = p
            .analyze(&request(TimePivotSpec { period: PivotPeriod::Month, stat: PivotStat::Sum, ..Default::default() }))
            .unwrap();
        let t = out.table.as_ref().unwrap();
        assert_eq!(t.rows.iter().map(|(_, v)| v[1]).collect::<Vec<_>>(), [14.0, 7.0]);
        assert_eq!(t.headers, ["Period start", "Sum of B", "N of B"]);
    }

    #[test]
    fn custom_intervals_count_from_midnight_the_first_day_or_the_year() {
        // Five months: January, June and November of each year.
        let spec = |period, every| TimePivotSpec { period, every, ..Default::default() };
        let v = at(2026, 11, 22, 14, 37);
        let first = at(2026, 11, 3, 9, 0);
        assert_eq!(spec(PivotPeriod::Minute, 15).start(v, first), at(2026, 11, 22, 14, 30));
        assert_eq!(spec(PivotPeriod::Hour, 6).start(v, first), at(2026, 11, 22, 12, 0));
        // Days from the first day of the data: Nov 3, 10, 17, 24 ... for 7 days.
        assert_eq!(spec(PivotPeriod::Day, 7).start(v, first), at(2026, 11, 17, 0, 0));
        // Weeks from the first Monday (Nov 2): Nov 2, 16, 30 ... for 2 weeks.
        assert_eq!(spec(PivotPeriod::Week, 2).start(v, first), at(2026, 11, 16, 0, 0));
        assert_eq!(spec(PivotPeriod::Month, 2).start(v, first), at(2026, 11, 1, 0, 0));
        assert_eq!(spec(PivotPeriod::Month, 5).start(at(2026, 4, 9, 0, 0), first), at(2026, 1, 1, 0, 0));
        assert_eq!(spec(PivotPeriod::Month, 5).start(v, first), at(2026, 11, 1, 0, 0));
        assert_eq!(spec(PivotPeriod::Month, 5).start(at(2027, 1, 9, 0, 0), first), at(2027, 1, 1, 0, 0));
        assert_eq!(spec(PivotPeriod::Quarter, 2).start(v, first), at(2026, 7, 1, 0, 0), "half years");
        assert_eq!(spec(PivotPeriod::Year, 10).start(v, first), at(2020, 1, 1, 0, 0));
        // 0 and 1 are one period.
        assert_eq!(spec(PivotPeriod::Hour, 0).start(v, first), PivotPeriod::Hour.start(v));
        assert_eq!(spec(PivotPeriod::Minute, 15).describe(), "15 minutes");
    }

    #[test]
    fn fiscal_years_start_in_their_month_and_are_named_by_their_end() {
        let fy = |period| TimePivotSpec { period, fiscal_start: 10, ..Default::default() };
        let v = at(2026, 11, 22, 14, 37);
        let first = v;
        assert_eq!(fy(PivotPeriod::Year).start(v, first), at(2026, 10, 1, 0, 0));
        assert_eq!(fy(PivotPeriod::Year).label(at(2026, 10, 1, 0, 0)), "FY2027");
        assert_eq!(fy(PivotPeriod::Year).start(at(2026, 9, 30, 0, 0), first), at(2025, 10, 1, 0, 0));
        assert_eq!(fy(PivotPeriod::Year).label(at(2025, 10, 1, 0, 0)), "FY2026");
        assert_eq!(fy(PivotPeriod::Quarter).start(v, first), at(2026, 10, 1, 0, 0));
        assert_eq!(fy(PivotPeriod::Quarter).label(at(2026, 10, 1, 0, 0)), "FY2027 Q1");
        assert_eq!(fy(PivotPeriod::Quarter).start(at(2027, 2, 3, 0, 0), first), at(2027, 1, 1, 0, 0));
        assert_eq!(fy(PivotPeriod::Quarter).label(at(2027, 7, 1, 0, 0)), "FY2027 Q4");
        assert_eq!(fy(PivotPeriod::Month).label(at(2026, 11, 1, 0, 0)), "2026-11");
        assert_eq!(fy(PivotPeriod::Day).describe(), "day", "days have no fiscal year");
        assert_eq!(fy(PivotPeriod::Quarter).describe(), "fiscal quarter (from October)");
        // Through the tool: two fiscal years summed.
        let mut p = Project::new();
        let book = p.add_workbook("fy");
        let xs = vec![at(2026, 9, 30, 12, 0), at(2026, 10, 1, 0, 0), at(2027, 3, 1, 0, 0)];
        p.add_column(book, 0, Designation::X, ColumnData::numeric(xs)).unwrap();
        let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0, 4.0])).unwrap();
        let spec = TimePivotSpec { stat: PivotStat::Sum, ..fy(PivotPeriod::Year) };
        let request = Request { columns: vec![y], rows: RowRange::All, tool: Tool::TimePivot(spec) };
        let out = p.analyze(&request).unwrap();
        let t = out.table.as_ref().unwrap();
        let rows: Vec<(&str, f64)> = t.rows.iter().map(|(l, v)| (l.as_str(), v[1])).collect();
        assert_eq!(rows, [("FY2026", 1.0), ("FY2027", 6.0)]);
    }
}
