#!/usr/bin/env python3
"""Validate source-to-documentation coverage for public product surfaces."""

from __future__ import annotations

from pathlib import Path
import re
import sys


ROOT = Path(__file__).resolve().parents[2]
CONFIG_STRUCTS = {
    "rio-backend/src/config/mod.rs": {
        "Config",
        "Shell",
        "Scroll",
        "Developer",
        "CursorConfig",
    },
    "rio-backend/src/config/window.rs": {"Window"},
    "rio-backend/src/config/navigation.rs": {"Navigation", "ColorAutomation"},
    "rio-backend/src/config/renderer.rs": {"Renderer"},
    "rio-backend/src/config/keyboard.rs": {"Keyboard"},
    "rio-backend/src/config/layout.rs": {"Panel"},
    "rio-backend/src/config/title.rs": {"Title"},
    "rio-backend/src/config/bell.rs": {"Bell"},
    "rio-backend/src/config/effects.rs": {"Effects"},
    "rio-backend/src/config/hints.rs": {
        "Hints",
        "Hint",
        "HintMouse",
        "HintBinding",
    },
    "rio-backend/src/config/bindings.rs": {"Bindings", "KeyBinding"},
    "rio-backend/src/config/platform.rs": {
        "Platform",
        "PlatformConfig",
        "PlatformWindow",
        "PlatformNavigation",
        "PlatformRenderer",
    },
    "sugarloaf/src/font/fonts.rs": {"SugarloafFonts", "SugarloafFont", "SymbolMap"},
    "rio-vt/src/config/colors/mod.rs": {"Colors"},
    "sugarloaf/src/sugarloaf/primitives.rs": {"ImageProperties"},
}
INTERNAL_CONFIG_KEYS = {"adaptive_colors"}
REQUIRED_PAGES = {
    "docs/index.md",
    "docs/GETTING-STARTED.md",
    "docs/FEATURES.md",
    "docs/CONFIGURATION.md",
    "docs/KEYBOARD.md",
    "docs/CLI-REFERENCE.md",
    "docs/SHELL-INTEGRATION.md",
    "docs/TROUBLESHOOTING.md",
    "docs/PLATFORMS.md",
    "docs/DECISIONS.md",
    "docs/DOCUMENTATION.md",
    "docs/PHASE-IMPLEMENTATION-AUDIT.md",
}


class DocumentationCoverageError(ValueError):
    """The public documentation no longer covers its source contract."""


def struct_body(source: str, name: str) -> str:
    match = re.search(rf"\bpub\s+struct\s+{re.escape(name)}\s*\{{", source)
    if not match:
        raise DocumentationCoverageError(f"could not find public struct {name}")
    start = match.end() - 1
    depth = 0
    for index in range(start, len(source)):
        if source[index] == "{":
            depth += 1
        elif source[index] == "}":
            depth -= 1
            if depth == 0:
                return source[start + 1 : index]
    raise DocumentationCoverageError(f"unterminated public struct {name}")


def serde_keys(body: str) -> set[str]:
    keys: set[str] = set()
    pending_attributes: list[str] = []
    attribute_lines: list[str] = []
    for line in body.splitlines():
        stripped = line.strip()
        if attribute_lines:
            attribute_lines.append(stripped)
            if "]" in stripped:
                pending_attributes.append(" ".join(attribute_lines))
                attribute_lines.clear()
            continue
        if stripped.startswith("#["):
            if "]" in stripped:
                pending_attributes.append(stripped)
            else:
                attribute_lines.append(stripped)
            continue
        field = re.match(r"pub\s+(\w+)\s*:", stripped)
        if not field:
            if stripped and not stripped.startswith("///"):
                pending_attributes.clear()
            continue
        name = field.group(1)
        attributes = " ".join(pending_attributes)
        rename = re.search(r'\brename\s*=\s*"([^"]+)"', attributes)
        keys.add(rename.group(1) if rename else name)
        pending_attributes.clear()
    return keys

def source_config_keys(root: Path = ROOT) -> set[str]:
    keys: set[str] = set()
    for relative, structures in CONFIG_STRUCTS.items():
        source = (root / relative).read_text(encoding="utf-8")
        for structure in structures:
            keys.update(serde_keys(struct_body(source, structure)))
    return keys - INTERNAL_CONFIG_KEYS


def binding_action_names(root: Path = ROOT) -> set[str]:
    source = (root / "apps/automexia-terminal/src/bindings/mod.rs").read_text(
        encoding="utf-8"
    )
    match = re.search(
        r"let action_from_string = match action\.as_str\(\) \{(?P<body>.*?)\n\s*\};",
        source,
        flags=re.DOTALL,
    )
    if not match:
        raise DocumentationCoverageError("could not find binding action registry")
    return set(re.findall(r'^\s*"([a-z0-9]+)"\s*=>', match.group("body"), re.MULTILINE))


def clap_long_flags(body: str) -> set[str]:
    flags: set[str] = set()
    pending_attributes: list[str] = []
    attribute_lines: list[str] = []
    for line in body.splitlines():
        stripped = line.strip()
        if attribute_lines:
            attribute_lines.append(stripped)
            if "]" in stripped:
                pending_attributes.append(" ".join(attribute_lines))
                attribute_lines.clear()
            continue
        if stripped.startswith("#["):
            if "]" in stripped:
                pending_attributes.append(stripped)
            else:
                attribute_lines.append(stripped)
            continue
        field = re.match(r"pub\s+(\w+)\s*:", stripped)
        if not field:
            if stripped and not stripped.startswith("///"):
                pending_attributes.clear()
            continue
        attributes = " ".join(pending_attributes)
        pending_attributes.clear()
        if "subcommand" in attributes or "flatten" in attributes:
            continue
        long_option = re.search(r'\blong(?:\s*=\s*"([^"]+)")?', attributes)
        if not long_option:
            continue
        explicit_name = re.search(r'\bname\s*=\s*"([^"]+)"', attributes)
        name = (
            explicit_name.group(1)
            if explicit_name
            else long_option.group(1) or field.group(1).replace("_", "-")
        )
        flags.add("--" + name)
    return flags


def application_cli_flags(root: Path = ROOT) -> set[str]:
    source = (root / "apps/automexia-terminal/src/cli.rs").read_text(encoding="utf-8")
    return clap_long_flags(struct_body(source, "Cli")) | clap_long_flags(
        struct_body(source, "TerminalOptions")
    )


def application_cli_commands(root: Path = ROOT) -> set[str]:
    source = (root / "apps/automexia-terminal/src/cli.rs").read_text(encoding="utf-8")
    match = re.search(
        r"pub\s+enum\s+CliCommand\s*\{(?P<body>.*?)^\}",
        source,
        flags=re.DOTALL | re.MULTILINE,
    )
    if not match:
        raise DocumentationCoverageError("could not find application command registry")
    variants = re.findall(r"^\s*(\w+)\s*\(", match.group("body"), re.MULTILINE)
    return {
        re.sub(r"(?<!^)(?=[A-Z])", "-", variant).lower()
        for variant in variants
    }


def xtask_commands(root: Path = ROOT) -> set[str]:
    source = (root / "tools/xtask/src/main.rs").read_text(encoding="utf-8")
    match = re.search(r'"usage: cargo xtask <([^\"]+)>"', source)
    if not match:
        raise DocumentationCoverageError("could not find xtask usage registry")
    return {item.strip() for item in re.split(r"\|(?=[a-z])", match.group(1))}


def require_tokens(owner: str, expected: set[str], content: str) -> int:
    folded = content.casefold().replace("\\|", "|")
    missing = sorted(token for token in expected if token.casefold() not in folded)
    if missing:
        raise DocumentationCoverageError(
            f"{owner} is missing source-owned entries: {missing}"
        )
    return len(expected)


def validate_canonical_pages(
    root: Path = ROOT, pages: set[str] = REQUIRED_PAGES
) -> int:
    missing_pages = sorted(path for path in pages if not (root / path).is_file())
    if missing_pages:
        raise DocumentationCoverageError(
            f"missing canonical documentation pages: {missing_pages}"
        )
    for relative in sorted(pages):
        content = (root / relative).read_text(encoding="utf-8")
        headings = [line for line in content.splitlines() if line.startswith("# ")]
        if not content.startswith("# ") or len(headings) != 1:
            raise DocumentationCoverageError(
                f"{relative} must start with exactly one level-one heading"
            )
    return len(pages)

def validate(root: Path = ROOT) -> dict[str, int]:

    config_content = (root / "docs/CONFIGURATION.md").read_text(encoding="utf-8")
    keyboard_content = (root / "docs/KEYBOARD.md").read_text(encoding="utf-8")
    cli_content = (root / "docs/CLI-REFERENCE.md").read_text(encoding="utf-8")

    config_keys = source_config_keys(root)
    actions = binding_action_names(root)
    flags = application_cli_flags(root)
    application_commands = application_cli_commands(root)
    commands = xtask_commands(root)
    counts = {
        "pages": validate_canonical_pages(root),
        "config_keys": require_tokens(
            "configuration reference", config_keys, config_content
        ),
        "binding_actions": require_tokens(
            "keyboard reference", actions, keyboard_content
        ),
        "cli_flags": require_tokens("CLI reference", flags, cli_content),
        "cli_commands": require_tokens(
            "CLI reference",
            {f"automexia {command}" for command in application_commands},
            cli_content,
        ),
        "xtask_commands": require_tokens(
            "CLI reference", {f"cargo xtask {item}" for item in commands}, cli_content
        ),
    }
    return counts

def main() -> int:
    try:
        counts = validate()
    except (DocumentationCoverageError, OSError) as error:
        print(f"documentation coverage validation failed: {error}", file=sys.stderr)
        return 1
    print(
        "PASS: public documentation covers source registries "
        f"(pages={counts['pages']}, config_keys={counts['config_keys']}, "
        f"binding_actions={counts['binding_actions']}, cli_flags={counts['cli_flags']}, "
        f"cli_commands={counts['cli_commands']}, "
        f"xtask_commands={counts['xtask_commands']})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
