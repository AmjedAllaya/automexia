#!/usr/bin/env python3
"""Create one private, content-addressed CP3.1 generation for native shell tests."""

from __future__ import annotations

import argparse
import hashlib
import os
from pathlib import Path
import re


FILES = {
    "powershell": "automexia-aliases.ps1",
    "bash": "automexia-aliases.bash",
    "zsh": "automexia-aliases.zsh",
    "fish": "automexia-aliases.fish",
    "cmd": "automexia-aliases.doskey",
}


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def private_directory(path: Path) -> None:
    path.mkdir(exist_ok=True)
    if path.is_symlink() or not path.is_dir():
        raise ValueError(f"unsafe fixture directory: {path}")
    os.chmod(path, 0o700)


def private_file(path: Path, data: bytes) -> None:
    if path.exists() or path.is_symlink():
        raise ValueError(f"fixture file already exists: {path}")
    path.write_bytes(data)
    os.chmod(path, 0o600)


def artifacts(name: str, value: str) -> dict[str, bytes]:
    return {
        "powershell": f"Set-Alias -Name '{name}' -Value 'Write-Output' -Scope Global\n".encode(),
        "bash": f"alias {name}='printf %s {value}'\n".encode(),
        "zsh": f"alias {name}='printf %s {value}'\n".encode(),
        "fish": f"function {name}; printf %s {value}; end\n".encode(),
        "cmd": f"{name}=echo {value} $*\n".encode(),
    }


def create(
    config_root: Path,
    name: str,
    value: str,
    generator: str = "automexia-devops/0.4.0",
) -> str:
    if not config_root.is_absolute():
        raise ValueError("config root must be absolute")
    if not re.fullmatch(r"[a-z][a-z0-9-]{1,31}", name):
        raise ValueError("fixture alias name is invalid")
    if not re.fullmatch(r"[a-z0-9-]{1,32}", value):
        raise ValueError("fixture value is invalid")
    if not re.fullmatch(r"automexia-devops/[0-9]+\.[0-9]+\.[0-9]+", generator):
        raise ValueError("fixture generator is invalid")

    private_directory(config_root)
    generated = config_root / "generated"
    private_directory(generated)
    root = generated / "aliases"
    private_directory(root)
    generations = root / "generations"
    private_directory(generations)

    bodies = artifacts(name, value)
    shell_lines = []
    for shell, file_name in FILES.items():
        body = bodies[shell]
        shell_lines.append(
            f"shell={shell}|{file_name}|{sha256(body)}|{'b' * 64}|1|0|{name}|"
        )
    manifest = (
        "automexia-alias-generation-v1\n"
        "schema=1\n"
        "source-revision=1\n"
        f"source-digest={'a' * 64}\n"
        f"generator={generator}\n"
        + "\n".join(shell_lines)
        + "\n"
    ).encode()
    generation = sha256(manifest)
    generation_root = generations / generation
    if generation_root.exists():
        if generation_root.is_symlink() or not generation_root.is_dir():
            raise ValueError("unsafe existing fixture generation")
    else:
        private_directory(generation_root)
        for shell, file_name in FILES.items():
            shell_root = generation_root / shell
            private_directory(shell_root)
            private_file(shell_root / file_name, bodies[shell])
        private_file(generation_root / "generation.manifest", manifest)

    current = root / "current"
    temporary = root / ".current.fixture.tmp"
    if temporary.exists() or temporary.is_symlink():
        temporary.unlink()
    private_file(temporary, f"{generation}\n".encode())
    os.replace(temporary, current)
    os.chmod(current, 0o600)
    print(generation)
    return generation


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--config-root", type=Path, required=True)
    parser.add_argument("--alias", required=True)
    parser.add_argument("--value", required=True)
    parser.add_argument("--generator", default="automexia-devops/0.4.0")
    arguments = parser.parse_args()
    create(
        arguments.config_root,
        arguments.alias,
        arguments.value,
        arguments.generator,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())