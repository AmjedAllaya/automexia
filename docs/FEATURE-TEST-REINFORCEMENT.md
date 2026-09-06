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

For command completion metadata, include multiple adjacent prompt lifecycles
whose source result and preceding boundary share a physical prompt row. Resize
between narrow and wide grids, navigate in both directions, and independently
compare stable IDs, timestamps, boundaries, visible rows, and unchanged PTY
bytes after each transition.

### pty-scheduler-process-lifecycle

Reinforce exact executable/argument launch, ordered input, resize/output storms,
interrupt, EOF, exit, cancellation, close, and shutdown on each native adapter.
Assert no shell evaluation, implicit Enter, stale publication, orphan process,
leaked handle, worker, or temporary file. Treat Linux master-side `EIO` after a
one-shot slave closes as a platform EOF signal without weakening exact output,
completion-marker, child-exit, or cleanup oracles; do not treat temporary
zero-byte ConPTY reads as permanent closure.

Exercise ordinary and exact Windows ConPTY sessions under kill-on-close Job
ownership plus the Unix process-group owner. Broadcast one idempotent shutdown
request to active, background, split, pane-tab, parked, and top-level-window
sessions before sequential joins. Test one and many sessions, repeated requests,
window close, explicit quit, and the final event-loop callback. Capture exact
temporary-fixture process identities before close so ConPTY reparenting cannot
escape a parent-only oracle; require every identity and owner to exit inside a
declared many-session wall-clock ceiling.

### renderer-fonts-responsive-ui

Reinforce immutable generation-labelled snapshots, font fallback, cell geometry,
clipping, z-order, cursor, selection, themes, high contrast, reduced motion, and
small through high-resolution layouts. Require renderer-neutral state, exact
controlled rasters, and native frames.

Project row-anchored metadata through one frame-local ownership pass. Exercise
input permutations and malformed duplicates, then prove each result identity is
painted once and every measured label rectangle is pairwise disjoint after
reflow. Expose co-located prompt-context rectangles and reject cross-owner
intersection under one shared reservation. Freeze command duration as well as
wall time before exact raster comparison. A latest-result-only hook is not a
multi-result or cross-overlay paint oracle.

Publish a feature-gated native checkpoint only after the matching frame has
presented successfully. Controls consumed after overlay construction must force
a subsequent frame; skipped or dropped frames publish nothing. Retained native
captures establish foreground ownership, reject detectable dialog occlusion,
and require two consecutive identical full-frame pixel digests before exact
backend comparison and independent frame inspection. Two identical occluded or
stale captures are not visual proof.

Frame-skip identities must include the physical framebuffer extent, and the
cache may record only a successfully presented frame. Exercise unchanged
content across initial sizing and later resize, acquisition or presentation
failure followed by identical retry, a completely opaque on-screen client, and
zero-tolerance full-client comparison across available renderer backends. Hold
live editor input at a fixed unsent sentinel so cursor-line drift cannot
masquerade as a renderer difference.

### windows-tabs-sessions-input

Reinforce independent windows, global and pane-local tabs, fresh/cloned splits,
route and focus isolation, geometric navigation, pointer routing, divider
resize, clipboard, IME, search scope, overlays, and focus restoration.

For previous/next command shortcuts, perform an actual native resize first,
assert the freshly reflowed frame, and inspect every intermediate and restored
frame for result-ID uniqueness, non-intersecting badge geometry, stable pane and
prompt ownership, and absence of terminal input.

Require the state checkpoint to follow the corresponding successful present,
then stabilize the retained native image with two exact full-frame digests. On
multi-pane exit, require broadcast-first PTY shutdown and exact owned-process
cleanup within the native wall-clock ceiling.

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
PowerShell assurance must capture real identity control bytes in a child process,
verify internally that user and path fields exist, and discard those bytes before
they reach local or hosted logs. CMD identity fixtures must use stable fictional
values rather than contributor account or executable-path data.

### packaging-release-provenance

Reinforce artifact identity, licenses, checksums, SBOM, provenance, signatures,
install, upgrade, rollback, uninstall, startup hooks, user-data policy, and final
cleanup. Missing native signing or notarization evidence remains not run.
The native nFPM revision is one repository-owned input for both DEB and RPM
names; the post-download assembler must consume the exact native filenames, and
mutation coverage must reject a missing, duplicate, zero, or split revision.

Linux Early Access authorization additionally validates the pinned owner's
author/merger/sender/original/rerun identities, a same-repository merged release
PR, and exact current-main identity under ADR 0040. The existing
`tools/ci/public_distribution.py` / `tools/ci/test_public_distribution.py` pair
owns realistic event and CLI tests, 1 MiB/32-depth/32,768-node bounds, duplicate
and malformed JSON rejection, no writes, and redacted diagnostics. Mutations
must reject removed, duplicated, reordered or ignored authorization and forged
contexts while preserving manual rehearsal isolation and all artifact gates.
Only the real post-merge run supplies signed-release evidence.
The explicitly approved public-archive solo policy requires zero approvals,
no mandatory second reviewer, and retains PRs, resolved threads, signatures,
linear squash history, no bypass and protected tags. Mutate every field's
absence, value and JSON type. Public documentation uses the separate read-only
`check_public_release_metadata.py` / `test_public_release_metadata.py` owner:
check exact file inventory, logo bytes, accessible image text, local anchors,
six pinned download URLs, trusted verification key and private-path rejection.
Check live anonymous links and the actual GitHub-rendered page separately;
offline tests neither activate downloads nor certify rendered pixels.
Read-only App governance must combine REST rules with repository/identity-bound
GraphQL bypass evidence. Preserve the real omitted REST field and redacted
nonempty GraphQL actor as regression seeds. Test zero/nonzero/inconsistent
counts, missing/duplicate/substituted identities, incomplete pages, query errors,
1 MiB/depth/node limits, strict redacted CLI failure and no writes. Mutation-test
query removal, comments, error suppression, deadline changes, stale evidence,
reordering and permission expansion. A native App positive control must detect
an actual actor on an isolated disabled rule; remove that exact rule and revoke
the diagnostic token afterward. Never modify live main/tag rules for this test.
Draft lifecycle tests must use GitHub's observed temporary release and asset
locators before tag creation, not a published-only fixture relabeled as draft.
Bind the create response URL and numeric ID through lookup, complete signed
bundle verification, the exact publication write and immutable response. Reject
missing/changed IDs, cross-draft locators, malformed temporary URLs, stale
published URLs, digest drift, stable/latest-channel drift, skipped or reordered
checks, and extra writes. Verify the actual least-privilege App draft path before
discarding a failed draft; keep final version-pinned URLs mandatory and do not
claim publication until release and every asset attestation pass.
`tools/ci/test_free_plan_contract.py` also removes and inverts the stable-lane
Linux-namespace exclusion; the selector must reject that overlap before stable
authorization rather than failing an unrelated branch grammar later.

### contributor-automation-quality-policy

Mutation-test repository validators so deleted owners, weakened limits, stale
paths, fabricated evidence, missing private-documentation exclusions, and
confidential-data canaries fail closed.

Public-documentation mutations cover restored future announcements, renamed
pages and changelog entries, duplicate or unreviewed status rows, and unresolved
merge markers inside and outside code fences. Current limitations and missing
release evidence remain allowed. Diagnostics must not echo rejected content.
These checks supplement human publication review; they do not prove the absence
of every possible confidential statement. Nonactivation, architecture, security
and release-evidence gates remain enforced.

The documentation checker/hygiene pairs own publication regressions. The
existing `test_phase_implementation_audit.py` and
`test_production_operations_po0.py` suites additionally enforce current-status
row identity and the absence of runtime activation.

For caches and build storage, additionally mutate content identity, integrity
manifests, atomic publication, link/reparse rejection, traversal ceilings,
current and dirty worktree protection, live/dead owner probes, leases, grace
periods, dry-run/apply behavior, exact deletion allowlists, de-duplicated usage,
success/failure cleanup, and the dormant pre-push hook. Compare the generated
tree and hashes independently. Exercise native process ownership on each
claimed platform and keep unexecuted hosts external.

The summarized workspace-test owner must also keep its 30-minute deadline,
16 MiB stdout ceiling, live compiler diagnostics, Unix process group or Windows
Job Object, and real success/deadline/overflow child-process tests. Mutation
coverage must reject a missing or relaxed limit, wrapper, cleanup path, partial
failure diagnostic, or real-process oracle.

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
