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
