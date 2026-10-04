#!/usr/bin/env python3
"""Keep workspace recovery separate from preferences, process and SSH state."""
from pathlib import Path
import re
import hashlib
import tomllib

ROOT = Path(__file__).resolve().parents[2]
BASE = "apps/automexia-terminal/src/"
FILES = [BASE + path for path in (
    "automexia/session_recovery/mod.rs", "automexia/session_recovery/store.rs",
    "automexia/preferences.rs", "context/recovery.rs", "application/recovery.rs",
    "application.rs", "renderer/confirm_quit.rs",
    "automexia/session_recovery/protection.rs", "automexia/session_recovery/history.rs",
    "automexia/session_recovery/protection/macos.rs",
)]

# Reviewed, modified dependency sources. Never certify these as registry bytes.
VENDOR_HASHES = {
    "Cargo.toml": "abaa7acf69355e3ee754b05b7205b480d98f64ec4c6b82481033c528262a5efb",
    "LICENSE-APACHE": "a9040321c3712d8fd0b09cf52b17445de04a23a10165049ae187cd39e5c86be5",
    "LICENSE-MIT": "a07fcacc3c60de4dc0fab10ac9d6aaba7379974e28451c99da7f7df09c25b28c",
    "README.md": "8a0d31460aca51513a49ccaf074127e711ac43e7fbc66f15b26dce820880fb31",
    "src/errors.rs": "00f87579dfb9d81d3b9ec33bb55fb6f3c493cca87e50ce69ac48e54c4eb62fec",
    "src/inout.rs": "be58c7768fc882cf8d1a4d2d563ea7f43a0d52468db02e63df94f6cbb98e7609",
    "src/inout_buf.rs": "ca69448df3ad726fc36a0cd7d4f0735cb0b0dc8f4c737147cfd3ebe7c0062661",
    "src/lib.rs": "a81e26cd5514ec7ef9e66b22d9fddea00749cdda53550025f808042d32d61f53",
    "src/reserved.rs": "5cc5f7738f3745878e4ce8bb503171c08728c05638034bb6bedde8120fe12476",
    "tests/reserved-buffer.rs": "b4b14a43494c677b8f739bc0420cf32916cc09ad15a8029683fb65d5da7e3ab9",
    "tests/split-inout.rs": "8c6ca509b2c1f656ac1821299e189f1fb568aae2ec34512ca2c2c4f8962ff2c2",
    "UPSTREAM.md": "3a4353212051b8adbb214148c394c90eb9e6f623603996c704401d2fcf453d68",
}


def validate_vendor(files: dict[str, str], manifest: str, lock: str, policy: str) -> None:
    assert set(files) == set(VENDOR_HASHES), "Reviewed dependency file inventory changed"
    for path, digest in VENDOR_HASHES.items():
        assert hashlib.sha256(files[path].encode()).hexdigest() == digest, f"Review required for {path}"
    cargo = tomllib.loads(manifest)
    assert cargo["patch"]["crates-io"]["inout"] == {"path": "third-party/inout"}, "Reviewed dependency override missing"
    assert "third-party/inout" in cargo["workspace"]["members"], "Dependency regression tests must run in workspace gates"
    packages = [p for p in tomllib.loads(lock)["package"] if p["name"] == "inout"]
    assert len(packages) == 1 and packages[0]["version"] == "0.2.2" and "source" not in packages[0], "Only reviewed local InOut is allowed"
    assert tomllib.loads(policy)["policy"]["inout"]["audit-as-crates-io"] is False, "Local source must not masquerade as registry certification"


def vendor_inputs(root: Path = ROOT) -> tuple[dict[str, str], str, str, str]:
    vendor = root / "third-party/inout"
    paths = list(vendor.rglob("*"))
    assert len(paths) <= 32 and not any(p.is_symlink() for p in paths), "Unexpected dependency inventory"
    files = {p.relative_to(vendor).as_posix(): p.read_text(encoding="utf-8") for p in paths if p.is_file()}
    return (files, *( (root / p).read_text(encoding="utf-8") for p in ("Cargo.toml", "Cargo.lock", "supply-chain/config.toml") ))


def validate_dependencies(manifest: str) -> None:
    cargo = tomllib.loads(manifest)
    native = dict(cargo.get("dependencies", {}))
    native.update(cargo.get("target", {}).get('cfg(not(target_arch = "wasm32"))', {}).get("dependencies", {}))
    for name in ("base64", "flate2", "serde", "serde_json", "sha2", "zeroize"):
        assert name in native, f"Shared recovery dependency {name} must be available in native production builds"
        dependency = native[name]
        assert not isinstance(dependency, dict) or not dependency.get("optional", False), f"Shared recovery dependency {name} must not be optional"


def validate(sources: dict[str, str]) -> None:
    model = sources[FILES[0]]
    for name, expected in {
        "Snapshot": {"version", "windows"},
        "Window": {"width", "height", "position", "active", "tabs"},
        "Tab": {"title", "color", "root", "focused", "nodes"},
        "Session": {"profile", "cwd", "disconnected", "history", "source"},
        "Checkpoint": {"version", "snapshot", "significant_activity", "incomplete_restore"},
    }.items():
        match = re.search(r"pub struct " + name + r"\s*\{(.*?)\n\}", model, re.S)
        assert match and set(re.findall(r"pub\s+(\w+)\s*:", match[1])) == expected, f"Unexpected persisted {name} fields"
    assert re.search(r"#\[serde\(skip\)\]\s*pub source:", model), "Live capture capability must never be persisted"
    protection = sources[FILES[7]]
    assert "CryptProtectData" in protection and "CRYPTPROTECT_UI_FORBIDDEN" in protection
    assert "CRYPTPROTECT_LOCAL_MACHINE" not in protection, "Recovery must remain user scoped"
    assert "XChaCha20Poly1305" in protection and "getrandom::fill(&mut nonce)" in protection
    assert "MAX_PLAIN_BYTES + 1" in protection, "Decompression must be bounded"
    macos = sources[FILES[9]]
    assert "SecKeychainSetUserInteractionAllowed(0)" in macos and "allowed == 0" in macos
    assert "OnceLock<Result<(), StoreError>>" in macos, "Native no-dialog policy must be initialized once"
    for forbidden in ("SecKeychainUnlock", "SecItemUpdate", "SecItemDelete", "SecKeychainSetUserInteractionAllowed(1)"):
        assert forbidden not in macos, "Recovery must not unlock, overwrite, delete or enable native dialogs"
    store = sources[FILES[1]]
    assert re.search(r"protection\s*\.borrow_mut\(\)\s*\.seal", store) and re.search(r"protection\s*\.borrow_mut\(\)\s*\.open", store), "Disk history must be encrypted"
    assert "retire_previous" in store and "retire_pending = false" in store

    for required in ('"session-v1"',
                     "WriteLock::try_acquire", "atomic_write_private", "read_bounded_regular"):
        assert required in store, f"Recovery storage owner missing {required}"
    assert re.search(r'BoundedWorker::new\(\s*"session-recovery",\s*1,', store)
    for source in (model, store):
        for forbidden in ("std::process::Command", "Command::new", "TcpStream", "SessionLaunchDescriptor", "UserPreferences"):
            assert forbidden not in source, f"Recovery data/store gained execution or preference authority: {forbidden}"
    assert "session_recovery" not in sources[FILES[2]], "Recovery data must not enter UserPreferences"
    context = sources[FILES[3]]
    assert "Self::create_context" in context and "current_config.shell.clone()" in context
    assert "config.use_fork = false" in context, "Restore must use per-session directory-aware Unix spawning"
    assert "integration_scope_active" in context and "disconnected: remote" in context
    application = sources[FILES[5]]
    assert "startup_config.defer_initial_pty" in application
    assert "self.poll_recovery(event_loop)" in application
    assert "self.checkpoint_recovery(true)" in application
    coordinator = sources[FILES[4]]
    assert "take_recovery_choice" in coordinator and "Duration::from_millis(100)" in coordinator
    assert "service.prepare(snapshot)" in coordinator and "MAX_SESSIONS" in coordinator
    dialog = sources[FILES[6]]
    assert "ElementState::Released" in dialog and "take_recovery_choice" in dialog


def main() -> None:
    validate({path: (ROOT / path).read_text(encoding="utf-8") for path in FILES})
    validate_vendor(*vendor_inputs())
    validate_dependencies((ROOT / "apps/automexia-terminal/Cargo.toml").read_text(encoding="utf-8"))
    print("PASS: bounded encrypted display recovery ownership and consent boundaries")


if __name__ == "__main__":
    main()
