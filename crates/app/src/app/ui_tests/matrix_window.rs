//! File > New > Matrix and the matrix window (T192 slice 2).

use super::*;

#[test]
fn a_matrix_is_made_shown_and_a_typed_cell_is_one_undo_step() {
    // An empty project, so that no other window lies over the new one.
    let mut h = start(Vec::new());
    menu_path(&mut h, &["File", "New", "Matrix..."]);
    let books: Vec<_> = project(&h).matrix_books().map(|m| (m.id, m.name.clone())).collect();
    assert_eq!(books.len(), 1);
    let (id, name) = books[0].clone();
    assert_eq!(name, "MBook1");
    assert_eq!(h.state().doc.undo_label(), Some("New matrix"));
    let sheet = |h: &Harness<'static, SheafApp>| project(h).matrix_sheet(id, 0).unwrap().clone();
    assert_eq!((sheet(&h).rows, sheet(&h).cols), (32, 32), "Origin's default size");
    settle(&mut h);
    // The grid shows the column numbers and the missing values.
    assert!(has_label(&h, "32") && has_label(&h, "--"), "column numbers and empty cells are drawn");
    // A double click on the first cell edits it; Enter writes it.
    let cell = h.get_all_by_label("--").next().expect("a cell").rect().center();
    double_click_at(&mut h, cell);
    assert_eq!(h.state().matrices.get(&id).and_then(|m| m.editing_cell()), Some((0, 0, 0)), "the first cell is edited");
    h.event(egui::Event::Text("2.5".into()));
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert_eq!(sheet(&h).value(0, 0), 2.5);
    assert_eq!(h.state().doc.undo_label(), Some("Edit matrix cell"));
    assert!(has_label(&h, "2.5"));
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    assert!(sheet(&h).value(0, 0).is_nan(), "one undo step");
    // Text that is not a number changes nothing.
    h.state_mut().apply(Action::SetMatrixCell { id, sheet: 0, row: 1, col: 1, value: Some(-4.0) }, &ctx);
    settle(&mut h);
    assert_eq!(sheet(&h).value(1, 1), -4.0);
    // The Standard toolbar's New Matrix makes another book.
    let action = menus::resolve(&menus::Cmd::NewMatrix, &h.state().menu_context()).unwrap();
    h.state_mut().apply(action, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).matrix_books().count(), 2);
}

#[test]
fn the_matrix_menu_transforms_sets_values_converts_and_plots_the_active_sheet() {
    let mut h = start(Vec::new());
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::NewMatrix, &ctx);
    settle(&mut h);
    let id = project(&h).matrix_books().next().unwrap().id;
    let sheet = |h: &Harness<'static, SheafApp>| project(h).matrix_sheet(id, 0).unwrap().clone();
    assert_eq!(h.state().menu_context().kind, menus::WindowKind::Matrix, "the new matrix window is active");
    // Set Matrix Values... with a formula in i and j.
    menu_path(&mut h, &["Matrix", "Set Matrix Values..."]);
    h.query_all_by_role(egui::accesskit::Role::TextInput).last().expect("the formula box").focus();
    settle(&mut h);
    h.event(egui::Event::Text("i*100+j".into()));
    settle(&mut h);
    click(&mut h, "OK");
    assert_eq!(sheet(&h).value(0, 1), 102.0, "row 1, column 2");
    assert_eq!(h.state().doc.undo_label(), Some("Set Matrix Values"));
    // Transpose swaps rows and columns, one undo step.
    menu_path(&mut h, &["Matrix", "Transpose"]);
    assert_eq!(sheet(&h).value(1, 0), 102.0);
    assert_eq!(h.state().doc.undo_label(), Some("Transpose Matrix"));
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    assert_eq!(sheet(&h).value(0, 1), 102.0);
    // Flip Horizontal mirrors the columns.
    menu_path(&mut h, &["Matrix", "Flip", "Horizontal"]);
    assert_eq!(sheet(&h).value(0, 31), 101.0);
    // Convert to Worksheet makes a workbook of the cells.
    let books = project(&h).workbooks().count();
    let action = menus::resolve(&menus::Cmd::Matrix(menus::MatrixCmd::ToWorksheet), &h.state().menu_context()).unwrap();
    h.state_mut().apply(action, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).workbooks().count(), books + 1);
    // Plot > Contour/Heat Map > Heat Map of the active matrix.
    show_window(&mut h, "MBook1");
    let graphs = project(&h).graphs().count();
    let action =
        menus::resolve(&menus::Cmd::PlotField(sheaf_core::template::FieldTemplate::HeatMap), &h.state().menu_context())
            .unwrap();
    h.state_mut().apply(action, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).graphs().count(), graphs + 1);
    assert!(project(&h).graphs().last().unwrap().layers[0].field.is_some(), "a heat map layer");
}

#[test]
fn worksheet_convert_to_matrix_direct_and_xyz_gridding_make_matrix_books() {
    // The Worksheet menu has both items.
    let mut found = Vec::new();
    for m in menus::bar(menus::WindowKind::Workbook) {
        fn walk(entries: &[menus::Entry], found: &mut Vec<menus::Cmd>) {
            for e in entries {
                match e {
                    menus::Entry::Item(i) => found.extend(i.cmd.clone()),
                    menus::Entry::Submenu(s) => walk(&s.entries, found),
                    _ => {}
                }
            }
        }
        walk(&m.entries, &mut found);
    }
    for xyz in [false, true] {
        assert!(found.contains(&menus::Cmd::ConvertToMatrix { xyz }), "Convert to Matrix xyz={xyz}");
    }
    let mut h = start(Vec::new());
    let ctx = h.ctx.clone();
    h.state_mut().doc.apply("Data", |p| {
        let book = p.add_workbook("grid");
        // A 3 x 2 grid of XYZ points, and two plain columns.
        let xs = vec![1.0, 2.0, 3.0, 1.0, 2.0, 3.0];
        let ys = vec![10.0, 10.0, 10.0, 20.0, 20.0, 20.0];
        let zs = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        for (d, v) in [(Designation::X, xs), (Designation::Y, ys), (Designation::Z, zs)] {
            p.add_column(book, 0, d, ColumnData::numeric(v)).unwrap();
        }
    });
    settle(&mut h);
    let book = project(&h).workbooks().next().unwrap().id;
    let cols: Vec<sheaf_core::ColumnId> =
        project(&h).workbook(book).unwrap().sheet().columns.iter().map(|c| c.id).collect();
    h.state_mut().apply(Action::ConvertToMatrix { book, columns: cols.clone(), xyz: true }, &ctx);
    settle(&mut h);
    assert_eq!(h.state().doc.undo_label(), Some("Convert to Matrix"));
    let m = project(&h).matrix_books().next().expect("a matrix book").sheets[0].clone();
    assert_eq!((m.cols, m.rows), (3, 2), "X across the columns, Y down the rows");
    assert_eq!((m.x_from, m.x_to, m.y_from, m.y_to), (1.0, 3.0, 10.0, 20.0));
    assert_eq!(m.value(1, 2), 6.0);
    // Direct: the two first columns become two matrix columns of six rows.
    h.state_mut().apply(Action::ConvertToMatrix { book, columns: cols[..2].to_vec(), xyz: false }, &ctx);
    settle(&mut h);
    let m = project(&h).matrix_books().nth(1).expect("a second matrix book").sheets[0].clone();
    assert_eq!((m.cols, m.rows), (2, 6));
    assert_eq!(m.value(5, 1), 20.0);
}

#[test]
fn analysis_mathematics_inverse_writes_the_inverse_to_a_new_matrix_book() {
    use sheaf_core::matrix_project::MatrixOp;
    let mut h = start(Vec::new());
    let ctx = h.ctx.clone();
    let mut id = sheaf_core::WindowId(0);
    h.state_mut().doc.apply("Matrix", |p| {
        id = p.add_matrix_book("");
        p.apply_matrix_op(id, 0, &MatrixOp::SetDimensions { rows: 2, cols: 2, ranges: None }).unwrap();
        p.apply_matrix_op(id, 0, &MatrixOp::SetValues("i==j ? 2 : 1".into())).unwrap();
    });
    settle(&mut h);
    // The menu item is in the matrix bar's Analysis menu.
    let analysis = menus::bar(menus::WindowKind::Matrix).iter().find(|m| m.label == "&Analysis").expect("Analysis");
    let has_inverse = analysis.entries.iter().any(|e| match e {
        menus::Entry::Submenu(s) => s.entries.iter().any(
            |e| matches!(e, menus::Entry::Item(i) if i.cmd == Some(menus::Cmd::Matrix(menus::MatrixCmd::Inverse))),
        ),
        _ => false,
    });
    assert!(has_inverse, "Analysis > Mathematics > Inverse...");
    h.state_mut().apply(Action::MatrixCmd { id, cmd: menus::MatrixCmd::Inverse }, &ctx);
    settle(&mut h);
    assert_eq!(h.state().doc.undo_label(), Some("Matrix Inverse"));
    let inv = project(&h).matrix_books().nth(1).expect("a new matrix book").sheets[0].clone();
    // [[2, 1], [1, 2]]^-1 = [[2, -1], [-1, 2]] / 3
    for (r, c, v) in [(0, 0, 2.0 / 3.0), (0, 1, -1.0 / 3.0), (1, 0, -1.0 / 3.0), (1, 1, 2.0 / 3.0)] {
        assert!((inv.value(r, c) - v).abs() < 1e-12, "{r},{c}: {}", inv.value(r, c));
    }
}

#[test]
fn two_d_fft_and_ifft_round_trip_through_new_matrix_books() {
    use sheaf_core::matrix_project::MatrixOp;
    let mut h = start(Vec::new());
    let ctx = h.ctx.clone();
    let mut id = sheaf_core::WindowId(0);
    h.state_mut().doc.apply("Matrix", |p| {
        id = p.add_matrix_book("");
        p.apply_matrix_op(id, 0, &MatrixOp::SetDimensions { rows: 2, cols: 3, ranges: None }).unwrap();
        p.apply_matrix_op(id, 0, &MatrixOp::SetValues("i*3+j-3".into())).unwrap();
    });
    settle(&mut h);
    let a: Vec<f64> = project(&h).matrix_sheet(id, 0).unwrap().values().to_vec();
    assert_eq!(a, [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    h.state_mut().apply(Action::MatrixCmd { id, cmd: menus::MatrixCmd::Fft2 }, &ctx);
    settle(&mut h);
    assert_eq!(h.state().doc.undo_label(), Some("2D FFT"));
    let fft = project(&h).matrix_books().nth(1).expect("the FFT book").clone();
    let names: Vec<&str> = fft.sheets.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["Real", "Imaginary", "Amplitude", "Phase"]);
    assert_eq!(fft.sheets[0].value(0, 0), 21.0, "the zero frequency is the sum");
    assert!((fft.sheets[2].value(0, 1) - 12f64.sqrt()).abs() < 1e-12, "|-3 + 1.732i|");
    // The inverse of that book gives the data back.
    h.state_mut().apply(Action::MatrixCmd { id: fft.id, cmd: menus::MatrixCmd::Ifft2 }, &ctx);
    settle(&mut h);
    let back = project(&h).matrix_books().nth(2).expect("the IFFT book").sheets[0].clone();
    for (k, v) in a.iter().enumerate() {
        assert!((back.values()[k] - v).abs() < 1e-12, "{k}: {}", back.values()[k]);
    }
}

#[test]
fn a_binary_2d_array_is_imported_into_a_matrix_book_named_after_the_file() {
    use sheaf_io::bin2d::{Bin2dOptions, BinType};
    let dir = std::env::temp_dir().join(format!("sheaf-bin2d-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("frame.bin");
    let bytes: Vec<u8> = (0..6).flat_map(|k| (k as f32 * 0.5).to_le_bytes()).collect();
    std::fs::write(&path, bytes).unwrap();
    let mut h = start(Vec::new());
    // The options window guesses 32-bit floats; six of them are one row.
    let d = crate::dialogs::import_bin2d::ImportBin2dDialog::new(path.clone()).unwrap();
    assert_eq!((d.options.kind, d.options.cols, d.options.rows), (BinType::F32, 6, 1));
    let opts = Bin2dOptions { kind: BinType::F32, cols: 3, rows: 2, header: 0, big_endian: false };
    crate::dialogs::import_bin2d::import_bin2d(&mut h.state_mut().doc, &path, &opts).unwrap();
    settle(&mut h);
    assert_eq!(h.state().doc.undo_label(), Some("Import Binary 2D Array"));
    let book = project(&h).matrix_books().next().expect("a matrix book");
    assert_eq!(book.long_name, "frame");
    let s = &book.sheets[0];
    assert_eq!((s.cols, s.rows, s.value(1, 2)), (3, 2, 2.5));
    // Too many numbers for the file are refused and add nothing.
    let big = Bin2dOptions { rows: 3, ..opts };
    assert!(crate::dialogs::import_bin2d::import_bin2d(&mut h.state_mut().doc, &path, &big).is_err());
    assert_eq!(project(&h).matrix_books().count(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_imported_picture_becomes_a_matrix_of_gray_levels() {
    let dir = std::env::temp_dir().join(format!("sheaf-import-image-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("photo.png");
    // 3 x 2 pixels: a gray ramp on top, white below.
    let rgb: Vec<u8> = [0u8, 100, 200, 255, 255, 255].iter().flat_map(|&v| [v, v, v]).collect();
    let mut bytes = Vec::new();
    let enc = image::codecs::png::PngEncoder::new(&mut bytes);
    image::ImageEncoder::write_image(enc, &rgb, 3, 2, image::ExtendedColorType::Rgb8).unwrap();
    std::fs::write(&path, bytes).unwrap();
    let mut h = start(Vec::new());
    crate::matrix_view::import_image(&mut h.state_mut().doc, &path).unwrap();
    settle(&mut h);
    assert_eq!(h.state().doc.undo_label(), Some("Import Image"));
    let book = project(&h).matrix_books().next().expect("a matrix book");
    assert_eq!(book.long_name, "photo");
    let s = &book.sheets[0];
    assert_eq!((s.cols, s.rows), (3, 2));
    assert_eq!((s.value(0, 0), s.value(0, 1), s.value(0, 2), s.value(1, 0)), (0.0, 100.0, 200.0, 255.0));
    let not_a_picture = dir.join("notes.png");
    std::fs::write(&not_a_picture, b"hello").unwrap();
    assert!(crate::matrix_view::import_image(&mut h.state_mut().doc, &not_a_picture).is_err());
    assert_eq!(project(&h).matrix_books().count(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_image_menu_adjusts_and_filters_the_active_matrix_in_place() {
    use sheaf_core::matrix_image::ImageOp;
    use sheaf_core::matrix_project::MatrixOp;
    let mut h = start(Vec::new());
    let ctx = h.ctx.clone();
    let mut id = sheaf_core::WindowId(0);
    h.state_mut().doc.apply("Matrix", |p| {
        id = p.add_matrix_book("");
        p.apply_matrix_op(id, 0, &MatrixOp::SetDimensions { rows: 3, cols: 3, ranges: None }).unwrap();
        p.apply_matrix_op(id, 0, &MatrixOp::SetValues("i==2 && j==2 ? 9 : 0".into())).unwrap();
    });
    settle(&mut h);
    assert!(menus::bar(menus::WindowKind::Matrix).iter().any(|m| m.label == "&Image"), "the matrix bar has Image");
    // Smooth runs at once, in place, as one undo step.
    h.state_mut().apply(Action::ImageCmd { id, cmd: menus::ImageCmd::Smooth }, &ctx);
    settle(&mut h);
    assert_eq!(h.state().doc.undo_label(), Some("Image Smooth"));
    assert_eq!(project(&h).matrix_sheet(id, 0).unwrap().value(1, 1), 1.0);
    assert_eq!(project(&h).matrix_books().count(), 1, "no new book");
    // Brightness opens its window; OK adds the amount.
    h.state_mut().apply(Action::ImageCmd { id, cmd: menus::ImageCmd::Brightness }, &ctx);
    settle(&mut h);
    click(&mut h, "OK");
    assert_eq!(h.state().doc.undo_label(), Some("Image Brightness"));
    assert_eq!(project(&h).matrix_sheet(id, 0).unwrap().value(1, 1), 11.0, "the default amount is 10");
    // The other operations run through the same in-place path.
    crate::dialogs::matrix_ops::apply_image(&mut h.state_mut().doc, id, 0, &ImageOp::ToBinary(5.0)).unwrap();
    let s = project(&h).matrix_sheet(id, 0).unwrap().clone();
    assert!(s.values().iter().all(|v| *v == 1.0), "every cell is at least 5 now: {:?}", s.values());
}

#[test]
fn image_levels_crop_resize_and_subtract_change_the_active_matrix() {
    use crate::dialogs::matrix_ops::{apply_image, subtract_image};
    use sheaf_core::matrix_image::ImageOp;
    use sheaf_core::matrix_project::MatrixOp;
    let mut h = start(Vec::new());
    let ctx = h.ctx.clone();
    let (mut a, mut b) = (sheaf_core::WindowId(0), sheaf_core::WindowId(0));
    h.state_mut().doc.apply("Matrices", |p| {
        for id in [&mut a, &mut b] {
            *id = p.add_matrix_book("");
            p.apply_matrix_op(*id, 0, &MatrixOp::SetDimensions { rows: 2, cols: 2, ranges: None }).unwrap();
            p.apply_matrix_op(*id, 0, &MatrixOp::SetValues("i*10+j".into())).unwrap();
        }
    });
    settle(&mut h);
    // Subtract opens its window with the other book chosen.
    h.state_mut().apply(Action::ImageCmd { id: a, cmd: menus::ImageCmd::Subtract }, &ctx);
    settle(&mut h);
    click(&mut h, "OK");
    assert_eq!(h.state().doc.undo_label(), Some("Image Subtract"));
    assert!(project(&h).matrix_sheet(a, 0).unwrap().values().iter().all(|v| *v == 0.0));
    subtract_image(&mut h.state_mut().doc, b, 0, b).unwrap();
    let doc = &mut h.state_mut().doc;
    apply_image(doc, b, 0, &ImageOp::Resize { rows: 4, cols: 4 }).unwrap();
    apply_image(doc, b, 0, &ImageOp::Crop { rows: (0, 1), cols: (2, 3) }).unwrap();
    apply_image(doc, b, 0, &ImageOp::Levels { input: (-1.0, 1.0), output: (0.0, 10.0) }).unwrap();
    let s = project(&h).matrix_sheet(b, 0).unwrap().clone();
    assert_eq!((s.rows, s.cols), (2, 2));
    assert!(s.values().iter().all(|v| *v == 5.0), "zeros map to the middle of the output: {:?}", s.values());
    assert_eq!(h.state().doc.undo_label(), Some("Image Levels"));
}

#[test]
fn a_colour_picture_imports_as_rgb_sheets_that_turn_gray_or_split() {
    let dir = std::env::temp_dir().join(format!("sheaf-import-rgb-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("flag.png");
    let rgb: Vec<u8> = vec![255, 0, 0, 0, 255, 0];
    let mut bytes = Vec::new();
    let enc = image::codecs::png::PngEncoder::new(&mut bytes);
    image::ImageEncoder::write_image(enc, &rgb, 2, 1, image::ExtendedColorType::Rgb8).unwrap();
    std::fs::write(&path, bytes).unwrap();
    let mut h = start(Vec::new());
    let ctx = h.ctx.clone();
    crate::matrix_view::import_image(&mut h.state_mut().doc, &path).unwrap();
    settle(&mut h);
    let book = project(&h).matrix_books().next().unwrap().clone();
    let names: Vec<&str> = book.sheets.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["Red", "Green", "Blue"]);
    h.state_mut().apply(Action::ImageCmd { id: book.id, cmd: menus::ImageCmd::ColorToGray }, &ctx);
    settle(&mut h);
    assert_eq!(h.state().doc.undo_label(), Some("Color to Gray"));
    let gray = project(&h).matrix_books().nth(1).expect("a gray book").sheets[0].clone();
    assert!((gray.value(0, 0) - 0.2126 * 255.0).abs() < 1e-9 && (gray.value(0, 1) - 0.7152 * 255.0).abs() < 1e-9);
    h.state_mut().apply(Action::ImageCmd { id: book.id, cmd: menus::ImageCmd::RgbSplit }, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).matrix_books().count(), 5, "three one-channel books more");
    // A book without colour sheets refuses, and adds nothing.
    let gray_id = gray_book(&h);
    h.state_mut().apply(Action::ImageCmd { id: gray_id, cmd: menus::ImageCmd::RgbSplit }, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).matrix_books().count(), 5);
    let _ = std::fs::remove_dir_all(&dir);
}

fn gray_book(h: &Harness<'static, SheafApp>) -> sheaf_core::WindowId {
    project(h).matrix_books().nth(1).unwrap().id
}
