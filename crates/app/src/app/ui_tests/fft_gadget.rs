//! Gadgets > FFT (T142, graphing "FFT gadget").

use super::*;
use crate::gadget::GadgetKind;

#[test]
fn the_fft_gadget_finds_the_strongest_frequency_inside_its_range() {
    let dir = std::env::temp_dir().join(format!("sheaf-app-ui-fft-gadget-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("wave.csv");
    let mut text = String::from("t,v\n");
    // 5 Hz sampled every 0.01 s for 2 s.
    for i in 0..200 {
        let t = f64::from(i) * 0.01;
        text.push_str(&format!("{t},{}\n", (2.0 * std::f64::consts::PI * 5.0 * t).sin()));
    }
    std::fs::write(&path, text).unwrap();
    let mut h = start(vec![path]);
    let ctx = h.ctx.clone();
    let graph = project(&h).graphs().next().unwrap().id;
    h.state_mut().apply(Action::StartFft { graph }, &ctx);
    settle(&mut h);
    let mut g = h.state().graphs[&graph].gadget.expect("the gadget is open");
    assert_eq!(g.kind, GadgetKind::Fft);
    (g.x0, g.x1) = (0.0, 2.0);
    let (x, y) = crate::gadget::plot_xy(project(&h), graph, 0, 0).unwrap();
    let (f, a) = g.spectrum(&x, &y).unwrap();
    let (fmax, _) = f.iter().zip(&a).filter(|(fv, _)| **fv > 0.0).max_by(|p, q| p.1.total_cmp(q.1)).unwrap();
    assert!((fmax - 5.0).abs() < 0.1, "strongest at {fmax}");
    h.state_mut().graphs.get_mut(&graph).unwrap().gadget = Some(g);
    settle(&mut h);
    (g.x0, g.x1) = (0.0, 0.02);
    assert!(g.spectrum(&x, &y).is_none(), "too few points");
    let _ = std::fs::remove_dir_all(&dir);
}
