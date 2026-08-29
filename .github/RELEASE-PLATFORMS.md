# Release platform contract (2026)

This repository intentionally builds stable release executables on native GitHub-hosted
architectures instead of cross-compiling the official binaries from one host.

| Artifact target | Native build runner | Final packaging runner |
| --- | --- | --- |
| `x86_64-pc-windows-msvc` | `windows-2025` | `windows-2025` |
| `aarch64-pc-windows-msvc` | `windows-11-arm` | `windows-2025` (central signing/MSI tooling) |
| `x86_64-apple-darwin` | `macos-26-intel` | `macos-26` (universal assembly/signing) |
| `aarch64-apple-darwin` | `macos-26` | `macos-26` |
| `x86_64-unknown-linux-gnu` | `ubuntu-24.04` | `ubuntu-24.04` |
| `aarch64-unknown-linux-gnu` | `ubuntu-24.04-arm` | `ubuntu-24.04-arm` |

The GNU/Linux release workflow additionally fails if the final ELF requires a glibc symbol
newer than `GLIBC_2.35`, and it records the dynamic dependency resolution from the native
build runner. This is an explicit portability contract; change it only as a reviewed release
policy decision.

The macOS universal application is assembled only after its Intel and ARM64 slices have each
been built and executed on their native runner. Code signing, hardened runtime validation,
notarization, stapling, and Gatekeeper assessment remain fail-closed.

Windows ARM64 binaries are likewise built and executed on native ARM64 Windows. Final MSI
and Authenticode signing are intentionally centralized on the x64 Windows packaging runner,
which receives the already-tested ARM64 executable as an artifact.

Do not replace the explicit stable runner labels with `*-latest` in release workflows. The
explicit labels make release-environment migrations reviewable rather than silent.
