#!/usr/bin/env python3
"""Compare two clean, unsigned native builds of an immutable Git revision."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tarfile
import tempfile
import time

RUNTIMES = ("automexia", "amx", "automexia-suggestion-helper", "automexia-ssh-helper")
TARGETS = frozenset(("x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu",
                     "x86_64-pc-windows-msvc", "aarch64-pc-windows-msvc",
                     "x86_64-apple-darwin", "aarch64-apple-darwin"))


class RuntimeComparisonError(ValueError):
    """Public diagnostics contain only fixed runtime/stage names and byte offsets."""

    def __init__(self, runtime: str, reason: str, stage: str, offset: int | None = None):
        labels = {"missing": "missing regular runtime", "size": "invalid runtime size",
                  "different": "non-reproducible unsigned runtime"}
        names = {name + suffix for name in RUNTIMES for suffix in ("", ".exe")}
        if (runtime not in names or reason not in labels
                or stage not in ("comparison", "cold-builds", "packaged-inputs")
                or offset is not None and (type(offset) is not int or not 0 <= offset <= 512 * 1024 * 1024)):
            raise ValueError("invalid comparison diagnostic")
        detail = "" if offset is None else f"; first differing byte={offset}"
        super().__init__(f"{labels[reason]}: {runtime}; stage={stage}{detail}")


def build_environment(source: Path, target: str, epoch: int) -> dict[str, str]:
    if target not in TARGETS:
        raise ValueError("unsupported reproducibility target")
    env = dict(os.environ)
    for name in ("RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER"):
        env.pop(name, None)
    # Cargo/rustc may canonicalize symlinks and Windows short paths. Map both
    # spellings and keep build outputs under the canonical root (macOS /var
    # temporary directories commonly resolve beneath /private/var).
    logical_source = source.absolute()
    source = source.resolve()
    prefixes = dict.fromkeys((logical_source, source))
    flags = [f"--remap-path-prefix={prefix}=/usr/src/automexia" for prefix in prefixes]
    if "windows" in target:
        # Encoded flags replace Cargo's target config, including static CRT.
        flags.extend(("--cfg=windows_slim_errors", "-Clink-arg=/Brepro", "-Ctarget-feature=+crt-static"))
    elif "linux" in target:
        flags.append("-Clink-arg=-Wl,--build-id=sha1")
    else:
        # rustc source remapping does not cover the Apple linker's OSO debug
        # map. Absolute object paths change LC_UUID even after symbols are
        # stripped (and consequently change the ARM64 ad-hoc signature).
        # Relativize that metadata before linking; retain content-derived UUIDs
        # and the unchanged, strict all-byte verification of every runtime.
        flags.append("-Clink-arg=-Wl,-oso_prefix,.")
        if target == "aarch64-apple-darwin":
            flags.append("-Ctarget-cpu=apple-m1")
    env.update(CARGO_ENCODED_RUSTFLAGS="\x1f".join(flags), CARGO_INCREMENTAL="0",
               CARGO_TARGET_DIR=str(source / "target"),
               CARGO_BUILD_BUILD_DIR=str(source / "target/build"),
               SOURCE_DATE_EPOCH=str(epoch), TZ="UTC", LC_ALL="C.UTF-8")
    return env


def compare_binaries(first: Path, second: Path, target: str, *, stage: str = "comparison") -> dict:
    if target not in TARGETS:
        raise ValueError("unsupported reproducibility target")
    results = {}
    for runtime in RUNTIMES:
        name = runtime + (".exe" if "windows" in target else "")
        left, right = first / name, second / name
        if not all(path.is_file() and not path.is_symlink() for path in (left, right)):
            raise RuntimeComparisonError(name, "missing", stage)
        size = left.stat().st_size
        if not 0 < size <= 512 * 1024 * 1024:
            raise RuntimeComparisonError(name, "size", stage)
        digest = hashlib.sha256()
        with left.open("rb") as before, right.open("rb") as after:
            remaining = size
            while remaining:
                chunk = before.read(min(1024 * 1024, remaining))
                other = after.read(len(chunk))
                if not chunk or chunk != other:
                    differing = next((index for index, pair in enumerate(zip(chunk, other))
                                      if pair[0] != pair[1]), min(len(chunk), len(other)))
                    raise RuntimeComparisonError(name, "different", stage, size - remaining + differing)
                digest.update(chunk)
                remaining -= len(chunk)
            if before.read(1) or after.read(1):
                raise RuntimeComparisonError(name, "different", stage, size)
        results[name] = {"sha256": digest.hexdigest(), "size": size}
    return results


def capture(root: Path, *args: str) -> str:
    return subprocess.check_output(args, cwd=root, text=True, timeout=60).strip()


def run(root: Path, target: str, output: Path, reference: Path | None = None) -> None:
    root = root.resolve()
    if target not in TARGETS:
        raise ValueError("unsupported reproducibility target")
    host = next((line[6:] for line in capture(root, "rustc", "-vV").splitlines()
                 if line.startswith("host: ")), "")
    if host != target:
        raise ValueError("reproducibility requires the native host target")
    commit = capture(root, "git", "rev-parse", "--verify", "HEAD")
    if not re.fullmatch(r"[0-9a-f]{40}", commit):
        raise ValueError("invalid source revision")
    # Source and epoch always come from this commit; dirty local changes cannot
    # silently enter evidence labelled as a committed release.
    epoch = int(capture(root, "git", "log", "-1", "--format=%ct", commit))
    durations = []
    with tempfile.TemporaryDirectory(prefix="automexia-repro-") as temporary:
        staging = Path(temporary)
        archive = staging / "source.tar"
        subprocess.run(["git", "archive", "--format=tar", "--output", str(archive), commit],
                       cwd=root, check=True, timeout=120)
        for attempt in (1, 2):
            source = staging / str(attempt)
            source.mkdir()
            with tarfile.open(archive) as content:
                content.extractall(source, filter="data")
            started = time.monotonic()
            subprocess.run(["cargo", "build", "--release", "--locked", "-p", "automexia-terminal",
                            "--target", target], cwd=source, env=build_environment(source, target, epoch),
                           check=True, timeout=7200)
            durations.append(round(time.monotonic() - started, 3))
        results = compare_binaries(staging / "1/target" / target / "release",
                                   staging / "2/target" / target / "release", target, stage="cold-builds")
        if reference is not None:
            compare_binaries(staging / "1/target" / target / "release", reference, target, stage="packaged-inputs")
    main = results["automexia.exe" if "windows" in target else "automexia"]
    report = {"schema": 1, "result": "pass", "target": target, "commit": commit,
              "source_date_epoch": epoch, **main, "runtimes": results,
              "cold_build_seconds": durations, "matches_packaged_inputs": reference is not None, "scope": "unsigned native runtimes; excludes signatures and notarization"}
    output.parent.mkdir(parents=True, exist_ok=True)
    # Keep a previous report intact on failure, and publish only a complete one.
    temporary_output = None
    try:
        with tempfile.NamedTemporaryFile(mode="w", encoding="utf-8", dir=output.parent,
                                         prefix=".repro-", delete=False) as stream:
            temporary_output = Path(stream.name)
            json.dump(report, stream, indent=2, sort_keys=True)
            stream.write("\n")
        os.replace(temporary_output, output)
    finally:
        if temporary_output is not None:
            temporary_output.unlink(missing_ok=True)
    print(f"PASS: four unsigned runtimes are byte-identical on {target}")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path, nargs="?")
    parser.add_argument("--target", required=True, choices=sorted(TARGETS))
    parser.add_argument("--reference", type=Path, help="exact unsigned inputs selected for packaging")
    parser.add_argument("--build-once", action="store_true", help="build release inputs with the same deterministic flags")
    args = parser.parse_args()
    try:
        root = Path(__file__).resolve().parents[2]
        if args.build_once:
            if args.output is not None or args.reference is not None:
                parser.error("build-once does not generate reproducibility evidence")
            host = next((line[6:] for line in capture(root, "rustc", "-vV").splitlines()
                         if line.startswith("host: ")), "")
            if host != args.target:
                raise ValueError("release build requires the native host target")
            epoch = int(capture(root, "git", "log", "-1", "--format=%ct", "HEAD"))
            subprocess.run(["cargo", "build", "--release", "--locked", "-p", "automexia-terminal",
                            "--target", args.target], cwd=root, env=build_environment(root, args.target, epoch),
                           check=True, timeout=7200)
        else:
            if args.output is None:
                parser.error("reproducibility evidence output is required")
            run(root, args.target, args.output, args.reference)
    except RuntimeComparisonError as error:
        parser.exit(1, f"Native reproducibility verification failed: {error}.\n")
    except (ValueError, OSError, tarfile.TarError, subprocess.SubprocessError):
        # Build logs stay with the private CI run; the public status carries no
        # machine paths, environment values, or command arguments.
        parser.exit(1, "Native reproducibility verification failed; inspect private build evidence.\n")


if __name__ == "__main__":
    main()
