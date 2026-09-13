# Public stabilization roadmap

This roadmap contains only maintenance work for the public free terminal. It
does not list new products or private feature plans.

## Priorities

1. Correctness for VT state, Unicode, reflow, selection, search, images, and
   alternate-screen behavior.
2. Native PTY/process lifecycle, resize, output pressure, cancellation, child
   cleanup, and shutdown.
3. Route, focus, window, tab, pane, clipboard, IME, and overlay isolation.
4. Configuration validation, migration, private persistence, recovery, and
   uninstall.
5. Shell integration fallback and native-shell compatibility.
6. System OpenSSH interoperability and explicit inventory safety.
7. Accessibility, high contrast, reduced motion, scaling, and native
   assistive-technology evidence.
8. Startup, input latency, memory, GPU memory, handles, threads, caches, storage,
   and long-session cleanup.
9. Package identity, provenance, signatures, install, upgrade, rollback, and
   uninstall.
10. Documentation integrity, confidentiality, links, and exact status.

## Performance proof plan

Measure startup, input-to-render latency, sustained output, resize pressure,
memory, GPU memory where observable, handles, threads, child processes, cache
and storage growth, cancellation, and final cleanup on the real owning paths.
Compare release candidates with same-host baselines using documented,
noise-aware limits. A helper benchmark or cross-compile does not replace
native end-to-end evidence on a claimed platform.

## Exit criteria

A stable baseline has current deterministic tests, native evidence for every
claimed platform, reviewed visual/accessibility evidence, bounded resource
measurements, clean lifecycle repetition, verified packages, and no known
real-workflow failure.

Unreleased advanced and commercial sequencing is private.

## Assurance evidence anchor compatibility

These headings preserve source-owned feature-matrix references after the
public documentation consolidation. They do not expand shipped behavior,
reintroduce private plans, or replace the current status stated above.

### S1 Accessibility Baseline

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### S1 Executable Benchmark Pipeline

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### S2 Enforcement

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

## Command productivity delivery track

See [Command Productivity](COMMAND-PRODUCTIVITY.md) for the current implemented contract and release limitations. Stabilization requires the source-owned
policy, persistence, native-shell, UI, security, recovery, performance,
resource, accessibility, and packaged-artifact evidence to agree before an
availability claim advances.

Alias-specific scenarios are tracked in
[DEVOPS-ALIASES.md](DEVOPS-ALIASES.md); their source status does not replace
native-shell and packaged-artifact evidence.
