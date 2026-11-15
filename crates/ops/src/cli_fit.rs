//! CLI fit arguments converted to a shared operation specification.
use crate as sheaf_ops;
use crate::args::{BandKind, FitArgs, WeightKind};
use crate::fit::FitSpec;

pub fn fit_spec(a: &FitArgs) -> FitSpec {
    FitSpec {
        model: a.model.clone(),
        formula: a.formula.clone(),
        params: a.params.clone(),
        fdf: a.fdf.clone(),
        init: a.init.clone(),
        fix: a.fix.clone(),
        bounds: a.bounds.clone(),
        weight: match a.weight {
            WeightKind::None => sheaf_ops::fit::WeightKind::None,
            WeightKind::Instrumental => sheaf_ops::fit::WeightKind::Instrumental,
            WeightKind::Statistical => sheaf_ops::fit::WeightKind::Statistical,
            WeightKind::Direct => sheaf_ops::fit::WeightKind::Direct,
        },
        yerr: a.yerr.clone(),
        xerr: a.xerr.clone(),
        regress: a.regress.clone(),
        compare: a.compare.clone(),
        rank: a.rank.clone(),
        no_scale_errors: a.no_scale_errors,
        global: a.global,
        share: a.share.clone(),
        confidence: a.confidence,
        bands: match a.bands {
            BandKind::None => sheaf_ops::fit::BandKind::None,
            BandKind::Confidence => sheaf_ops::fit::BandKind::Confidence,
            BandKind::Prediction => sheaf_ops::fit::BandKind::Prediction,
            BandKind::Both => sheaf_ops::fit::BandKind::Both,
        },
        level: a.level,
        max_iterations: a.max_iterations,
        tolerance: a.tolerance,
        y: a.y.clone(),
        book: a.book.clone(),
        sheet: a.sheet.clone(),
        rows: a.rows.clone(),
        x_range: a.x_range.clone(),
        z: a.z.clone(),
        x: a.x.clone(),
    }
}
