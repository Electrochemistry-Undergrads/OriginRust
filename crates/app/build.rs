fn main() {
    println!("cargo:rerun-if-changed=assets/icon/windows.rc");
    println!("cargo:rerun-if-changed=assets/icon/sheaf.ico");

    // Build scripts execute on the host. Resource compilation follows the target instead.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_resource::compile_for("assets/icon/windows.rc", ["sheaf-app"], embed_resource::NONE)
            .manifest_required()
            .expect("could not embed Sheaf's Windows icon; install the Windows SDK resource compiler (rc.exe) or configure the resource compiler for the Windows target");
    }
}
