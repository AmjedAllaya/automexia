#[cfg(windows)]
fn main() {
    use std::path::PathBuf;

    println!("cargo:rerun-if-changed=../../assets/brand/automexia-terminal.ico");
    println!("cargo:rerun-if-changed=../../packaging/windows/automexia.manifest");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let package_version = std::env::var("CARGO_PKG_VERSION")
        .expect("Cargo must provide CARGO_PKG_VERSION to the build script");
    let mut version_fields = package_version
        .split('.')
        .map(|field| field.split(['-', '+']).next().unwrap_or(field))
        .collect::<Vec<_>>();
    version_fields.resize(4, "0");
    let manifest_version = version_fields[..4].join(".");
    let manifest_template = include_str!("../../packaging/windows/automexia.manifest");
    let generated_manifest =
        manifest_template.replace("@AUTOMEXIA_VERSION@", &manifest_version);
    let manifest_path = PathBuf::from(
        std::env::var_os("OUT_DIR")
            .expect("Cargo must provide OUT_DIR to the build script"),
    )
    .join("automexia.manifest");
    std::fs::write(&manifest_path, generated_manifest)
        .expect("failed to generate the Automexia Windows manifest");
    let manifest_path = manifest_path
        .to_str()
        .expect("generated Windows manifest path must be UTF-8");

    let mut resource = winres::WindowsResource::new();
    resource
        .set_manifest_file(manifest_path)
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
