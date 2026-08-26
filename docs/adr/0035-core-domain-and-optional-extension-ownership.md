# ADR 0035: Core domain and optional extension ownership

- Status: accepted
- Date: 2026-08-26
- Owners: Automexia maintainers
- Supersedes: the CP2/CP3 package placement described by ADR 0019; its acyclic dependency rule remains in force

## Context

Automexia had a sound engine/application/extension split, but three baseline
product domains were physically mixed into the optional DevOps context package:

- provider-neutral connection, SSH review, authentication, and workspace models;
- capability-free Quick Action and native-editor suggestion contracts; and
- generic command-result spacing, success/failure color, and completion pulse.

The last item was also painted only while the DevOps context extension was
enabled. That made core terminal feedback disappear when an unrelated optional
feature was disabled. Provider extensions imported the mixed DevOps package to
reach generic models, so package edges did not express actual ownership.

Cargo workspaces are intended to manage distinct packages together with a shared
lockfile, output directory, and workspace-wide commands. This lets these domains
have explicit package boundaries without introducing a second build or release
universe. Cargo features are conditional compilation and optional-dependency
mechanisms whose dependency features are unified additively; they are not used
as a substitute for ownership of always-available baseline product contracts.
See the official [Cargo workspace reference](https://doc.rust-lang.org/cargo/reference/workspaces.html)
and [Cargo features reference](https://doc.rust-lang.org/cargo/reference/features.html).

## Decision

Automexia uses these owners:

1. `apps/automexia-terminal/src/renderer/command_results.rs` owns generic
   per-route command-result geometry, exit classification, colors, spacing,
   animation timing, lifecycle, and native test hooks. It remains active when
   every optional extension is disabled.
2. `automexia-connectivity` owns capability-free, provider-neutral connection
   documents, plans, direct/routed/tunnel SSH review, provider authentication,
   and declarative multi-environment workspace contracts.
3. `automexia-command-productivity` owns capability-free Quick Action,
   alias/projection/import/pack/provider-action, and native-editor suggestion
   contracts. It may depend on `automexia-connectivity` for typed provider and
   workspace context.
4. `automexia-devops` is narrowed to optional local context discovery and
   semantic classification. It does not re-export the two baseline domains.
5. Provider-specific parsing and exact typed intent remain in independently
   disabled `extensions/devops-*` packages. Those adapters depend directly on
   the provider-neutral domains and extension API, never on the unrelated
   DevOps context package.
6. `automexia-ui-model` remains the renderer-independent projection owner and
   consumes domain contracts directly. The desktop application remains the only
   composition root and authority owner.
7. Terminal engines (`rio-vt`, `teletypewriter`, `sugarloaf`,
   `rio-window`) may not depend on product-domain or optional extension
   packages.

The architecture verifier and a mutation suite enforce package locations,
direct dependency direction, provider isolation, engine isolation, and the
invariant that command-result paint and state cleanup are independent of DevOps
activation.

## Authority and lifecycle

The new domain packages are private, capability-free libraries. They gain no
filesystem, process, network, credential, PTY, renderer, window, extension
lifecycle, or ambient environment authority. Existing application adapters own
I/O, bounded workers, cancellation, publication, persistence, and cleanup.
Existing provider extensions own bounded provider parsing and typed intent only.

The move changes Rust paths and evidence paths, not persisted public schemas.
Historical D0 session-launch contracts remain byte-for-byte immutable. Active
schema 6 differs from schema 5 only by current source ownership references and
ratchets schema 5 by exact digest.

## Alternatives considered

- Keep generic features in `automexia-devops` and re-export them. Rejected:
  disabling or replacing that extension would still affect baseline behavior,
  and provider packages would retain a misleading dependency.
- Move all behavior into the desktop application. Rejected: capability-free
  models would become difficult to test and reuse across the UI and provider
  adapters, and the application would become a second domain registry.
- Create one package per small module. Rejected: the two cohesive domains already
  have clear lifecycles and dependency direction; finer packages would add build
  and maintenance cost without new authority isolation.
- Make the domains optional Cargo features. Rejected: connections, workspaces,
  Quick Actions, and editor suggestion contracts are baseline product policy,
  while Cargo features are additive build configuration rather than runtime
  extension ownership.
- Add new third-party libraries. Rejected: existing workspace dependencies and
  the standard library satisfy the move.

## Consequences

- Disabling DevOps context no longer removes command-result feedback.
- Provider adapters are independently replaceable and no longer import an
  unrelated extension to obtain shared contracts.
- Workspace checks, Clippy, tests, benchmarks, fuzz targets, and the shared
  lockfile continue to cover the moved code as normal workspace members.
- Private Rust import paths change. There is no supported public crate API or
  user-data migration.
- Existing serialized/compiler identity strings that contain `automexia-devops` remain
  unchanged for compatibility; they are format identities, not package ownership.
- A future optional feature belongs in an extension only when it has an
  independently disabled lifecycle or provider-specific authority. Generic
  always-available terminal feedback stays in core/application ownership.
- ADR 0019 remains authoritative for acyclic dependency direction, but its
  historical statement that CP2/CP3 live in `automexia-devops` is superseded.

## Verification

- `python tools/ci/check_feature_ownership.py`
- `python tools/ci/test_feature_ownership.py`
- `cargo xtask verify architecture`
- focused tests for both new domain packages, DevOps context, UI model, provider
  adapters, renderer command results, and immutable D0 contracts
- workspace format, Clippy, nextest, doc tests, full QA, benchmark compilation,
  and `cargo ready`
