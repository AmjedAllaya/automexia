# Public feature catalog

This catalog describes only the free terminal capabilities intended for public
documentation. It is not a roadmap. Unreleased advanced work and all commercial
planning are maintained only in the ignored private documentation area.

A source module, fixture, test, or internal type is not evidence that an
unlisted product capability is available.

## Terminal capabilities

| Capability | Public behavior | Documentation |
|---|---|---|
| Terminal emulation | VT/CSI/OSC/DCS parsing, Unicode text, scrollback, reflow, search, mouse reporting, and keyboard selection | [Architecture](ARCHITECTURE.md), [testing](TESTING.md) |
| Native shell sessions | Unix PTY and Windows ConPTY launch, resize, ordered input, exit reporting, and cleanup | [Platforms](PLATFORMS.md), [troubleshooting](TROUBLESHOOTING.md) |
| Windows, tabs, and panes | Independent windows, tabs, pane-local tabs, mnemonic fresh/cloned splits, focus movement, divider resizing and grouped discovery with left-arrow Back; current-source shortcut labels follow effective configuration | [Keyboard](KEYBOARD.md), [terminal guide](guide/terminal-experience.md) |
| Search and selection | Scoped search, keyboard and pointer selection, copy, paste, and command navigation | [Keyboard](KEYBOARD.md), [productivity guide](user-guide/productivity.md) |
| Appearance | Fonts, themes, opacity, cursor choices, line spacing, and saved appearance preferences | [Configuration](CONFIGURATION.md), [customization](user-guide/customization.md) |
| Shell integration | Session-local integration for supported shells, prompt metadata, and object-preserving listings | [Shell integration](SHELL-INTEGRATION.md) |
| Explicit web search | `amx google`, `amx search` and `amx docs` use encoded queries, default-browser handoff and offline URL preview | [Browser search](user-guide/google-search.md) |
| Explicit local tools | `amx find` uses installed ripgrep; `amx explain` displays offline tealdeer examples, with bounded execution and argument previews | [Local tools](user-guide/local-tools.md) |
| Directory opening | `amx open` hands an existing directory to the desktop file manager, with a non-launching destination preview | [Open a directory](user-guide/open-directory.md) |
| File editing | `amx edit` requests VS Code or Insiders at a file/line/column, with user preferences, disable and no-launch preview | [Edit a file](user-guide/edit-file.md) |
| Repository navigation | `amx repo` opens GitHub.com/GitLab.com repository or issue pages from local Git metadata, with offline preview | [Open a repository](user-guide/open-repository.md) |
| Local and inline images | Bounded local raster preview plus Sixel, Kitty, and iTerm2 protocol images | [Image previews](IMAGE-PREVIEWS.md) |
| OpenSSH interoperability | Ordinary system OpenSSH use and explicit, reviewed local inventory behavior described in the public guide | [Remote connections](guide/remote-connections.md), [SSH inventory](SSH-INVENTORY.md) |
| Accessibility | Keyboard-only operation, focus models, high-contrast support, reduced motion, and renderer-neutral accessibility semantics | [Accessibility](ACCESSIBILITY.md) |
| Configuration and migration | TOML configuration, transactional reload, platform overrides, and non-destructive migration | [Configuration](CONFIGURATION.md), [migration](MIGRATION.md) |
| Platform packages | Windows, macOS, and Linux packaging with checksums, provenance, and platform signing requirements | [Installation](INSTALLATION.md), [release trust](RELEASE-TRUST.md) |

## Contributor capabilities

The public repository also documents build, test, packaging, security, and
extension-safety contracts needed to maintain the free terminal:

- bounded and cancellable background work;
- one owner for every PTY, process, terminal state, and renderer surface;
- explicit capability boundaries for installed code;
- untrusted-input validation and redacted diagnostics;
- architecture, dependency, fuzz, resource, and release checks; and
- honest separation between local source evidence and native release evidence.

See [Architecture](ARCHITECTURE.md), [Extensions](EXTENSIONS.md),
[Testing](TESTING.md), [Security](../SECURITY.md), and
[Contributor CLI](CLI-REFERENCE.md).

## Publication boundary

The absence of a feature from this catalog is intentional. Do not infer
unreleased integrations, services, specialized workflows, automation products,
provider products, organization features, or commercial packages from source
names or private development scaffolding. Publication requires an explicit
review under [the private documentation policy](PRIVATE-DOCUMENTATION-POLICY.md).

## Free shell-productivity source foundations

The free source boundary includes explicit insert-without-execute actions,
opt-in native aliases, reviewed static packs, and bounded candidate imports.
CP3.3 remains non-executing and release-gated by the feature catalog and native
evidence. See [DEVOPS-ALIASES.md](DEVOPS-ALIASES.md).
