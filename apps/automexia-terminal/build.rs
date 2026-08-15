#[cfg(windows)]
fn main() {
    println!("cargo:rerun-if-changed=../../assets/brand/automexia-terminal.ico");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let mut resource = winres::WindowsResource::new();
    resource
        .set_icon("../../assets/brand/automexia-terminal.ico")
        .set("FileDescription", "Automexia Terminal")
        .set("InternalName", "automexia.exe")
        .set("OriginalFilename", "automexia.exe")
        .set("ProductName", "Automexia Terminal");
    resource
        .compile()
        .expect("failed to embed Automexia Windows resources");
}

#[cfg(not(windows))]
fn main() {
    // Keep CoreGraphics optional at load time. Restrict this application-only
    // linker contract to macOS binary targets.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rustc-link-arg-bins=-weak_framework");
        println!("cargo:rustc-link-arg-bins=CoreGraphics");
    }
}
