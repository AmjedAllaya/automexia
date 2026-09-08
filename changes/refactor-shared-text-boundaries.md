# Shared text boundaries

The capability-free text owner now exposes borrowed grapheme prefixes and an
explicit whitespace-preserving label variant. Existing trimmed end/middle
compaction keeps its output contract without materializing every input cluster.
Independent Unicode fixtures and a development-only same-host benchmark cover
the shared mechanics; byte validation and renderer measurement remain separate.
