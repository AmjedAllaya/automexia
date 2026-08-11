# Automexia extension platform

## Current stage

v0.3 supports compiled, trusted first-party extensions while defining the contracts needed for a future sandboxed ecosystem.

Each extension has:

- stable ID;
- name/description/version;
- declared capability set;
- activation state;
- optional background discovery/model production;
- optional terminal-row semantic analysis;
- optional UI model/overlay contribution.

## First-party DevOps reference implementation

`automexia.devops` is the reference compiled extension for these contracts. v0.3.2 keeps its responsibilities split into manifest, renderer-neutral model, background local discovery, and pure semantic classification. It is passive: it adds no shell/CLI commands and requests neither process-spawn nor network capability. The native HUD is enabled by default on a fresh install, with an explicit persisted disable state. See `DEVOPS-EXTENSION.md`.

## Capability broker plan

Third-party extensions must not be native dynamic libraries loaded into the terminal process. The planned model is a Wasm guest with host calls such as:

```text
host.fs.read_scoped(...)
host.env.get(...)
host.terminal.subscribe_rows(...)
host.ui.publish_status(...)
host.clipboard.read/write(...)
host.process.spawn_approved(...)
host.net.fetch_approved(...)
```

Every host call maps to a declared capability and policy. Filesystem/network/process access should be denied by default and constrained by user-approved scopes.

## Lifecycle

```text
discover package
 → validate manifest/signature/hash
 → show requested capabilities
 → install package atomically
 → activate sandbox instance
 → receive events / publish models
 → suspend/deactivate
 → uninstall owned package data only
```

## Compatibility policy

The extension API should use independently versioned Automexia contracts. Extensions must not import renderer, PTY, parser, or window implementation types directly.
