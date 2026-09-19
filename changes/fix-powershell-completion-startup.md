### Fixed

- Reduced PowerShell completion startup overhead by replacing duplicate provider
  filesystem probes with direct, bounded metadata checks while preserving link,
  type, size, consent, and digest validation.
