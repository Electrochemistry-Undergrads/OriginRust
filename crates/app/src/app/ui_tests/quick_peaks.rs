//! Gadgets > Quick Peaks (T140, graphing "Quick Peaks gadget").

use super::*;
use crate::gadget::GadgetKind;

#[test]
fn quick_peaks_marks_the_peaks_inside_the_range() {
    let dir = std::env::temp_dir().join(format!("sheaf-app-ui-quick-peaks-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("peaks.csv");
    let mut text = String::from("x,y\n");
    for i in 0..=200 {
        let x = f64::from(i) * 0.1;
        let g = |c: f64, h: f64| h * (-0.5 * ((x - c) / 0.5).powi(2)).exp();
        text.push_str(&format!("{x},{}\n", g(5.0, 10.0) + g(12.0, 6.0)));
    }
    std::fs::write(&path, text).unwrap();
    let mut h = start(vec![path]);
    let ctx = h.ctx.clone();
    let graph = project(&h).graphs().next().unwrap().id;
    h.state_mut().apply(Action::StartQuickPeaks { graph }, &ctx);
    settle(&mut h);
    let mut g = h.state().graphs[&graph].gadget.expect("the gadget is open");
    assert_eq!(g.kind, GadgetKind::QuickPeaks);
    let (x, y) = crate::gadget::plot_xy(project(&h), graph, 0, 0).unwrap();
    (g.x0, g.x1) = (0.0, 20.0);
    let both = g.peaks(&x, &y);
    assert_eq!(both.len(), 2, "{both:?}");
    assert!((both[0].0 - 5.0).abs() < 1e-9 && (both[1].0 - 12.0).abs() < 1e-9);
    (g.x0, g.x1) = (8.0, 16.0);
    let one = g.peaks(&x, &y);
    assert_eq!(one.len(), 1, "only the peak inside the range: {one:?}");
    assert!((one[0].0 - 12.0).abs() < 1e-9);
    let _ = std::fs::remove_dir_all(&dir);
}
