#!/usr/bin/env python3
"""Fail closed when baseline features drift into optional/domain-specific owners."""

from __future__ import annotations

from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parents[2]


def read(relative: str) -> str:
    path = ROOT / relative
    if not path.is_file():
        raise AssertionError(f"required ownership source is missing: {relative}")
    return path.read_text(encoding="utf-8")


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def main() -> int:
    command_manifest = read("automexia-command-productivity/Cargo.toml")
    connectivity_manifest = read("automexia-connectivity/Cargo.toml")
    devops_manifest = read("automexia-devops/Cargo.toml")
    ui_manifest = read("automexia-ui-model/Cargo.toml")
    app_manifest = read("apps/automexia-terminal/Cargo.toml")

    live_ownership_claims = (
        (
            "apps/automexia-terminal/src/automexia/quick_actions/aliases.rs",
            ("automexia-command-productivity",),
            ("compiler remains in `automexia-devops`",),
        ),
        (
            "docs/DEVOPS-ALIASES.md",
            ("automexia-command-productivity",),
            (
                "`automexia-devops` pure modules",
                "owned by `automexia-devops`",
                "into `automexia-devops`",
            ),
        ),
        (
            "docs/CONNECTION-HUB.md",
            ("automexia-connectivity",),
            ("provider-neutral `automexia-devops`",),
        ),
        (
            "docs/SSH-CONNECTION-AUTOMATION.md",
            ("automexia-connectivity", "automexia-command-productivity"),
            (
                "source model should live in `automexia-devops`",
                "provider-neutral state | `automexia-devops`",
            ),
        ),
        (
            "docs/CONNECTIVITY-COMMAND-PRODUCTIVITY-ROADMAP.md",
            ("automexia-connectivity", "automexia-command-productivity"),
            (
                "automexia-devops model",
                "all `automexia-devops` and `automexia-ui-model` tests",
            ),
        ),
        (
            "docs/ROADMAP.md",
            ("automexia-connectivity", "automexia-command-productivity"),
            (),
        ),
        (
            "docs/PHASE-IMPLEMENTATION-AUDIT.md",
            ("automexia-connectivity", "automexia-command-productivity"),
            (),
        ),
    )
    for relative, required, forbidden in live_ownership_claims:
        source = read(relative)
        for owner in required:
            require(owner in source, f"{relative} is missing current owner {owner}")
        for stale in forbidden:
            require(stale not in source, f"{relative} retains stale owner claim: {stale}")

    require(
        'name = "automexia-command-productivity"' in command_manifest,
        "command productivity must have a capability-free private domain owner",
    )
    require(
        'name = "automexia-connectivity"' in connectivity_manifest,
        "connectivity must have a provider-neutral private domain owner",
    )
    require(
        "automexia-extension-api" in command_manifest,
        "command-productivity benchmark helpers require a direct dev dependency",
    )
    for misplaced in ("src/actions", "src/connections", "src/suggestions"):
        require(
            not (ROOT / "automexia-devops" / misplaced).exists(),
            f"generic feature remains under the DevOps extension: {misplaced}",
        )
    for dependency in ("automexia-command-productivity", "automexia-connectivity"):
        require(
            dependency not in devops_manifest,
            f"DevOps context must not become a facade over {dependency}",
        )
    require(
        "automexia-devops" not in ui_manifest,
        "the generic UI model must not depend on the DevOps context extension",
    )
    for dependency in ("automexia-command-productivity", "automexia-connectivity"):
        require(
            dependency in ui_manifest and dependency in app_manifest,
            f"the UI model and composition root must consume {dependency} directly",
        )

    command_results = read(
        "apps/automexia-terminal/src/renderer/command_results.rs"
    )
    devops_status = read("apps/automexia-terminal/src/renderer/devops_status.rs")
    for forbidden in (
        "automexia_devops",
        "automexia_extension_api",
        "crate::automexia::runtime",
        "ContextContribution",
        "SessionFacts",
    ):
        require(
            forbidden not in command_results,
            f"generic command-result rendering depends on DevOps/extension state: {forbidden}",
        )
    for misplaced in (
        "CommandResultAnchor",
        "CommandResultPulse",
        "render_command_results",
        "RESULT_PULSE_DURATION",
    ):
        require(
            misplaced not in devops_status,
            f"command-result behavior remains mixed into DevOpsStatus: {misplaced}",
        )

    renderer = read("apps/automexia-terminal/src/renderer/mod.rs")
    require(
        "command_results: command_results::CommandResults" in renderer,
        "the renderer lacks a core command-result lifecycle owner",
    )
    require(
        "command_result_states: FxHashMap<usize, command_results::CommandResults>"
        in renderer,
        "inactive panes lack isolated core command-result state",
    )

    devops_start = renderer.find("        if self.devops_enabled {")
    route_guard = renderer.find(
        "        if self.command_result_route != Some(active_route) {",
        devops_start,
    )
    core_render = renderer.find(
        "        self.command_results.render_command_results(",
        route_guard,
    )
    require(
        devops_start >= 0 and route_guard > devops_start and core_render > route_guard,
        "core command-result rendering is missing or ordered inside extension activation",
    )
    require(
        "render_command_results(" not in renderer[devops_start:route_guard],
        "command-result paint is still conditional on DevOps activation",
    )
    sync_start = renderer.find("    pub fn sync_extension_state")
    sync_end = renderer.find("    pub(crate) fn native_test_pane_context", sync_start)
    require(
        sync_start >= 0
        and sync_end > sync_start
        and "command_results.clear()" not in renderer[sync_start:sync_end]
        and "command_result_states.clear()" not in renderer[sync_start:sync_end],
        "disabling DevOps must not clear core command-result state",
    )
    for extension in (
        "devops-aws",
        "devops-azure",
        "devops-gcp",
        "devops-kubernetes",
        "devops-openshift",
        "devops-teleport",
    ):
        manifest = read(f"extensions/{extension}/Cargo.toml")
        require(
            "automexia-devops =" not in manifest,
            f"provider adapter {extension} depends on the unrelated DevOps context extension",
        )
        for dependency in (
            "automexia-command-productivity",
            "automexia-connectivity",
        ):
            require(
                dependency in manifest,
                f"provider adapter {extension} is missing direct {dependency} ownership",
            )

    for engine in ("rio-vt", "teletypewriter", "sugarloaf", "rio-window"):
        manifest = read(f"{engine}/Cargo.toml")
        for domain in (
            "automexia-command-productivity",
            "automexia-connectivity",
            "automexia-devops",
        ):
            require(
                domain not in manifest,
                f"terminal engine {engine} depends on product domain {domain}",
            )

    print("PASS: feature ownership and extension/core boundaries verified")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except AssertionError as error:
        print(f"FAIL: {error}", file=sys.stderr)
        raise SystemExit(1)
