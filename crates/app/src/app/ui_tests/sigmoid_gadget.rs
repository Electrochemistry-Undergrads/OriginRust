//! Gadgets > Quick Sigmoidal Fit (T141, graphing "Quick Sigmoidal Fit gadget").

use super::*;
use crate::gadget::GadgetKind;

#[test]
fn the_sigmoid_gadget_fits_a_boltzmann_curve_in_its_range() {
    let dir = std::env::temp_dir().join(format!("sheaf-app-ui-sigmoid-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("sigmoid.csv");
    let mut text = String::from("x,y\n");
    for i in 0..=100 {
        let x = f64::from(i) * 0.2;
        text.push_str(&format!("{x},{}\n", 8.0 + (1.0 - 8.0) / (1.0 + ((x - 9.0) / 1.5).exp())));
    }
    std::fs::write(&path, text).unwrap();
    let mut h = start(vec![path]);
    let ctx = h.ctx.clone();
    let graph = project(&h).graphs().next().unwrap().id;
    h.state_mut().apply(Action::StartSigmoid { graph }, &ctx);
    settle(&mut h);
    let mut g = h.state().graphs[&graph].gadget.expect("the gadget is open");
    assert_eq!(g.kind, GadgetKind::Sigmoid);
    (g.x0, g.x1) = (0.0, 20.0);
    let (x, y) = crate::gadget::plot_xy(project(&h), graph, 0, 0).unwrap();
    let fit = g.sigmoid(&x, &y).expect("the fit succeeds");
    assert!((fit.value("x0") - 9.0).abs() < 1e-4, "x0 {}", fit.value("x0"));
    assert!((fit.value("dx") - 1.5).abs() < 1e-4, "dx {}", fit.value("dx"));
    h.state_mut().graphs.get_mut(&graph).unwrap().gadget = Some(g);
    settle(&mut h);
    (g.x0, g.x1) = (0.0, 0.5);
    assert!(g.sigmoid(&x, &y).is_none(), "too few points");
    let _ = std::fs::remove_dir_all(&dir);
}
