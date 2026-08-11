#!/usr/bin/env python3
from __future__ import annotations

import os
import shutil
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent
BASH_SCRIPT = ROOT / "shell-integration/bash/automexia.bash"
UNIX_INSTALLER = ROOT / "install-shell-integration.sh"


def require(ok: bool, message: str) -> None:
    if not ok:
        raise SystemExit(f"FAIL: {message}")


def run_bash(script: str, *args: str, env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
    bash = shutil.which("bash")
    require(bash is not None, "bash runtime disappeared during shell behavior verification")
    return subprocess.run(
        [bash, "--noprofile", "--norc", "-c", script, "_", *args],
        text=True,
        capture_output=True,
        env=env,
    )


def main() -> None:
    require(BASH_SCRIPT.is_file(), f"missing {BASH_SCRIPT}")
    require(UNIX_INSTALLER.is_file(), f"missing {UNIX_INSTALLER}")

    bash = shutil.which("bash")
    sh = shutil.which("sh")
    if bash is None or sh is None:
        print("SKIP: bash/sh runtime unavailable; source-quality gate still verifies shell integration statically")
        return

    with tempfile.TemporaryDirectory(prefix="automexia-shell-verify-") as tmp:
        tmp_path = Path(tmp)
        home = tmp_path / "home"
        home.mkdir()
        env = os.environ.copy()
        env.update({"HOME": str(home), "TERM_PROGRAM": "Automexia"})

        result = run_bash(
            r'''
set -e
source "$1" >/dev/null
[[ "$PS1" == *"133;A"* ]]
[[ "$PS1" == *"133;P;k=c"* ]]
[[ "$PS0" == *"automexia_prompt_active=MA=="* ]]
printf '%s' "$PS1" | grep -q 'λ'
''',
            str(BASH_SCRIPT),
            env=env,
        )
        require(result.returncode == 0, "Bash prompt lifecycle/lambda expansion failed: " + result.stderr)

        result = run_bash(
            r'''
USER_STATUS=999
PROMPT_COMMAND='USER_STATUS=$?;'
source "$1" >/dev/null
false
eval "$PROMPT_COMMAND" >/dev/null
[[ "$USER_STATUS" -eq 1 ]]
[[ "$PROMPT_COMMAND" == USER_STATUS=*';__automexia_pre_prompt' ]]
''',
            str(BASH_SCRIPT),
            env=env,
        )
        require(result.returncode == 0, "Bash string PROMPT_COMMAND status/order preservation failed: " + result.stderr)

        result = run_bash(
            r'''
seen=999
first_hook(){ seen=$?; return "$seen"; }
PROMPT_COMMAND=(first_hook)
source "$1" >/dev/null
source "$1" >/dev/null
count=0
for hook in "${PROMPT_COMMAND[@]}"; do
  [[ $hook == __automexia_pre_prompt ]] && ((++count)) || true
done
[[ $count -eq 1 ]]
false
for hook in "${PROMPT_COMMAND[@]}"; do "$hook" >/dev/null; done
[[ $seen -eq 1 ]]
''',
            str(BASH_SCRIPT),
            env=env,
        )
        require(result.returncode == 0, "Bash array PROMPT_COMMAND idempotency/order failed: " + result.stderr)

        installer_env = os.environ.copy()
        installer_env.update({"HOME": str(home), "XDG_CONFIG_HOME": str(home / ".config")})
        for _ in range(2):
            result = subprocess.run(
                [sh, str(UNIX_INSTALLER)],
                text=True,
                capture_output=True,
                env=installer_env,
            )
            require(result.returncode == 0, "Unix shell installer failed: " + result.stderr)

        bashrc = (home / ".bashrc").read_text(encoding="utf-8")
        zshrc = (home / ".zshrc").read_text(encoding="utf-8")
        require(bashrc.count("# >>> AUTOMEXIA SHELL INTEGRATION >>>") == 1, "Unix Bash install is not idempotent")
        require(zshrc.count("# >>> AUTOMEXIA SHELL INTEGRATION >>>") == 1, "Unix Zsh install is not idempotent")
        require((home / ".config/automexia/shell-integration.bash").read_bytes() == BASH_SCRIPT.read_bytes(), "installed Bash integration differs from package source")
        require((home / ".config/automexia/shell-integration.zsh").read_bytes() == (ROOT / "shell-integration/zsh/automexia.zsh").read_bytes(), "installed Zsh integration differs from package source")

    print("PASS: Bash prompt lifecycle, exit-status preservation, UTF-8 lambda, hook idempotency, and Unix shell installer behavior verified")


if __name__ == "__main__":
    main()
