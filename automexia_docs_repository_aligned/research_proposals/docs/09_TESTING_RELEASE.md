
# Testing and Release Evidence

This is one of the strongest proposal areas and should be integrated through existing `xtask`, fixtures, assurance matrices, and CI.

Track dimensions separately:
- correctness;
- resilience;
- security;
- performance;
- Windows/Linux/macOS;
- accessibility;
- packaging/signing;
- cleanup/resource duration.

Do not collapse them into "done."

Useful additions:
- deterministic scenario IDs/seeds;
- Process Spy;
- environment poisoning;
- PTY fixture;
- filesystem attacks;
- network chaos where relevant;
- event replay;
- targeted Loom;
- targeted mutation;
- unsafe/FFI sanitizer jobs;
- evidence receipts tied to exact commit/artifact.

Keep this as a thin evidence/scenario layer over existing owners rather than a parallel test authority.

"Fuzz target compiles" is not "fuzz campaign passed."
