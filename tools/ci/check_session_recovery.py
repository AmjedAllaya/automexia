#!/usr/bin/env python3
"""Keep workspace recovery separate from preferences, process and SSH state."""
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
BASE = "apps/automexia-terminal/src/"
FILES = [BASE + path for path in (
    "automexia/session_recovery/mod.rs", "automexia/session_recovery/store.rs",
    "automexia/preferences.rs", "context/recovery.rs", "application/recovery.rs",
    "application.rs", "renderer/confirm_quit.rs",
)]


def validate(sources: dict[str, str]) -> None:
    model = sources[FILES[0]]
    for name, expected in {
        "Snapshot": {"version", "windows"},
        "Window": {"width", "height", "position", "active", "tabs"},
        "Tab": {"title", "color", "root", "focused", "nodes"},
        "Session": {"profile", "cwd", "disconnected"},
    }.items():
        match = re.search(r"pub struct " + name + r"\s*\{(.*?)\n\}", model, re.S)
        assert match and set(re.findall(r"pub\s+(\w+)\s*:", match[1])) == expected, f"Unexpected persisted {name} fields"
    store = sources[FILES[1]]
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
    print("PASS: topology-only recovery ownership and consent boundaries")


if __name__ == "__main__":
    main()
