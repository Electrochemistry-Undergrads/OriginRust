use sheaf_core::worksheet::Worksheet;
use sheaf_core::{ColumnData, ColumnId, Designation, Document, FormulaState, Project, RecalcMode, WindowId};

fn col(project: &mut Project, book: WindowId, sheet: usize, values: &[f64]) -> ColumnId {
    project.add_column(book, sheet, Designation::Y, ColumnData::numeric(values.to_vec())).unwrap()
}

fn book(project: &mut Project, name: &str) -> WindowId {
    let id = project.add_workbook("");
    project.workbook_mut(id).unwrap().name = name.into();
    id
}

fn fixture() -> (Project, WindowId, ColumnId, ColumnId, WindowId, ColumnId) {
    let mut project = Project::new();
    let src = book(&mut project, "Src");
    let a = col(&mut project, src, 0, &[2.0, 4.0, 8.0]);
    project.workbook_mut(src).unwrap().sheets.push(Worksheet::new("Other"));
    let other = col(&mut project, src, 1, &[10.0, 20.0, 30.0, 40.0]);
    let dst = book(&mut project, "Dst");
    let target = col(&mut project, dst, 0, &[]);
    (project, src, a, other, dst, target)
}

#[test]
fn cross_sheet_origin_range_fixtures_fill_an_empty_target_and_resolve_names_and_positions() {
    // Origin 2016 range variables, scratch/formula-cross-sheet/ranges.lines.txt.
    let (mut p, src, a, _, _, target) = fixture();
    p.set_column_values_with(target, "[sRc]sHeEt1!col(A)*3", None, RecalcMode::Auto).unwrap();
    assert_eq!(p.numeric(target).unwrap(), [6.0, 12.0, 24.0]);
    p.set_column_values(target, "[Src]2!wcol(1)[2] + mean([Src]Other!col(A))", None).unwrap();
    assert_eq!(p.numeric(target).unwrap(), [45.0; 4]);
    p.set_column_values(a, "Other!col(A)+1", None).unwrap();
    assert_eq!(p.numeric(a).unwrap(), [11.0, 21.0, 31.0, 41.0]);
    p.workbook_mut(src).unwrap().sheets[1].name = "Raw Data".into();
    p.column_mut(a).unwrap().long_name = "Current µA".into();
    p.set_column_values(target, "[Src]Sheet1!col(\"Current µA\") + [Src]\"Raw Data\"!col(A)", None).unwrap();
    assert_eq!(p.numeric(target).unwrap(), [21.0, 41.0, 61.0, 81.0]);
}

#[test]
fn reversed_book_order_recalculates_a_cross_book_chain_in_one_undo_step() {
    let mut p = Project::new();
    let dst = book(&mut p, "Dst");
    let result = col(&mut p, dst, 0, &[]);
    let local = col(&mut p, dst, 0, &[]);
    let mid = book(&mut p, "Mid");
    let middle = col(&mut p, mid, 0, &[]);
    let src = book(&mut p, "Src");
    let source = col(&mut p, src, 0, &[1.0, 2.0, 3.0]);
    p.set_column_values_with(middle, "[Src]Sheet1!col(A)*2", None, RecalcMode::Auto).unwrap();
    p.set_column_values_with(result, "[Mid]Sheet1!col(A)+1", None, RecalcMode::Auto).unwrap();
    p.set_column_values_with(local, "col(A)*3", None, RecalcMode::Auto).unwrap();
    let mut doc = Document::new(p);
    doc.apply("Change remote source", |p| p.column_mut(source).unwrap().data = ColumnData::numeric(vec![3.0, 5.0]));
    assert_eq!(doc.project().numeric(middle).unwrap(), [6.0, 10.0]);
    assert_eq!(doc.project().numeric(result).unwrap(), [7.0, 11.0]);
    assert_eq!(doc.project().numeric(local).unwrap(), [21.0, 33.0]);
    doc.undo();
    assert_eq!(doc.project().numeric(local).unwrap(), [9.0, 15.0, 21.0]);
    doc.redo();
    assert_eq!(doc.project().numeric(local).unwrap(), [21.0, 33.0]);
}

#[test]
fn manual_remote_formulas_hold_their_dependents_until_recalculated() {
    let (mut p, src, a, _, _, target) = fixture();
    let middle = col(&mut p, src, 1, &[]);
    p.set_column_values_with(middle, "Sheet1!col(A)*2", None, RecalcMode::Manual).unwrap();
    p.set_column_values_with(target, "[Src]Other!col(B)+1", None, RecalcMode::Auto).unwrap();
    let mut doc = Document::new(p);
    doc.apply("Change source", |p| p.column_mut(a).unwrap().data = ColumnData::numeric(vec![3.0, 5.0, 7.0]));
    assert_eq!(doc.project().column(middle).unwrap().formula_state, FormulaState::Pending);
    assert_eq!(doc.project().numeric(target).unwrap()[..3], [5.0, 9.0, 17.0]);
    doc.apply("Recalculate pending source sheet", |p| assert!(p.recalculate_pending(src, 1).is_empty()));
    assert_eq!(doc.project().numeric(target).unwrap(), [7.0, 11.0, 15.0]);
    doc.undo();
    assert_eq!(doc.project().column(middle).unwrap().formula_state, FormulaState::Pending);
    doc.apply("Recalculate column", |p| p.recalculate(middle).unwrap());
    assert_eq!(doc.project().numeric(target).unwrap(), [7.0, 11.0, 15.0]);
}

#[test]
fn cross_book_cycles_are_refused_without_mutation_and_scoped_self_reads_run_once() {
    let (mut p, _, a, _, _, target) = fixture();
    p.set_column_values_with(target, "[Src]Sheet1!col(A)*2", None, RecalcMode::Auto).unwrap();
    let before = p.clone();
    let error = p.set_column_values_with(a, "[Dst]Sheet1!col(A)+1", None, RecalcMode::Auto).unwrap_err();
    assert!(error.to_string().contains("depend on itself"));
    assert!(error.to_string().contains("[Src]Sheet1!col(A)"));
    assert!(error.to_string().contains("[Dst]Sheet1!col(A)"));
    assert_eq!(p, before);
    p.set_column_values_with(a, "[Src]Sheet1!col(A)*3", None, RecalcMode::Auto).unwrap();
    assert_eq!(p.column(a).unwrap().recalc, RecalcMode::None);
    assert_eq!(p.numeric(target).unwrap(), [12.0, 24.0, 48.0]);
}

#[test]
fn remote_text_headers_and_parameter_row_alias_changes_invalidate_the_right_readers() {
    let (mut p, src, a, _, dst, target) = fixture();
    p.column_mut(a).unwrap().params.insert("Scale".into(), "2.5".into());
    p.column_mut(a).unwrap().units = "mV".into();
    let sibling = col(&mut p, src, 0, &[]);
    let units = col(&mut p, dst, 0, &[]);
    let alias = col(&mut p, dst, 0, &[]);
    p.set_column_values_with(target, "value([Src]Sheet1!col(A)[Scale]$)*[Src]Sheet1!col(A)", None, RecalcMode::Auto)
        .unwrap();
    p.set_column_values_with(units, "[Src]Sheet1!col(A)[U]$", None, RecalcMode::Auto).unwrap();
    p.set_column_values_with(alias, "value([Src]Sheet1!col(A)[D1]$)", None, RecalcMode::Auto).unwrap();
    let mut doc = Document::new(p);
    assert_eq!(doc.project().numeric(target).unwrap(), [5.0, 10.0, 20.0]);
    assert_eq!(doc.project().column(units).unwrap().data.text_cell(0), Some("mV"));
    assert_eq!(doc.project().numeric(alias).unwrap(), [2.5; 3]);
    doc.apply("Add earlier source parameter row", |p| {
        p.column_mut(sibling).unwrap().params.insert("AAA".into(), "9".into());
    });
    assert_eq!(doc.project().numeric(alias).unwrap(), [0.0; 3]);
    assert_eq!(doc.project().numeric(target).unwrap(), [5.0, 10.0, 20.0]);
    doc.apply("Edit remote metadata", |p| {
        let column = p.column_mut(a).unwrap();
        column.params.insert("Scale".into(), "4".into());
        column.units = "V".into();
    });
    assert_eq!(doc.project().numeric(target).unwrap(), [8.0, 16.0, 32.0]);
    assert_eq!(doc.project().column(units).unwrap().data.text_cell(0), Some("V"));
    p = doc.project().clone();
    p.column_mut(a).unwrap().data = ColumnData::text(vec!["red".into(), "green".into()]);
    p.set_column_values(target, "[Src]Sheet1!col(A)$", Some(0..2)).unwrap();
    assert_eq!(p.column(target).unwrap().data.text_cell(1), Some("green"));
}

#[test]
fn removing_or_renaming_a_source_breaks_the_link_and_undo_restores_it() {
    let (mut p, src, _, _, _, target) = fixture();
    let error = p.set_column_values(target, "[Missing]Sheet1!col(A)", None).unwrap_err();
    assert!(error.to_string().contains("unknown column [Missing]Sheet1!col(A)"));
    p.set_column_values_with(target, "[Src]Sheet1!col(A)*2", None, RecalcMode::Auto).unwrap();
    let mut doc = Document::new(p);
    doc.apply("Rename source sheet", |p| p.workbook_mut(src).unwrap().sheets[0].name = "Renamed".into());
    assert_eq!(doc.project().column(target).unwrap().formula_state, FormulaState::Broken);
    assert_eq!(doc.project().numeric(target).unwrap(), [4.0, 8.0, 16.0]);
    doc.undo();
    assert_eq!(doc.project().column(target).unwrap().formula_state, FormulaState::Current);
    doc.apply("Delete source book", |p| p.windows.retain(|w| w.id() != src));
    assert_eq!(doc.project().column(target).unwrap().formula_state, FormulaState::Broken);
    doc.undo();
    assert_eq!(doc.project().column(target).unwrap().formula_state, FormulaState::Current);
    let mut project = doc.project().clone();
    let before = project.clone();
    assert!(project.set_column_values(target, "[Missing]Sheet1!col(A)", None).is_err());
    assert_eq!(project, before);
}

#[test]
fn positional_references_rebind_when_unchanged_source_sheets_move() {
    let (mut p, src, _, _, _, target) = fixture();
    p.set_column_values_with(target, "[Src]1!col(A)*2", None, RecalcMode::Auto).unwrap();
    let mut doc = Document::new(p);
    doc.apply("Swap source sheets", |p| p.workbook_mut(src).unwrap().sheets.swap(0, 1));
    assert_eq!(doc.project().numeric(target).unwrap(), [20.0, 40.0, 60.0, 80.0]);
    doc.undo();
    assert_eq!(doc.project().numeric(target).unwrap(), [4.0, 8.0, 16.0]);
}

#[test]
fn explicit_row_ranges_and_refresh_formulas_follow_remote_source_changes() {
    let (mut p, _, a, _, _, target) = fixture();
    p.column_mut(target).unwrap().data = ColumnData::numeric(vec![9.0; 5]);
    p.set_column_values_with(target, "[Src]Sheet1!col(A)*2", Some(1..3), RecalcMode::Auto).unwrap();
    assert_eq!(p.numeric(target).unwrap(), [9.0, 8.0, 16.0, 9.0, 9.0]);
    p.column_mut(a).unwrap().data = ColumnData::numeric(vec![1.0, 3.0, 5.0]);
    assert!(p.refresh_formulas(&[a]).is_empty());
    assert_eq!(p.numeric(target).unwrap(), [9.0, 6.0, 10.0, 9.0, 9.0]);
}

#[test]
fn an_unrelated_book_does_not_invalidate_manual_cross_sheet_formulas() {
    let (mut p, _, _, _, _, target) = fixture();
    p.set_column_values_with(target, "[Src]Sheet1!col(A)*2", None, RecalcMode::Manual).unwrap();
    let mut doc = Document::new(p);
    doc.apply("Add unrelated book", |p| {
        book(p, "Unrelated");
    });
    assert_eq!(doc.project().column(target).unwrap().formula_state, FormulaState::Current);
    assert_eq!(doc.project().numeric(target).unwrap(), [4.0, 8.0, 16.0]);
}

#[test]
fn stored_cross_book_cycles_are_marked_broken_without_evaluating_stale_values() {
    let (mut p, _, a, _, _, target) = fixture();
    p.column_mut(target).unwrap().data = ColumnData::numeric(vec![90.0; 3]);
    for (id, formula) in [(a, "[Dst]Sheet1!col(A)+1"), (target, "[Src]Sheet1!col(A)+1")] {
        let c = p.column_mut(id).unwrap();
        c.formula = Some(formula.into());
        c.recalc = RecalcMode::Auto;
    }
    assert_eq!(p.refresh_formulas(&[a]).len(), 2);
    assert_eq!(p.column(a).unwrap().formula_state, FormulaState::Broken);
    assert_eq!(p.column(target).unwrap().formula_state, FormulaState::Broken);
    assert_eq!(p.numeric(a).unwrap(), [2.0, 4.0, 8.0]);
    assert_eq!(p.numeric(target).unwrap(), [90.0; 3]);
}

#[test]
fn renaming_a_source_rebinds_an_unchanged_column_with_the_same_long_name() {
    let (mut p, src, a, _, _, target) = fixture();
    let b = col(&mut p, src, 0, &[9.0, 7.0, 5.0]);
    p.column_mut(a).unwrap().long_name = "Signal".into();
    p.column_mut(b).unwrap().long_name = "Signal".into();
    p.set_column_values_with(target, "[Src]Sheet1!col(\"Signal\")", None, RecalcMode::Auto).unwrap();
    let mut doc = Document::new(p);
    doc.apply("Rename first matching source", |p| p.column_mut(a).unwrap().long_name = "Other".into());
    assert_eq!(doc.project().numeric(target).unwrap(), [9.0, 7.0, 5.0]);
    doc.undo();
    assert_eq!(doc.project().numeric(target).unwrap(), [2.0, 4.0, 8.0]);
}

#[test]
fn book_scopes_prefer_short_names_accept_unique_titles_and_refuse_ambiguous_titles() {
    let (mut p, src, _, _, _, target) = fixture();
    let extra = book(&mut p, "Extra");
    col(&mut p, extra, 0, &[99.0, 99.0, 99.0]);
    p.workbook_mut(extra).unwrap().long_name = "Src".into();
    p.set_column_values(target, "[Src]Sheet1!col(A)", None).unwrap();
    assert_eq!(p.numeric(target).unwrap(), [2.0, 4.0, 8.0]);
    p.workbook_mut(src).unwrap().long_name = "Source Data".into();
    p.set_column_values_with(target, "[\"source data\"]Sheet1!col(A)*2", None, RecalcMode::Auto).unwrap();
    let mut doc = Document::new(p);
    assert_eq!(doc.project().numeric(target).unwrap(), [4.0, 8.0, 16.0]);
    doc.apply("Make book title ambiguous", |p| p.workbook_mut(extra).unwrap().long_name = "Source Data".into());
    assert_eq!(doc.project().column(target).unwrap().formula_state, FormulaState::Broken);
    assert_eq!(doc.project().numeric(target).unwrap(), [4.0, 8.0, 16.0]);
    doc.undo();
    assert_eq!(doc.project().column(target).unwrap().formula_state, FormulaState::Current);
}
