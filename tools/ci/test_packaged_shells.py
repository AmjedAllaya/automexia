#!/usr/bin/env python3
"""Exercise shipped Unix prompt hooks with empty settings; never print shell data."""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import shutil
import tempfile

from test_shell_location_hints import run_prompt_fixture


def verify(root: Path) -> list[dict]:
    required = ("install-unix.sh", "posix/automexia-os.sh", "bash/automexia.bash",
                "bash/session.bash", "bash/login-session.bash", "zsh/automexia.zsh",
                "zsh/session/.zshenv", "fish/automexia.fish")
    for name in required:
        path = root / name
        if not path.is_file() or path.is_symlink():
            raise ValueError(f"missing regular packaged resource: {name}")
    reports = []
    with tempfile.TemporaryDirectory(prefix="automexia-packaged-shells-") as temporary:
        for shell in ("bash", "zsh", "fish"):
            if not shutil.which(shell):
                raise ValueError(f"required native shell unavailable: {shell}")
            frames, elapsed = run_prompt_fixture(
                shell, ["/fixture/first", "", "/fixture/restored"], repeat=3,
                no_encoder=True, inspect_identity=True,
                identity_environment={"WSL_DISTRO_NAME": ""},
                integration_root=root, fixture_parent=Path(temporary))
            if [len(group) for group in frames] != [1, 1, 1, 3]:
                raise ValueError(f"{shell}: missing startup or cached prompt frames")
            for group, expected in zip(frames, ("/fixture/first", "", "/fixture/restored", "/fixture/restored")):
                for frame in group:
                    if (frame.get("automexia_shell") != "1"
                            or frame.get("automexia_shell_name") != shell
                            or frame.get("automexia_env_KUBECONFIG") != expected
                            or frame.get("automexia_distro") != ""):
                        raise ValueError(f"{shell}: packaged metadata did not update or replay")
            reports.append({"shell": shell, "metadata_and_cache": "pass",
                            "scenario_ms": round(elapsed * 1000, 2)})
    return reports


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", type=Path)
    args = parser.parse_args()
    print(json.dumps(verify(args.root.resolve())))
