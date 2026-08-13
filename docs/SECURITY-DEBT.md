# Dependency security debt

`cargo deny check` remains mandatory. Exceptions are allowed only for
unmaintained advisories with no safe compatible upgrade, must include a reason
in `deny.toml`, and must be reassessed on every stable patch.

| Advisory | Current path | v0.4 decision | Removal condition |
|---|---|---|---|
| RUSTSEC-2025-0141 | `librashader-cache` -> `bincode` | Temporary exception; this is an unmaintained notice, not a reported vulnerability. | Upgrade or replace the renderer filter cache in v0.5. |
| RUSTSEC-2024-0436 | `baremetal` -> `paste` | Temporary exception; `baremetal` 0.33 is the latest release. | Adopt an upstream `baremetal` release using `pastey`, or replace the backend. |
| RUSTSEC-2026-0249 | `glslang` -> `smartstring` | Temporary exception; `glslang` 0.8.1 is the latest release. | Update the `glslang`/`librashader` chain or remove that shader path. |
| RUSTSEC-2026-0192 | direct and window-stack `ttf-parser` | Temporary exception; a `skrifa` migration changes renderer behavior and needs visual regressions. | Complete the audited font-parser migration in v0.5. |

The yanked `wide` 1.6.0 dependency was removed from the v0.4 lockfile by
updating to compatible `wide` 1.6.1. Vulnerability, unsoundness, and yanked
advisories have no standing exception policy.

## Runtime hardening debt

| Risk | Current protection | Required closure |
|---|---|---|
| Oversized or unterminated OSC/APC/DCS/XTGETTCAP input | Normal streams have optimized fixed/bulk paths and parser parity tests. | Add reviewed hard caps, discard-until-terminator states, deterministic recovery, rate-limited diagnostics, memory-bound tests, fuzzing, and normal-path benchmarks. A local, SSH, container, WSL, or multiplexer process is an untrusted PTY producer. |
| Hosted assurance not yet observed | Local dependency policy, architecture, identity, provenance, focused security regressions, and feature-gated test surfaces pass. | Require hosted CodeQL, fuzz, sanitizers, Miri, cross-platform jobs, signed artifacts, SBOMs, checksums, and provenance attestations before stable release. |

The control-string issue is a v0.4 source blocker, not an accepted advisory or
performance-only concern. Its implementation and acceptance gate are defined
in the [stabilization roadmap](STABILIZATION-ROADMAP.md#s0-bounded-control-strings).

## Duplicate dependency baseline

The v0.4 renderer, windowing, font, PTY, and operating-system backends contain
some incompatible transitive dependency generations. Cargo cannot merge these
entries because their dependants request non-overlapping semantic-version
ranges. They are compatibility and binary-size debt, not test failures or
security advisories.

`deny.toml` records every reviewed older version with an exact version and a
reason. The general `multiple-versions` policy is `deny`, so a pull request that
introduces any duplicate outside that baseline fails `cargo ready` and CI.
Removing a dependency from the graph also requires removing its stale baseline
entry. This makes the list an auditable ceiling rather than a blanket
suppression. Reassess and reduce the baseline during renderer/platform upgrades
and on every stable release.
