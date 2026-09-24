# ADR 0072: resolve the terminal palette before rendering

Status: accepted for current source; native and full contributor evidence remain tracked with the change.

## Contract and ownership

User-selected terminal colors take precedence over the product palette. The existing
backend configuration owner resolves the active platform theme before reading theme
files. Only the selected platform theme is read; an empty platform override clears
the base selection. Named themes retain their established precedence over an inline
`[colors]` table, and adaptive palettes retain their existing appearance selection.
A complete named/adaptive set is prepared before any candidate fields are published.
Runtime errors preserve the application's existing last-known-good transaction.

The existing backend defaults module owns the unchanged product palette constants.
Application configuration context supplies these defaults and the established
`AUTOMEXIA_UNIFIED_COLORS` opt-out. A narrow callback receives the validated candidate
and previews global then active-platform environment entries with the shared strict
parser. Last matching assignment wins, including Windows case-insensitive names,
without mutating the environment or parsing the configuration file again. Pure defaults and renderer code do not discover
the environment. An explicit inline palette, even an empty table or a value equal
to a legacy default, remains explicit. Unspecified fields inside a selected palette
retain the existing `Colors` defaults; this change does not add a new per-field
merge policy. The public backend loader retains legacy defaults for embedding callers.

Renderer construction and reload consume prepared colors directly. Explicit RGB
cell values and session-local OSC overrides retain their existing terminal owners.
Shared chrome contrast policy is unchanged. Platform environment entries are still
appended exactly once by the existing application platform merge.

## Reuse, limits and compatibility

A private Serde input wrapper consumes optional colors and flattens the existing
configuration schema. This records presence without comparing values, copying the
schema or parsing TOML twice. The serialized configuration contract is unchanged.
See [Serde field attributes](https://serde.rs/field-attrs.html) and
[struct flattening](https://serde.rs/attr-flatten.html).

No dependency, configuration key, extension, worker, capability or persisted schema
is added. Palette literals retain the shared stack-only decoder from
[ADR 0058](0058-bounded-colour-setup.md). Existing file-byte ceilings and configuration
I/O ownership remain in place. No file loading or default discovery enters drawing.
The app palette helper remains identity compatibility forwarding; it is no longer
an alternative palette authority.

Value-equality heuristics were rejected because they cannot distinguish an explicit
default-valued choice from omission. A second color schema, whole-config custom
visitor and new shared crate would duplicate existing ownership. The application
branding choice is a user-approved behavior correction; preserving its old renderer
override is not an acceptable rollback while documenting user palette precedence.

## Evidence

Regressions reproduce loss of configured colors in actual renderer construction,
incorrect platform-theme palette selection and missing adaptive-only fixture
resolution. Focused cases cover construction/reload, session isolation and RGB,
explicit default-valued/partial/empty palettes, legacy and product defaults,
named-theme precedence, absent inactive/base themes, failed adaptive pairs,
serialization compatibility and single platform environment merge. A dedicated
allocation oracle checks first and repeated fixed-palette setup with an allocating
canary and unwind-safe counters. Existing application palette benchmarks retain
independent numerical assertions.

Native Windows/Linux/macOS and full contributor evidence are recorded by the
integrating review; a source check is not a native or compositor result. This change
does not claim to repair unrelated theme-path containment or configuration watcher
I/O scheduling.
