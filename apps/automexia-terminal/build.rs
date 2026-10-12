#[cfg(windows)]
fn main() {
    use std::path::PathBuf;

    println!("cargo:rerun-if-changed=../../assets/brand/automexia-terminal.ico");
    println!("cargo:rerun-if-changed=../../packaging/windows/automexia.manifest");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    // Naga's HLSL writer recursively lowers the bundled grid shaders during
    // DX12 initialization. The Windows 1 MiB main-stack default can overflow
    // in debug builds. Reserve a bounded 8 MiB (committed on demand), without
    // adding a compiler thread or changing stacks in the console launcher.
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        println!("cargo:rustc-link-arg-bin=automexia=/STACK:8388608");
    } else {
        println!("cargo:rustc-link-arg-bin=automexia=-Wl,--stack,8388608");
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
    let output_directory = PathBuf::from(
        std::env::var_os("OUT_DIR")
            .expect("Cargo must provide OUT_DIR to the build script"),
    );
    let manifest_path = output_directory.join("automexia.manifest");
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
    // winres 0.1.12 emits VERSIONINFO maps in randomized HashMap order.
    // Keep its escaping, metadata and compiler discovery, but canonicalize
    // contiguous generated records before compiling the same resource file.
    let resource_path = output_directory.join("resource.rc");
    resource
        .write_resource_file(&resource_path)
        .expect("failed to generate Automexia Windows resources");
    let generated = std::fs::read_to_string(&resource_path)
        .expect("failed to read generated Automexia Windows resources");
    let mut lines: Vec<_> = generated.lines().collect();
    let sortable = |line: &str| {
        [
            "VALUE ",
            "FILEVERSION ",
            "PRODUCTVERSION ",
            "FILEFLAGSMASK ",
            "FILEFLAGS ",
            "FILEOS ",
            "FILETYPE ",
            "FILESUBTYPE ",
        ]
        .iter()
        .any(|prefix| line.starts_with(prefix))
    };
    let mut start = 0;
    while start < lines.len() {
        if !sortable(lines[start]) {
            start += 1;
            continue;
        }
        let mut end = start + 1;
        while end < lines.len() && sortable(lines[end]) {
            end += 1;
        }
        lines[start..end].sort_unstable();
        start = end;
    }
    std::fs::write(&resource_path, lines.join("\n") + "\n")
        .expect("failed to canonicalize Automexia Windows resources");
    resource
        .set_resource_file(
            resource_path
                .to_str()
                .expect("generated Windows resource path must be UTF-8"),
        )
        .compile()
        .expect("failed to embed Automexia Windows resources");

    // winres's link-lib directive attaches resources to the package library.
    // The lightweight console launcher does not link that library, so attach
    // the same compiled resource directly to that binary. Other binaries keep
    // their existing library linkage and must not receive duplicate resources.
    // Successful winres compilation accepts only MSVC or GNU environments.
    let resource_file = if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
    {
        "resource.lib"
    } else {
        "resource.o"
    };
    println!(
        "cargo:rustc-link-arg-bin=amx={}",
        output_directory.join(resource_file).display()
    );
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
