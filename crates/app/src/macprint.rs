//! The macOS graph print panel. PDFKit receives a vector PDF already placed on the paper by
//! `PageSetup`; the operation must not scale, rotate, or apply the margins a second time.

use objc2::{AnyThread, MainThreadMarker, rc::Retained};
use objc2_app_kit::{NSPaperOrientation, NSPrintInfo, NSPrintOperation, NSPrintingPaginationMode};
use objc2_foundation::{NSData, NSSize, NSString};
use objc2_pdf_kit::{PDFDisplayBox, PDFDocument, PDFPrintScalingMode};
use sheaf_core::page_setup::{Orientation, PageSetup};

#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Printed,
    /// PDFKit returns false both for cancellation and for a printing failure.
    CancelledOrFailed,
}

/// Keeps the document alive for the whole operation, including while its native panel is open.
pub struct PreparedPrint {
    document: Retained<PDFDocument>,
    _data: Retained<NSData>,
    operation: Retained<NSPrintOperation>,
}

impl PreparedPrint {
    /// Number of pages, useful when checking the prepared operation without showing a panel.
    #[allow(dead_code)]
    pub fn page_count(&self) -> usize {
        // SAFETY: `document` is a live PDFDocument created from valid PDF data on this thread.
        unsafe { self.document.pageCount() }
    }

    /// Size of the PDF page in points.
    #[allow(dead_code)]
    pub fn pdf_paper_size(&self) -> Option<(f64, f64)> {
        // SAFETY: the retained document is valid; the page is only inspected on the main thread.
        let page = unsafe { self.document.pageAtIndex(0) }?;
        // SAFETY: PDFKit returned this page; MediaBox is a valid display box.
        let bounds = unsafe { page.boundsForBox(PDFDisplayBox::MediaBox) };
        Some((bounds.size.width, bounds.size.height))
    }

    #[allow(dead_code)]
    pub fn operation(&self) -> &NSPrintOperation {
        &self.operation
    }

    #[allow(dead_code)]
    pub fn info(&self) -> Retained<NSPrintInfo> {
        self.operation.printInfo()
    }

    /// Runs the native panel and the operation. Do not call this from automated tests.
    pub fn run(&self) -> Outcome {
        if self.operation.runOperation() { Outcome::Printed } else { Outcome::CancelledOrFailed }
    }
}

/// Build an operation but do not open the print panel or submit a job.
pub fn prepare(pdf: &[u8], setup: &PageSetup, page_pt: (f64, f64), title: &str) -> Result<PreparedPrint, String> {
    let mtm = MainThreadMarker::new().ok_or_else(|| "the macOS print panel must run on the main thread".to_string())?;
    if ![page_pt.0, page_pt.1].iter().all(|n| n.is_finite() && *n > 0.0) {
        return Err("the graph page size is invalid".into());
    }
    let data = NSData::with_bytes(pdf);
    // SAFETY: NSData owns a copy of the bytes; PDFKit validates them and returns nil on failure.
    let document = unsafe { PDFDocument::initWithData(PDFDocument::alloc(), &data) }
        .ok_or_else(|| "PDFKit could not open the graph PDF".to_string())?;
    // SAFETY: a live PDFDocument returned from initWithData; this getter has no extra preconditions.
    if unsafe { document.pageCount() } != 1 {
        return Err("the graph PDF must have exactly one page".into());
    }
    // SAFETY: the retained, single-page PDFDocument is valid; index 0 exists.
    let pdf_page = unsafe { document.pageAtIndex(0) }.ok_or_else(|| "the PDF page is missing".to_string())?;
    // SAFETY: the returned PDFPage is valid and MediaBox is a defined PDF box.
    let pdf_bounds = unsafe { pdf_page.boundsForBox(PDFDisplayBox::MediaBox) };
    let expected_paper = setup.paper_pt(page_pt.0, page_pt.1);
    if !pdf_bounds.size.width.is_finite()
        || !pdf_bounds.size.height.is_finite()
        || (pdf_bounds.size.width - expected_paper.0).abs() > 0.1
        || (pdf_bounds.size.height - expected_paper.1).abs() > 0.1
    {
        return Err("the PDF paper size does not match Page Setup".into());
    }
    let info = NSPrintInfo::new();
    let paper = setup.paper.size_pt();
    info.setPaperSize(NSSize::new(paper.0, paper.1));
    info.setOrientation(if setup.orientation_for(page_pt.0, page_pt.1) == Orientation::Landscape {
        NSPaperOrientation::Landscape
    } else {
        NSPaperOrientation::Portrait
    });
    info.setLeftMargin(0.0);
    info.setRightMargin(0.0);
    info.setTopMargin(0.0);
    info.setBottomMargin(0.0);
    info.setScalingFactor(1.0);
    info.setHorizontallyCentered(false);
    info.setVerticallyCentered(false);
    info.setHorizontalPagination(NSPrintingPaginationMode::Clip);
    info.setVerticalPagination(NSPrintingPaginationMode::Clip);
    // SAFETY: the document and info remain alive, and `mtm` proves the main thread. PDFKit returns
    // nil if it cannot create an operation. ScaleNone and no autorotation preserve our placement.
    let operation = unsafe {
        document.printOperationForPrintInfo_scalingMode_autoRotate(
            Some(&info),
            PDFPrintScalingMode::PageScaleNone,
            false,
            mtm,
        )
    }
    .ok_or_else(|| "PDFKit could not create a print operation".to_string())?;
    operation.setShowsPrintPanel(true);
    operation.setJobTitle(Some(&NSString::from_str(title)));
    Ok(PreparedPrint { document, _data: data, operation })
}

pub fn print_with_dialog(pdf: &[u8], setup: &PageSetup, page_pt: (f64, f64), title: &str) -> Result<Outcome, String> {
    Ok(prepare(pdf, setup, page_pt, title)?.run())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_to_construct_a_print_operation_on_a_worker_thread() {
        let error = std::thread::spawn(|| prepare(b"not a PDF", &PageSetup::default(), (400.0, 300.0), "test").err())
            .join()
            .unwrap()
            .unwrap();
        assert!(error.contains("main thread"), "{error}");
    }
}
