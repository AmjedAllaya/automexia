
# Supply-Chain Integration Delta

Integrate the useful dependency/supply-chain research into the project's existing:

```text
SECURITY.md
RELEASING.md
dependency-review policy
CI/xtask
```

Do not establish a parallel security authority.

## Recommended controls

```text
lockfile committed
exact source provenance
dependency review
build-script review
artifact checksums
SBOM
signed releases
offline/reproducible path where practical
```

For high-assurance release builds, separate:

```text
dependency acquisition
        ↓
review/cache/vendor
        ↓
network-restricted build/test
```

This reduces the authority of compromised build scripts and transient registries.

## New dependency gate

Before adopting a proposed dependency:

```text
need
maintenance health
security advisories
license
MSRV/toolchain compatibility
binary/startup impact
native platform behavior
rollback path
```

Current upstream freshness is not sufficient reason for adoption.
