//! Statistics > Stationarity Tests as a tool of `Project::analyze` (T372; Origin 2024b's Time Series Analysis app,
//! not in Origin 2016, so the layout is Sheaf's own). The numerics are in `sheaf_calc::stationarity`;
//! `docs/spec/statistics.md` section 41 says what is computed and written.
//!
//! Every chosen column is a series of its own, in row order; it must be complete in the chosen rows.

use super::dist::Output;
use super::statistics::num;
use super::*;
pub use calc::stationarity::{AdfTerms, KpssNull};
use calc::stationarity::{adf, kpss};

/// What [`Tool::Stationarity`] does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StationaritySpec {
    pub adf_terms: AdfTerms,
    /// Lagged differences of the ADF regression; `None` chooses them by AIC.
    pub adf_lags: Option<usize>,
    pub kpss_null: KpssNull,
    /// Autocovariances in the KPSS long-run variance; `None` takes `ceil(12 (n / 100)^(1/4))`.
    pub kpss_lags: Option<usize>,
    /// With `kpss_lags` `None`: the lags of Hobijn et al. (T497, statsmodels' `nlags="auto"`) instead.
    pub kpss_auto: bool,
}

fn column_of(name: &str, designation: Designation, values: Vec<f64>) -> SheetColumn {
    SheetColumn {
        designation,
        long_name: name.to_string(),
        units: String::new(),
        comments: String::new(),
        data: ColumnData::numeric(values),
    }
}

pub(super) fn run(spec: &StationaritySpec, inputs: &[Input<'_>]) -> Result<Output, AnalysisError> {
    let mut table =
        ResultTable::new(&["Statistic", "Lags", "Points", "Critical 1%", "Critical 5%", "Critical 10%", "Prob"]);
    let mut notes = Vec::new();
    let terms = match spec.adf_terms {
        AdfTerms::Constant => "constant",
        AdfTerms::Trend => "constant and trend",
        AdfTerms::None => "no constant",
    };
    let null = match spec.kpss_null {
        KpssNull::Level => "level",
        KpssNull::Trend => "trend",
    };
    for input in inputs {
        let bad = |e: calc::stationarity::StationarityError| AnalysisError::BadOption(format!("{}: {e}", input.label));
        let a = adf(input.ys(), spec.adf_terms, spec.adf_lags).map_err(bad)?;
        let kpss_lags = match (spec.kpss_lags, spec.kpss_auto) {
            (None, true) => Some(calc::stationarity::kpss_auto_lags(input.ys(), spec.kpss_null).map_err(bad)?),
            (lags, _) => lags,
        };
        let k = kpss(input.ys(), spec.kpss_null, kpss_lags).map_err(bad)?;
        let blank = vec![String::new(); 7];
        table.push_row(
            format!("{}, ADF ({terms})", input.label),
            vec![a.statistic, a.lags as f64, a.nobs as f64, a.critical[0], a.critical[1], a.critical[2], a.p],
            blank.clone(),
        );
        table.push_row(
            format!("{}, KPSS ({null})", input.label),
            vec![k.statistic, k.lags as f64, input.ys().len() as f64, k.critical[3], k.critical[1], k.critical[0], k.p],
            blank,
        );
        let adf_says = if a.statistic < a.critical[1] {
            "rejects a unit root at 5% (stationary)"
        } else {
            "does not reject a unit root at 5%"
        };
        let p = if k.p >= 0.10 {
            "Prob >= 0.1".to_string()
        } else if k.p <= 0.01 {
            "Prob <= 0.01".to_string()
        } else {
            format!("Prob = {}", num(k.p))
        };
        let kpss_says = if k.p < 0.05 {
            format!("rejects {null} stationarity at 5%")
        } else {
            format!("does not reject {null} stationarity at 5%")
        };
        notes.push(format!(
            "{}: ADF = {} with {} lag(s) (Prob = {}) {adf_says}; KPSS = {} ({p}) {kpss_says}.",
            input.label,
            num(a.statistic),
            a.lags,
            num(a.p),
            num(k.statistic)
        ));
    }
    notes.push(
        "ADF critical values are MacKinnon's (2010) for the regression's points and its probability MacKinnon's (1994) approximation, as statsmodels' adfuller; the KPSS probability is interpolated in Kwiatkowski et al.'s table and held at 0.1 and 0.01.".into(),
    );
    let mut sheet = vec![SheetColumn {
        designation: Designation::Label,
        long_name: "Test".into(),
        units: String::new(),
        comments: String::new(),
        data: ColumnData::text(table.rows.iter().map(|(l, _)| l.clone()).collect()),
    }];
    for (i, h) in table.headers.iter().enumerate() {
        sheet.push(column_of(h, Designation::Y, table.rows.iter().map(|(_, v)| v[i]).collect()));
    }
    Ok(Output { table: Some(table), on_sheet: sheet, notes, graph: None, more_graphs: Vec::new() })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(columns: Vec<ColumnId>, spec: StationaritySpec) -> Request {
        Request { columns, rows: RowRange::All, tool: Tool::Stationarity(spec) }
    }

    #[test]
    fn a_walk_and_its_steps() {
        let mut p = Project::new();
        let book = p.add_workbook("ts");
        let mut st: u64 = 7;
        let mut y = 0.0;
        let walk: Vec<f64> = (0..150)
            .map(|_| {
                st = st.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                y += (st >> 11) as f64 / (1u64 << 53) as f64 - 0.5;
                y
            })
            .collect();
        let steps: Vec<f64> = walk.windows(2).map(|w| w[1] - w[0]).collect();
        let w = p.add_column(book, 0, Designation::Y, ColumnData::numeric(walk)).unwrap();
        let s = p.add_column(book, 0, Designation::Y, ColumnData::numeric(steps)).unwrap();
        let out = p.analyze(&request(vec![w, s], StationaritySpec::default())).unwrap();
        let t = out.table.as_ref().unwrap();
        assert_eq!(t.rows.len(), 4);
        assert!(out.notes[0].contains("does not reject a unit root"), "{:?}", out.notes);
        assert!(
            out.notes[1].contains("rejects a unit root") && out.notes[1].contains("Prob >= 0.1"),
            "{:?}",
            out.notes
        );
        // T497: the finite-sample 5 % critical value at the regression's points, and the ADF probability.
        let nobs = t.rows[0].1[2] as usize;
        assert_eq!(t.rows[0].1[4], calc::stationarity::adf_critical_values(AdfTerms::Constant, nobs)[1]);
        assert!(t.rows[0].1[6] > 0.05 && t.rows[2].1[6] < 0.01, "{:?}", t.rows);
        assert!(out.notes[0].contains("(Prob = "), "{:?}", out.notes);
        // Automatic KPSS lags (Hobijn et al.).
        let auto = StationaritySpec { kpss_auto: true, ..Default::default() };
        let out = p.analyze(&request(vec![w], auto)).unwrap();
        let lags = out.table.as_ref().unwrap().rows[1].1[1] as usize;
        assert_eq!(lags, calc::stationarity::kpss_auto_lags(p.numeric(w).unwrap(), KpssNull::Level).unwrap());
        let spec = StationaritySpec { adf_lags: Some(200), ..Default::default() };
        assert!(p.analyze(&request(vec![w], spec)).is_err());
    }
}
