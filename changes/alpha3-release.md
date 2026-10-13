Changed

- Prepare 0.0.3-alpha with self-contained Windows C++ runtimes and branded MSI
  setup. Preserve prior immutable releases, clear download names, signing
  disclosures, and the native package and reproducibility gates.
- Check the Debian package version before starting native release builds.
- Isolate native PowerShell scope tests from the parent shell's module paths
  and report a fixed, bounded stage when a probe cannot finish.
- Preserve CMD's native command quoting so packaged prompt integration loads
  from paths with spaces. Cover startup, command execution, and idle behavior
  through the real ConPTY launch path.
