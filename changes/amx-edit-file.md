Added `amx edit` for explicit existing-file handoff to VS Code or Insiders with
line/column positioning, strict user-level preferences and a no-launch preview.
Editing can be disabled; invalid configuration and ambiguous paths fail closed.
WSL uses the existing isolated leased path probe and host desktop handoff without
requesting a remote editor installation. No arbitrary editor command strings or
project scripts are executed. Native desktop visibility remains a separate check.
