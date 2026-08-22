# Connection Hub and SSH

> **Implemented locally / release-gated:** v0.5 source builds expose a read-only
> Connection Hub. It can review one bounded host typed by the user or, after
> exact OpenSSH file review, browse public inventory and edit public
> favorites/tags. Both paths end at a disabled preparation review. The Hub
> cannot connect, login, refresh a cloud provider, run a recipe, launch a
> process, access the network, or create a PTY. Automexia v0.4 users should
> continue using normal system OpenSSH in the shell.
## Use the read-only Connection Hub

1. Press `Ctrl+Shift+H` on Windows/Linux/BSD or `Cmd+Shift+H` on macOS. You can
   also open the command palette and choose **Connection Hub (read-only)**.
2. To review one host without adding inventory, choose **Enter host** or press
   `L`. Enter a host or alias containing only ASCII letters, numbers, dots,
   underscores, or hyphens (maximum 512 bytes). The value cannot start with a
   hyphen. Press Enter to review or Escape to cancel.
3. Alternatively, select **Choose SSH files**. The native file picker accepts
   one or more exact files, including an extensionless `config` file.
4. Read the canonical-path review. Use Up/Down, Page Up/Page Down, Home/End, or
   the pointer to inspect it. Press Enter or choose the confirmation action to
   scan; press Escape or choose Cancel to revoke the selection.
5. Browse the resulting aliases. Select a row and press Enter to open its
   disabled Connection Review.
6. Review the public target, identity verification, direct system-OpenSSH route,
   strict host-key policy, requested capability, environment risk, destination,
   and `ssh <destination>` shape. Press Escape, activate the focused Back
   control with Enter, or choose Back with the pointer to return. The review
   never launches a connection.

Opening the Hub does not scan standard locations. The selected files remain an
in-memory grant for the current application lifetime and must be selected again
after restart. Canceling or making another selection revokes the older review.
Automexia never modifies the selected OpenSSH files.

The direct shortcut is inactive while Search, Vi mode, or an alternate-screen
terminal application owns input. It never sends a key or command to the PTY.

An exact-file review belongs only to the Hub screen that initiated it. Another
window or screen cannot display, confirm, cancel, or revoke that review. If the
bounded background worker is temporarily full, the request fails immediately
with a path-free busy state; close obsolete work or retry after the active local
operation settles.

## Browse efficiently

The first-run setup keeps two clear choices visible: **Enter host** and
**Choose SSH files**. Search, filters, grouping, and catalog shortcuts appear
only after a usable catalog exists; hidden controls cannot receive pointer,
keyboard, or IME input. **Clear** appears only while a filter is active. The
modal keeps terminal input inert until it closes. These controls work from the
results route while the search field is not receiving text:

| Key | Result |
|---|---|
| Up/Down, Page Up/Page Down, Home/End | Move the managed selection |
| Enter | Open the disabled preparation review; activate Back when it is focused |
| `L` | Open the direct-host editor; Search still receives `l` as text while focused |
| `/` or `Ctrl/Cmd+F` | Focus search |
| Tab / Shift+Tab | Move modal focus |
| Space | Review a favorite change for the selected row |
| `T` | Edit public tags for the selected row |
| `G` | Cycle grouping |
| `V` | Toggle favorites-only |
| `R` | Toggle recent-only; M1 never writes recent-use |
| `S` | Cycle source filter |
| `X` | Clear search and all filters |
| Escape | Cancel the active review/editor, then close the Hub |

Search and tag fields support normal text input and IME composition. Tags are
case-insensitively deduplicated. Control and bidirectional formatting characters
are rejected so labels cannot disguise their visible meaning.

Committed search or filter changes update the bounded catalog once. Normal
unchanged frames reuse that immutable result, and IME preedit is validated
without re-searching the complete catalog.

## Review favorites and tags

Favorite and tag changes are public Automexia metadata; they do not edit
`ssh_config` or grant connection authority.

1. Select a row and press Space to toggle its favorite state, or press `T` to
   edit its tags.
2. Review the before/after diff.
3. Press Enter to save or Escape to cancel.

The save uses the revision you reviewed. If another writer changed the document,
Automexia reloads it and reports a conflict instead of overwriting it. A scan,
review, canceled action, or failed future connection never marks a host recent.

## What the status states mean

- **Setup** — no file has been confirmed; choose exact files when ready.
- **Loading** — the bounded background worker is scanning the reviewed grant.
- **Ready** — a current immutable catalog is available.
- **Stale** — a refresh failed; the last-known-good catalog remains visible.
- **Unavailable/recovery** — a private metadata/library operation failed or a
  recovery generation was used. The displayed diagnostic is intentionally
  path-free; browsing can continue when safe, but edits may be disabled.
- **Filtered** — the catalog exists but the active query/filters match no rows;
  use `X` to clear them.

Candidate platform paths are guidance only. Typical files are `%USERPROFILE%\.ssh\config`
on Windows and `~/.ssh/config` on macOS/Linux/BSD, but Automexia reads neither
until you select the exact file. Linux/BSD desktop packages need a working XDG
portal backend or the documented native-dialog fallback; if no picker is
available, the Hub stays open and fails closed.

## What the current managed-SSH preparation means

For a selected direct inventory alias, the Hub builds a current,
generation-bound public profile and canonical pending F2 plan. A typed literal
host uses the same pure preparation, but remains transient and is classified as
production risk until a trusted profile can classify it. Both paths end in the
compact Connection/Safety/Launch review. Identity and executable verification
are explicitly pending, host-key handling stays owned by system OpenSSH, the
only requested capability is `session.launch`, and the primary action remains
disabled.

The literal editor accepts one exact host argument only. User, port, URI, IPv6,
jump, tunnel, wildcard, Unicode, whitespace, control, bidirectional, option, and
shell forms fail closed; rejected paste or IME text is not partially inserted.
Tab and Shift+Tab stay inside the editor, Review, and Cancel controls. While
the nested editor is open, Cancel replaces the redundant top-level close icon
so scaled small viewports keep one clear, non-overlapping dismissal action;
Escape performs the same cancellation. Cancel, owning-surface close, or
successful preparation clears the editor.

Exact inventory aliases, opaque identity/source references, executable digests,
and plan fingerprints are not copied into presentation state. Inventory
selection, route, catalog, metadata, runtime-state, or generation changes
discard or rebuild inventory preparation; an unrelated catalog refresh cannot
rewrite a transient literal review. Unsupported ProxyJump inventory entries
fail closed with a path-free diagnostic. No OpenSSH process, network
connection, PTY, login, reconnect, persistence, history, or recent-use write
occurs.

This is a review-preparation workflow, not a Connect button. Continue using the
system client below; if future managed SSH is disabled or unavailable, that
manual path remains the recovery path.
## Connect with system OpenSSH today

The read-only Hub does not replace the shell. Use the system client normally:

```text
ssh production
ssh user@example.com
ssh -p 2222 user@example.com
scp local.txt production:/tmp/
sftp production
```

OpenSSH remains authoritative for includes, wildcard/negated hosts, `Match`,
proxies, keys, certificates, agents, host-key policy, and authentication. The Hub
indexes only a bounded static public subset and never reads private-key bytes or
evaluates `Match exec`, `ProxyCommand`, `LocalCommand`, or shell text.

## Privacy, storage, recovery, and rollback

Automexia stores only its bounded public metadata below the normal configuration
root:

- Windows: `%LOCALAPPDATA%\Automexia\Terminal`
- macOS: `~/Library/Application Support/io.github.AmjedAllaya.AutomexiaTerminal`
- Linux: `$XDG_CONFIG_HOME/automexia`, or `~/.config/automexia`

D4 favorite/tag metadata is under `extensions/devops-ssh`; the private local
profile/recipe/preference document is under `connections`. Raw selected paths,
typed literal hosts, keys, passphrases, tokens, provider credentials, search
text, and terminal history are not stored by the M1/M3 Hub. Each owned store is bounded, user-private,
compare-and-swap protected, atomically replaced, and retains at most one
validated recovery generation.

To disable the feature, close it and do not invoke the palette action; it does
no passive scanning or provider work. Removing the palette/runtime code is a
clean product rollback and leaves normal terminal/manual OpenSSH behavior
unchanged. Before deleting Automexia-owned connection metadata manually, stop
all Automexia processes and make a backup; never delete or rewrite OpenSSH
configuration, keys, agents, or `known_hosts` as a Hub recovery step.

## What remains planned

D5.2 still owns reviewed system-OpenSSH launch, current executable/identity
observation, independent PTYs, typed user/port/routes, jumps, tunnels, full
host-trust explanation, cancellation, reconnect, receipts, and lifecycle
evidence. D6 separately owns AWS, Azure, Google Cloud, Kubernetes,
OpenShift, Teleport, and OpenBao adapters. There is no current Hub remote-file
browser, credential vault, automatic provider login, or cloud refresh.

See [Remote sessions and WSL](remote-and-wsl.md), [OpenSSH inventory](../SSH-INVENTORY.md),
and the [connectivity roadmap](../SSH-CONNECTIVITY-MULTI-ENVIRONMENT-MULTI-CLOUD-PLAN.md).
