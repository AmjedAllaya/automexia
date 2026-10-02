Fix passive provider path validation on Linux and macOS by checking the original
path before normalization removes double slashes or control characters. Add
regressions for provider, kubeconfig and local-file callers, while preserving
ordinary local paths.

Repair the dependency trust gate with a restricted, pinned Bytecode Alliance
audit import and a reviewed exact-version process-wrap compatibility record.
Keep the existing dependency versions and all security checks enabled.

Group Dependabot's main and fuzz Cargo updates so shared manifest changes and
both lockfiles travel together, including security updates. Add a CI contract
regression for the coupled workspaces.

Keep the generated SSH Bash path limit in a local numeric variable so the source
template also passes ShellCheck without suppressing arithmetic warnings.

Correct Linux regression fixtures to require Windows home hints only on Windows
and accept an already-stopped native table fixture during cleanup. Keep the
table contents, readiness and bounded worker-join assertions intact.
