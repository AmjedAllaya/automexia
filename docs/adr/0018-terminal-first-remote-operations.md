# ADR 0018: Terminal-first remote operations

- Status: Accepted for the v0.5+ roadmap
- Date: 2026-08-16

## Context

Automexia aims to cover the remote-access, host inventory, identity, tunnel,
automation, workspace, file, session-memory, collaboration, multi-cloud, and
governance workflows associated with products such as Termius. Copying those
products' screen hierarchy would duplicate GUI complexity and conflict with
Automexia's terminal-native, Vim-speed direction. A raw CLI alone would be
scriptable but would make fuzzy discovery, comparison, preview, and security
review unnecessarily difficult.

The repository already has separate D3-D7 capability/session/provider phases,
CP2-CP6 command-productivity phases, a renderer-independent UI model, native
shell ownership, system-OpenSSH strategy, and external credential-custody
policy. A new product surface must compose those boundaries rather than create
another launcher, vault, command editor, or extension host.

## Decision

1. The canonical automation surface is the `automexia` CLI with stable typed
   domains. `ax` may be offered only as an explicit collision-checked,
   reversible CP3 shorthand.
2. `Ctrl+Shift+P` remains the universal command-palette entry point.
   Interactive use may add a configurable, conflict-checked leader-key command
   mode backed by the same typed action registry; there is no unconditional
   cross-shell leader default. The registry generates palette entries,
   shortcuts, CLI help, accessibility actions, and documentation metadata.
3. Keyboard overlays are used only for fuzzy selection, structured browsing,
   comparison, preview, and explicit confirmation. They remain renderer-owned
   and never write to or resize the PTY grid.
4. Native shells retain editable-buffer, cursor, history, completion, quoting,
   and normal control-key ownership. Terminal cells and remote output are never
   interpreted as trusted command intent.
5. D3-D7 own launch, inventory, identity references, providers, files,
   collaboration, policy, and AI authority. CP2-CP6 own Quick Actions, aliases,
   completion, suggestions, and packs. This decision adds no authority itself.
6. System OpenSSH, official provider CLIs, native agents/keychains, hardware
   tokens, and organization-managed identity stores remain primary. Automexia
   stores opaque secret references and public metadata, not credentials.
7. Every operation exposes exact target, environment, identity reference,
   route, risk, capability, freshness, cancellation, and recovery behavior.
   The default reusable action inserts for review without Enter.
8. All operations have hard resource ceilings, cross-session isolation,
   last-known-good behavior where appropriate, accessible/responsive semantics,
   deterministic tests, native evidence, benchmarks, and cleanup contracts.

The complete command grammar, feature mapping, UI behavior, delivery sequence,
security/performance invariants, and acceptance criteria are in
[Terminal-first remote operations](../TERMINAL-FIRST-OPERATIONS.md).

## Alternatives

- **Copy a Termius-style GUI.** Rejected because it makes common operations
  slower, duplicates screen navigation, and weakens the terminal-first product
  identity. Focused overlays remain available where they materially improve
  safety or comprehension.
- **Expose only shell aliases and scripts.** Rejected because they lack a
  portable typed model for parameters, risk, scope, target review, completion,
  collision handling, capability approval, cancellation, and audit.
- **Build a second command editor inside the renderer.** Rejected because it
  would break native PSReadLine/Readline/ZLE/Fish behavior, IME, history,
  quoting, and accessibility.
- **Store all credentials in an Automexia cloud vault.** Rejected because it
  expands secret custody and duplicates mature agents, hardware, OS stores, and
  organization-managed identity systems.
- **Make every capability a plain CLI stream.** Rejected because route graphs,
  host-key review, large fuzzy lists, file navigation, diffs, and multi-target
  confirmation benefit from bounded keyboard-driven overlays.
- **Enable provider APIs or AI automatically.** Rejected because background
  network/authentication and ambient data access harm latency, privacy,
  resilience, and least privilege.

## Verification

- Architecture policy must prove the registry/model has no renderer, PTY,
  process, network, filesystem, secret, clipboard, or provider authority.
- Registry generation tests must keep CLI, palette, shortcuts, accessibility,
  and reference metadata consistent and collision-free.
- Each D/CP slice supplies its existing model, property, fuzz, mutation,
  security, native-platform, shell, accessibility, performance, leak, cleanup,
  rollback, and documentation evidence before activation.
- Mutation tests must fail if exact-argv, shell ownership, external secret
  custody, target/risk review, resource ceilings, session isolation, or
  extension-disable behavior is removed.
- Roadmap examples remain marked planned until their feature-ledger entry and
  user/reference documentation satisfy the release gate.

## Consequences

Users receive one learnable command language and mnemonic keyboard model across
hosts, actions, workspaces, contexts, files, and logs. Automation and recovery
remain possible without the renderer, while overlays make high-information or
high-risk decisions understandable.

The approach requires a generated action registry, robust shell projections,
more accessibility semantics, and native tests across multiple shells and
operating systems. Some GUI-like features arrive later because secure file
writes, shared PTYs, team synchronization, and AI need independent authority
and threat reviews. These costs are deliberate: convenience cannot bypass the
project's launch, secret, provider, resource, and cross-session boundaries.
