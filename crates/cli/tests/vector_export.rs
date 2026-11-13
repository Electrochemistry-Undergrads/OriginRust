//! `sheaf plot -o fig.eps` and `-o fig.emf` write the vector formats.

use std::path::PathBuf;
use std::process::Command;

#[test]
fn plot_writes_eps_and_emf() {
    let dir: PathBuf = std::env::temp_dir().join(format!("sheaf-cli-vector-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut text = String::from("t,ramp\ns,A\n");
    for i in 0..=10 {
        text.push_str(&format!("{i},{i}\n"));
    }
    let data = dir.join("data.csv");
    std::fs::write(&data, text).unwrap();
    for (name, magic) in [("fig.eps", &b"%!PS-Adobe-3.0 EPSF-3.0"[..]), ("fig.emf", &[1, 0, 0, 0][..])] {
        let out = dir.join(name);
        let run =
            Command::new(env!("CARGO_BIN_EXE_sheaf")).args(["plot"]).arg(&data).arg("-o").arg(&out).output().unwrap();
        assert!(run.status.success(), "{name}: {}", String::from_utf8_lossy(&run.stderr));
        assert!(std::fs::read(&out).unwrap().starts_with(magic), "{name}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}
