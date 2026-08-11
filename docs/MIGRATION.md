# Rio configuration migration

Automexia and Rio can be installed and run side by side. Automexia never installs
a `rio` executable alias, declares a package conflict, writes into Rio's root, or
deletes Rio data.

On v0.4 startup, if no Automexia `config.toml` exists, Automexia validates and
imports only:

- Rio's `config.toml`;
- `.toml` files beneath Rio's `themes/` directory;
- bounded `installed` and `disabled` marker files beneath the legacy
  `automexia/extensions/` state.

Logs, caches, backups, symlinks, executable content, large marker payloads, and
unknown files are not copied. Each file is staged and renamed atomically. An
in-progress marker allows an interrupted migration to resume; a completion
marker makes repeated starts idempotent. Existing Automexia files win conflicts.
Malformed legacy configuration aborts import without installing `config.toml`.

The source tree is read-only from the migration's perspective. Users can remove
their Rio installation later, but Automexia never does so automatically.
