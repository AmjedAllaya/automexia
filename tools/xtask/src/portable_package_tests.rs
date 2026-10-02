//! Portable package contents, resource bounds, and staging preservation.

use super::*;

#[test]
fn portable_resource_copy_is_bounded_and_preserves_layout() {
    let temporary = tempfile::tempdir().unwrap();
    let source = temporary.path().join("source");
    let destination = temporary.path().join("destination");
    fs::create_dir_all(source.join("nested")).unwrap();
    fs::write(source.join("root.ps1"), b"root").unwrap();
    fs::write(source.join("nested").join("child.cmd"), b"child").unwrap();

    copy_bounded_resource_tree(&source, &destination, 2, 9).unwrap();
    assert_eq!(fs::read(destination.join("root.ps1")).unwrap(), b"root");
    assert_eq!(
        fs::read(destination.join("nested").join("child.cmd")).unwrap(),
        b"child"
    );

    let error =
        copy_bounded_resource_tree(&source, &temporary.path().join("too-small"), 1, 9)
            .unwrap_err();
    assert!(error.contains("file-count or byte ceiling"));
}

#[test]
fn portable_packages_preserve_all_runtime_binaries_and_target_names() {
    // Windows ships bsdtar with ZIP support; GNU tar uses tar.gz here.
    let native_extension = if cfg!(windows) { "zip" } else { "tar.gz" };
    for (target, extension, suffix) in [
        ("x86_64-pc-windows-msvc", native_extension, ".exe"),
        ("aarch64-pc-windows-msvc", "tar.gz", ".exe"),
        ("x86_64-unknown-linux-gnu", "tar.gz", ""),
        ("aarch64-unknown-linux-gnu", native_extension, ""),
    ] {
        let temporary = tempfile::tempdir().unwrap();
        let source = temporary.path().join("release inputs");
        let output = temporary.path().join("package output");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&output).unwrap();
        let names = [
            "automexia",
            "amx",
            "automexia-suggestion-helper",
            "automexia-ssh-helper",
        ];
        for name in names {
            let path = source.join(format!("{name}{suffix}"));
            fs::write(&path, name).unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
            }
        }
        if target.contains("windows") {
            for relative in ["conpty.dll", "x64/OpenConsole.exe", "arm64/OpenConsole.exe"]
            {
                let asset = source.join(relative);
                fs::create_dir_all(asset.parent().unwrap()).unwrap();
                fs::write(asset, relative.as_bytes()).unwrap();
            }
        }
        let identity = product_identity().unwrap();
        portable_archive(
            &identity,
            target,
            &source.join(format!("automexia{suffix}")),
            &output,
            extension,
        )
        .unwrap();
        let archive = output.join(format!(
            "{}-{}-{target}.{extension}",
            identity.package_name, identity.version
        ));
        let extracted = temporary.path().join("extracted");
        fs::create_dir_all(&extracted).unwrap();
        assert!(Command::new("tar")
            .arg("-xf")
            .arg(archive)
            .arg("-C")
            .arg(&extracted)
            .status()
            .unwrap()
            .success());
        if target.contains("windows") {
            for relative in ["conpty.dll", "x64/OpenConsole.exe", "arm64/OpenConsole.exe"]
            {
                assert_eq!(
                    fs::read(extracted.join(relative)).unwrap(),
                    relative.as_bytes()
                );
            }
            assert!(!extracted.join("OpenConsole.exe").exists());
        }
        for name in names {
            let path = extracted.join(format!("{name}{suffix}"));
            assert_eq!(
                fs::read(&path).unwrap(),
                name.as_bytes(),
                "{target}: {name} must retain the exact supplied bytes"
            );
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                assert_eq!(
                    fs::metadata(path).unwrap().permissions().mode() & 0o777,
                    0o755
                );
            }
        }
    }
}

#[test]
fn portable_packages_reject_missing_conpty_before_replacing_staging() {
    for missing in ["conpty.dll", "x64/OpenConsole.exe", "arm64/OpenConsole.exe"] {
        let temporary = tempfile::tempdir().unwrap();
        let source = temporary.path().join("release");
        let output = temporary.path().join("packages");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(output.join("portable")).unwrap();
        let retained = output.join("portable/retained.txt");
        fs::write(&retained, b"previous package").unwrap();
        for relative in [
            "automexia.exe",
            "amx.exe",
            "automexia-suggestion-helper.exe",
            "automexia-ssh-helper.exe",
            "conpty.dll",
            "x64/OpenConsole.exe",
            "arm64/OpenConsole.exe",
        ] {
            if relative != missing {
                let asset = source.join(relative);
                fs::create_dir_all(asset.parent().unwrap()).unwrap();
                fs::write(asset, relative.as_bytes()).unwrap();
            }
        }
        assert!(portable_archive(
            &product_identity().unwrap(),
            "x86_64-pc-windows-msvc",
            &source.join("automexia.exe"),
            &output,
            "zip"
        )
        .is_err());
        assert_eq!(fs::read(retained).unwrap(), b"previous package");
    }
}

#[test]
fn portable_packages_reject_missing_companions_before_replacing_staging() {
    for missing in [
        "amx.exe",
        "automexia-suggestion-helper.exe",
        "automexia-ssh-helper.exe",
    ] {
        let temporary = tempfile::tempdir().unwrap();
        let source = temporary.path().join("release");
        let output = temporary.path().join("packages");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(output.join("portable")).unwrap();
        let retained = output.join("portable/retained.txt");
        fs::write(&retained, b"previous package").unwrap();
        for name in [
            "automexia.exe",
            "amx.exe",
            "automexia-suggestion-helper.exe",
            "automexia-ssh-helper.exe",
        ] {
            if name != missing {
                fs::write(source.join(name), name).unwrap();
            }
        }
        assert!(portable_archive(
            &product_identity().unwrap(),
            "x86_64-pc-windows-msvc",
            &source.join("automexia.exe"),
            &output,
            "zip",
        )
        .is_err());
        assert_eq!(fs::read(retained).unwrap(), b"previous package");
    }
}
