use super::*;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;

const HELPER: &str = "windows::conpty::loader_tests::native_loader_helper";

fn bounded_status(mut command: Command, timeout: Duration) -> bool {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("native fixture command starts");
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(10));
            }
            _ => {
                let _ = child.kill();
                child.wait().expect("timed-out fixture process is reaped");
                return false;
            }
        }
    }
}

fn compile_fixture(directory: &Path, variant: &str) -> PathBuf {
    std::fs::create_dir(directory).expect("fixture directory created");
    let source = directory.join("fixture.rs");
    std::fs::write(
        &source,
        include_str!("../../tests/fixtures/conpty-loader.rs"),
    )
    .expect("fixture source written");
    let output = directory.join("fixture.dll");
    let compiler = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let mut command = Command::new(compiler);
    // Keep the repository's pinned toolchain; only helper children change CWD.
    command
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args([
            "--crate-type=cdylib",
            "--edition=2021",
            "--crate-name=conpty_fixture",
            "-Cpanic=abort",
            "-Cdebuginfo=0",
            "--cfg",
            variant,
        ])
        .arg(&source)
        .arg("-o")
        .arg(&output);
    assert!(
        bounded_status(command, Duration::from_secs(120)),
        "native fixture compilation failed: {variant}"
    );
    output
}

#[test]
fn native_loader_restricts_discovery_and_releases_every_owned_reference() {
    let root = tempfile::tempdir().expect("isolated loader fixture root");
    let variants = [
        "complete",
        "missing_create",
        "missing_resize",
        "missing_close",
        "create_failure",
        "needs_dependency",
        "dependency",
    ];
    let fixtures: Vec<_> = variants
        .iter()
        .map(|variant| compile_fixture(&root.path().join(variant), variant))
        .collect();
    let current_exe = std::env::current_exe().expect("test executable available");
    let cases = [
        ("absent", None),
        ("cwd_only", Some(0)),
        ("path_only", Some(0)),
        ("load_control", Some(0)),
        ("lifetime", Some(0)),
        ("missing_create", Some(1)),
        ("missing_resize", Some(2)),
        ("missing_close", Some(3)),
        ("create_failure", Some(4)),
        ("dependency_sibling", Some(5)),
        ("dependency_cwd", Some(5)),
        ("dependency_path", Some(5)),
        ("malformed", None),
        ("incompatible_image", Some(0)),
    ];
    let mut failures = Vec::new();
    for (case, fixture) in cases {
        let case_root = root.path().join(case);
        let app = case_root.join("application");
        let untrusted = case_root.join("untrusted");
        let empty = case_root.join("empty");
        for directory in [&app, &untrusted, &empty] {
            std::fs::create_dir_all(directory).expect("case directory created");
        }
        let helper = app.join("loader-helper.exe");
        std::fs::copy(&current_exe, &helper).expect("isolated test executable copied");
        if let Some(index) = fixture {
            let destination = if matches!(case, "cwd_only" | "path_only") {
                &untrusted
            } else {
                &app
            };
            std::fs::copy(&fixtures[index], destination.join("conpty.dll"))
                .expect("inert DLL fixture copied");
        }
        if case.starts_with("dependency_") {
            let destination = if case == "dependency_sibling" {
                &app
            } else {
                &untrusted
            };
            std::fs::copy(
                &fixtures[6],
                destination.join("conpty_fixture_dependency.dll"),
            )
            .expect("inert dependency copied");
        }
        if case == "malformed" {
            std::fs::write(app.join("conpty.dll"), b"inert invalid image")
                .expect("malformed fixture written");
        } else if case == "incompatible_image" {
            let path = app.join("conpty.dll");
            let mut bytes = std::fs::read(&path).expect("inert PE fixture read");
            let offset =
                u32::from_le_bytes(bytes[0x3c..0x40].try_into().unwrap()) as usize;
            // An obsolete MIPS machine type is incompatible with supported hosts.
            bytes[offset + 4..offset + 6].copy_from_slice(&0x0166u16.to_le_bytes());
            std::fs::write(path, bytes).expect("incompatible PE fixture written");
        }
        let mut command = Command::new(helper);
        command
            .args(["--ignored", "--exact", HELPER, "--test-threads=1"])
            .env("AUTOMEXIA_LOADER_FIXTURE", case)
            .env(
                "PATH",
                if matches!(case, "path_only" | "dependency_path") {
                    &untrusted
                } else {
                    &empty
                },
            )
            .current_dir(if matches!(case, "cwd_only" | "dependency_cwd") {
                &untrusted
            } else {
                &empty
            });
        let passed = bounded_status(command, Duration::from_secs(20))
            && std::fs::read(app.join("fixture-complete"))
                .is_ok_and(|bytes| bytes == b"executed native loader helper");
        eprintln!("native loader case {case}: {passed}");
        if !passed {
            failures.push(case);
        }
        // Each helper has exited before removing its owned tree; keep only one
        // copied executable at a time, even when multiple scenarios fail.
        std::fs::remove_dir_all(&case_root).expect("case artifacts removed");
    }
    root.close().expect("all native fixture artifacts removed");
    assert!(
        failures.is_empty(),
        "native loader cases failed: {failures:?}"
    );
}

fn sibling_module_is_loaded() -> bool {
    let path = std::env::current_exe()
        .expect("helper executable available")
        .with_file_name("conpty.dll");
    let path: Vec<_> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    // SAFETY: NUL-terminated owned buffer lives through this observational call.
    // GetModuleHandleW returns a borrowed handle; it must never be freed here.
    !unsafe { GetModuleHandleW(path.as_ptr()) }.is_null()
}

fn assert_inbox_fallback() {
    let api = ConptyApi::new();
    assert!(std::ptr::fn_addr_eq(
        api.create,
        CreatePseudoConsole as CreatePseudoConsoleFn
    ));
    assert!(std::ptr::fn_addr_eq(
        api.resize,
        ResizePseudoConsole as ResizePseudoConsoleFn
    ));
    assert!(std::ptr::fn_addr_eq(
        api.close,
        ClosePseudoConsole as ClosePseudoConsoleFn
    ));
    drop(api);
    assert!(!sibling_module_is_loaded());
}

fn fixture_conpty(api: ConptyApi, sentinel: &mut usize) -> Conpty {
    let mut handle = 0;
    // SAFETY: only the inert fixture receives this fake handle; it preserves the
    // caller-owned sentinel, which remains live until Conpty::drop returns.
    let result = unsafe {
        (api.create)(
            COORD { X: 80, Y: 24 },
            (sentinel as *mut usize).cast(),
            ptr::null_mut(),
            0,
            &mut handle,
        )
    };
    assert_eq!(result, S_OK);
    Conpty {
        handle,
        api,
        managed_job: None,
    }
}

#[test]
#[ignore = "executed only by the isolated native loader controller"]
fn native_loader_helper() {
    let case = std::env::var("AUTOMEXIA_LOADER_FIXTURE")
        .expect("isolated controller selects a fixture");
    match case.as_str() {
        "load_control" | "dependency_sibling" => {
            let api = ConptyApi::load_conpty().expect("sibling fixture loads");
            let mut sentinel = 0;
            drop(fixture_conpty(api, &mut sentinel));
            assert_eq!(sentinel, 0xC105E);
        }
        "lifetime" => {
            for _ in 0..16 {
                let first = ConptyApi::load_conpty().expect("first owner loads");
                let second = ConptyApi::load_conpty().expect("second owner loads");
                let (mut one, mut two) = (0, 0);
                let first = fixture_conpty(first, &mut one);
                let second = fixture_conpty(second, &mut two);
                assert!(sibling_module_is_loaded());
                drop(first);
                assert_eq!(one, 0xC105E);
                assert!(sibling_module_is_loaded());
                // SAFETY: the second API owner must retain the native module.
                assert_eq!(
                    unsafe { (second.api.resize)(second.handle, COORD { X: 80, Y: 24 }) },
                    S_OK
                );
                std::thread::scope(|scope| {
                    scope
                        .spawn(move || drop(second))
                        .join()
                        .expect("cross-thread close completes");
                });
                assert_eq!(two, 0xC105E);
                assert!(!sibling_module_is_loaded());
            }
        }
        "create_failure" => {
            assert!(
                new(Some("fixture.exe"), None, &None, None, true, false, 80, 24).is_err()
            );
            assert!(!sibling_module_is_loaded());
        }
        _ => {
            assert!(ConptyApi::load_conpty().is_none());
            assert!(!sibling_module_is_loaded());
            assert_inbox_fallback();
        }
    }
    // An exact-name invocation with zero tests also exits successfully. Require
    // this fresh marker so the controller proves the helper actually executed.
    let marker = std::env::current_exe()
        .expect("helper executable available")
        .with_file_name("fixture-complete");
    std::fs::write(marker, b"executed native loader helper")
        .expect("helper completion marker written");
}

#[test]
fn bundled_path_rejects_relative_root_only_and_embedded_nul_paths() {
    for path in [
        "automexia.exe",
        "C:automexia.exe",
        r"\automexia.exe",
        r"C:\",
        "C:\\app\0\\automexia.exe",
        "C:\\app\\automexia\0.exe",
    ] {
        assert_eq!(
            bundled_conpty_path(Path::new(path)).unwrap_err().kind(),
            std::io::ErrorKind::InvalidInput
        );
    }
}

#[test]
fn bundled_path_keeps_native_unicode_units_and_uses_backslashes() {
    use std::os::windows::ffi::OsStringExt;
    for path in [
        r"C:\automexia.exe",
        "C:/app/automexia.exe",
        "C:\\term-\u{1f680}\\automexia.exe",
    ] {
        let expected = Path::new(path).with_file_name("conpty.dll");
        let mut expected: Vec<_> = expected
            .as_os_str()
            .encode_wide()
            .map(|unit| if unit == 0x2f { 0x5c } else { unit })
            .collect();
        expected.push(0);
        assert_eq!(bundled_conpty_path(Path::new(path)).unwrap(), expected);
    }
    let native = std::ffi::OsString::from_wide(&[0x43, 0x3a, 0x5c, 0xd800, 0x5c, 0x61]);
    let mut expected = vec![0x43, 0x3a, 0x5c, 0xd800, 0x5c];
    expected.extend("conpty.dll".encode_utf16());
    expected.push(0);
    assert_eq!(bundled_conpty_path(Path::new(&native)).unwrap(), expected);
}

#[test]
fn bundled_path_checks_both_input_and_final_utf16_limits() {
    let directory = format!("C:\\{}", "a".repeat(MAX_NATIVE_STRING_UNITS - 15));
    let accepted = Path::new(&directory).join("app.exe");
    let path = bundled_conpty_path(&accepted).unwrap();
    assert_eq!(path.len(), MAX_NATIVE_STRING_UNITS);
    assert_eq!(path.last(), Some(&0));
    let over_output = PathBuf::from(format!("{directory}a")).join("app.exe");
    assert_eq!(
        bundled_conpty_path(&over_output).unwrap_err().kind(),
        std::io::ErrorKind::InvalidInput
    );
    let over_input =
        PathBuf::from(format!("C:\\{}", "a".repeat(MAX_NATIVE_STRING_UNITS)));
    assert_eq!(
        bundled_conpty_path(&over_input).unwrap_err().kind(),
        std::io::ErrorKind::InvalidInput
    );
}
