//! Gadgets > Rise Time (T138, analysis-tools "Rise Time gadget").

use super::*;
use crate::gadget::GadgetKind;

#[test]
fn the_rise_time_gadget_measures_the_step_inside_its_range() {
    let dir = std::env::temp_dir().join(format!("sheaf-app-ui-rise-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("step.csv");
    let mut text = String::from("t,v\n");
    for i in 0..=60 {
        let t = f64::from(i) * 0.1;
        text.push_str(&format!("{t},{}\n", ((t - 2.0) * 5.0).clamp(0.0, 10.0)));
    }
    std::fs::write(&path, text).unwrap();
    let mut h = start(vec![path]);
    let ctx = h.ctx.clone();
    let graph = project(&h).graphs().next().unwrap().id;
    h.state_mut().apply(Action::StartRiseTime { graph }, &ctx);
    settle(&mut h);
    let g = h.state().graphs[&graph].gadget.expect("the gadget is open");
    assert_eq!(g.kind, GadgetKind::RiseTime);
    // Over the whole curve the ramp rises from 10 % at x = 2.2 to 90 % at x = 3.8.
    let mut g = g;
    (g.x0, g.x1) = (0.0, 6.0);
    h.state_mut().graphs.get_mut(&graph).unwrap().gadget = Some(g);
    settle(&mut h);
    let (x, y) = crate::gadget::plot_xy(project(&h), graph, 0, 0).unwrap();
    let r = g.rise(&x, &y).unwrap();
    assert!((r.time - 1.6).abs() < 1e-9);
    let _ = std::fs::remove_dir_all(&dir);
}
