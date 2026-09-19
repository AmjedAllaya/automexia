# Development cache and build storage

Automexia keeps contributor builds fast without allowing generated data to
grow without an owner. The terminal runtime does not use this cache. Build and
assurance tools own it, and deleting it changes only the next build time.

## What is stored where

| Data | Owner and lifetime |
|---|---|
| Final Cargo artifacts | The workspace `target` directory; kept for normal development and removable with `cargo purge`. |
| Cargo intermediate artifacts | `target/build`; separated from final artifacts through Cargo's `build-dir` setting. |
| Complete-gate artifacts | A uniquely named verification target; removed after success or ordinary failure. |
| Assurance executables | A platform- and version-addressed immutable toolset in the shared Automexia cache. |
| Package-manager downloads and runtime state | Separate mutable `downloads` and `runtime` directories in the shared cache. |
| Staging and process temporary data | Separate generated directories; reclaimed after the grace period when no lease is active. |
| QA benchmark compilation | A per-run benchmark target; removed after both success and failure while the bounded report remains. |

On Windows the default shared cache is `.automexia-cache` at the checkout
drive root. On Unix it is beside the common Git checkout. Set
`AUTOMEXIA_DEV_CACHE_DIR` to an absolute, non-linked directory to choose a
different location. Do not point multiple operating systems at the same mutable
Cargo target.

The shared assurance toolset is content-addressed by operating system,
architecture, Python runtime contract, and every pinned tool version. It is
built in a staging directory, verified, assigned an integrity manifest, and
published atomically. A corrupt or incomplete current toolset is quarantined
inside generated staging storage and rebuilt. Concurrent assurance processes
use leases; cleanup does not remove leased or recently active data.

## Inspect storage

```text
cargo xtask cache status
cargo xtask cache status --warn-gib 20
```

The report distinguishes reusable toolsets, mutable downloads/runtime state,
temporary data, verification artifacts, worktree targets, and the legacy
repository-local tool directory. Totals are de-duplicated when one displayed
entry contains another. Inventory traversal has file and directory ceilings
and refuses symbolic links and Windows reparse points.

## Reclaim generated data

Cleanup is a dry run unless `--apply` is present:

```text
cargo xtask cache gc --scope automatic
cargo xtask cache gc --scope automatic --grace-hours 72 --apply
```

The default automatic scope considers only stale verification artifacts, old
immutable toolsets, staging directories, and process temporary directories.
It deliberately avoids walking every normal worktree target on every gate and
does not remove the legacy `.automexia-tools` directory.

Use broader scopes only after reviewing the dry run:

```text
cargo xtask cache gc --scope tools --grace-hours 72
cargo xtask cache gc --scope worktrees --grace-hours 72
cargo xtask cache gc --scope all --grace-hours 72
```

- `tools` includes generated shared tool data and the legacy local tool cache.
- `worktrees` includes inactive clean-worktree targets; a current or dirty
  worktree remains protected.
- `all` combines both explicit scopes.

Cleanup accepts only exact discovered cache entries. It rejects roots,
unexpected paths, links, reparse points, current targets, dirty-worktree
targets, live process markers, active leases, required toolsets, and entries
inside the grace period. Interrupted verification directories carry a process
marker so a live owner is protected and a dead owner can later be reclaimed.

Traversal enforces limits while reading entries. Queued directories count
toward the tree ceiling; ordinary files count toward collection entry ceilings.
Lease inspection accepts at most 128 named leases, with identifiers limited to
64 lowercase letters, digits or hyphens.

Applied cleanup retains admission and all acquired idle native leases through
fresh eligibility checks and deletion. Active named leases protect shared
candidates; concurrent collectors and new admissions fail closed with a
retry-later error. Distinct admitted users can run concurrently. Dry runs reserve
nothing. Inventory, deletion and traversal errors release the locks.

All users of a shared cache must use the admission-aware lease owner. Retained
native locks exclude older users of existing named lease files, but an older
writer introducing a previously unseen name does not honor admission. Stop such
older clients before applying cleanup; dry-run inspection remains available.

## Build profiles

Normal development and tests retain line-table debug information while
dependency debug information is disabled to reduce compilation and storage.
Use the explicit full-debug profile when source-level dependency debugging is
needed:

```text
cargo build --profile debugging
```

Cargo's downloaded global cache is configured for daily automatic cleanup.
This is separate from Automexia's project cache lifecycle.

## Hosted and local assurance

GitHub's free hosted `CI` workflow is the automatic push and pull-request
authority. It uses a pinned compiler cache only in non-shipping quality jobs,
caches Cargo source downloads but never `target`, and keeps native release
package builds cold.

The local pre-push hook is intentionally dormant. Installing it creates a
non-blocking placeholder containing the commented manual command; it does not
run a pipeline or block a push:

```text
cargo xtask assurance install-hook
```

Run local assurance explicitly when desired:

```text
cargo xtask assurance install-tools
cargo xtask assurance pre-push
```

To restore automatic local enforcement later, review the current assurance
contract and deliberately replace the commented hook line with the exact
manual command. Never overwrite an unrelated existing hook.

## Failure and recovery

- A missing cache is a cold-build condition, not a correctness failure.
- A cache-integrity failure stops assurance and requires `install-tools`; it
  never silently trusts partial executables.
- A failed cleanup leaves the original data or reports the exact generated
  category; it does not broaden its deletion scope.
- Use `AUTOMEXIA_KEEP_VERIFY_TARGET=1` only for diagnosis. The owner marker is
  released before retaining the target so a later reviewed cleanup can find it.
- `cargo purge` remains the Cargo-owned way to remove the current checkout's
  target after closing running Automexia processes.

Local `sccache`, alternative Rust backends, nightly toolchains, and a mandatory
Python tool manager are deliberately not part of the default contributor path.
They add installation, provenance, compatibility, and cache ownership costs
without improving the correctness contract. They may be evaluated later behind
an explicit opt-in profile.

## Evidence boundary

Mutation tests cover cache identity, path containment, bounded inventory,
leases, live and dead owners, current and dirty worktrees, recency, corruption,
atomic publication, cleanup after success and failure, and the dormant hook.
The native process-owner probe runs on the current host. Other operating
systems and long-running storage campaigns remain separate evidence; a passing
cache test never substitutes for the hosted CI result on the pushed commit.
