use sheaf_core::sheetops::SortKey;
use sheaf_core::{ColumnData, ColumnId, Designation, Document, Project, WindowId};

fn sample() -> (Document, WindowId, Vec<ColumnId>) {
    let mut p = Project::new();
    let book = p.add_workbook("");
    let mut ids = Vec::new();
    for values in [vec![9.0, 3.0, 1.0, 2.0, 8.0], vec![90.0, 10.0, 30.0, 20.0, 80.0]] {
        ids.push(p.add_column(book, 0, Designation::Y, ColumnData::numeric(values)).unwrap());
    }
    ids.push(
        p.add_column(
            book,
            0,
            Designation::Label,
            ColumnData::text(["top", "three", "one", "two", "bottom"].map(str::to_string).to_vec()),
        )
        .unwrap(),
    );
    ids.push(p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![5.0, 4.0, 3.0, 2.0, 1.0])).unwrap());
    p.column_mut(ids[0]).unwrap().mask.extend([1, 4]);
    (Document::new(p), book, ids)
}

fn values(d: &Document, c: ColumnId) -> Vec<f64> {
    d.project().numeric(c).unwrap().to_vec()
}

#[test]
fn selected_range_sort_keeps_outside_rows_columns_and_undo_intact() {
    let (mut d, book, ids) = sample();
    d.edit("Sort selected rows", |p| p.sort_range_in_rows(book, 0, ids[0], &ids[..3], false, 1..4)).unwrap();
    assert_eq!(values(&d, ids[0]), [9.0, 1.0, 2.0, 3.0, 8.0]);
    assert_eq!(values(&d, ids[1]), [90.0, 30.0, 20.0, 10.0, 80.0]);
    assert_eq!(d.project().column(ids[2]).unwrap().text().unwrap(), ["top", "one", "two", "three", "bottom"]);
    assert_eq!(values(&d, ids[3]), [5.0, 4.0, 3.0, 2.0, 1.0]);
    assert_eq!(d.project().column(ids[0]).unwrap().mask.iter().copied().collect::<Vec<_>>(), [3, 4]);
    d.undo();
    assert_eq!(values(&d, ids[0]), [9.0, 3.0, 1.0, 2.0, 8.0]);
    assert_eq!(d.project().column(ids[0]).unwrap().mask.iter().copied().collect::<Vec<_>>(), [1, 4]);
    d.redo();
    assert_eq!(values(&d, ids[1]), [90.0, 30.0, 20.0, 10.0, 80.0]);
}

#[test]
fn independent_column_sort_only_reorders_the_selected_rows() {
    let (mut d, book, ids) = sample();
    d.edit("Sort columns", |p| p.sort_columns_in_rows(book, 0, &ids[..2], false, 1..4)).unwrap();
    assert_eq!(values(&d, ids[0]), [9.0, 1.0, 2.0, 3.0, 8.0]);
    assert_eq!(values(&d, ids[1]), [90.0, 10.0, 20.0, 30.0, 80.0]);
    assert_eq!(d.project().column(ids[2]).unwrap().text().unwrap(), ["top", "three", "one", "two", "bottom"]);
}

#[test]
fn nested_sort_breaks_ties_only_inside_the_selected_rows() {
    let (mut d, book, ids) = sample();
    d.apply("Keys", |p| p.column_mut(ids[0]).unwrap().data = ColumnData::numeric(vec![9.0, 1.0, 1.0, 2.0, 8.0]));
    let keys = [SortKey { column: ids[0], descending: false }, SortKey { column: ids[1], descending: true }];
    d.edit("Sort nested", |p| p.sort_nested_in_rows(book, 0, &keys, &ids[..3], 1..4)).unwrap();
    assert_eq!(values(&d, ids[0]), [9.0, 1.0, 1.0, 2.0, 8.0]);
    assert_eq!(values(&d, ids[1]), [90.0, 30.0, 10.0, 20.0, 80.0]);
    assert_eq!(d.project().column(ids[2]).unwrap().text().unwrap(), ["top", "one", "three", "two", "bottom"]);
    let before = values(&d, ids[1]);
    d.edit("Outside range", |p| p.sort_nested_in_rows(book, 0, &keys, &ids[..3], 10..usize::MAX)).unwrap();
    assert_eq!(values(&d, ids[1]), before);
}
