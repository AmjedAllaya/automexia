# ADR 0093: Named terminal profiles

Status: Implemented in source; native desktop evidence is platform-specific.

## Decision

The backend configuration owns a version-1 typed profile document. Profiles layer
an exact executable and argument array, directory policy, theme reference and tab
presentation over the effective configuration. Environment overrides contain only
variable-name references; values are resolved at launch and are never serialized.

The application owns a lazy bounded file worker, private transactional storage,
conflict detection and generation-scoped publication. Settings uses its existing
search, field, color, confirmation, keyboard, pointer and IME controls. Import
creates a reviewable draft. Save, import and selection cannot launch a process.

An explicit Open action resolves the profile before invoking the ordinary
ContextManager launch path. The same SessionLaunchDescriptor and native adapters
own the resulting PTY. Unix profile launches use the existing spawn adapter so
per-session directory and environment settings are retained. Existing shell
configuration and ordinary new-tab behavior remain compatible.

The focused session supplies its optional palette to the shared window renderer;
returning to an ordinary tab restores the window palette. A temporary Theme Gallery
preview takes priority while its view is open; leaving restores the profile palette. The profile's tab icon
and accent belong to its session/tab, not a global preference. Named profiles do
not activate the managed extension launch broker, add vault access, evaluate
shell strings, or implement automatic command workflows. System OpenSSH retains
credential, host-key and authentication ownership.

## Compatibility and failure

Configured records use `[profiles]` in config.toml. Visual edits use a separate
private profiles/profiles-v1.toml document. Saved records replace matching
configuration IDs; other configured records remain available. Built-in starter
profiles are only templates. Existing configuration requires no migration.

Unknown versions, duplicate identifiers, malformed fields, unavailable environment
references and invalid local directories fail before launching. File writes use
private_fs atomic replacement under an advisory lock and compare the original
file bytes. A stale save cannot replace another writer's changes. Closed or
replaced views cannot consume stale results. Export creates a new private file.

Workspace recovery retains its existing restricted shell/topology contract: it
does not replay arbitrary profile arguments, resolve environment references, or
reconnect SSH automatically. Profile launch remains an explicit user action.

## Evidence

Backend profile tests cover roundtrip, malformed and future schemas, platform
matching, exact arguments and environment precedence. Storage tests cover private
writes, conflicting writers, canceled operations and invalid files. Native Windows
fixtures exercise keyboard and pointer editing, discard cancellation, saving and
explicit independent tab launch on CPU and WGPU. Native Linux/macOS and assistive
technology delivery are separate evidence, not inferred from these tests.
