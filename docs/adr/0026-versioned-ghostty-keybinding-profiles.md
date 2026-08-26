# ADR 0026: Versioned Ghostty keybinding profiles

- Status: accepted
- Date: 2026-08-23
- Owners: Automexia maintainers
- Supersedes: ADR 0010 for opt-in compatibility profiles
- Preserves: ADR 0011 for the implicit Automexia profile

## Context

Automexia's classic keybindings are the implicit and tested default. A strict
Ghostty compatibility profile cannot be maintained safely as another copied
frontend vector: Ghostty 1.3.1 includes prefixes, performability, sequences,
tables, chains, platform-specific defaults, and actions whose availability
depends on application state. The existing frontend also parses some actions
from strings and scans a flat binding vector for each key event.

Ghostty 1.3.1 was released on 2026-03-13. Its official `v1.3.1` tag resolves to
commit `22efb0be2bbea73e5339f5426fa3b20edabcaa11`. Compatibility fixtures must be
generated from a reviewed binary built from that exact source, not inferred
from current documentation or copied from a moving branch.

## Decision

Automexia adopts a private, renderer-independent `automexia-keybindings` crate.
It owns bounded typed actions, triggers, predicates, scopes, origins, policies,
profile composition, diagnostics, direct lookup, reverse lookup, sequence
tries, key tables, and per-surface sequence/table state. It has no filesystem,
process, environment, network, PTY, window, renderer, GPU, clipboard, or secret
authority.

`keyboard.binding-profile` accepts `automexia`, `ghostty`, and
`ghostty-1.3`.
`automexia` remains the default. `ghostty-1.3` is permanently bound to reviewed
Ghostty 1.3.1 fixtures; `ghostty` is only a moving alias to the newest bundled
verified profile. Selecting or migrating a profile is always explicit.

Profile compilation applies deterministic layers in this order: built-in,
profile, Windows adaptation, imported, legacy user, and user. Later layers and
higher explicit priorities win. Exact unbinds remove only the matching trigger,
predicate, scope, and table. In strict mode, invalid or unavailable entries
produce bounded diagnostics and prevent publication; permissive inspection
drops them visibly without creating an unrelated binding. The frontend compiles
a candidate away from the key-event path and atomically replaces one immutable
registry only after configuration and global-hotkey preparation succeeds.

The Linux/BSD fixture is normalized output of `ghostty +list-keybinds --default`
and `ghostty +list-actions` from the pinned source. macOS generation is a native
external prerequisite and selection fails closed until that reviewed fixture
exists; it is never synthesized from Linux. Automexia does not claim an
upstream Windows profile because Ghostty does not ship one. The Windows profile
is a deterministic, documented transform with explicit unsupported and
OS-intercepted diagnostics. Normal builds, tests, startup, and profile
selection are offline and never execute Ghostty.

The desktop frontend remains the only owner of concrete effects and
`can_perform` checks. A structured outcome distinguishes performed, consumed,
unconsumed, unavailable, and failed work and coalesces damage. All-surface
actions use a stable route snapshot. Pending sequences and table stacks are
isolated per surface and bounded; all-surface/global sequences are rejected.

## Alternatives considered

- Keep hand-maintained platform vectors. Rejected because action names,
  precedence, profile provenance, diagnostics, and palette/CLI output would
  continue to drift.
- Depend on Ghostty or execute it at runtime. Rejected because it would add an
  external process/network dependency to startup and make offline behavior and
  Windows support unreliable.
- Copy Ghostty's Zig binding engine. Rejected because Automexia needs a Rust
  owner integrated with its action capabilities, configuration transaction,
  route isolation, and platform adapters; copied protocol code would create a
  second maintenance and security authority.
- Replace Automexia defaults. Rejected by ADR 0011 and backward compatibility.

## Verification

- Fixture manifests bind schema, source tag/commit, generator, platform,
  commands, binary digest, and per-file SHA-256 checksums.
- Pure crate tests cover bounds, parsing, layer precedence, unbinds,
  collisions, direct/reverse lookup, sequences, tables, chains, cancellation,
  fallthrough, and deterministic Windows adaptation.
- Frontend equivalence tests prove the `automexia` profile preserves existing
  default/user behavior before strict profiles are exposed.
- CLI and xtask checks operate before GUI initialization and stay offline.
- Native Windows, Linux/BSD, and macOS smoke evidence remains a release gate;
  synthetic cross-platform fixtures do not replace it.

## Consequences

Compatibility becomes opt-in, versioned, explainable, and reproducible without
weakening Automexia's default input model. A new Ghostty version requires an
explicit fixture regeneration and review. The new crate remains below
configuration and the desktop composition root in the acyclic owned graph and
stays `publish = false` with a fail-closed dependency allowlist.
