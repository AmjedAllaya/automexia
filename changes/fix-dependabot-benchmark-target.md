Declare the private renderer benchmark's existing source path explicitly so
Dependabot can resolve the workspace from manifests alone. Keep the single
benchmark target and all runtime-target restrictions intact. Add a real Cargo
regression for the updater's synthetic source layout and enforce the exact
benchmark path in the source-scanning policy.
