# Optional self-hosted assurance runners

All specialized self-hosted assurance is optional in the GitHub-Free/private edition. The normal production release uses GitHub-hosted native runners.

Optional variables activate additional controls only when you actually operate the corresponding hardened runner:

```text
AUTOMEXIA_NATIVE_GUI_RUNNER=1
AUTOMEXIA_WSL_RUNNER=1
AUTOMEXIA_WINDOWS_PERFORMANCE_RUNNER=1
AUTOMEXIA_S1_ASSURANCE_RUNNER=1
AUTOMEXIA_HARDWARE_RELEASE_RUNNER=1
```

Do not set a variable to `1` until an online runner with the exact workflow labels exists. Self-hosted release/signing runners should be dedicated, patched, non-interactive, workspace-cleaned/reimaged, and restricted to trusted repositories.
