//! Tools > Protection > Audit Log (T131, gui-look "Audit Log").

use super::*;

#[test]
fn turning_the_audit_log_on_records_the_following_changes() {
    let mut h = start(vec![data_file("audit-log")]);
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::AuditLog, &ctx);
    settle(&mut h);
    assert!(h.query_by_label_contains("The audit log is off").is_some());
    h.get_by_label("Record the changes made to this project").click();
    settle(&mut h);
    assert!(project(&h).audit.is_some());
    let book = h.state().books.keys().next().copied().unwrap();
    h.state_mut().doc.apply("Rename workbook", |p| {
        p.workbook_mut(book).unwrap().long_name = "renamed".into();
    });
    settle(&mut h);
    let entries = &project(&h).audit.as_ref().unwrap().entries;
    let changes: Vec<&str> = entries.iter().map(|e| e.change.as_str()).collect();
    assert_eq!(changes, ["Audit log started", "Rename workbook"]);
    assert!(h.query_by_label("Rename workbook").is_some(), "the window lists it");
}
