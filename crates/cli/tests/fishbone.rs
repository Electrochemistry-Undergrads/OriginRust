//! `sheaf plot -t fishbone` (T375, Origin 2026's cause-and-effect diagram).

use std::process::Command;

fn sheaf(args: &[&str]) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_sheaf")).args(args).output().expect("the program starts");
    (out.status.success(), String::from_utf8_lossy(&out.stderr).into_owned())
}

#[test]
fn a_cause_and_effect_diagram_names_its_categories_and_causes() {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-fishbone-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let csv = dir.join("causes.csv");
    std::fs::write(&csv, "People,Machines,Methods\ntraining,old press,no checklist\nshifts,,rushed setup\n").unwrap();
    let (png, saved) = (dir.join("f.png"), dir.join("f.sheaf"));
    let (c, p, s) = (csv.to_str().unwrap(), png.to_str().unwrap(), saved.to_str().unwrap());
    // A file of text only needs its header line named (the detection cannot tell it from data).
    let head = ["--header-lines", "1", "--long-name-line", "1"];
    let mut args = vec!["plot", c, "-t", "fishbone", "--effect", "Late deliveries", "-o", p, "--save", s];
    args.extend(head);
    let (ok, e) = sheaf(&args);
    assert!(ok, "{e}");
    let project = sheaf_io::project_file::load(&saved).unwrap();
    let layer = &project.graphs().next().unwrap().layers[0];
    let texts: Vec<&str> = layer.labels.iter().map(|l| l.text.as_str()).collect();
    assert_eq!(
        texts,
        [
            "Late deliveries",
            "People",
            "training",
            "shifts",
            "Machines",
            "old press",
            "Methods",
            "no checklist",
            "rushed setup"
        ]
    );
    // The spine, three ribs and five twigs.
    assert_eq!(layer.shapes.len(), 9);
    assert!(std::fs::metadata(&png).unwrap().len() > 1000);
    // Only the categories named with -y.
    let mut args = vec!["plot", c, "-t", "fishbone", "-y", "Methods", "-o", p, "--save", s];
    args.extend(head);
    let (ok, e) = sheaf(&args);
    assert!(ok, "{e}");
    let project = sheaf_io::project_file::load(&saved).unwrap();
    assert_eq!(project.graphs().next().unwrap().layers[0].labels[0].text, "Effect");
    let (ok, e) = sheaf(&["plot", c, "-t", "line", "--effect", "x", "-o", p]);
    assert!(!ok && e.contains("--effect is for -t fishbone"), "{e}");
}
