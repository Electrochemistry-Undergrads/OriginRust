//! Three-way ANOVA (Origin's Statistics > ANOVA > Three-Way ANOVA, `ANOVAThreeWay`): reads the three factor
//! columns and the data column itself (the factors may hold text), lays the report out and leaves the numbers to
//! `sheaf_calc::anova_three_way`. See `docs/spec/statistics.md`, section 25.

use super::statistics::{Output, check_alpha, label_column, level_text, num, number_column};
use super::*;
use calc::ThreeWayEffect as Effect;

/// The number of a level in `levels`, which grows by the levels in the order they first appear (Origin numbers
/// the levels in order of appearance).
fn level_index(levels: &mut Vec<String>, name: &str) -> usize {
    levels.iter().position(|l| l == name).unwrap_or_else(|| {
        levels.push(name.to_string());
        levels.len() - 1
    })
}

/// The sign Origin gives a comparison: the difference is `mean(i) - mean(j)` and the statistic carries its sign.
fn too_many_levels() -> String {
    format!(
        "a factor has more than {} levels; is a column of IDs or continuous numbers used as a factor?",
        calc::MAX_THREE_WAY_LEVELS
    )
}

fn origin_sign(diff: f64) -> f64 {
    if diff < 0.0 { -1.0 } else { 1.0 }
}

pub(super) fn three_way_anova(
    spec: &ThreeWayAnovaSpec,
    sheet: &Worksheet,
    columns: &[ColumnId],
    rows: RowRange,
) -> Result<Output, AnalysisError> {
    check_alpha(spec.alpha)?;
    check_alpha(spec.comparison_alpha)?;
    check_alpha(spec.power_alpha)?;
    if let Some(bad) = spec.power_sizes.iter().find(|s| !(s.is_finite() && **s > 0.0)) {
        return Err(AnalysisError::BadOption(format!("a hypothetical sample size must be positive, not {bad}")));
    }
    if spec.comparisons.contains(&MeansComparison::Dunnett) {
        return Err(AnalysisError::BadOption(
            "Dunnett's test is not part of Origin's three-way ANOVA (it lists the other seven methods)".into(),
        ));
    }
    if columns.len() != 4 {
        return Err(AnalysisError::BadOption(format!(
            "the three-way ANOVA needs four columns (factor A, factor B, factor C, data), {} were chosen",
            columns.len()
        )));
    }
    let column = |id: ColumnId| sheet.column(id).ok_or(AnalysisError::NoSuchColumn(id));
    let (ca, cb, cc, cy) = (column(columns[0])?, column(columns[1])?, column(columns[2])?, column(columns[3])?);
    // Masked cells and rows the worksheet filter hides are missing values (T253).
    let ys = sheet.values_of(cy).ok_or_else(|| AnalysisError::NotNumeric(cy.short_name.clone()))?;
    let len = ca.data.len().max(cb.data.len()).max(cc.data.len()).max(ys.len());
    if len == 0 {
        return Err(AnalysisError::Empty(cy.short_name.clone()));
    }
    let (first, last) = match rows {
        RowRange::All => (0, len - 1),
        RowRange::Rows(..) => resolve_rows(rows, &cy.short_name, &[], len)?,
        RowRange::XBetween(..) => {
            return Err(AnalysisError::BadOption(
                "the three-way ANOVA has no X column to limit; choose rows with a row range".into(),
            ));
        }
    };
    let mut levels: [Vec<String>; 3] = [Vec::new(), Vec::new(), Vec::new()];
    let (mut a, mut b, mut c, mut y) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut left_out = 0usize;
    for row in first..=last.min(len - 1) {
        let value = ys.get(row).copied().unwrap_or(f64::NAN);
        let texts = [level_text(ca, row), level_text(cb, row), level_text(cc, row)];
        if value.is_nan() || texts.iter().any(Option::is_none) {
            // Rows that are empty in all four columns are just the end of a shorter column.
            left_out += usize::from(!(value.is_nan() && texts.iter().all(Option::is_none)));
            continue;
        }
        let [la, lb, lc] = &mut levels;
        if [&*la, &*lb, &*lc].iter().any(|l| l.len() > calc::MAX_THREE_WAY_LEVELS) {
            return Err(AnalysisError::BadOption(too_many_levels()));
        }
        a.push(level_index(la, texts[0].as_deref().unwrap_or_default()));
        b.push(level_index(lb, texts[1].as_deref().unwrap_or_default()));
        c.push(level_index(lc, texts[2].as_deref().unwrap_or_default()));
        y.push(value);
    }
    let r = calc::anova_three_way(&a, &b, &c, &y, spec.terms).map_err(|e| AnalysisError::BadOption(e.to_string()))?;
    let (na, nb, nc) = (ca.display_name().to_string(), cb.display_name().to_string(), cc.display_name().to_string());
    let [lev_a, lev_b, lev_c] = &levels;
    let effect_name = |e: Effect| -> String {
        match e {
            Effect::A => na.clone(),
            Effect::B => nb.clone(),
            Effect::C => nc.clone(),
            Effect::Ab => format!("{na}*{nb}"),
            Effect::Ac => format!("{na}*{nc}"),
            Effect::Bc => format!("{nb}*{nc}"),
            Effect::Abc => format!("{na}*{nb}*{nc}"),
        }
    };
    // The name of level or cell `i` of an effect.
    let level_name = |e: Effect, i: usize| -> String {
        let (la, lb, lc) = (lev_a.len(), lev_b.len(), lev_c.len());
        match e {
            Effect::A => lev_a[i].clone(),
            Effect::B => lev_b[i].clone(),
            Effect::C => lev_c[i].clone(),
            Effect::Ab => format!("{} / {}", lev_a[i / lb], lev_b[i % lb]),
            Effect::Ac => format!("{} / {}", lev_a[i / lc], lev_c[i % lc]),
            Effect::Bc => format!("{} / {}", lev_b[i / lc], lev_c[i % lc]),
            Effect::Abc => {
                let _ = la;
                format!("{} / {} / {}", lev_a[i / (lb * lc)], lev_b[(i / lc) % lb], lev_c[i % lc])
            }
        }
    };

    let mut on_sheet = Vec::new();
    // Descriptive statistics of the ticked effects and of all data.
    if !spec.descriptive.is_empty() || spec.descriptive_overall {
        let (mut group, mut stats) = (Vec::new(), Vec::new());
        for e in Effect::ALL.into_iter().filter(|e| spec.descriptive.contains(e)) {
            for (i, s) in r.stats_of(e).iter().enumerate() {
                group.push(format!("{}: {}", effect_name(e), level_name(e, i)));
                stats.push(*s);
            }
        }
        if spec.descriptive_overall {
            group.push("All data".to_string());
            stats.push(r.overall);
        }
        on_sheet.push(label_column("Group", group));
        on_sheet.push(number_column("N", "", stats.iter().map(|s| s.n as f64).collect()));
        on_sheet.push(number_column("Mean", "", stats.iter().map(|s| s.mean).collect()));
        on_sheet.push(number_column("SD", "", stats.iter().map(|s| s.sd).collect()));
        on_sheet.push(number_column("SEM", "", stats.iter().map(|s| s.sem).collect()));
        on_sheet.push(number_column("Variance", "", stats.iter().map(|s| s.sd * s.sd).collect()));
    }

    // The ANOVA table.
    let mut table = ResultTable::new(&["DF", "Sum of Squares", "Mean Square", "F Value", "Prob>F"]);
    let mut rows_out: Vec<(String, calc::TwoWayRow)> = r.rows.iter().map(|(e, row)| (effect_name(*e), *row)).collect();
    rows_out.extend([("Model".to_string(), r.model), ("Error".to_string(), r.error), ("Total".to_string(), r.total)]);
    for (name, row) in &rows_out {
        table.push_row(name.clone(), vec![row.df, row.sum_sq, row.mean_sq, row.f, row.p], vec![String::new(); 5]);
    }
    on_sheet.push(label_column("Source", rows_out.iter().map(|(n, _)| n.clone()).collect()));
    for (i, h) in table.headers.iter().enumerate() {
        on_sheet.push(number_column(h, "", table.rows.iter().map(|(_, v)| v[i]).collect()));
    }

    let mut notes = Vec::new();
    let terms: Vec<String> = r
        .rows
        .iter()
        .map(|(e, row)| {
            format!(
                "{}: F({}, {}) = {}, p = {} ({})",
                effect_name(*e),
                num(row.df),
                num(r.error.df),
                num(row.f),
                num(row.p),
                if row.p < spec.alpha { "significant" } else { "not significant" }
            )
        })
        .collect();
    notes.push(format!(
        "Three-way ANOVA of {} numbers ({} levels of {na}, {} of {nb}, {} of {nc}), type III sums of squares: {}; at the {} level.",
        y.len(),
        lev_a.len(),
        lev_b.len(),
        lev_c.len(),
        terms.join("; "),
        num(spec.alpha)
    ));
    let sizes: Vec<usize> = r.cell_stats.iter().map(|s| s.n).collect();
    if sizes.iter().any(|n| *n != sizes[0]) {
        notes.push("The cells have different sizes: every sum of squares is adjusted for all the other terms (type III, as Origin does).".to_string());
    }
    if left_out > 0 {
        notes.push(format!("{left_out} rows with a missing factor level or no number were left out."));
    }
    if r.error.df <= 0.0 {
        notes.push("There are no error degrees of freedom: no F tests and no means comparisons.".to_string());
    }

    // Levene's test.
    if spec.levene {
        match r.levene() {
            Some(l) => {
                on_sheet.push(label_column("Levene's Test", vec!["Model".into(), "Error".into()]));
                on_sheet.push(number_column("DF", "", vec![l.model.df, l.error.df]));
                on_sheet.push(number_column("Sum of Squares", "", vec![l.model.sum_sq, l.error.sum_sq]));
                on_sheet.push(number_column("Mean Square", "", vec![l.model.mean_sq, l.error.mean_sq]));
                on_sheet.push(number_column("F Value", "", vec![l.model.f, f64::NAN]));
                on_sheet.push(number_column("Prob>F", "", vec![l.model.p, f64::NAN]));
                notes.push(if l.model.f.is_finite() {
                    format!(
                        "Levene's test of equal variances (absolute deviations from the cell means): F({}, {}) = {}, p = {} ({}).",
                        num(l.model.df),
                        num(l.error.df),
                        num(l.model.f),
                        num(l.model.p),
                        if l.model.p < spec.alpha { "the variances differ" } else { "no difference of the variances shown" }
                    )
                } else {
                    "Levene's test: the absolute deviations do not vary within the cells (for example two numbers per cell), so F is not defined.".to_string()
                });
            }
            None => notes.push("Levene's test needs at least two cells and more numbers than cells.".to_string()),
        }
    }

    // Means comparisons: one block per method, the pairs of A, B, C and A*B.
    if r.error.df > 0.0 {
        for &method in &spec.comparisons {
            let mut label = Vec::new();
            let mut all = Vec::new();
            for e in [Effect::A, Effect::B, Effect::C, Effect::Ab] {
                for c in r.compare(e, method, spec.comparison_alpha) {
                    label.push(format!("{}: {} - {}", effect_name(e), level_name(e, c.i), level_name(e, c.j)));
                    all.push(c);
                }
            }
            if all.is_empty() {
                continue;
            }
            // Origin prints mean(i) - mean(j) and the statistic with the same sign (Scheffe: the signed t).
            let diff = |c: &calc::Comparison| -c.diff;
            let stat = |c: &calc::Comparison| match method {
                MeansComparison::Scheffe => diff(c) / c.se,
                _ => origin_sign(diff(c)) * c.stat.abs(),
            };
            let col = |f: &dyn Fn(&calc::Comparison) -> f64| all.iter().map(f).collect::<Vec<f64>>();
            let stat_name = match method {
                MeansComparison::Tukey => "q",
                _ => "t",
            };
            on_sheet.push(label_column(&format!("{} comparison", method.label()), label.clone()));
            on_sheet.push(number_column("Difference", "", col(&diff)));
            on_sheet.push(number_column("SE", "", col(&|c| c.se)));
            on_sheet.push(number_column(&format!("Statistic ({stat_name})"), "", col(&stat)));
            on_sheet.push(number_column("Prob", "", col(&|c| c.p)));
            on_sheet.push(number_column("Alpha", "", col(&|c| c.alpha)));
            on_sheet.push(number_column("Significant", "", col(&|c| f64::from(u8::from(c.significant)))));
            on_sheet.push(number_column("LCL", "", col(&|c| -c.ucl)));
            on_sheet.push(number_column("UCL", "", col(&|c| -c.lcl)));
            notes.push(format!(
                "{} means comparison at the {} level: {} of {} pairs significant.",
                method.label(),
                num(spec.comparison_alpha),
                all.iter().filter(|c| c.significant).count(),
                all.len()
            ));
        }
    }

    // Power of the F tests.
    if spec.power_actual || !spec.power_sizes.is_empty() {
        let sizes: &[f64] = &spec.power_sizes;
        let (mut effect, mut alpha, mut n, mut power) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        for p in r.power(spec.power_alpha, sizes) {
            for (k, (size, pw)) in p.powers.iter().enumerate() {
                if k == 0 && !spec.power_actual {
                    continue;
                }
                effect.push(effect_name(p.effect));
                alpha.push(p.alpha);
                n.push(*size);
                power.push(*pw);
            }
        }
        on_sheet.push(label_column("Power Analysis", effect));
        on_sheet.push(number_column("Alpha", "", alpha));
        on_sheet.push(number_column("N", "", n));
        on_sheet.push(number_column("Power", "", power));
    }
    Ok(Output { table: Some(table), on_sheet, notes, plots: Vec::new(), comparison_pairs: Vec::new() })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The balanced 2 x 3 x 2 design of the Origin run (24 numbers, two per cell).
    fn project() -> (Project, Vec<ColumnId>) {
        let rows: [[f64; 4]; 24] = [
            [1.0, 1.0, 1.0, 15.814],
            [1.0, 2.0, 2.0, 15.4569],
            [2.0, 1.0, 2.0, 17.6589],
            [2.0, 3.0, 1.0, 16.2762],
            [1.0, 2.0, 1.0, 14.8436],
            [1.0, 3.0, 2.0, 15.9664],
            [2.0, 2.0, 2.0, 20.2379],
            [1.0, 1.0, 1.0, 16.4812],
            [1.0, 3.0, 1.0, 14.5347],
            [2.0, 1.0, 2.0, 17.82],
            [2.0, 3.0, 2.0, 15.9952],
            [1.0, 2.0, 1.0, 13.867],
            [2.0, 1.0, 1.0, 17.812],
            [2.0, 2.0, 2.0, 20.4605],
            [1.0, 1.0, 2.0, 17.4733],
            [1.0, 3.0, 1.0, 15.7478],
            [2.0, 2.0, 1.0, 16.2572],
            [2.0, 3.0, 2.0, 15.4057],
            [1.0, 2.0, 2.0, 14.5925],
            [2.0, 1.0, 1.0, 16.4532],
            [2.0, 3.0, 1.0, 17.6181],
            [1.0, 1.0, 2.0, 16.3418],
            [1.0, 3.0, 2.0, 16.499],
            [2.0, 2.0, 1.0, 17.66],
        ];
        let mut p = Project::new();
        let book = p.add_workbook("a3");
        let ids = (0..4)
            .map(|k| {
                let data = rows.iter().map(|r| r[k]).collect::<Vec<f64>>();
                let column = if k < 3 {
                    ColumnData::text(data.iter().map(|v| format!("L{v}")).collect())
                } else {
                    ColumnData::numeric(data)
                };
                p.add_column(book, 0, Designation::Y, column).unwrap()
            })
            .collect();
        (p, ids)
    }

    fn request(ids: &[ColumnId], spec: ThreeWayAnovaSpec) -> Request {
        Request { columns: ids.to_vec(), rows: RowRange::All, tool: Tool::ThreeWayAnova(spec) }
    }

    #[test]
    fn three_way_anova_gives_origin_s_balanced_table() {
        let (mut p, ids) = project();
        let r = p.analyze(&request(&ids, ThreeWayAnovaSpec::default())).unwrap();
        assert!(r.message.contains("Three-way ANOVA") && r.message.contains("ANOVA3Way1"), "{}", r.message);
        let t = r.table.as_ref().unwrap();
        let names: Vec<&str> = t.rows.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names.len(), 10);
        assert_eq!(t.headers, ["DF", "Sum of Squares", "Mean Square", "F Value", "Prob>F"]);
        let a = &t.rows[0].1;
        assert_eq!(a[0], 1.0);
        assert!((a[1] - 20.23401).abs() < 1e-5 && (a[3] - 43.28302).abs() < 1e-4, "{a:?}");
        let model = &t.rows[7];
        assert_eq!(model.0, "Model");
        assert_eq!(model.1[0], 11.0);
        // Descriptive statistics come with the default; N of the first level of A is 12.
        let ws = &p.workbook(r.book).unwrap().sheets[r.new_sheet.unwrap()];
        let n_col = ws.columns.iter().find(|c| c.long_name == "N").unwrap().numeric().unwrap();
        assert_eq!(n_col[0], 12.0);
        // Tukey comparison block: factor A has one pair, factor B three, C one, A*B fifteen.
        let comp = ws.columns.iter().find(|c| c.long_name == "Tukey comparison").unwrap();
        assert_eq!(comp.text().unwrap().len(), 1 + 3 + 1 + 15);
        let diff = ws.columns.iter().find(|c| c.long_name == "Difference").unwrap().numeric().unwrap();
        assert!((diff[0] + 1.83639).abs() < 1e-5, "Origin prints mean(1) - mean(2): {}", diff[0]);
        let q = ws.columns.iter().find(|c| c.long_name == "Statistic (q)").unwrap().numeric().unwrap();
        assert!((q[0] + 13.15797).abs() < 1e-4);
    }

    #[test]
    fn levene_power_and_several_comparisons_are_added_on_request() {
        let (mut p, ids) = project();
        let spec = ThreeWayAnovaSpec {
            levene: true,
            power_actual: true,
            power_sizes: vec![50.0, 100.0, 200.0],
            comparisons: vec![MeansComparison::Bonferroni, MeansComparison::HolmBonferroni],
            descriptive: Vec::new(),
            descriptive_overall: false,
            ..Default::default()
        };
        let r = p.analyze(&request(&ids, spec)).unwrap();
        let ws = &p.workbook(r.book).unwrap().sheets[r.new_sheet.unwrap()];
        let names: Vec<&str> = ws.columns.iter().map(|c| c.long_name.as_str()).collect();
        assert!(names.contains(&"Levene's Test") && names.contains(&"Power Analysis"));
        assert!(names.contains(&"Bonferroni comparison") && names.contains(&"Bonferroni-Holm comparison"));
        assert!(!names.contains(&"Group"), "no descriptive statistics were asked for");
        let power = ws.columns.iter().find(|c| c.long_name == "Power").unwrap().numeric().unwrap();
        assert_eq!(power.len(), 7 * 4);
        assert!((power[0] - 0.99997).abs() < 6e-5 && (power[4] - 0.62766).abs() < 6e-5, "{power:?}");
    }

    #[test]
    fn three_way_anova_reports_bad_input_instead_of_panicking() {
        let (mut p, ids) = project();
        assert!(p.analyze(&request(&ids[..3], ThreeWayAnovaSpec::default())).is_err());
        let dunnett = ThreeWayAnovaSpec { comparisons: vec![MeansComparison::Dunnett], ..Default::default() };
        assert!(p.analyze(&request(&ids, dunnett)).is_err());
        // The data column must be numeric.
        let swapped = [ids[0], ids[1], ids[3], ids[2]];
        assert!(p.analyze(&request(&swapped, ThreeWayAnovaSpec::default())).is_err());
        let x = Request {
            columns: ids.clone(),
            rows: RowRange::XBetween(0.0, 1.0),
            tool: Tool::ThreeWayAnova(ThreeWayAnovaSpec::default()),
        };
        assert!(p.analyze(&x).is_err());
        let bad = ThreeWayAnovaSpec { power_sizes: vec![-5.0], ..Default::default() };
        assert!(p.analyze(&request(&ids, bad)).is_err());
        // A row range that leaves too few numbers.
        let few = Request {
            columns: ids.clone(),
            rows: RowRange::Rows(1, 6),
            tool: Tool::ThreeWayAnova(ThreeWayAnovaSpec::default()),
        };
        assert!(p.analyze(&few).is_err());
    }

    #[test]
    fn terms_can_be_switched_off() {
        let (mut p, ids) = project();
        let spec = ThreeWayAnovaSpec {
            terms: calc::ThreeWayTerms { ab: true, ac: false, bc: false, abc: false },
            comparisons: Vec::new(),
            ..Default::default()
        };
        let r = p.analyze(&request(&ids, spec)).unwrap();
        let t = r.table.as_ref().unwrap();
        assert_eq!(t.rows.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>().len(), 4 + 3);
    }

    #[test]
    fn masked_data_cells_are_left_out() {
        // T253: as the other tools, a masked cell is a missing value.
        let (mut p, ids) = project();
        let table = |p: &mut Project| {
            format!("{:?}", p.analyze(&request(&ids, ThreeWayAnovaSpec::default())).map(|r| r.table.map(|t| t.rows)))
        };
        let all = table(&mut p);
        let mut gone = p.clone();
        gone.column_mut(ids[3]).unwrap().numeric_mut().unwrap()[0] = f64::NAN;
        let want = table(&mut gone);
        assert_ne!(all, want);
        p.column_mut(ids[3]).unwrap().mask.insert(0);
        assert_eq!(table(&mut p), want);
    }
}
