
# Technology And Supply Chain

**Type:** Research/Request-for-Discussion note — not a project ADR
**Repository integration note:** Supply-chain deltas now; dependency adoption remains separate.

This file preserves proposal reasoning without claiming an ADR number or acceptance status.

---


## Candidate policy

Automexia architecture depends on Automexia-owned interfaces, while exact third-party versions are pinned and qualified per release.

Release builds separate dependency acquisition from compilation. After the reviewed dependency source set is available, release build/test jobs run without outbound network access where practical and use locked/offline dependency resolution.

Current release inputs:

- exact Rust toolchain per release;
- current pinned/benchmarked wgpu and existing locked dependencies.

Candidate inputs, only after an accepted owning milestone:

- Tokio only for a measured asynchronous ownership need;
- Wasmtime only after proposed ADR 0029 is accepted and runtime work authorized;
- SQLite only for an approved bounded persistence contract;
- managed or compatible system FFmpeg only after a distribution ADR;
- exact model/runtime assets only after artifact-level license, provenance,
  size, platform, security, and rollback review.

Known sandbox/supply-chain advisories trigger expedited patch qualification independent from normal feature release cadence.
