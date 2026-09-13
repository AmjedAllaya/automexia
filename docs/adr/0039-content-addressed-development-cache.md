# ADR 0039: Content-addressed development cache lifecycle

- Status: Accepted
- Date: 2026-09-05

## Context

Automexia's complete Rust and assurance gates create final binaries,
intermediate compiler objects, downloaded tools, package-manager state,
temporary scanner data, and benchmark targets. Historical per-worktree tool
caches duplicated several gigabytes, while an interrupted build could leave a
large verification target. Sharing one mutable Cargo target between worktrees
would trade disk use for races, stale objects, and incorrect cleanup ownership.

The existing storage-bounded workflow removes complete-gate targets, but it did
not own reusable assurance tools across worktrees or provide a safe inventory
and garbage-collection contract. The local pre-push hook also made an expensive
pipeline part of every push even though GitHub hosted CI is the automatic
repository authority.

## Decision

1. Keep this feature in contributor/build infrastructure. It does not belong in
   terminal core or a product extension because it owns no PTY, renderer,
   session, provider, network, or user-runtime behavior.
2. Keep final Cargo artifacts in `target`, separate intermediates through
   Cargo's workspace-root `build-dir`, and never share a mutable target between
   worktrees or operating systems.
3. Store assurance executables in one immutable, content-addressed toolset keyed
   by platform, architecture, Python contract, and all pinned tool versions.
   Publish only a complete, integrity-manifested toolset through atomic rename.
4. Separate immutable tools from mutable downloads, Cargo/Python runtime state,
   staging, process temporary data, and per-run benchmark compilation.
5. Protect cache users with file leases and verification processes with live
   process markers. Reject links, reparse points, broad paths, stale identity,
   and unbounded traversal.
6. Make cleanup dry-run by default. Automatic cleanup considers only stale
   verification, old toolsets, staging, and temporary data after a 72-hour grace
   period. Legacy tools and normal worktree targets require an explicit scope;
   current, dirty, leased, required, and recent entries remain protected.
7. Remove QA benchmark compiler targets after success and failure while
   retaining bounded reports.
8. Keep GitHub's free hosted CI automatic for pushes and pull requests. Keep
   the local pre-push hook as a dormant, non-blocking placeholder and retain the
   manual assurance command for explicit future use.
9. Treat every cache as acceleration only. Missing, evicted, corrupt, or
   incompatible cache data must cause a cold rebuild or fail-closed tool
   reinstall, never change a compiled or released artifact's authority.

## Alternatives considered

- A repository-local tool cache was rejected because every worktree duplicates
  immutable executables and their build dependencies.
- One shared Cargo target was rejected because worktrees and host platforms can
  concurrently require incompatible mutable compiler state.
- Automatic deletion of every worktree target was rejected because inspecting
  and removing unrelated or dirty worktrees on each gate is expensive and
  surprising.
- A mandatory local compiler cache, nightly compiler, alternative codegen
  backend, or Python tool manager was rejected for the default path because it
  expands provenance, compatibility, installation, and cleanup obligations.
- Removing local assurance was rejected because an explicit offline/manual path
  remains valuable even while push-time execution is paused.

## Consequences

- Repeated worktrees reuse one verified assurance toolset instead of compiling
  it per checkout.
- Normal incremental development stays fast, while complete gates and QA
  benchmarks clean their disposable compiler data.
- Explicit cleanup can reclaim old generated data without treating a cache
  root, active process, dirty checkout, or symbolic link as a deletion target.
- Pushes are not delayed by a local pipeline; the free hosted workflow remains
  the automatic exact-commit result.
- First use on a new platform or after a pinned-tool change is a cold install.
- Native validation on platforms not executed locally, cache-service
  availability, and long-duration resource campaigns remain external evidence.

## Rollback

Remove the shared-cache orchestration and return the assurance policy to a
repository-local generated tool directory. Remove Cargo `build-dir` and profile
overrides to restore Cargo defaults. Re-enabling automatic pre-push assurance is
a separate deliberate policy change; it must update the hook generator, tests,
and contributor documentation together.
