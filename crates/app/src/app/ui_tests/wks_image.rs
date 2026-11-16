//! File > Export > Worksheet as Image... (T97, workbook-model.md 13k).

use super::*;

#[test]
fn the_menu_item_is_enabled_and_the_picture_is_written_by_its_extension() {
    let mut h = start(vec![data_file("wks-image")]);
    show_window(&mut h, "Book1 - sample.csv");
    let book = h.state().books.keys().next().copied().unwrap();
    let menu = h.state().menu_context();
    assert!(matches!(
        crate::menus::resolve(&crate::menus::Cmd::ExportWorksheetImage, &menu),
        Ok(Action::ExportWorksheetImage(b)) if b == book
    ));
    let dir = std::env::temp_dir().join(format!("sheaf-wks-image-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let project = h.state().doc.project().clone();
    let sheet = project.workbook(book).unwrap().sheet();
    let png = crate::app::write_worksheet_image(sheet, &dir.join("one")).unwrap();
    assert_eq!(png.extension().unwrap(), "png", "no extension: PNG");
    assert_eq!(&std::fs::read(&png).unwrap()[1..4], b"PNG");
    let pdf = crate::app::write_worksheet_image(sheet, &dir.join("two.PDF")).unwrap();
    assert!(std::fs::read(pdf).unwrap().starts_with(b"%PDF"));
    let svg = crate::app::write_worksheet_image(sheet, &dir.join("three.svg")).unwrap();
    assert!(std::fs::read_to_string(svg).unwrap().contains(">2.5<"));
    assert!(crate::app::write_worksheet_image(sheet, &dir.join("four.bmp")).is_err());
    let _ = std::fs::remove_dir_all(&dir);
}
