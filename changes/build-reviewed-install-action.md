# Preserve assurance while updating the pinned tool installer

Adopt the reviewed taiki-e/install-action 2.87.5 commit across workflows.
Tool versions remain explicitly pinned; the upstream change updates unrelated
tool manifests and does not alter the installer runtime. Compiler/Python setup,
cache boundaries, security scans and release gates remain mandatory.

Keep compiler identity documentation under ADR 0053, including the Python
bootstrap contract, without duplicating renderer-benchmark ADR 0047.
