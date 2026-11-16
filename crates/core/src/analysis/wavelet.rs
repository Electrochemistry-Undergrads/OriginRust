//! The discrete wavelet tools of `Project::analyze` (Analysis > Signal Processing > Wavelet: Decompose,
//! Reconstruction, Denoise, Smooth; T96, analysis-tools.md 19). The numbers come from
//! `sheaf_calc::wavelet`; this file reads the inputs and names the result columns as Origin does
//! (`DWT_CA1`, `DWT_CD1`, `IDWT_Result1`, `Denoised Y1`, `WT Smooth Y1`, with Origin's comments).
//! The columns go in the result sheet, as for the other signal tools.

use super::*;
use sheaf_calc::wavelet as wt;
use sheaf_calc::{Wavelet, WaveletExtension};

/// What [`Tool::WaveletDecompose`] does (Origin's `dwt`, measured; 19.2): one level of the transform
/// of each input, the approximation and the detail coefficients in two new columns.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WaveletDecomposeSpec {
    pub wavelet: Wavelet,
    pub extension: WaveletExtension,
}

/// What [`Tool::WaveletReconstruct`] does (Origin's `idwt`, measured; 19.3): the inputs are the
/// approximation coefficients, `detail` the detail coefficients.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaveletReconstructSpec {
    pub detail: ColumnId,
    pub wavelet: Wavelet,
    pub extension: WaveletExtension,
}

/// The threshold rule of [`WaveletDenoiseSpec`] (Origin's `thrt`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WaveletThresholdKind {
    /// `custom`: a percentage of the largest detail coefficient.
    #[default]
    Custom,
    /// `sqtwolog`: a threshold from the number of points; the percentage is not used.
    SqtwoLog,
}

/// What [`Tool::WaveletDenoise`] does (Origin's `wtdenoise`, measured; 19.4).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaveletDenoiseSpec {
    pub wavelet: Wavelet,
    pub extension: WaveletExtension,
    pub threshold: WaveletThresholdKind,
    /// Origin's `level`, 1 to 7.
    pub level: usize,
    /// Origin's `thrd`: the threshold of every level, in percent of the largest coefficient.
    pub percent: f64,
}

impl Default for WaveletDenoiseSpec {
    fn default() -> Self {
        WaveletDenoiseSpec {
            wavelet: Wavelet::Haar,
            extension: WaveletExtension::Periodic,
            threshold: WaveletThresholdKind::Custom,
            level: 1,
            percent: 50.0,
        }
    }
}

/// What [`Tool::WaveletSmooth`] does (Origin's `wtsmooth`, measured; 19.5).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaveletSmoothSpec {
    pub wavelet: Wavelet,
    pub extension: WaveletExtension,
    /// Origin's `cutoff`, percent.
    pub cutoff: f64,
}

impl Default for WaveletSmoothSpec {
    fn default() -> Self {
        WaveletSmoothSpec { wavelet: Wavelet::Haar, extension: WaveletExtension::Periodic, cutoff: 20.0 }
    }
}

fn error(name: &str, e: wt::WaveletError) -> AnalysisError {
    AnalysisError::BadOption(format!("column {name}: {e}"))
}

/// `50`, `12.5`: the percentage as typed, without trailing zeros.
fn percent(v: f64) -> String {
    let s = format!("{v:.6}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn y_column(long_name: String, comments: String, data: Vec<f64>) -> SheetColumn {
    SheetColumn {
        designation: Designation::Y,
        long_name,
        units: String::new(),
        comments,
        data: ColumnData::numeric(data),
    }
}

pub(super) fn run_decompose(
    spec: &WaveletDecomposeSpec,
    inputs: &[Input],
    out: &mut Vec<SheetColumn>,
) -> Result<(), AnalysisError> {
    for (i, inp) in inputs.iter().enumerate() {
        let y = signal_values(inp.ys(), &inp.short)?;
        let (ca, cd) = wt::dwt(&y, spec.wavelet, spec.extension).map_err(|e| error(&inp.short, e))?;
        let w = spec.wavelet.name();
        let n = i + 1;
        out.push(y_column(
            format!("DWT_CA{n}"),
            format!("Approx Coeff of {w} dwt on {}{}", inp.short, inp.range_note()),
            ca,
        ));
        out.push(y_column(
            format!("DWT_CD{n}"),
            format!("Detail Coeff of {w} dwt on {}{}", inp.short, inp.range_note()),
            cd,
        ));
    }
    Ok(())
}

pub(super) fn run_reconstruct(
    spec: &WaveletReconstructSpec,
    inputs: &[Input],
    detail: &[f64],
    detail_short: &str,
    out: &mut Vec<SheetColumn>,
) -> Result<(), AnalysisError> {
    let cd = {
        let end = detail.iter().rposition(|x| !x.is_nan()).map_or(0, |i| i + 1);
        signal_values(&detail[..end], detail_short)?
    };
    for (i, inp) in inputs.iter().enumerate() {
        let ca = signal_values(inp.ys(), &inp.short)?;
        let y = wt::idwt(&ca, &cd, spec.wavelet, spec.extension).map_err(|e| error(&inp.short, e))?;
        out.push(y_column(
            format!("IDWT_Result{}", i + 1),
            format!("{} idwt of {}, {detail_short}", spec.wavelet.name(), inp.short),
            y,
        ));
    }
    Ok(())
}

pub(super) fn run_denoise(
    spec: &WaveletDenoiseSpec,
    inputs: &[Input],
    out: &mut Vec<SheetColumn>,
) -> Result<(), AnalysisError> {
    for (i, inp) in inputs.iter().enumerate() {
        let y = signal_values(inp.ys(), &inp.short)?;
        let threshold = match spec.threshold {
            WaveletThresholdKind::Custom => wt::Threshold::Custom(spec.percent),
            WaveletThresholdKind::SqtwoLog => wt::Threshold::SqtwoLog,
        };
        let d =
            wt::denoise(&y, spec.wavelet, spec.extension, spec.level, threshold).map_err(|e| error(&inp.short, e))?;
        let n = i + 1;
        let rule = match spec.threshold {
            WaveletThresholdKind::Custom => format!("{}% threshold", percent(spec.percent)),
            WaveletThresholdKind::SqtwoLog => "sqtwolog threshold".to_string(),
        };
        out.push(SheetColumn {
            designation: Designation::X,
            long_name: format!("Denoised X{n}"),
            units: inp.x_units.clone(),
            comments: String::new(),
            data: ColumnData::numeric(inp.xs()[..d.len()].to_vec()),
        });
        out.push(SheetColumn {
            designation: Designation::Y,
            long_name: format!("Denoised Y{n}"),
            units: inp.units.clone(),
            comments: format!(
                "{} level(s) {rule} {} denoise of {}{}",
                spec.level,
                spec.wavelet.name(),
                inp.short,
                inp.range_note()
            ),
            data: ColumnData::numeric(d),
        });
    }
    Ok(())
}

pub(super) fn run_smooth(
    spec: &WaveletSmoothSpec,
    inputs: &[Input],
    out: &mut Vec<SheetColumn>,
) -> Result<(), AnalysisError> {
    for (i, inp) in inputs.iter().enumerate() {
        let y = signal_values(inp.ys(), &inp.short)?;
        let s = wt::smooth(&y, spec.wavelet, spec.extension, spec.cutoff).map_err(|e| error(&inp.short, e))?;
        let n = i + 1;
        out.push(SheetColumn {
            designation: Designation::X,
            long_name: format!("WT Smoothed X{n}"),
            units: inp.x_units.clone(),
            comments: String::new(),
            data: ColumnData::numeric(inp.xs()[..s.len()].to_vec()),
        });
        out.push(SheetColumn {
            designation: Designation::Y,
            long_name: format!("WT Smooth Y{n}"),
            units: inp.units.clone(),
            comments: format!(
                "{}% cutoff {} smooth of {}{}",
                percent(spec.cutoff),
                spec.wavelet.name(),
                inp.short,
                inp.range_note()
            ),
            data: ColumnData::numeric(s),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(columns: &[ColumnId], tool: Tool) -> Request {
        Request { columns: columns.to_vec(), rows: RowRange::All, tool }
    }

    fn project(n: usize) -> (Project, WindowId, ColumnId) {
        let mut p = Project::new();
        let book = p.add_workbook("wt");
        let x: Vec<f64> = (0..n).map(|i| i as f64 * 0.5).collect();
        let y: Vec<f64> = (0..n).map(|i| (i as f64 * 0.7).sin() + 0.1 * i as f64).collect();
        let xc = p.add_column(book, 0, Designation::X, ColumnData::numeric(x)).unwrap();
        p.column_mut(xc).unwrap().units = "s".into();
        let yc = p.add_column(book, 0, Designation::Y, ColumnData::numeric(y)).unwrap();
        p.column_mut(yc).unwrap().short_name = "B".into();
        (p, book, yc)
    }

    #[test]
    fn decompose_then_reconstruct_gives_the_signal_back() {
        let (mut p, book, y) = project(16);
        let spec = WaveletDecomposeSpec { wavelet: Wavelet::Db(2), extension: WaveletExtension::Periodic };
        let r = p.analyze(&request(&[y], Tool::WaveletDecompose(spec))).unwrap();
        let rs = &p.workbook(book).unwrap().sheets[r.new_sheet.unwrap()];
        let names: Vec<&str> = rs.columns.iter().map(|c| c.long_name.as_str()).collect();
        assert_eq!(names, ["DWT_CA1", "DWT_CD1"]);
        assert_eq!(rs.columns[0].comments, "Approx Coeff of DB2 dwt on B");
        assert_eq!(rs.columns[1].comments, "Detail Coeff of DB2 dwt on B");
        assert_eq!(rs.columns[0].numeric().unwrap().len(), 8);
        let (ca, cd) = (rs.columns[0].id, rs.columns[1].id);
        let spec =
            WaveletReconstructSpec { detail: cd, wavelet: Wavelet::Db(2), extension: WaveletExtension::Periodic };
        let r = p.analyze(&request(&[ca], Tool::WaveletReconstruct(spec))).unwrap();
        let rs = &p.workbook(book).unwrap().sheets[r.new_sheet.unwrap()];
        assert_eq!(rs.columns[0].long_name, "IDWT_Result1");
        assert!(rs.columns[0].comments.starts_with("DB2 idwt of "), "{}", rs.columns[0].comments);
        let want = p.column(y).unwrap().numeric().unwrap().to_vec();
        for (a, b) in rs.columns[0].numeric().unwrap().iter().zip(&want) {
            assert!((a - b).abs() < 1e-12);
        }
    }

    #[test]
    fn odd_periodic_length_is_refused_with_the_column_name() {
        let (mut p, _, y) = project(21);
        let e = p.analyze(&request(&[y], Tool::WaveletDecompose(WaveletDecomposeSpec::default()))).unwrap_err();
        assert!(e.to_string().contains("column B") && e.to_string().contains("even"), "{e}");
        let zero = WaveletDecomposeSpec { extension: WaveletExtension::ZeroPadded, ..Default::default() };
        assert!(p.analyze(&request(&[y], Tool::WaveletDecompose(zero))).is_ok());
    }

    #[test]
    fn denoise_and_smooth_name_their_columns_as_origin_does() {
        let (mut p, book, y) = project(64);
        let spec = WaveletDenoiseSpec { wavelet: Wavelet::Db(2), level: 2, ..Default::default() };
        let r = p.analyze(&request(&[y], Tool::WaveletDenoise(spec))).unwrap();
        let rs = &p.workbook(book).unwrap().sheets[r.new_sheet.unwrap()];
        assert_eq!(
            (rs.columns[0].long_name.as_str(), rs.columns[1].long_name.as_str()),
            ("Denoised X1", "Denoised Y1")
        );
        assert_eq!(rs.columns[0].units, "s");
        assert_eq!(rs.columns[1].comments, "2 level(s) 50% threshold DB2 denoise of B");
        assert_eq!(rs.columns[1].numeric().unwrap().len(), 64);
        let too_deep = WaveletDenoiseSpec { level: 7, ..spec };
        assert!(p.analyze(&request(&[y], Tool::WaveletDenoise(too_deep))).is_err());

        let spec = WaveletSmoothSpec { wavelet: Wavelet::Db(2), cutoff: 20.0, ..Default::default() };
        let r = p.analyze(&request(&[y], Tool::WaveletSmooth(spec))).unwrap();
        let rs = &p.workbook(book).unwrap().sheets[r.new_sheet.unwrap()];
        assert_eq!(
            (rs.columns[0].long_name.as_str(), rs.columns[1].long_name.as_str()),
            ("WT Smoothed X1", "WT Smooth Y1")
        );
        assert_eq!(rs.columns[1].comments, "20% cutoff DB2 smooth of B");
    }
}
