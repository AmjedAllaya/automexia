
# Rust Core Principles

**Type:** Research/Request-for-Discussion note — not a project ADR
**Repository integration note:** Already implemented territory; retain safe-Rust principles only.

This file preserves proposal reasoning without claiming an ADR number or acceptance status.

---


## Decision
The trusted terminal platform is implemented primarily in Rust using a Cargo workspace. Safe Rust is the default; unsafe code is restricted to reviewed platform/FFI boundary crates.

## Consequences
Core domain types, process supervision, IPC orchestration, security brokers, extension host, storage, and diagnostics are Rust. Third-party extension ABI remains WIT/WebAssembly rather than Rust ABI.
