# Alpha 4 dependency refresh

The fresh October 26 advisory check rejected six vulnerabilities, three
upstream maintenance notices and a yanked clipboard release. Keep vulnerability
and yanked-package checks enabled; update only the affected dependency families.

## Patched dependencies

- Wasmtime/Pulley 48.0.7 and Cranelift 0.135.7 fix RUSTSEC-2026-0378 and
  RUSTSEC-2026-0379. Keep the existing optional component-host features, disabled
  public activation gate and resource limits. The upstream patch bounds branch
  tables and replaces the range-limited AArch64 default branch with a clamped
  table entry. Component stream forwarding validates payload types before using
  erased values. See the [upstream release](https://github.com/bytecodealliance/wasmtime/releases/tag/v48.0.7).
- ZIP 9.0.0 fixes RUSTSEC-2026-0367, RUSTSEC-2026-0368 and
  RUSTSEC-2026-0369. Keep default features disabled and the existing zlib-rs
  backend. Automexia reads bounded entries into memory; it never calls the
  high-level filesystem extraction API implicated by these advisories. Its
  existing path, link, permission, collision and expansion checks remain required.
  See the [upstream release](https://github.com/zip-rs/zip2/releases/tag/v9.0.0).
- ctutils 0.4.4 fixes RUSTSEC-2026-0385. Choice masks now use the existing
  optimization barrier; 32-bit `u128` comparisons combine opaque half-width
  predicates. The source-reviewed delta is recorded in `audits.toml`; this is
  neither a formal cryptographic audit nor measured timing certification.
- clipboard-win 5.4.3 replaces yanked 5.4.1. Its source delta bounds HTML
  fragment slicing and backs off five milliseconds between clipboard-open
  attempts. The Unicode APIs used by Copypasta, features, dependency requirements
  and BSL-1.0 license stay unchanged. Keep the existing clipboard adapter and
  the upstream retry-count bound; do not add an application retry loop.

All 30 old/new registry archives were verified against their lockfile checksums.
The Wasmtime family retains its license, Rust 1.95 requirement and build inputs.
ctutils raises its compiler minimum to 1.87; ZIP remains at 1.88. Both fit the
pinned toolchain and retain their licenses. The locked package graph adds no new
dependency names. ZIP's major-version source reorganization requires the owning
package-verification tests before delivery; the Windows clipboard consumer also
requires its owning target and native package evidence.

Retain the immutable Bytecode Alliance Cargo Vet import and refresh only the
27 existing publisher records for this Wasmtime patch. Preserve all wildcard
criteria and import scope. ZIP retains an exact-version compatibility exemption
after review, not a claim of a full independent source audit. Do not regenerate
the trust baseline or accept unrelated records.

## Owner-approved maintenance exceptions

On October 26, the repository owner explicitly approved only these three new
maintenance exceptions:

| Advisory | Standalone crate | Upstream replacement |
|---|---|---|
| RUSTSEC-2026-0370 | zvariant | zbus 6 wire/types |
| RUSTSEC-2026-0371 | zbus_names | zbus 6 names |
| RUSTSEC-2026-0372 | zvariant_derive | zbus 6 derives |

These are informational retirement notices, with no patched standalone version.
The [upstream migration](https://github.com/z-galaxy/zbus/issues/2043) consolidates
the crates in zbus 6. The latest Secret Service 5.2.0 still requires zbus 5;
AT-SPI common/proxies 0.14.0 also require zbus 5 and its standalone types.
AccessKit AT-SPI common 0.21.1 retains zvariant 5 and AT-SPI common 0.13.
Upgrading only Automexia's direct zbus dependency would leave these transitive
dependencies in place and introduce a second incompatible protocol stack.

Keep the three IDs mirrored in Cargo Audit, Cargo Deny and the exact-set
regression. Reassess them on every accessibility or credential-store dependency
change and remove them when both upstream consumer families support zbus 6.
Any newly reported vulnerability remains blocking; no vulnerability advisory is
added to the exception list. This decision does not replace or fork the mature
accessibility, D-Bus or credential-storage implementations.

## Required verification

Run the exact locked Cargo Audit, Cargo Deny and Cargo Vet gates; the package
verifier with all component-host features; the affected policy mutations; and
the required pre-push assurance profile in the existing disposable VM or hosted
runners. Rebuild every release package from the final source and require all
native package, session, accessibility, visual and reproducibility jobs before
publication. Preserve the earlier Windows settings failure separately from its
successful replay and stress experiment; neither experiment establishes a fix
for an unexplained intermittent failure.
