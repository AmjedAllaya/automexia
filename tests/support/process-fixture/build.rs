use std::{env, ffi::OsString, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=src/standalone.rs");
    println!("cargo:rerun-if-changed=src/workload.rs");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let output =
        PathBuf::from(env::var_os("OUT_DIR").expect("Cargo fixture output directory"))
            .join("automexia-process-test-fixture.exe");
    let source = PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR").expect("Cargo fixture source directory"),
    )
    .join("src/standalone.rs");
    let mut compiler = Command::new(env::var_os("RUSTC").expect("Cargo target compiler"));
    if let Some(linker) = env::var_os("RUSTC_LINKER") {
        let mut option = OsString::from("linker=");
        option.push(linker);
        compiler.arg("-C").arg(option);
    }
    if let Ok(flags) = env::var("CARGO_ENCODED_RUSTFLAGS") {
        compiler.args(flags.split('\x1f').filter(|flag| !flag.is_empty()));
    }
    let status = compiler
        .args(["--crate-name", "automexia_process_test_fixture_child"])
        .args(["--edition=2021", "--target"])
        .arg(env::var_os("TARGET").expect("Cargo fixture target triple"))
        .arg("-o")
        .arg(&output)
        .arg(source)
        .status()
        .expect("start the process fixture compiler");
    assert!(
        status.success(),
        "process fixture compiler exited with {status}"
    );
    println!(
        "cargo:rustc-env=AUTOMEXIA_PROCESS_TEST_FIXTURE_EXECUTABLE={}",
        output
            .to_str()
            .expect("Cargo fixture output path must be UTF-8")
    );
}
