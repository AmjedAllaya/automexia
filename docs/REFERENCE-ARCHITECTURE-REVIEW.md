# Reference architecture review

This document records the architecture review performed for Automexia v0.3. It is not a claim that Automexia copies another terminal's implementation; it identifies reusable engineering principles and applies them to the existing Rust/Rio-derived codebase.

## Existing Automexia v0.2 documentation and implementation reviewed

The project already had several strong design constraints. They are retained rather than discarded:

- extension logic stays out of VT/PTY/font/backend code;
- no terminal output may execute processes;
- local context reads are bounded and sanitized;
- activation state is cached by generation;
- semantic rendering preserves explicit non-neutral/truecolor application colors and selection/search precedence while allowing neutral white/default shell output to be normalized;
- future third-party extensions should use Wasm plus a capability broker;
- `/market` initially activates trusted code already compiled into the application rather than downloading arbitrary native libraries.

The review found implementation gaps where the code was weaker than the written intent:

1. DevOps discovery still happened synchronously inside the HUD render path when its refresh interval expired.
2. One global DevOps snapshot could be overwritten by another terminal window/pane with a different CWD.
3. v0.2 extension ownership was spread between renderer integration and `crate::extensions`, making the future platform boundary ambiguous.
4. fresh bootstrap could still inherit a newer moving fork branch despite having an audited SHA variable.
5. optional extension-worker startup should degrade an extension, not crash terminal startup.

v0.3 corrects these rather than writing a second, conflicting architecture document.

## Ghostty principles used as a professional reference

Primary Ghostty project documentation/source descriptions reviewed during this architecture pass describe these relevant principles:

1. **Shared terminal core / application separation.** Ghostty's native GUI applications consume a shared cross-platform core (`libghostty`) instead of making the GUI itself the terminal engine.
2. **Independent latency domains.** Ghostty documents dedicated read, write and render scheduling per terminal as a major performance architecture choice.
3. **Shared renderer semantics across backends.** Ghostty's renderer rework moved core renderer logic into shared code so OpenGL/Metal backends do not drift in behavior.
4. **Terminal state as a consumable library boundary.** The libghostty/ghostling work demonstrates terminal parsing/state and render-state APIs that consumers can pair with their own windowing/rendering.
5. **Different stability clocks for core and app.** Ghostty's libghostty work is being versioned separately from the desktop app, reinforcing the value of explicit subsystem/API boundaries.

Automexia applies the principles, not the implementation language or exact scheduler.

## Automexia decisions derived from the review

### Engine/application boundary

Automexia product features live under `frontends/rioterm/src/automexia/`; VT/PTY/font/backend code remains engine-owned. The 0.x line still uses a pinned Rio-derived engine, but the boundary is designed to survive the standalone fork migration.

### Rendering

Extensions publish models/snapshots; renderer adapters turn them into visuals. Extension code does not own GPU objects. New cross-platform visual features should have shared semantics with thin backend adapters rather than independent per-backend implementations.

### Threading

Automexia does not rewrite working Rio terminal scheduling simply to resemble another terminal. It does make its own services obey strict latency boundaries: extension/config discovery runs on a bounded worker, rendering submits work non-blockingly, and cached state is published by generation.

### Multi-session correctness

DevOps snapshots are keyed by terminal route/session in a bounded cache and retain the SessionFacts that produced each result. Multiple windows/tabs/splits can therefore refresh context without one global model replacing another or a stale nested-shell completion being accepted.

### Extension platform

The project keeps the original Wasm/capability-broker plan, but v0.3 makes capability declarations real source contracts now so the future sandbox does not need a different extension API.

### Upstream stability

Fresh stable integration branches start from the exact audited terminal-engine SHA. Upstream/fork changes are audited explicitly and never merged automatically into a stable release.

## Deliberate differences

Automexia is Windows-first today and retains the existing Rio-derived windowing, ConPTY, rendering and font stack in v0.3. Rewriting those subsystems without performance/conformance evidence would create risk rather than professionalism. The architecture first establishes ownership and contracts, then allows measured internal refactors.

## Resulting v0.3 architecture decisions

- exact fresh-checkout engine SHA pin;
- one `crate::automexia` product/extension namespace;
- transactional retirement of v0.2 compiled extension shims;
- background bounded extension discovery worker;
- failure-tolerant optional extension service startup;
- bounded per-working-directory extension context cache;
- cached snapshots + atomic generations;
- capability manifests and least-privilege first-party extension declaration;
- renderer consumes cached models only;
- shared-render-model/backend-adapter policy;
- explicit source ownership/dependency rules;
- separate fast-development and full-release gates;
- documented path to a self-contained Automexia repository.
