#!/usr/bin/env python3
from __future__ import annotations

import argparse
import re
from pathlib import Path

WINDOWS_UTIL = Path("rio-window/src/platform_impl/windows/util.rs")
SUGARLOAF = Path("sugarloaf/src/renderer/mod.rs")
TITLE = Path("frontends/rioterm/src/context/title.rs")
DEVOPS_MOD = Path("frontends/rioterm/src/automexia/builtins/devops/mod.rs")

BAD_WINDOWS_NAMES = (
    "dwStyle",
    "bMenu",
    "dwExStyle",
    "pointerId",
    "entriesCount",
    "pointerCount",
    "pointerInfo",
    "pointerDeviceRect",
    "displayRect",
    "touchInfo",
    "pointId",
    "penInfo",
)


def require_file(root: Path, relative: Path) -> str:
    path = root / relative
    if not path.is_file():
        raise SystemExit(f"warning-clean verification: missing Rio source file: {path}")
    return path.read_text(encoding="utf-8")


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Verify the Automexia warning-clean transforms in an applied Rio checkout"
    )
    parser.add_argument("project_root", type=Path)
    args = parser.parse_args()
    root = args.project_root.resolve()

    util = require_file(root, WINDOWS_UTIL)
    block_match = re.search(
        r'pub type AdjustWindowRectExForDpi\s*=.*?'
        r'pub type GetPointerPenInfo\s*=\s*unsafe extern "system" fn\(.*?\)\s*->\s*BOOL;',
        util,
        flags=re.DOTALL,
    )
    if not block_match:
        raise SystemExit("warning-clean verification: Win32 dynamic function-pointer block not found")
    ffi_block = block_match.group(0)
    remaining = [name for name in BAD_WINDOWS_NAMES if re.search(rf"\b{re.escape(name)}\b", ffi_block)]
    if remaining:
        raise SystemExit(
            "warning-clean verification: non-snake-case Win32 parameters remain: "
            + ", ".join(remaining)
        )

    sugar = require_file(root, SUGARLOAF)
    old_irrefutable = re.findall(
        r"if let ImageTexture::Wgpu \{ view, \.\. \} = &(?:bg_tex|img)\.gpu \{",
        sugar,
    )
    if old_irrefutable:
        raise SystemExit(
            f"warning-clean verification: {len(old_irrefutable)} irrefutable WGPU if-let site(s) remain"
        )
    cleaned_wgpu = re.findall(
        r"let ImageTexture::Wgpu \{ view, \.\. \} = &(?:bg_tex|img)\.gpu;",
        sugar,
    )
    if len(cleaned_wgpu) != 3:
        raise SystemExit(
            f"warning-clean verification: expected 3 cleaned WGPU let sites, found {len(cleaned_wgpu)}"
        )

    title = require_file(root, TITLE)
    context_marker = "#[cfg(not(unix))]\n    let _ = context;"
    path_marker = "#[cfg(not(unix))]\n    let _ = path;"
    if context_marker not in title:
        raise SystemExit("warning-clean verification: Windows context unused-variable guard is missing")
    if path_marker not in title:
        raise SystemExit("warning-clean verification: Windows path unused-variable guard is missing")

    devops_mod = require_file(root, DEVOPS_MOD)
    bad_reexports = [name for name in ("KubernetesContext", "WslContext") if re.search(rf"pub use model::\{{[^}}]*\b{name}\b", devops_mod)]
    if bad_reexports:
        raise SystemExit(
            "warning-clean verification: unused DevOps public re-export(s) remain: "
            + ", ".join(bad_reexports)
        )
    obsolete_icon_constants = re.findall(r"pub const ICON_[A-Z_]+\s*:", devops_mod)
    if obsolete_icon_constants:
        raise SystemExit(
            "warning-clean verification: obsolete unused DevOps string icon constants remain: "
            + ", ".join(icon.replace("pub const ", "").replace(":", "") for icon in obsolete_icon_constants)
        )

    required_reexports = ("CloudContext", "DevOpsSnapshot")
    missing_reexports = [name for name in required_reexports if name not in devops_mod]
    if missing_reexports:
        raise SystemExit(
            "warning-clean verification: required DevOps renderer/runtime re-export(s) missing: "
            + ", ".join(missing_reexports)
        )

    print("PASS: applied checkout contains all reported warning cleanups, including DevOps public surface and obsolete icon constants")


if __name__ == "__main__":
    main()
