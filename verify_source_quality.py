#!/usr/bin/env python3
from __future__ import annotations

import argparse
import ast
from pathlib import Path

ROOT = Path(__file__).resolve().parent


def require(ok: bool, message: str) -> None:
    if not ok:
        raise SystemExit(f"FAIL: {message}")


def read(path: Path) -> str:
    require(path.is_file(), f"missing {path}")
    data = path.read_bytes()
    require(b"\0" not in data, f"NUL byte in source file: {path}")
    return data.decode("utf-8")


def rust_balanced(text: str, label: str) -> None:
    """Lexically balance Rust delimiters while skipping strings/comments.

    This is intentionally not a compiler replacement. It catches truncated or
    malformed generated overlays before a Windows host spends minutes entering
    Cargo. Raw strings, byte strings, chars and nested block comments are
    handled because the overlay uses all of those patterns over time.
    """
    stack: list[tuple[str, int]] = []
    pairs = {')': '(', ']': '[', '}': '{'}
    openers = set(pairs.values())
    i = 0
    n = len(text)
    block_depth = 0
    while i < n:
        if block_depth:
            if text.startswith("/*", i):
                block_depth += 1; i += 2; continue
            if text.startswith("*/", i):
                block_depth -= 1; i += 2; continue
            i += 1; continue
        if text.startswith("//", i):
            end = text.find("\n", i + 2)
            i = n if end < 0 else end + 1
            continue
        if text.startswith("/*", i):
            block_depth = 1; i += 2; continue

        # Rust raw strings: r"...", r#"..."#, br#"..."#, rb#"..."#.
        raw_start = i
        if text.startswith("br", i) or text.startswith("rb", i):
            raw_start = i + 1
        if text.startswith("r", raw_start):
            j = raw_start + 1
            while j < n and text[j] == '#':
                j += 1
            if j < n and text[j] == '"':
                hashes = text[raw_start + 1:j]
                terminator = '"' + hashes
                end = text.find(terminator, j + 1)
                require(end >= 0, f"unterminated Rust raw string in {label}")
                i = end + len(terminator)
                continue

        # Normal/byte/C strings.
        quote_i = i
        if text.startswith('b"', i) or text.startswith('c"', i):
            quote_i = i + 1
        if quote_i < n and text[quote_i] == '"':
            j = quote_i + 1
            while j < n:
                if text[j] == '\\':
                    j += 2; continue
                if text[j] == '"':
                    break
                j += 1
            require(j < n, f"unterminated Rust string in {label}")
            i = j + 1
            continue

        # Character / byte-character literals. Lifetimes like 'a have no
        # closing quote and therefore fall through as ordinary tokens.
        char_i = i + 1 if text.startswith("b'", i) else i
        if char_i < n and text[char_i] == "'":
            j = char_i + 1
            if j < n and text[j] == '\\':
                j += 2
            else:
                j += 1
            if j < n and text[j] == "'":
                i = j + 1
                continue

        ch = text[i]
        if ch in openers:
            stack.append((ch, i))
        elif ch in pairs:
            require(stack and stack[-1][0] == pairs[ch], f"unbalanced {ch} in {label} at byte {i}")
            stack.pop()
        i += 1
    require(block_depth == 0, f"unterminated Rust block comment in {label}")
    if stack:
        ch, pos = stack[-1]
        require(False, f"unclosed Rust delimiter {ch} in {label} at byte {pos}")


def powershell_balanced(text: str, label: str) -> None:
    # Lightweight structural gate. PowerShell itself remains the authoritative
    # parser on Windows, but this catches accidental truncation/quote damage.
    stack: list[str] = []
    pairs = {')': '(', ']': '[', '}': '{'}
    i = 0
    n = len(text)
    quote: str | None = None
    while i < n:
        ch = text[i]
        if quote:
            if quote == '"' and ch == '`':
                i += 2; continue
            if ch == quote:
                if quote == "'" and i + 1 < n and text[i + 1] == "'":
                    i += 2; continue
                quote = None
            i += 1; continue
        if ch == '#':
            end = text.find('\n', i + 1)
            i = n if end < 0 else end + 1
            continue
        if ch == '@' and i + 1 < n and text[i + 1] in ('"', "'"):
            marker = text[i + 1] + '@'
            end = text.find('\n' + marker, i + 2)
            require(end >= 0, f"unterminated PowerShell here-string in {label}")
            line_end = text.find('\n', end + 1 + len(marker))
            i = n if line_end < 0 else line_end + 1
            continue
        if ch in ('"', "'"):
            quote = ch; i += 1; continue
        if ch in '([{':
            stack.append(ch)
        elif ch in pairs:
            require(stack and stack[-1] == pairs[ch], f"unbalanced {ch} in {label}")
            stack.pop()
        i += 1
    require(quote is None, f"unterminated PowerShell quote in {label}")
    if stack:
        require(False, f"unclosed PowerShell delimiter {stack[-1]} in {label}")


def verify_tree(base: Path) -> None:
    rels = [
        "frontends/rioterm/src/automexia/api.rs",
        "frontends/rioterm/src/automexia/runtime.rs",
        "frontends/rioterm/src/automexia/shell.rs",
        "frontends/rioterm/src/automexia/state.rs",
        "frontends/rioterm/src/automexia/theme.rs",
        "frontends/rioterm/src/automexia/ui.rs",
        "frontends/rioterm/src/automexia/builtins/devops/context.rs",
        "frontends/rioterm/src/automexia/builtins/devops/semantics.rs",
        "frontends/rioterm/src/renderer/devops_status.rs",
    ]
    sources = {rel: read(base / rel) for rel in rels}
    for rel, source in sources.items():
        rust_balanced(source, f"{base}/{rel}")

    status = sources["frontends/rioterm/src/renderer/devops_status.rs"]
    require("fn parse_shell_title(" in status, "live prompt calls parse_shell_title without defining it")
    require("request_in_flight" in status and "RefreshSubmission::Busy" in status, "DevOps refresh in-flight/busy state missing")
    require("RefreshSubmission::Busy =>" in status and "self.request_in_flight = false;" in status, "Busy refresh can be mistaken for an in-flight request")
    require("existing.segments = segments;" in status, "freezing a prompt cannot update an existing history snapshot")
    require("active_prompt_key" not in status, "unused live-prompt public API leaked into renderer surface")

    context = sources["frontends/rioterm/src/automexia/builtins/devops/context.rs"]
    require("read_dir(" not in context, "DevOps discovery enumerates directories/providers on its bounded worker")
    require(r"\\wsl.localhost" not in context and r"\\wsl$" not in context, "DevOps discovery touches WSL UNC providers")
    for forbidden in ("std::process::Command", "Command::new(", "std::net::", "TcpStream", "UdpSocket", "reqwest::"):
        require(forbidden not in context, f"DevOps discovery contains forbidden process/network primitive: {forbidden}")

    shell = sources["frontends/rioterm/src/automexia/shell.rs"]
    require("-encodedcommand" in shell, "PowerShell normalizer does not protect encoded-command invocations")
    require("has_no_exit" in shell, "PowerShell normalizer can duplicate -NoExit")


def verify_package_sources() -> None:
    # Python parser gate.
    for path in ROOT.glob("*.py"):
        source = read(path)
        ast.parse(source, filename=str(path))

    # PowerShell structural gate.
    for path in ROOT.glob("*.ps1"):
        powershell_balanced(read(path), str(path))

    bash = read(ROOT / "shell-integration/bash/automexia.bash")
    zsh = read(ROOT / "shell-integration/zsh/automexia.zsh")
    ps = read(ROOT / "shell-integration/powershell/automexia.ps1")
    require("base64 -w0" not in bash and "base64 -w0" not in zsh, "shell metadata relies on GNU-only base64 -w0")
    require("PROMPT_COMMAND+=(__automexia_pre_prompt)" in bash, "Bash integration does not append to array PROMPT_COMMAND")
    require("${PROMPT_COMMAND%';'}" in bash, "Bash string PROMPT_COMMAND trailing-semicolon normalization missing")
    require("local status=$?" in bash and 'return "$status"' in bash, "Bash Automexia hook does not preserve prompt status")
    require("$'\\xCE\\xBB'" in bash and r"\xCE\xBB" in zsh and "[char]0x03BB" in ps, "portable lambda prompt encoding missing")

    installer = read(ROOT / "INSTALL-SHELL-INTEGRATION-WINDOWS.ps1")
    require("ToBase64String" in installer and "base64 -d | sh" in installer, "WSL installer is not UTF-8-safe")
    require("$PROFILE.CurrentUserCurrentHost" in installer, "PowerShell installer guesses profile path")
    shell_check = read(ROOT / "CHECK-SHELL-INTEGRATION-WINDOWS.ps1")
    require('throw "PowerShell integration is stale' in shell_check and 'throw "PowerShell integration file missing' in shell_check, "shell checker reports PowerShell failures without failing")
    require('throw "WSL integration check failed' in shell_check, "shell checker reports WSL failure without failing")

    direct_apply = read(ROOT / "APPLY-AUTOMEXIA.ps1")
    for verifier in ("verify_warning_cleanup.py", "verify_architecture.py", "verify_devops_extension.py", "verify_visual_system.py", "verify_source_quality.py"):
        require(verifier in direct_apply, f"direct apply does not post-verify with {verifier}")
    require("exit $LASTEXITCODE" not in direct_apply, "direct apply can terminate a dot-sourced PowerShell host")

    patcher = read(ROOT / "apply_automexia.py")
    require("SemanticPrompt::PromptContinuation" in patcher and "semantic_live_anchor" in patcher, "patcher lacks semantic-row live prompt anchor")
    require("matches!(sq.c(), '\\\\0' | ' ')" in patcher, "patcher prompt-row blank-cell literal is malformed")

    # Bootstrap resume safety: every overlay file must be considered
    # Automexia-managed, otherwise a perfectly valid repeated bootstrap can
    # misclassify our own shell/theme files as unrelated user modifications.
    bootstrap = read(ROOT / "BOOTSTRAP-WINDOWS.ps1")
    for overlay_file in (ROOT / "overlay").rglob("*"):
        if not overlay_file.is_file():
            continue
        relative = str(overlay_file.relative_to(ROOT / "overlay")).replace("\\", "/")
        require(relative in bootstrap.replace("\\", "/"), f"bootstrap managed-path list misses overlay file: {relative}")
    require("automexia/release-audited-v0.3.13" in bootstrap, "bootstrap branch does not identify the audited release line")

    # Source transformation and source verification have separate ownership:
    # bootstrap/direct-apply normalize generated Rust once with the checkout's
    # pinned rustfmt; build/check/dev remain non-mutating verification gates.
    formatter = read(ROOT / "FORMAT-WINDOWS.ps1")
    require("cargo fmt --all" in formatter, "formatter does not normalize generated Rust")
    require("cargo fmt --all -- --check" in formatter, "formatter does not verify normalization")
    require("[switch]$CheckOnly" in formatter, "formatter lacks non-mutating check mode")

    direct_apply = read(ROOT / "APPLY-AUTOMEXIA.ps1")
    require("FORMAT-WINDOWS.ps1" in direct_apply, "direct apply does not normalize generated Rust when Cargo is available")
    require("FORMAT-WINDOWS.ps1" in bootstrap, "bootstrap does not normalize generated Rust before source gates")
    require(
        bootstrap.index("FORMAT-WINDOWS.ps1") < bootstrap.index("Verifying warning-clean source transforms"),
        "bootstrap verifies source before rustfmt-normalizing generated Rust",
    )
    require(
        direct_apply.index("FORMAT-WINDOWS.ps1") < direct_apply.index("verify_warning_cleanup.py"),
        "direct apply verifies source before rustfmt-normalizing generated Rust",
    )

    build = read(ROOT / "BUILD-WINDOWS.ps1")
    check = read(ROOT / "CHECK-WINDOWS.ps1")
    dev = read(ROOT / "DEV-WINDOWS.ps1")
    for name, script in (("BUILD-WINDOWS.ps1", build), ("CHECK-WINDOWS.ps1", check), ("DEV-WINDOWS.ps1", dev)):
        require("FORMAT-WINDOWS.ps1" in script and "-CheckOnly" in script, f"{name} does not use non-mutating rustfmt verification")
        require("cargo fmt --all\n" not in script, f"{name} mutates formatting instead of checking generated source")
        require("verify_source_quality.py" in script, f"{name} does not run source-quality gate")
    for name, script in (("BUILD-WINDOWS.ps1", build), ("CHECK-WINDOWS.ps1", check)):
        require("cargo clippy" in script and "-D warnings" in script, f"{name} does not deny warnings in clippy gate")
        require("verify_shell_behavior.py" in script, f"{name} does not run shell-behavior gate")
    require("exit $LASTEXITCODE" not in check, "check script can terminate a dot-sourced PowerShell host")


def main() -> None:
    parser = argparse.ArgumentParser(description="Verify Automexia source/shell/release quality invariants")
    parser.add_argument("project_root", nargs="?", type=Path)
    args = parser.parse_args()
    verify_package_sources()
    verify_tree(ROOT / "overlay")
    if args.project_root:
        verify_tree(args.project_root.resolve())
    print("PASS: source lexical integrity, prompt lifecycle/retry invariants, non-blocking WSL discovery, shell portability, and release-gate contracts verified")


if __name__ == "__main__":
    main()
