# Disposable environment contract

Read this when selecting or preparing an execution environment. Discover the actual supported tools and images; do not hardcode one developer's machine, VM, paths, OS edition, or installed software into shared tooling.

## Select by evidence

| Purpose | Environment | Limits to state |
| --- | --- | --- |
| Unit/integration/build checks | Disposable guest or ephemeral runner with declared pinned tools and isolated mutable output | Developer prerequisites may hide missing package dependencies |
| Installation and first-run behavior | Clean customer OS baseline, no prior Automexia state or undeclared developer tools | Baseline still has an explicit OS version, updates and installed components |
| Upgrade/migration | Reset guest plus declared supported prior package and fictional user data | Proves the tested upgrade path, not every historical version |
| Native terminal/UI/accessibility | Guest or ephemeral machine with the actual OS, interactive desktop, shell and required accessibility services | Virtual GPU, remote desktop, font and scale differences constrain claims |
| Linux package/dependency/shell checks | Disposable distro VM, or a container when userland-only evidence suffices | A container shares its kernel and does not prove desktop, boot, kernel or system-service behavior |
| macOS native behavior | Authorized macOS environment on supported Apple hardware | Cross-compilation and another OS's UI cannot substitute |
| Performance regression | Controlled isolated baseline/candidate environments on comparable hardware and guest settings | Shared hosted load, virtualization and software rendering can dominate results |

Run containers inside an approved guest or ephemeral runner. Their suitability for userland checks does not permit executing project tests on the personal host.

A disposable VM needs a trusted baseline. Record installation-media provenance and available official integrity verification; do not call an arbitrary existing ISO trusted. Licensing, hardware and required user setup remain prerequisites. Guest additions/agents are declared test infrastructure, not proof of customer defaults.

## Baseline separation

- **Customer baseline:** OS updates and only declared essential guest infrastructure. No application, source tree, toolchain, development shell profiles, custom fonts, or saved application preferences.
- **Builder baseline:** explicit compiler, SDK and dependency tools. Follow repository compiler selection; do not change a host's global toolchain. Treat dependency caches as declared inputs and do not inherit mutable build outputs for cold-build claims.
- **Performance baseline:** controlled resource allocation, power/load conditions, renderer, clock, fonts and workload. Cold and warm measurements are separate cases.

Record baseline versions and rebuild them deliberately when inputs change. Prefer resetting the existing authorized VM between independent cases. Separate snapshots preserve customer and builder states without requiring extra VMs. Verify the restored prerequisites and application state; a snapshot label alone proves neither. Never update the baseline while another run relies on it.

## Input and output boundary

Required run identity:

- stable run label, requested claim, source revision and dirty source/archive digest;
- OS/architecture, baseline image or snapshot identity, compiler/dependency versions;
- package digest when executing a package, features and configuration origins;
- prerequisite inventory, relevant shell policy, network and mount permissions;
- resource/time budgets, expected tests and permissible skips;
- export allowlist, cleanup owner and verified final state.

Retain machine-specific identifiers in local operational state only. Public evidence uses stable labels and sanitized paths. Never copy credentials into a guest to make unrelated commands convenient. Separate test fixture credentials from real accounts.

A source archive must include required untracked inputs and preserve relevant permissions/symlink semantics safely. Reject paths that escape staging. Do not recursively copy the entire working directory and hope ignore rules remove private data.

## Failure cases for an environment driver

When implementing or changing a runner, prove applicable boundaries with focused tests:

- wrong baseline, wrong instance, host execution, missing desktop, and reset failure are rejected;
- source/package digest mismatch and missing required inputs cannot produce passing evidence;
- path traversal, links/reparse escapes, unexpected writable mounts, and foreign ownership cannot affect the host;
- timeout, guest crash, interrupted transfer and incomplete result files remain failures with retained cleanup ownership;
- cleanup targets only the recorded disposable instance and cannot remove the baseline or unrelated files;
- a passed process with zero expected test cases, skipped required cases, or an invalid report is not accepted.

Keep pure manifest/path/ownership decisions separately testable from thin hypervisor adapters. Reuse maintained virtualization tools rather than rebuilding their protocols. Never weaken an existing hosted-only guard before a real guest boundary and its negative cases exist.
