# Rio upstream audit — Automexia Terminal v0.3.2

Audit date: 2026-08-10.  
Pinned terminal-engine commit: `7d595af583f6ef1ea6036a66b367ba1e5a84d4a2`.

## Base relationship

The original connected fork `AmjedAllaya/rio` was at `a0835a7d7e6db607a702f24fa96337ecd566e78e` when the v0.2 audit was performed. Official `raphamorim/rio` commit `7d595af...` is a clean descendant containing the audited fixes used as the Automexia 0.x engine baseline.

**v0.3 changes the bootstrap policy:** stable builds no longer start from moving `origin/main`. A fresh integration branch is created directly from the exact audited SHA. This prevents future daily Rio/fork changes from silently altering the Automexia release source.

## Important fixes in the pinned baseline

- `b24a75e` — Windows startup-window flash correction;
- `9193685` — Windows foreground-process lookup correction;
- `3c205fc` — hollow cursor for unfocused windows;
- `d52809a` — safer teardown around dead contexts;
- `bec04f5` — color emoji/bitmap rendering correction;
- `167e6c1` + `6abd2b7` — CBLC/CBDT color-bitmap selection hardening/tests;
- `680c713` — macOS input/IME corrections;
- `7d595af` — macOS nil input-context panic avoidance;
- `f2efba0` — progress-bar placement correction;
- `4ebcb61` — macOS navigation-strip adjustment;
- `990d654` — Wayland background-effect protocol support;
- `690c889` — upstream 0.5.20 release preparation.

## Automexia-owned changes layered on the baseline

v0.3 includes:

- Automexia Windows keyboard defaults;
- command-palette `/market` mode;
- `frontends/rioterm/src/automexia/` product/application boundary;
- extension capability manifests;
- persisted activation state with Automexia namespace and v0.2 compatibility;
- bounded extension worker for asynchronous DevOps discovery;
- cached status HUD and semantic row highlighting;
- cached current-directory field on the render snapshot;
- warning-clean transforms reported from Windows builds;
- transactional/idempotent integration and architecture verification.

Automexia still does **not** put marketplace/DevOps behavior into VT parsing, PTY/ConPTY transport, font shaping or low-level renderer backends.

## Future updates

See `UPSTREAM-STRATEGY.md`. Upstream changes are evaluated on explicit compatibility branches and adopted only when the full Automexia gates pass.
