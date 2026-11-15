//! Printing a graph page on Windows (File > Print). The page's enhanced metafile
//! (`sheaf_plot::render_emf`) is played into a printer device context, scaled and placed by
//! `sheaf_core::page_setup::placement`, the same function Print Preview draws with. Nothing is
//! rasterised: the printer driver receives the vector drawing.
//!
//! The printing calls ([`print_with_dialog`], [`print_to_printer`]) are never made by the tests;
//! the arithmetic that turns a printer's measurements into the playback rectangle
//! ([`device_rect`]) is pure and tested.

use sheaf_core::page_setup::{PageSetup, Rect, placement};

/// What a printer device context reports, in device pixels (`GetDeviceCaps`).
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct DeviceCaps {
    pub dpi_x: f64,
    pub dpi_y: f64,
    /// `PHYSICALWIDTH`, `PHYSICALHEIGHT`: the whole sheet.
    pub physical_w: f64,
    pub physical_h: f64,
    /// `PHYSICALOFFSETX`, `PHYSICALOFFSETY`: where the printable area starts on the sheet.
    pub offset_x: f64,
    pub offset_y: f64,
    /// `HORZRES`, `VERTRES`: the printable area.
    pub horz_res: f64,
    pub vert_res: f64,
}

/// The rectangle `[left, top, right, bottom]` in device pixels, relative to the printable area's
/// corner (the origin a device context draws from), into which the page's metafile is played.
/// `None` when the page cannot be placed (no size, margins leave no room, a bad device).
#[cfg_attr(not(any(windows, test)), allow(dead_code))]
pub fn device_rect(caps: &DeviceCaps, setup: &PageSetup, page_w_pt: f64, page_h_pt: f64) -> Option<[i32; 4]> {
    if !(caps.dpi_x > 0.0 && caps.dpi_y > 0.0) {
        return None;
    }
    let (kx, ky) = (72.0 / caps.dpi_x, 72.0 / caps.dpi_y);
    let paper = (caps.physical_w * kx, caps.physical_h * ky);
    let printable = Rect { x: caps.offset_x * kx, y: caps.offset_y * ky, w: caps.horz_res * kx, h: caps.vert_res * ky };
    let r = placement(page_w_pt, page_h_pt, paper, printable, &setup.margins)?;
    let left = (r.x / kx - caps.offset_x).round() as i32;
    let top = (r.y / ky - caps.offset_y).round() as i32;
    let right = ((r.x + r.w) / kx - caps.offset_x).round() as i32;
    let bottom = ((r.y + r.h) / ky - caps.offset_y).round() as i32;
    Some([left, top, right, bottom])
}

/// What came of asking the user to print.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(not(windows), allow(dead_code))]
pub enum Outcome {
    Printed,
    /// The user closed the printer dialog.
    Cancelled,
}

/// Elsewhere than Windows: the page's PDF goes to the default printer with `lp` (paper size and
/// orientation from the Page Setup; the margins are the printer's). A first step: no print dialog.
#[cfg(not(any(windows, target_os = "macos")))]
pub fn print_pdf(pdf: &[u8], setup: &PageSetup, page_pt: (f64, f64), title: &str) -> Result<(), String> {
    use sheaf_core::page_setup::Orientation;
    let path = std::env::temp_dir().join(format!("sheaf-print-{}.pdf", std::process::id()));
    std::fs::write(&path, pdf).map_err(|e| e.to_string())?;
    let media = match setup.paper {
        sheaf_core::page_setup::Paper::Letter => "Letter",
        sheaf_core::page_setup::Paper::Legal => "Legal",
        sheaf_core::page_setup::Paper::Tabloid => "Tabloid",
        sheaf_core::page_setup::Paper::A3 => "A3",
        sheaf_core::page_setup::Paper::A4 => "A4",
        sheaf_core::page_setup::Paper::A5 => "A5",
    };
    let orientation =
        if setup.orientation_for(page_pt.0, page_pt.1) == Orientation::Landscape { "landscape" } else { "portrait" };
    let out = std::process::Command::new("lp")
        .args(["-t", title, "-o", &format!("media={media}"), "-o", orientation, "-o", "fit-to-page"])
        .arg(&path)
        .output()
        .map_err(|e| format!("lp could not be started: {e}"));
    let _ = std::fs::remove_file(&path);
    let out = out?;
    if out.status.success() { Ok(()) } else { Err(String::from_utf8_lossy(&out.stderr).trim().to_string()) }
}

/// Printing to a named printer without a dialog; the only caller is the ignored test below, which
/// prints to "Microsoft Print to PDF" by hand.
#[cfg(windows)]
#[cfg_attr(not(test), allow(unused_imports))]
pub use imp::print_to_printer;
#[cfg(windows)]
pub use imp::print_with_dialog;

#[cfg(windows)]
mod imp {
    use super::{DeviceCaps, Outcome, device_rect};
    use sheaf_core::page_setup::{Orientation, PageSetup};
    use std::ffi::c_void;
    use std::path::Path;
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::Foundation::{GetLastError, GlobalFree, HWND};
    use windows_sys::Win32::Graphics::Gdi::{
        CreateDCW, DEVMODEW, DM_ORIENTATION, DM_PAPERSIZE, DeleteDC, DeleteEnhMetaFile, GetDeviceCaps, HDC, HORZRES,
        LOGPIXELSX, LOGPIXELSY, PHYSICALHEIGHT, PHYSICALOFFSETX, PHYSICALOFFSETY, PHYSICALWIDTH, PlayEnhMetaFile,
        SetEnhMetaFileBits, VERTRES,
    };
    use windows_sys::Win32::Graphics::Printing::{ClosePrinter, DocumentPropertiesW, OpenPrinterW, PRINTER_HANDLE};
    use windows_sys::Win32::Storage::Xps::{AbortDoc, DOCINFOW, EndDoc, EndPage, StartDocW, StartPage};
    use windows_sys::Win32::System::Memory::{GlobalLock, GlobalUnlock};
    use windows_sys::Win32::UI::Controls::Dialogs::{
        PD_NOPAGENUMS, PD_NOSELECTION, PD_PAGENUMS, PD_RETURNDC, PD_RETURNDEFAULT, PD_USEDEVMODECOPIESANDCOLLATE,
        PRINTDLGW, PrintDlgW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

    #[cfg_attr(not(test), allow(dead_code))]
    const DM_IN_BUFFER: u32 = 8;
    #[cfg_attr(not(test), allow(dead_code))]
    const DM_OUT_BUFFER: u32 = 2;
    const DMORIENT_PORTRAIT: i16 = 1;
    const DMORIENT_LANDSCAPE: i16 = 2;

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// Sets paper and orientation of a device mode to the Page Setup.
    fn apply_setup(dm: &mut DEVMODEW, setup: &PageSetup, page_pt: (f64, f64)) {
        dm.dmFields |= DM_PAPERSIZE | DM_ORIENTATION;
        // The printer half of the union is the one that holds these fields.
        dm.Anonymous1.Anonymous1.dmPaperSize = setup.paper.dmpaper();
        dm.Anonymous1.Anonymous1.dmOrientation = match setup.orientation_for(page_pt.0, page_pt.1) {
            Orientation::Landscape => DMORIENT_LANDSCAPE,
            _ => DMORIENT_PORTRAIT,
        };
    }

    /// Plays the metafiles into `hdc`, one printed page each (`range` picks pages, 1-based and
    /// inclusive), then ends the document. `output` redirects the job to a file (the driver may
    /// still ask where to save it).
    fn print_on_dc(
        hdc: HDC,
        pages: &[Vec<u8>],
        range: Option<(usize, usize)>,
        page_pt: (f64, f64),
        setup: &PageSetup,
        title: &str,
        output: Option<&Path>,
    ) -> Result<(), String> {
        // SAFETY: `hdc` is a live device context.
        let caps = |i: u32| f64::from(unsafe { GetDeviceCaps(hdc, i as i32) });
        let caps = DeviceCaps {
            dpi_x: caps(LOGPIXELSX),
            dpi_y: caps(LOGPIXELSY),
            physical_w: caps(PHYSICALWIDTH),
            physical_h: caps(PHYSICALHEIGHT),
            offset_x: caps(PHYSICALOFFSETX),
            offset_y: caps(PHYSICALOFFSETY),
            horz_res: caps(HORZRES),
            vert_res: caps(VERTRES),
        };
        let [left, top, right, bottom] = device_rect(&caps, setup, page_pt.0, page_pt.1)
            .ok_or_else(|| "the margins leave no room on the paper".to_string())?;
        let (first, last) = range.unwrap_or((1, pages.len()));
        let (first, last) = (first.max(1), last.min(pages.len()));
        if first > last {
            return Err("the page range has no page in it".to_string());
        }
        let rect = windows_sys::Win32::Foundation::RECT { left, top, right, bottom };
        let name = wide(title);
        let out = output.map(|p| wide(&p.to_string_lossy()));
        let doc = DOCINFOW {
            cbSize: std::mem::size_of::<DOCINFOW>() as i32,
            lpszDocName: name.as_ptr(),
            lpszOutput: out.as_ref().map_or(null(), |o| o.as_ptr()),
            lpszDatatype: null(),
            fwType: 0,
        };
        // SAFETY: `hdc` is a live printer device context; `doc` and its strings outlive the calls.
        unsafe {
            if StartDocW(hdc, &doc) <= 0 {
                Err(format!("the printer did not start the job (error {})", GetLastError()))
            } else {
                let mut result = Ok(());
                for emf in &pages[first - 1..last] {
                    result = play_page(hdc, emf, &rect);
                    if result.is_err() {
                        break;
                    }
                }
                if result.is_ok() {
                    EndDoc(hdc);
                } else {
                    // A failed page cancels the job instead of submitting a partial one.
                    AbortDoc(hdc);
                }
                result
            }
        }
    }

    /// One printed page: the metafile played into `rect`.
    ///
    /// # Safety
    /// `hdc` must be a live printer device context with a document started on it.
    unsafe fn play_page(hdc: HDC, emf: &[u8], rect: &windows_sys::Win32::Foundation::RECT) -> Result<(), String> {
        let size = u32::try_from(emf.len()).map_err(|_| "the metafile is too large".to_string())?;
        // SAFETY: `emf` is valid for `size` bytes; the call copies them into a new metafile handle.
        let hemf = unsafe { SetEnhMetaFileBits(size, emf.as_ptr()) };
        if hemf.is_null() {
            return Err("the page's metafile could not be read".to_string());
        }
        // SAFETY: the caller guarantees the device context and the started document.
        let result = unsafe {
            if StartPage(hdc) <= 0 {
                Err(format!("the printer did not start the page (error {})", GetLastError()))
            } else {
                let played = PlayEnhMetaFile(hdc, hemf, rect);
                EndPage(hdc);
                if played == 0 { Err("the page could not be drawn on the printer".to_string()) } else { Ok(()) }
            }
        };
        // SAFETY: the handle was created above and is not used again.
        unsafe { DeleteEnhMetaFile(hemf) };
        result
    }

    /// Asks for a printer with the system's print dialog (paper and orientation start from the
    /// Page Setup) and prints the pages, metafiles of `page_pt` points each, on it. With more than
    /// one page the dialog offers a page range.
    pub fn print_with_dialog(
        pages: &[Vec<u8>],
        page_pt: (f64, f64),
        setup: &PageSetup,
        title: &str,
    ) -> Result<Outcome, String> {
        // SAFETY: PRINTDLGW is plain data; zero means "nothing set".
        let mut pd: PRINTDLGW = unsafe { std::mem::zeroed() };
        pd.lStructSize = std::mem::size_of::<PRINTDLGW>() as u32;
        // First the default printer's device mode without any window, so the dialog opens on the
        // Page Setup's paper and orientation.
        pd.Flags = PD_RETURNDEFAULT;
        // SAFETY: `pd` is initialised; with PD_RETURNDEFAULT the call only fills the two handles.
        if unsafe { PrintDlgW(&mut pd) } == 0 {
            return Err("no printer is installed".to_string());
        }
        if !pd.hDevMode.is_null() {
            // SAFETY: the handle is the device mode the call returned; it is locked while edited.
            unsafe {
                let dm = GlobalLock(pd.hDevMode).cast::<DEVMODEW>();
                if !dm.is_null() {
                    apply_setup(&mut *dm, setup, page_pt);
                    GlobalUnlock(pd.hDevMode);
                }
            }
        }
        pd.Flags = PD_RETURNDC | PD_NOSELECTION | PD_USEDEVMODECOPIESANDCOLLATE;
        if pages.len() > 1 {
            let n = u16::try_from(pages.len()).unwrap_or(u16::MAX);
            pd.nMinPage = 1;
            pd.nMaxPage = n;
            pd.nFromPage = 1;
            pd.nToPage = n;
        } else {
            pd.Flags |= PD_NOPAGENUMS;
        }
        // SAFETY: no window is the owner of last resort; the foreground window is normally ours.
        pd.hwndOwner = unsafe { GetForegroundWindow() } as HWND;
        // SAFETY: `pd` holds the handles from the first call.
        let ok = unsafe { PrintDlgW(&mut pd) };
        let hdc = pd.hDC;
        let result = if ok == 0 || hdc.is_null() {
            Ok(Outcome::Cancelled)
        } else {
            let range = (pd.Flags & PD_PAGENUMS != 0).then(|| (usize::from(pd.nFromPage), usize::from(pd.nToPage)));
            print_on_dc(hdc, pages, range, page_pt, setup, title, None).map(|()| Outcome::Printed)
        };
        // SAFETY: each handle is released once, after its last use.
        unsafe {
            if !hdc.is_null() {
                DeleteDC(hdc);
            }
            if !pd.hDevMode.is_null() {
                GlobalFree(pd.hDevMode);
            }
            if !pd.hDevNames.is_null() {
                GlobalFree(pd.hDevNames);
            }
        }
        result
    }

    /// Prints the page on the named printer without any dialog (paper and orientation from the
    /// Page Setup). With `output`, the job is written to that file instead of the port; this suits
    /// "Microsoft Print to PDF" for a check without a printer.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn print_to_printer(
        printer: &str,
        pages: &[Vec<u8>],
        page_pt: (f64, f64),
        setup: &PageSetup,
        title: &str,
        output: Option<&Path>,
    ) -> Result<(), String> {
        let name = wide(printer);
        let mut handle = PRINTER_HANDLE { Value: null_mut::<c_void>() };
        // SAFETY: `name` is NUL-terminated and outlives the call.
        if unsafe { OpenPrinterW(name.as_ptr(), &mut handle, null()) } == 0 {
            return Err(format!("no printer named \"{printer}\""));
        }
        // The device mode is built by the driver, then paper and orientation are set on it and the
        // driver merges them.
        // SAFETY: the sizing call with no buffers returns the byte count the driver needs.
        let size = unsafe { DocumentPropertiesW(null_mut(), handle, name.as_ptr(), null_mut(), null(), 0) };
        let result = if size <= 0 {
            Err("the printer's settings could not be read".to_string())
        } else {
            // u32 elements keep the buffer aligned for DEVMODEW.
            let mut buf = vec![0u32; (size as usize).div_ceil(4)];
            let dm = buf.as_mut_ptr().cast::<DEVMODEW>();
            // SAFETY: `buf` holds `size` bytes; the driver fills it, then it is edited and merged.
            let hdc = unsafe {
                if DocumentPropertiesW(null_mut(), handle, name.as_ptr(), dm, null(), DM_OUT_BUFFER) < 0 {
                    null_mut()
                } else {
                    apply_setup(&mut *dm, setup, page_pt);
                    DocumentPropertiesW(null_mut(), handle, name.as_ptr(), dm, dm, DM_IN_BUFFER | DM_OUT_BUFFER);
                    CreateDCW(wide("WINSPOOL").as_ptr(), name.as_ptr(), null(), dm)
                }
            };
            if hdc.is_null() {
                Err("the printer could not be opened for printing".to_string())
            } else {
                let r = print_on_dc(hdc, pages, None, page_pt, setup, title, output);
                // SAFETY: the context was created above.
                unsafe { DeleteDC(hdc) };
                r
            }
        };
        // SAFETY: opened above.
        unsafe { ClosePrinter(handle) };
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sheaf_core::page_setup::{Margins, Orientation, Paper};

    /// A 600 dpi laser printer: A4, the printable area 4.2 mm in from each edge.
    fn a4_600dpi() -> DeviceCaps {
        let px = |mm: f64| mm / 25.4 * 600.0;
        DeviceCaps {
            dpi_x: 600.0,
            dpi_y: 600.0,
            physical_w: px(210.0).round(),
            physical_h: px(297.0).round(),
            offset_x: px(4.2).round(),
            offset_y: px(4.2).round(),
            horz_res: px(210.0 - 8.4).round(),
            vert_res: px(297.0 - 8.4).round(),
        }
    }

    #[test]
    fn a_wide_page_fills_the_width_between_the_margins() {
        let caps = a4_600dpi();
        let setup = PageSetup { paper: Paper::A4, orientation: Orientation::Portrait, margins: Margins::all(36.0) };
        let r = device_rect(&caps, &setup, 6.4 * 72.0, 4.8 * 72.0).unwrap();
        // The margin is 0.5 in = 300 px from the sheet edge; the context starts at the offset.
        assert_eq!(r[0], 300 - caps.offset_x as i32);
        let width_px = f64::from(r[2] - r[0]);
        assert!((width_px - (caps.physical_w - 600.0)).abs() <= 1.0);
        let h = f64::from(r[3] - r[1]);
        assert!((h / width_px - 4.8 / 6.4).abs() < 1e-3);
        // Centred on the sheet vertically.
        let centre = f64::from(r[1] + r[3]) / 2.0 + caps.offset_y;
        assert!((centre - caps.physical_h / 2.0).abs() <= 1.0);
    }

    #[test]
    fn a_margin_inside_the_unprintable_edge_is_pushed_out_to_it() {
        let caps = a4_600dpi();
        let setup = PageSetup { margins: Margins::all(0.0), ..PageSetup::default() };
        let r = device_rect(&caps, &setup, 100.0, 1000.0).unwrap(); // tall page: height limits
        assert_eq!(r[1], 0);
        assert!(i64::from(r[3]) <= caps.vert_res as i64);
    }

    #[test]
    fn landscape_letter_at_300_dpi() {
        // The driver reports the rotated sheet: 11 x 8.5 in, nothing unprintable.
        let caps = DeviceCaps {
            dpi_x: 300.0,
            dpi_y: 300.0,
            physical_w: 3300.0,
            physical_h: 2550.0,
            offset_x: 0.0,
            offset_y: 0.0,
            horz_res: 3300.0,
            vert_res: 2550.0,
        };
        let setup = PageSetup {
            paper: Paper::Letter,
            orientation: Orientation::Landscape,
            margins: Margins { left: 72.0, top: 72.0, right: 72.0, bottom: 72.0 },
        };
        // A 4:3 page: height limits, (8.5 - 2) in = 1950 px, width 2600 px, centred in 3300.
        let r = device_rect(&caps, &setup, 800.0, 600.0).unwrap();
        assert_eq!(r, [350, 300, 2950, 2250]);
    }

    #[test]
    fn no_room_or_a_broken_device_gives_none() {
        let caps = a4_600dpi();
        let setup = PageSetup { margins: Margins::all(1000.0), ..PageSetup::default() };
        assert!(device_rect(&caps, &setup, 100.0, 100.0).is_none());
        let broken = DeviceCaps { dpi_x: 0.0, ..caps };
        assert!(device_rect(&broken, &PageSetup::default(), 100.0, 100.0).is_none());
    }

    /// The headless check, run by hand (never in the normal test run, it needs a printer driver):
    /// `SHEAF_PRINT_EMF` is a page's metafile (`sheaf plot FILE -o page.emf`; several, separated by `;`, print as pages), `SHEAF_PRINT_OUT` the
    /// file the job is written to, `SHEAF_PRINTER` the printer (default "Microsoft Print to PDF"),
    /// `SHEAF_PRINT_PAGE` the page size in points as `WxH` (default 460.8x345.6, the 6.4 x 4.8 in
    /// page). The call runs on a thread so a driver that opens a window cannot hang the test.
    #[cfg(windows)]
    #[test]
    #[ignore = "prints through a real printer driver; run by hand"]
    fn prints_a_page_to_a_named_printer_without_a_dialog() {
        let emfs: Vec<Vec<u8>> = std::env::var("SHEAF_PRINT_EMF")
            .expect("SHEAF_PRINT_EMF")
            .split(';')
            .map(|p| std::fs::read(p).unwrap())
            .collect();
        let out = std::path::PathBuf::from(std::env::var("SHEAF_PRINT_OUT").expect("SHEAF_PRINT_OUT"));
        let printer = std::env::var("SHEAF_PRINTER").unwrap_or_else(|_| "Microsoft Print to PDF".to_string());
        let page = std::env::var("SHEAF_PRINT_PAGE").unwrap_or_else(|_| "460.8x345.6".to_string());
        let (w, h) = page.split_once('x').unwrap();
        let page = (w.parse::<f64>().unwrap(), h.parse::<f64>().unwrap());
        let (tx, rx) = std::sync::mpsc::channel();
        let target = out.clone();
        std::thread::spawn(move || {
            let r = print_to_printer(&printer, &emfs, page, &PageSetup::default(), "Sheaf print check", Some(&target));
            let _ = tx.send(r);
        });
        rx.recv_timeout(std::time::Duration::from_secs(90)).expect("the printer did not finish in 90 s").unwrap();
        assert!(out.exists(), "the job wrote no file");
    }

    /// The same check for a worksheet: a synthetic 130-row sheet prints as three pages on Letter
    /// portrait. `SHEAF_PRINT_OUT` is the job's file; the Sheaf PDF of the same sheet is written
    /// beside it (`.sheaf.pdf`) for comparison. Run by hand, with a timeout like the test above.
    #[cfg(windows)]
    #[test]
    #[ignore = "prints through a real printer driver; run by hand"]
    fn prints_a_three_page_sheet_to_a_named_printer_without_a_dialog() {
        use sheaf_core::column::{Column, ColumnData, Designation};
        use sheaf_core::ids::ColumnId;
        let out = std::path::PathBuf::from(std::env::var("SHEAF_PRINT_OUT").expect("SHEAF_PRINT_OUT"));
        let printer = std::env::var("SHEAF_PRINTER").unwrap_or_else(|_| "Microsoft Print to PDF".to_string());
        let n = 130;
        let mut ws = sheaf_core::worksheet::Worksheet::new("Sheet1");
        ws.columns = vec![
            Column::new(ColumnId(1), "A", Designation::X, ColumnData::numeric((1..=n).map(f64::from).collect()))
                .with_labels("Index", "n", ""),
            Column::new(
                ColumnId(2),
                "B",
                Designation::Y,
                ColumnData::numeric((1..=n).map(|i| f64::from(i).sqrt()).collect()),
            )
            .with_labels("Root", "", "of the index"),
            Column::new(
                ColumnId(3),
                "C",
                Designation::Y,
                ColumnData::text((1..=n).map(|i| format!("row {i}")).collect()),
            ),
        ];
        let setup = PageSetup::default();
        let pages = sheaf_plot::render_sheet_emf_pages(&ws, &setup).unwrap();
        assert_eq!(pages.len(), 3);
        let area = sheaf_plot::sheet_pages(&ws, &setup).unwrap().area;
        std::fs::write(out.with_extension("sheaf.pdf"), sheaf_plot::render_sheet_pdf(&ws, &setup).unwrap()).unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        let target = out.clone();
        std::thread::spawn(move || {
            let r = print_to_printer(&printer, &pages, area, &setup, "Sheaf sheet print check", Some(&target));
            let _ = tx.send(r);
        });
        rx.recv_timeout(std::time::Duration::from_secs(90)).expect("the printer did not finish in 90 s").unwrap();
        assert!(out.exists(), "the job wrote no file");
    }
}
