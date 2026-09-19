# Architecture and reuse decisions

Read this reference before adding or moving shared code, dependencies, capabilities, extensions, workers, caches, persistence, protocols, or platform integrations.

## Search order

Search by contract before building:

1. authoritative project function, module, type, or adapter;
2. existing shared workspace crate whose responsibility and dependency direction fit;
3. existing repository tool, generator, scaffolder, SDK, or test harness;
4. Rust standard library or platform API already wrapped by the project;
5. existing reviewed dependency;
6. maintained external library, SDK, CLI, or service evaluated from primary sources;
7. a new local module or internal workspace crate.

Do not choose a new dependency or abstraction until the earlier candidates have been examined and rejected with reasons.

## Build, wrap, or adopt

**Build** only when Automexia needs a small mechanism tied to its core invariants, external options cannot satisfy the contract, and the maintenance and security cost is justified.

**Wrap** when a mature dependency or platform API provides the mechanism but Automexia needs a narrow typed contract, authority boundary, cancellation, limits, redaction, compatibility policy, or replaceable adapter.

**Adopt** directly when the dependency's public contract already matches the need and a wrapper would only rename it or create a second policy owner.

For each serious external candidate evaluate:

- license and redistribution compatibility;
- maintainers, ownership, release cadence, provenance, advisories, and response history;
- required features, default features, unsafe code, transitive dependencies, binary size, build time, startup impact, and MSRV;
- Windows, Linux/BSD, macOS, architecture, shell, and offline behavior;
- threads, tasks, cancellation, queues, timeouts, retries, storage, network, and cleanup;
- authority and capability footprint, credential handling, sandbox expectations, and logging;
- migration, rollback, disable, uninstall, replacement, and failure behavior;
- benchmark quality and native-platform evidence.

New dependencies, capabilities, unsafe code, persistence, protocols, threading, security boundaries, and public behavior normally require an ADR and architecture-checker coverage.

## Feature placement

Record one explicit placement decision before implementation.

### Core terminal

Use core only for behavior fundamental to terminal operation or required by every installation: PTY/process ownership, terminal state, input, rendering, panes/tabs, windowing, clipboard, common configuration, or a stable capability-broker boundary.

Keep provider, network, authentication, filesystem, optional process authority, and product integrations out of input, PTY, resize, renderer, and startup hot paths.

### Existing extension

Use an existing extension when the feature belongs to its cohesive domain and can remain behind its capability, dependency, lifecycle, persistence, enable/disable, failure, and uninstall boundaries.

Do not add unrelated authority to a convenient extension merely to avoid creating a boundary.

### New extension

Create a new extension only when the feature is optional, cohesive, and has a distinct authority or dependency footprint. Define before implementation:

- public typed contract;
- capability manifest and least authority;
- input, output, resource, concurrency, and storage limits;
- lifecycle, cancellation, cleanup, and shutdown;
- failure isolation and safe degradation;
- feature gate, disable, uninstall, and rollback behavior;
- test, packaging, versioning, and documentation owner.

### Cross-cutting behavior

Split at trust boundaries. Keep capability-free shared contracts or indispensable terminal mechanisms with their existing core owner. Keep provider, network, authentication, filesystem, process, or optional authority in extensions. Route privileged activation through the application-owned broker.

## Shared boundary selection

Choose the narrowest boundary real consumers require:

| Boundary | Use when | Avoid when |
|---|---|---|
| Existing function or module | Consumers share one crate and one mechanism | The behavior has different policy or lifecycle owners |
| Existing shared crate | Its existing responsibility and dependency direction fit | It would acquire unrelated authority |
| New internal crate | Real cross-crate reuse or independent compilation/dependency boundaries require it | Reuse is speculative or motivated only by file length |
| Separate implementations | Platform, lifecycle, resource, policy, or independent-oracle differences are essential | Similarity is merely being hidden behind switches |

Shared code must expose a minimal typed API, prefer private or crate visibility, keep dependencies acyclic, and remain free of hidden I/O, environment discovery, implicit startup, and global mutable services.

Preserve distinct units in types and tests: bytes, Unicode scalar values, graphemes, terminal cells, logical pixels, and physical pixels are not interchangeable. Queue bounds, task bounds, shutdown deadlines, entry counts, byte budgets, metadata equality, and file identity are also distinct contracts.

## Staged consolidation

1. Characterize every consumer and reproduce observed drift.
2. Define and independently test the shared contract.
3. Extract only the common mechanism.
4. Migrate one consumer at a time, retaining real-path tests.
5. Compare hot-path allocations and performance where relevant.
6. Remove old implementations only after call sites, feature combinations, targets, packaging, docs, and provenance are verified.
7. Give temporary adapters one implementation owner and explicit removal criteria.

Do not combine repository-wide cleanup with a focused change unless that consolidation is required to close the requested contract.
