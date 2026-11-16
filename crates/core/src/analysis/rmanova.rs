//! One-way and two-way repeated measures ANOVA (Origin's Statistics > ANOVA > One-Way and Two-Way Repeated
//! Measures ANOVA): reads the level, data and subject columns itself (levels and subjects may be text), lays the
//! report out and leaves the numbers to `sheaf_calc::rm_anova`. See `docs/spec/statistics.md`, section 26.

use super::statistics::{Output, check_alpha, label_column, level_text, num, number_column};
use super::*;
use calc::RmTerm;

/// The number of `name` in `names`, which grows by the names in the order they first appear.
fn index_of(names: &mut Vec<String>, name: &str) -> usize {
    names.iter().position(|l| l == name).unwrap_or_else(|| {
        names.push(name.to_string());
        names.len() - 1
    })
}

pub(super) fn rm_anova(
    spec: &RmAnovaSpec,
    sheet: &Worksheet,
    columns: &[ColumnId],
    rows: RowRange,
) -> Result<Output, AnalysisError> {
    check_alpha(spec.alpha)?;
    check_alpha(spec.comparison_alpha)?;
    let factors = if spec.two_way { 2 } else { 1 };
    if columns.len() != factors + 2 {
        return Err(AnalysisError::BadOption(if spec.two_way {
            format!(
                "the two-way repeated measures ANOVA needs four columns (factor A, factor B, data, subject), {} were chosen",
                columns.len()
            )
        } else {
            format!(
                "the one-way repeated measures ANOVA needs three columns (factor, data, subject), {} were chosen",
                columns.len()
            )
        }));
    }
    if spec.two_way && !spec.comparisons.is_empty() {
        return Err(AnalysisError::BadOption(
            "the means comparisons of the two-way repeated measures ANOVA were not measured against Origin and are not built".into(),
        ));
    }
    if spec.comparisons.contains(&MeansComparison::Dunnett) {
        return Err(AnalysisError::BadOption("Dunnett's test is not part of this tool".into()));
    }
    if spec.interaction && !spec.two_way {
        return Err(AnalysisError::BadOption("the interaction belongs to the two-way design".into()));
    }
    let column = |id: ColumnId| sheet.column(id).ok_or(AnalysisError::NoSuchColumn(id));
    let level_cols: Vec<&Column> = columns[..factors].iter().map(|&id| column(id)).collect::<Result<_, _>>()?;
    let (cy, cs) = (column(columns[factors])?, column(columns[factors + 1])?);
    let ys = cy.numeric().ok_or_else(|| AnalysisError::NotNumeric(cy.short_name.clone()))?;
    let len = level_cols.iter().map(|c| c.data.len()).chain([ys.len(), cs.data.len()]).max().unwrap_or(0);
    if len == 0 {
        return Err(AnalysisError::Empty(cy.short_name.clone()));
    }
    let (first, last) = match rows {
        RowRange::All => (0, len - 1),
        RowRange::Rows(..) => resolve_rows(rows, &cy.short_name, &[], len)?,
        RowRange::XBetween(..) => {
            return Err(AnalysisError::BadOption(
                "the repeated measures ANOVA has no X column to limit; choose rows with a row range".into(),
            ));
        }
    };
    let mut levels: Vec<Vec<String>> = vec![Vec::new(); factors];
    let mut subjects: Vec<String> = Vec::new();
    // (subject, level of A, level of B, value) of every usable row.
    let mut entries: Vec<(usize, usize, usize, f64)> = Vec::new();
    let mut left_out = 0usize;
    for row in first..=last.min(len - 1) {
        let value = ys.get(row).copied().unwrap_or(f64::NAN);
        let texts: Vec<Option<String>> =
            level_cols.iter().map(|c| level_text(c, row)).chain([level_text(cs, row)]).collect();
        if value.is_nan() || texts.iter().any(Option::is_none) {
            left_out += usize::from(!(value.is_nan() && texts.iter().all(Option::is_none)));
            continue;
        }
        let idx: Vec<usize> =
            (0..factors).map(|k| index_of(&mut levels[k], texts[k].as_deref().unwrap_or_default())).collect();
        let subject = index_of(&mut subjects, texts[factors].as_deref().unwrap_or_default());
        entries.push((subject, idx[0], idx.get(1).copied().unwrap_or(0), value));
    }
    let level_counts: Vec<usize> = levels.iter().map(Vec::len).collect();
    if level_counts.iter().any(|&l| l < 2) {
        return Err(AnalysisError::BadOption("every factor needs at least two levels".into()));
    }
    let cells: usize = level_counts.iter().product();
    let cell_of = |a: usize, b: usize| if spec.two_way { a * level_counts[1] + b } else { a };
    let mut grid: Vec<Vec<Option<f64>>> = vec![vec![None; cells]; subjects.len()];
    for &(s, a, b, v) in &entries {
        let slot = &mut grid[s][cell_of(a, b)];
        if slot.is_some() {
            return Err(AnalysisError::BadOption(format!(
                "subject {} has more than one number in the same cell; a repeated measures ANOVA needs exactly one",
                subjects[s]
            )));
        }
        *slot = Some(v);
    }
    // Subjects without a number in every cell are left out (Sheaf's own rule: Origin's was not measured).
    let (mut data, mut kept) = (Vec::new(), Vec::new());
    for (s, row) in grid.iter().enumerate() {
        if row.iter().all(Option::is_some) {
            data.push(row.iter().map(|v| v.unwrap_or(f64::NAN)).collect::<Vec<f64>>());
            kept.push(s);
        }
    }
    let dropped_subjects = subjects.len() - kept.len();
    let r =
        calc::rm_anova(&data, &level_counts, spec.interaction).map_err(|e| AnalysisError::BadOption(e.to_string()))?;

    let names: Vec<String> = level_cols.iter().map(|c| c.display_name().to_string()).collect();
    let term_name = |t: RmTerm| match t {
        RmTerm::A => names[0].clone(),
        RmTerm::B => names[1].clone(),
        RmTerm::Ab => format!("{}*{}", names[0], names[1]),
    };
    let level_name = |t: RmTerm, i: usize| match t {
        RmTerm::A => levels[0][i].clone(),
        RmTerm::B => levels[1][i].clone(),
        RmTerm::Ab => format!("{} / {}", levels[0][i / level_counts[1]], levels[1][i % level_counts[1]]),
    };

    let mut on_sheet = Vec::new();
    let mut notes = Vec::new();

    // Multivariate tests of each effect that has them (one-way: the factor).
    let mv: Vec<_> = r.effects.iter().filter_map(|e| e.multivariate.map(|m| (e.term, m))).collect();
    if !mv.is_empty() && !spec.two_way {
        let mut source = Vec::new();
        let mut cols: [Vec<f64>; 6] = Default::default();
        for (t, m) in &mv {
            for (name, v) in [
                ("Pillai's Trace", m.pillai),
                ("Wilks' Lambda", m.wilks),
                ("Hotelling's Trace", m.hotelling),
                ("Roy's Largest Root", m.roy),
            ] {
                source.push(format!("{} ({name})", term_name(*t)));
                for (c, x) in cols.iter_mut().zip([v, m.f, m.df, m.df_error, m.p, f64::NAN]) {
                    c.push(x);
                }
            }
        }
        on_sheet.push(label_column("Multivariate Tests", source));
        for (name, c) in ["Value", "F Value", "DF", "Error DF", "Prob>F"].into_iter().zip(cols) {
            on_sheet.push(number_column(name, "", c));
        }
    }

    // Mauchly's test and the epsilons of the effects with at least two degrees of freedom.
    let sph: Vec<_> = r.effects.iter().filter_map(|e| e.sphericity.map(|s| (e.term, s))).collect();
    if !sph.is_empty() {
        on_sheet.push(label_column("Mauchly's Test of Sphericity", sph.iter().map(|(t, _)| term_name(*t)).collect()));
        let col = |f: &dyn Fn(&calc::RmSphericity) -> f64| sph.iter().map(|(_, s)| f(s)).collect::<Vec<f64>>();
        on_sheet.push(number_column("Mauchly's W", "", col(&|s| s.w)));
        on_sheet.push(number_column("Chi-Square", "", col(&|s| s.chi_sq)));
        on_sheet.push(number_column("DF", "", col(&|s| s.df)));
        on_sheet.push(number_column("Prob>ChiSq", "", col(&|s| s.p)));
        on_sheet.push(number_column("Epsilon (Greenhouse-Geisser)", "", col(&|s| s.greenhouse_geisser)));
        on_sheet.push(number_column("Epsilon (Huynh-Feldt)", "", col(&|s| s.huynh_feldt)));
        on_sheet.push(number_column("Epsilon (Lower-bound)", "", col(&|s| s.lower_bound)));
    }

    // The within-subjects table: each effect and its error term under every way of treating sphericity.
    let mut table = ResultTable::new(&["DF", "Sum of Squares", "Mean Square", "F Value", "Prob>F"]);
    let (mut source, mut df, mut ss, mut ms, mut f, mut p) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut push = |name: String, d: f64, s: f64, m: f64, fv: f64, pv: f64| {
        source.push(name);
        df.push(d);
        ss.push(s);
        ms.push(m);
        f.push(fv);
        p.push(pv);
    };
    let between = |b: &calc::RmBetween| vec![b.df, b.ss, b.mean_sq, b.f, b.p];
    table.push_row("Intercept".into(), between(&r.intercept), vec![String::new(); 5]);
    table.push_row("Subjects".into(), between(&r.subjects), vec![String::new(); 5]);
    for e in &r.effects {
        let name = term_name(e.term);
        for row in &e.rows {
            let way = row.correction.label();
            push(format!("{name} ({way})"), row.df, e.ss, row.mean_sq, row.f, row.p);
            push(format!("Error({name}) ({way})"), row.df_error, e.ss_error, row.mean_sq_error, f64::NAN, f64::NAN);
        }
        let a = &e.rows[0];
        table.push_row(name.clone(), vec![a.df, e.ss, a.mean_sq, a.f, a.p], vec![String::new(); 5]);
        table.push_row(
            format!("Error({name})"),
            vec![a.df_error, e.ss_error, a.mean_sq_error, f64::NAN, f64::NAN],
            vec![String::new(); 5],
        );
    }
    on_sheet.push(label_column("Within Subjects", source));
    for (name, c) in ["DF", "Sum of Squares", "Mean Square", "F Value", "Prob>F"].into_iter().zip([df, ss, ms, f, p]) {
        on_sheet.push(number_column(name, "", c));
    }
    on_sheet.push(label_column("Between Subjects", vec!["Intercept".into(), "Subjects".into()]));
    for (i, name) in ["DF", "Sum of Squares", "Mean Square", "F Value", "Prob>F"].into_iter().enumerate() {
        on_sheet.push(number_column(name, "", table.rows[..2].iter().map(|(_, v)| v[i]).collect()));
    }

    // Descriptive statistics.
    let mut group = Vec::new();
    let mut stats = Vec::new();
    for t in [RmTerm::A, RmTerm::B, RmTerm::Ab] {
        if !spec.descriptive.contains(&t) || r.level_count(t) == 0 || (t == RmTerm::Ab && !spec.interaction) {
            continue;
        }
        for (i, l) in r.level_stats(t, spec.alpha).into_iter().enumerate() {
            group.push(if spec.two_way { format!("{}: {}", term_name(t), level_name(t, i)) } else { level_name(t, i) });
            stats.push(l);
        }
    }
    if !stats.is_empty() {
        on_sheet.push(label_column("Descriptive Statistics", group));
        on_sheet.push(number_column("N", "", vec![r.n as f64; stats.len()]));
        on_sheet.push(number_column("Mean", "", stats.iter().map(|l| l.mean).collect()));
        on_sheet.push(number_column("SE", "", stats.iter().map(|l| l.se).collect()));
        let pct = num(100.0 * (1.0 - spec.alpha));
        on_sheet.push(number_column(&format!("Lower {pct}% CL"), "", stats.iter().map(|l| l.lcl).collect()));
        on_sheet.push(number_column(&format!("Upper {pct}% CL"), "", stats.iter().map(|l| l.ucl).collect()));
    }

    // Verdict lines.
    for e in &r.effects {
        let a = &e.rows[0];
        let mut line = format!(
            "{}: F({}, {}) = {}, p = {} ({}",
            term_name(e.term),
            num(a.df),
            num(a.df_error),
            num(a.f),
            num(a.p),
            if a.p < spec.alpha { "significant" } else { "not significant" }
        );
        match e.sphericity {
            Some(s) if s.p < spec.alpha => line.push_str(&format!(
                "; sphericity is rejected by Mauchly's test (p = {}), see the Greenhouse-Geisser row: p = {}",
                num(s.p),
                num(e.rows[1].p)
            )),
            Some(s) => line.push_str(&format!("; Mauchly's test of sphericity p = {}", num(s.p))),
            None => {}
        }
        line.push(')');
        notes.push(line);
    }
    notes.insert(
        0,
        format!(
            "{} repeated measures ANOVA of {} subjects ({}), at the {} level:",
            if spec.two_way { "Two-way" } else { "One-way" },
            r.n,
            level_counts
                .iter()
                .zip(&names)
                .map(|(n, name)| format!("{n} levels of {name}"))
                .collect::<Vec<_>>()
                .join(", "),
            num(spec.alpha)
        ),
    );
    if spec.two_way && !spec.interaction {
        notes.push(
            "The interaction is not in the model (Origin's default). With the interaction off the error terms of A and B are those of the full design (Sheaf's own rule, not measured in Origin).".to_string(),
        );
    }
    if left_out > 0 {
        notes.push(format!("{left_out} rows with a missing level, subject or number were left out."));
    }
    if dropped_subjects > 0 {
        notes.push(format!(
            "{dropped_subjects} subjects without a number in every cell were left out (Sheaf's own rule: Origin's treatment of missing values was not measured)."
        ));
    }

    // Means comparisons of the one-way design: one block per method, pairs in Origin's order.
    if !spec.two_way {
        for &method in &spec.comparisons {
            let list = r.compare(method, spec.comparison_alpha);
            if list.is_empty() {
                continue;
            }
            let holm = method.is_holm();
            let label: Vec<String> = list
                .iter()
                .map(|c| format!("{} - {}", level_name(RmTerm::A, c.i), level_name(RmTerm::A, c.j)))
                .collect();
            let col = |f: &dyn Fn(&calc::Comparison) -> f64| list.iter().map(f).collect::<Vec<f64>>();
            let stat_name = if method == MeansComparison::Tukey { "q" } else { "t" };
            // Origin marks 1 where significant, 0 for the first failure and -1 for the pairs after it (Holm).
            let mut failed = false;
            let flags: Vec<f64> = list
                .iter()
                .map(|c| {
                    if c.significant {
                        1.0
                    } else if holm && failed {
                        -1.0
                    } else {
                        failed = true;
                        0.0
                    }
                })
                .collect();
            let mut after_fail = false;
            let alphas: Vec<f64> = list
                .iter()
                .zip(&flags)
                .map(|(c, fl)| {
                    if holm && after_fail {
                        0.0
                    } else {
                        after_fail = *fl == 0.0;
                        c.alpha
                    }
                })
                .collect();
            on_sheet.push(label_column(&format!("{} comparison", method.label()), label));
            on_sheet.push(number_column("Difference", "", col(&|c| c.diff)));
            on_sheet.push(number_column("SE", "", col(&|c| c.se)));
            on_sheet.push(number_column("DF", "", vec![r.effects[0].rows[0].df_error; list.len()]));
            on_sheet.push(number_column(&format!("Statistic ({stat_name})"), "", col(&|c| c.stat)));
            on_sheet.push(number_column("Prob", "", col(&|c| c.p)));
            on_sheet.push(number_column("Alpha", "", alphas));
            on_sheet.push(number_column("Significant", "", flags));
            if !holm {
                on_sheet.push(number_column("LCL", "", col(&|c| c.lcl)));
                on_sheet.push(number_column("UCL", "", col(&|c| c.ucl)));
            }
            notes.push(format!(
                "{} means comparison at the {} level: {} of {} pairs significant.",
                method.label(),
                num(spec.comparison_alpha),
                list.iter().filter(|c| c.significant).count(),
                list.len()
            ));
        }
    }
    Ok(Output { table: Some(table), on_sheet, notes, plots: Vec::new(), comparison_pairs: Vec::new() })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The one-way run of Origin: 8 subjects by 4 levels, laid out as indexed rows (level, data, subject).
    const ROWS: [[f64; 4]; 8] = [
        [22.5244, 23.5244, 24.5244, 25.5244],
        [18.138, 19.677, 22.5219, 24.6265],
        [17.0564, 21.0059, 19.8137, 22.3672],
        [23.5919, 22.9043, 26.4075, 24.4996],
        [19.4001, 19.1614, 18.8032, 21.3784],
        [20.3396, 22.2064, 23.9391, 25.4933],
        [21.2722, 21.8676, 23.4055, 25.5475],
        [15.6833, 19.3813, 19.8708, 19.163],
    ];

    fn project(two_way: bool) -> (Project, Vec<ColumnId>) {
        let mut p = Project::new();
        let book = p.add_workbook("rm");
        let (mut a, mut b, mut y, mut s) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        for (subject, row) in ROWS.iter().enumerate() {
            for (level, v) in row.iter().enumerate() {
                if two_way {
                    // 2 x 2: levels (0,1) x (0,1) from the four numbers; shuffled order within a subject.
                    a.push(format!("a{}", level / 2));
                    b.push(format!("b{}", level % 2));
                } else {
                    a.push(format!("L{level}"));
                }
                y.push(*v);
                s.push(format!("S{subject}"));
            }
        }
        let text = |v: Vec<String>| ColumnData::text(v);
        let mut ids = vec![p.add_column(book, 0, Designation::Y, text(a)).unwrap()];
        if two_way {
            ids.push(p.add_column(book, 0, Designation::Y, text(b)).unwrap());
        }
        ids.push(p.add_column(book, 0, Designation::Y, ColumnData::numeric(y)).unwrap());
        ids.push(p.add_column(book, 0, Designation::Y, text(s)).unwrap());
        (p, ids)
    }

    fn run(p: &mut Project, ids: &[ColumnId], spec: RmAnovaSpec) -> Result<Report, AnalysisError> {
        p.analyze(&Request { columns: ids.to_vec(), rows: RowRange::All, tool: Tool::RmAnova(spec) })
    }

    #[test]
    fn one_way_gives_origin_s_within_and_between_tables() {
        let (mut p, ids) = project(false);
        let r = run(&mut p, &ids, RmAnovaSpec::new(false)).unwrap();
        assert!(
            r.message.contains("One-way repeated measures ANOVA") && r.message.contains("ANOVAOneWayRM1"),
            "{}",
            r.message
        );
        let ws = &p.workbook(r.book).unwrap().sheets[r.new_sheet.unwrap()];
        let col = |name: &str| ws.columns.iter().find(|c| c.long_name == name).unwrap().numeric().unwrap().to_vec();
        // The columns of the within-subjects block follow its label column.
        let start = ws.columns.iter().position(|c| c.long_name == "Within Subjects").unwrap();
        let within =
            |name: &str| ws.columns[start..].iter().find(|c| c.long_name == name).unwrap().numeric().unwrap().to_vec();
        let ss = within("Sum of Squares");
        // The within block comes first: factor and error under the four ways, then the between block.
        assert!((ss[0] - 64.39032).abs() < 1e-5 && (ss[1] - 28.76007).abs() < 1e-5, "{ss:?}");
        let df = within("DF");
        let want = [3.0, 21.0, 2.89097, 20.23682, 3.0, 21.0, 1.0, 7.0];
        assert!(df[..8].iter().zip(want).all(|(g, w)| (g - w).abs() < 1e-5), "{df:?}");
        let f = within("F Value");
        assert!((f[0] - 15.67215).abs() < 1e-4);
        let names: Vec<&str> = ws.columns.iter().map(|c| c.long_name.as_str()).collect();
        assert!(names.contains(&"Multivariate Tests") && names.contains(&"Mauchly's Test of Sphericity"));
        assert!(names.contains(&"Descriptive Statistics") && names.contains(&"Tukey comparison"));
        let tukey = col("Statistic (q)");
        assert!((tukey[0] - 3.54149).abs() < 1e-4 && tukey.len() == 6);
        let t = r.table.unwrap();
        assert_eq!(t.rows[0].0, "Intercept");
        assert!((t.rows[0].1[3] - 804.75036).abs() < 1e-3);
    }

    #[test]
    fn two_way_has_one_error_term_per_effect_and_comparisons_are_refused() {
        let (mut p, ids) = project(true);
        let mut spec = RmAnovaSpec::new(true);
        spec.interaction = true;
        let r = run(&mut p, &ids, spec.clone()).unwrap();
        assert!(r.message.contains("ANOVATwoWayRM1"));
        let t = r.table.unwrap();
        let names: Vec<&str> = t.rows.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names.len(), 2 + 2 * 3, "{names:?}");
        assert!(names.iter().any(|n| n.starts_with("Error(") && n.contains('*')));
        spec.comparisons = vec![MeansComparison::Tukey];
        assert!(run(&mut p, &ids, spec).is_err());
        // Interactions off: no A*B rows.
        let r = run(&mut p, &ids, RmAnovaSpec::new(true)).unwrap();
        assert_eq!(r.table.unwrap().rows.len(), 2 + 2 * 2);
    }

    #[test]
    fn bad_input_is_reported() {
        let (mut p, ids) = project(false);
        assert!(run(&mut p, &ids[..2], RmAnovaSpec::new(false)).is_err());
        let mut dun = RmAnovaSpec::new(false);
        dun.comparisons = vec![MeansComparison::Dunnett];
        assert!(run(&mut p, &ids, dun).is_err());
        let mut inter = RmAnovaSpec::new(false);
        inter.interaction = true;
        assert!(run(&mut p, &ids, inter).is_err());
        // Data must be numeric.
        let swapped = [ids[0], ids[2], ids[1]];
        assert!(run(&mut p, &swapped, RmAnovaSpec::new(false)).is_err());
        // A repeated number in one cell.
        let book = p.add_workbook("dup");
        let a = p
            .add_column(book, 0, Designation::Y, ColumnData::text(vec!["x".into(), "x".into(), "y".into(), "y".into()]))
            .unwrap();
        let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0, 3.0, 4.0])).unwrap();
        let s = p.add_column(book, 0, Designation::Y, ColumnData::text(vec!["s".into(); 4])).unwrap();
        assert!(run(&mut p, &[a, y, s], RmAnovaSpec::new(false)).is_err());
        let rows = Request {
            columns: ids.clone(),
            rows: RowRange::XBetween(0.0, 1.0),
            tool: Tool::RmAnova(RmAnovaSpec::new(false)),
        };
        assert!(p.analyze(&rows).is_err());
    }

    #[test]
    fn subjects_missing_a_cell_are_left_out_with_a_note() {
        let (mut p, ids) = project(false);
        // Row range that cuts the last subject short: 3 numbers of 4.
        let req =
            Request { columns: ids.clone(), rows: RowRange::Rows(1, 31), tool: Tool::RmAnova(RmAnovaSpec::new(false)) };
        let r = p.analyze(&req).unwrap();
        assert!(r.notes.iter().any(|n| n.contains("1 subjects without a number")), "{:?}", r.notes);
    }
}
