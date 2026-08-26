# ADR 0002: Non-destructive configuration migration

Status: accepted for v0.4; compatibility aliases expire in v0.5.0.

Automexia uses separate platform roots and imports a strict allowlist from Rio
only when Automexia configuration is absent. Atomic staging, progress/completion
markers, conflict preservation, and source immutability make migration repeatable
and allow both terminals to coexist.
