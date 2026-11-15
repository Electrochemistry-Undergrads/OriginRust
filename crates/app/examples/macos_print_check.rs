//! Validate PDFKit operation construction on the main thread without running a print job.
//! `--show-panel` is for a person to open the panel and choose Cancel; never use Print here.
#[cfg(target_os = "macos")]
#[allow(dead_code)]
#[path = "../src/macprint.rs"]
mod macprint;

#[cfg(target_os = "macos")]
fn main() {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSPaperOrientation};
    use sheaf_core::page_setup::{Margins, Orientation, Paper};
    use sheaf_core::template::Template;
    use sheaf_core::{ColumnData, Designation, Project};

    let mtm = MainThreadMarker::new().expect("run this example on the main thread");
    let _app = NSApplication::sharedApplication(mtm);
    let show = std::env::args().skip(1).any(|a| a == "--show-panel");
    for (paper, orientation, page) in [
        (Paper::Letter, Orientation::Portrait, (300.0, 400.0)),
        (Paper::Letter, Orientation::Auto, (400.0, 200.0)),
        (Paper::A4, Orientation::Portrait, (300.0, 400.0)),
        (Paper::A4, Orientation::Landscape, (300.0, 400.0)),
    ] {
        let mut project = Project::new();
        let book = project.add_workbook("data");
        project.add_column(book, 0, Designation::X, ColumnData::numeric(vec![0., 1., 2.])).unwrap();
        let y = project.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1., 2., 4.])).unwrap();
        let graph = project.add_graph(Template::Line);
        project.add_plot(graph, 0, y).unwrap();
        let g = project.graph_mut(graph).unwrap();
        g.page.width_pt = page.0;
        g.page.height_pt = page.1;
        project.page_setup.paper = paper;
        project.page_setup.orientation = orientation;
        project.page_setup.margins = Margins { left: 36., right: 72., top: 100., bottom: 80. };
        let pdf = sheaf_plot::render_print_pdf(&project, graph).unwrap();
        let prepared = macprint::prepare(&pdf, &project.page_setup, page, "Sheaf validation (cancel only)").unwrap();
        assert_eq!(prepared.page_count(), 1);
        let expected_paper = project.page_setup.paper_pt(page.0, page.1);
        let pdf_paper = prepared.pdf_paper_size().unwrap();
        assert!((pdf_paper.0 - expected_paper.0).abs() < 0.01);
        assert!((pdf_paper.1 - expected_paper.1).abs() < 0.01);
        let operation = prepared.operation();
        assert!(operation.showsPrintPanel());
        assert_eq!(operation.jobTitle().unwrap().to_string(), "Sheaf validation (cancel only)");
        let info = prepared.info();
        let expected_orientation = if orientation == Orientation::Portrait {
            NSPaperOrientation::Portrait
        } else {
            NSPaperOrientation::Landscape
        };
        assert_eq!(info.orientation(), expected_orientation);
        let size = info.paperSize();
        assert!((size.width - expected_paper.0).abs() < 0.01, "paper width");
        assert!((size.height - expected_paper.1).abs() < 0.01, "paper height");
        println!("Selected paper: {:?}", info.paperName().map(|v| v.to_string()));
        assert!(macprint::prepare(b"invalid PDF", &project.page_setup, page, "invalid").is_err());
        let mut wrong = project.page_setup;
        wrong.paper = if paper == Paper::A4 { Paper::Letter } else { Paper::A4 };
        assert!(macprint::prepare(&pdf, &wrong, page, "wrong size").is_err());
        println!(
            "{orientation:?}: NSPrintInfo paper {} x {}, saved PDF paper {:?}",
            size.width,
            size.height,
            project.page_setup.paper_pt(page.0, page.1)
        );
        assert_eq!((info.leftMargin(), info.rightMargin(), info.topMargin(), info.bottomMargin()), (0., 0., 0., 0.));
        assert_eq!(info.scalingFactor(), 1.);
        if show {
            println!("Opening the print panel. Choose Cancel; this example must not print.");
            assert_eq!(prepared.run(), macprint::Outcome::CancelledOrFailed, "the panel was not cancelled");
        }
    }
    println!("PDFKit prepared the print operations without submitting a job");
}

#[cfg(not(target_os = "macos"))]
fn main() {
    println!("This validation example is macOS-only.");
}
