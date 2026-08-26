
# Technology Baseline — Current vs Candidate vs Deferred

## Current repository snapshot

According to the supplied 2026-08-23 source audit:

```text
Rust: 1.96.1
Edition: 2021
Terminal/PTY owner: existing application/ContextManager architecture
Terminal parser/renderer: existing Rio-derived implementation
First-party extensions: private linked contracts
```

The audit reports no current platform dependency on:
- Tokio;
- Wasmtime;
- SQLite;
- AccessKit;
- ONNX Runtime;
- FFmpeg.

## Candidate research

These are **not adopted merely because they are current upstream**:

- newer stable Rust / edition migration;
- Tokio LTS if async orchestration needs justify it;
- Wasmtime LTS + WIT for a future public/untrusted ecosystem;
- SQLite if persistence use-cases require it;
- AccessKit if it improves the actual accessibility integration;
- libghostty-vt as an optional terminal-engine/reference candidate;
- ONNX Runtime/FFmpeg only for future approved media features.

## Adoption gate

For each dependency:

```text
need
compatibility/MSRV
security advisories
license
binary/startup cost
native platform behavior
maintenance
rollback
benchmark
```

Freshness alone is not a reason to migrate.
