# Source ownership and dependency rules

This file is the contributor-facing map for deciding **where code belongs**. The architecture is intentionally stricter than a feature-by-feature folder layout: terminal correctness, rendering latency, application UX, platform integration and extensions are different ownership domains.

## Ownership domains

| Domain | Owns | Must not own |
|---|---|---|
| Terminal protocol/state | VT parsing, terminal model, scrollback, selection protocol state | marketplace, cloud/project discovery, UI policy |
| Session/transport | PTY/ConPTY lifecycle, reads/writes, child process lifecycle | renderer UI, extension discovery |
| Font/text core | font lookup, shaping, rasterization, glyph/cache primitives | app commands, extension lifecycle |
| Renderer core/backend | damage, draw models, GPU resources, backend-specific submission | filesystem/network/process discovery |
| Application/runtime | windows/tabs/splits, commands, app state, engine adapters | raw VT parsing or GPU backend internals |
| Platform adapters | Windows/macOS/Linux native integration | cross-platform business policy |
| Automexia extension platform | manifests, capabilities, lifecycle, catalog, cached extension models | mutable PTY/parser/GPU internals |
| Extension worker/sandbox | bounded capability work outside latency-critical threads | direct renderer mutation |

## Dependency direction

```text
platform adapters ─┐
commands / UX ─────┼──► application/runtime ───► terminal engine contracts
extension UI ──────┘             │
                                 ├──► renderer adapter/model API
extension runtime ───────────────┘
        │
        ▼
capability worker / future sandbox
```

Forbidden dependency directions:

```text
VT/parser ─X─► automexia marketplace
PTY/ConPTY ─X─► extensions
GPU backend ─X─► filesystem discovery
extension ─X─► mutable parser/PTY/GPU objects
renderer hot path ─X─► disk/network/process APIs
```

## Current v0.3 Automexia ownership

```text
frontends/rioterm/src/automexia/
├── api.rs           engine-independent contracts and capability vocabulary
├── runtime.rs       lifecycle, activation generation, cached snapshots, worker dispatch
├── state.rs         Automexia-owned persistence and migration
├── marketplace.rs   catalog-facing data model only
├── ui.rs            generic semantic-prompt contribution anchors + bounded UI contracts
├── theme.rs         Automexia named/ANSI palette and cross-shell visual roles
├── shell.rs         shell launch normalization only (no command framework)
└── builtins/
    ├── mod.rs       first-party registration boundary
    └── devops/
        ├── mod.rs        manifest/capabilities/public boundary
        ├── model.rs      renderer-neutral snapshot model
        ├── context.rs    background local discovery + sanitization
        └── semantics.rs  pure terminal-row semantic classifier

frontends/rioterm/src/renderer/devops_status.rs
└── render adapter that consumes cached Automexia models only and draws only on generic semantic-prompt anchors
```

The v0.2 `frontends/rioterm/src/extensions/` namespace and the v0.3.0 monolithic `builtins/devops.rs` are intentionally retired. Do not reintroduce either as a second owner for extension behavior. DevOps discovery and semantic classification remain separate services even though they share one first-party extension manifest.

## Review checklist

Before approving a change, ask:

1. Does it touch terminal bytes/state? If yes, it belongs in the terminal-engine domain and needs protocol/conformance tests.
2. Does it touch GPU resources or damage? If yes, keep the logic renderer-owned and input it through models/snapshots.
3. Does it perform filesystem, process or network work? If yes, it must be outside render/PTY latency-critical paths and eventually go through a capability broker.
4. Is it product policy, command UX or extension lifecycle? If yes, it belongs to Automexia application/runtime.
5. Is it OS-specific? If yes, put it behind a platform adapter rather than scattering `cfg` behavior through business logic.
6. Does an extension import a concrete Rio renderer/PTY/parser type? If yes, reject the dependency and add/narrow an Automexia contract instead.

7. Does extension UI use a hard-coded window/top-strip coordinate? If yes, reject it and route geometry through Automexia application chrome/layout.
8. Does default UI require a private-use/Nerd Font glyph to be understandable? If yes, provide a standard-symbol/text fallback or renderer-owned vector asset.

## Visual-system ownership

The terminal/application layer owns named/ANSI palette mapping and generic semantic-prompt UI geometry. Shell-specific prompt/editor glue lives only under the package `shell-integration/` directory and communicates through standard terminal metadata/colors; terminal engine, VT, PTY and GPU modules must not import PowerShell/Bash/Zsh profile logic. Explicit application RGB colors remain application-owned.
