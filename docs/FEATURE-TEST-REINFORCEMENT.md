# Public feature-test reinforcement

This reinforcement plan covers only the free terminal capabilities in
[the public feature catalog](FEATURES.md). Advanced unreleased and commercial
feature assurance remains private.

The machine-enforced feature matrix may contain internal source owners, but
public documentation must not turn those owners into product announcements.

## Required assurance pattern

For every public behavior:

1. reproduce the real user path;
2. add the smallest deterministic regression test;
3. cover boundary, malformed, stale, cancelled, repeated, and concurrent cases;
4. assert intended effects and forbidden side effects;
5. use an independent oracle;
6. exercise cleanup, disable, recovery, rollback, and restart where applicable;
7. measure the real owning path when performance or resources can regress; and
8. retain native, visual, accessibility, signing, hardware, and long-duration
   work as explicit external gates until it actually runs.

## Public feature owners

### identity-config-migration

Reinforce first-run identity, non-overwrite, strict bounded parsing,
transactional reload, private permissions, atomic replacement,
last-known-good recovery, migration preview/apply/rollback, and side-by-side
coexistence. Assert that real profile paths and environment values never enter
fixtures or reports.

### terminal-protocols-grid-history

Reinforce fragmented and malformed control sequences, Unicode widths,
combining/bidi/control input, alternate-screen transitions, scrollback, reflow,
search, selection, cursor state, and exact visible cells. Fuzz every structured
terminal-input boundary with historical failures retained.

### pty-scheduler-process-lifecycle

Reinforce exact executable/argument launch, ordered input, resize/output storms,
interrupt, EOF, exit, cancellation, close, and shutdown on each native adapter.
Assert no shell evaluation, implicit Enter, stale publication, orphan process,
leaked handle, worker, or temporary file.

### renderer-fonts-responsive-ui

Reinforce immutable generation-labelled snapshots, font fallback, cell geometry,
clipping, z-order, cursor, selection, themes, high contrast, reduced motion, and
small through high-resolution layouts. Require renderer-neutral state, exact
controlled rasters, and native frames.

### windows-tabs-sessions-input

Reinforce independent windows, global and pane-local tabs, fresh/cloned splits,
route and focus isolation, geometric navigation, pointer routing, divider
resize, clipboard, IME, search scope, overlays, and focus restoration.

### ghostty-compatibility-g0-g6

Reinforce documented Ghostty-style keyboard compatibility with exact binding
resolution, left/right modifier handling, pane isolation, focus restoration,
clipboard behavior, IME safety, and absence of unintended PTY input. Treat
unsupported Ghostty behavior as an explicit compatibility limit.

### prompt-context-devops-semantics

Reinforce bounded route-scoped prompt metadata, directory and status updates,
Git state, long and hostile labels, stale generation rejection, and redaction.
Context rendering must never change commands or trigger network work.

### openssh-inventory-persistence

Reinforce explicit file selection, bounded includes and record counts,
duplicates, malformed data, links, replacement, revocation, public metadata,
last-known-good recovery, and absence of secrets, passive scanning, login, or
network authority.

### extension-contract-runtime

Reinforce version negotiation, strict message schemas and ceilings, exact
capability denial, session isolation, cancellation, queue saturation, stale
generation rejection, crash/restart, disable, uninstall, and shutdown. Optional
failure must leave the core terminal functional.

### image-protocols-local-preview

Reinforce input, dimension, pixel, frame, cache, and lifetime limits; malformed
protocol images; local regular-file identity; links; replacement; permission
errors; cancellation; stale results; clipping; scrolling; eviction; and cleanup.

### shell-integration-listings

Reinforce supported-shell startup, prompt boundaries, object-preserving
pipelines, quoting, Unicode, missing-resource fallback, explicit disable/remove,
and cleanup. The native shell remains the independent command-editor oracle.

### packaging-release-provenance

Reinforce artifact identity, licenses, checksums, SBOM, provenance, signatures,
install, upgrade, rollback, uninstall, startup hooks, user-data policy, and final
cleanup. Missing native signing or notarization evidence remains not run.
The native nFPM revision is one repository-owned input for both DEB and RPM
names; the post-download assembler must consume the exact native filenames, and
mutation coverage must reject a missing, duplicate, zero, or split revision.

### contributor-automation-quality-policy

Mutation-test repository validators so deleted owners, weakened limits, stale
paths, fabricated evidence, missing private-documentation exclusions, and
confidential-data canaries fail closed.

For caches and build storage, additionally mutate content identity, integrity
manifests, atomic publication, link/reparse rejection, traversal ceilings,
current and dirty worktree protection, live/dead owner probes, leases, grace
periods, dry-run/apply behavior, exact deletion allowlists, de-duplicated usage,
success/failure cleanup, and the dormant pre-push hook. Compare the generated
tree and hashes independently. Exercise native process ownership on each
claimed platform and keep unexecuted hosts external.

### stabilization-release-assurance

Keep first failures, compare same-host performance baselines, require bounded
resource growth and cleanup, and bind release evidence to the exact commit and
package. Cross-compiles and mocked UI evidence do not satisfy native gates.

### ffi-wasm-embedding

Where retained for compatibility, reinforce ABI/version negotiation, bounded
input/output, ownership, cancellation, panic containment, hostile input,
disable/unload, and absence of ambient authority.

## Source-owned reinforcement plans

These anchors preserve machine-enforced traceability for implemented or
explicitly nonactivated source owners. They are assurance boundaries, not a
public product schedule or commercial feature announcement.

### command-productivity-cp0-policy

Reinforce the pure policy/schema boundary with exact limits, hostile structured
input, deterministic decisions, and proof of no process, network, credential,
profile, or execution authority.

### command-productivity-cp2-persistence

Reinforce bounded private persistence, revision compare-and-swap, atomic update,
last-known-good recovery, permissions, corruption, concurrency, rollback,
disable, removal, restart, and exact residue checks.

### command-productivity-cp22-quick-actions

Reinforce list, search, review, insert-without-Enter, copy, placeholder binding,
risk confirmation, import/export, keyboard access, redaction, and route/generation
isolation without granting launch authority.

### command-productivity-cp4-provider-actions

Reinforce that provider-aware public actions consume bounded caller-owned cached
observations, report stale/unknown state, preserve external authentication, and
cannot perform hidden discovery or execution.

### command-productivity-cp31-persistent-aliases

Reinforce exact preview, shell-specific compilation, collision ownership,
explicit activation, private publication, native parsing, rollback, disable,
uninstall, startup bounds, and absence of raw shell evaluation.

### command-productivity-cp32-devops-packs

Reinforce immutable reviewed manifests, disabled-by-default actions, exact
provenance/digests, conservative risk floors, explicit enablement, provider-free
inspection, update planning, and safe removal.

### command-productivity-cp33-native-imports-workspace

Reinforce user-supplied bounded inventories, strict simple-command parsing,
explicit selection, dry-run, collision review, workspace trust/revocation,
source-change invalidation, and no native discovery or task execution.

### command-productivity-cp1-native-completion

Reinforce shell-native ownership, generated artifact integrity, completion
fallback, buffer/cursor preservation, bounded startup, explicit install/remove,
and no provider, network, credential, or implicit execution while typing.

### command-productivity-cp50-research

Keep research evidence clearly non-implementation: source dates, alternatives,
security/performance constraints, unresolved platform questions, and the exact
gate required before a proposal may change public behavior.

### command-productivity-cp51-proposal

Require proposal-only owners to remain nonactivated until architecture,
capability, persistence, native-platform, UX, performance, and negative-side-
effect contracts are accepted and independently evidenced.

### situation-aware-production-operations-po0-proposal

Keep this proposal nonactivated and noncommercial in public source. Assurance
checks verify separation from credential, provider, approval, remote-execution,
and terminal hot-path authority; detailed future planning remains private.

### ecosystem-d7-cp6-proposal

Keep optional ecosystem proposals behind explicit capability, signing,
sandboxing, lifecycle, disable/uninstall, resource, and failure-isolation gates.
This anchor is not an availability claim.

### stabilization-release-assurance-s1-s2

Reinforce staged stabilization with exact failure retention, same-host baselines,
native evidence, packaged-artifact identity, bounded resources, cleanup, and
explicit external gates before release claims advance.

### connection-hub-f2-models

Reinforce pure bounded models for public connection metadata, provenance,
freshness, unknown state, stable identity, duplicates, hostile labels, and
absence of credentials, login, network, filesystem mutation, or process launch.

### connection-hub-f3-catalog

Reinforce immutable snapshots, deterministic filtering/grouping, route and
generation isolation, size ceilings, stale-result rejection, and truthful
empty/loading/unavailable states.

### connection-hub-f3-composition

Reinforce single-owner composition of public connection sources with explicit
precedence, conflicts, provenance, cancellation, publication-before-wake, and
last-known-good behavior.

### connection-hub-m1-product

Reinforce the read-only keyboard workflow, focus restoration, accessibility,
responsive layouts, review-before-activation, and proof that browsing or
selection sends no PTY input and starts no connection.

### connection-hub-f3-library

Reinforce bounded user-private connection metadata, atomic persistence,
conflicts, permissions, import/export, rollback, disable, uninstall, corruption,
and strict exclusion of secret material.

### connection-automation-m6-workspaces

Reinforce declarative topology, profile/recipe review, generation binding,
bounded restore planning, revision conflicts, cancellation, recovery, and no
implicit PTY/process start from list, show, edit, or restore review.

### provider-auth-m7-capsules

Reinforce external credential ownership, opaque references, identity/context
freshness, per-pane isolation, expiry/revocation, offline/unknown states,
redaction, and absence of provider-global context mutation.

### direct-openssh-m3-review

Reinforce exact executable/argument review, host-key and route evidence,
cancel/timeout/cleanup, external agent/key ownership, hostile metadata, and no
shell command-string evaluation or implicit connection.

### provider-aws-m8-source

Reinforce bounded cached public AWS observations, explicit profile/account/region
provenance, stale/expired/unknown states, per-pane isolation, redaction, and no
credential copying or background provider work on terminal hot paths.

### provider-azure-m9-source

Reinforce bounded cached public Azure observations, explicit tenant/subscription
provenance, stale/expired/unknown states, per-pane isolation, redaction, and
external CLI/identity authority.

### provider-gcp-m10-source

Reinforce bounded cached public Google Cloud observations, explicit project/
account provenance, stale/expired/unknown states, per-pane isolation, redaction,
and external CLI/identity authority.

### provider-kubernetes-openshift-m11-source

Reinforce bounded context/cluster/namespace metadata, exec-auth opacity,
certificate/path redaction, stale source handling, per-pane isolation, and no
global kubeconfig mutation or implicit cluster operation.

### provider-teleport-m12-source

Reinforce bounded public profile/proxy metadata, external Teleport identity and
certificate authority, expiry/unknown states, redaction, route review, and no
credential custody or hidden login.

## Documentation reinforcement

Every documentation change must prove:

- all public Markdown loads as UTF-8;
- titles and code fences are structurally valid;
- relative file and heading links resolve;
- no machine-local or confidential values are present;
- no unreleased advanced or commercial plans appear publicly;
- the lossless private archive is present and ignored; and
- the public index, feature catalog, architecture, testing, and roadmap agree.

Exact private schemas, algorithms, provider matrices, future workflows, market
plans, pricing, and commercial packaging must never be copied into a public
fixture merely to satisfy a checker.
