# Connection Hub and SSH

> **Implemented locally / release-gated:** v0.5 source builds expose a read-only
> Connection Hub plus an actionable managed-SSH approval review. Allow once,
> Allow for session, and Deny exercise the fail-closed policy boundary; current
> builds report that protected review is pending. The dormant lifecycle source
> also owns exact direct/routed options, typed host/user/port, bounded config
> jumps, full host-key/public-identity evidence, safe command copy, child outcomes,
> notifications, receipts, and stale-source reconnect preparation. The activation gate is false
> and the linked package is unverified, so the Hub cannot launch a process,
> access the network, or create a managed PTY. Continue using system OpenSSH in
> the shell.
## Use the read-only Connection Hub

1. Press `Ctrl+Shift+H` on Windows/Linux/BSD or `Cmd+Shift+H` on macOS. You can
   also open the command palette and choose **Connection Hub (read-only)**.
2. To review one host without adding inventory, choose **Enter host** or press
   `L`. Enter the required host plus an optional user and decimal port in their
   separate fields. The host is limited to 512 bytes and cannot start with a
   hyphen; user and port have their own bounded validation. Tab/Shift+Tab moves
   Host → User → Port → Review → Cancel. Press Enter on Review or Escape to cancel.
3. Alternatively, select **Choose SSH files**. The native file picker accepts
   one or more exact files, including an extensionless `config` file.
4. Read the canonical-path review. Use Up/Down, Page Up/Page Down, Home/End, or
   the pointer to inspect it. Press Enter or choose the confirmation action to
   scan; press Escape or choose Cancel to revoke the selection.
5. Browse the resulting aliases. Select a row and press Enter to open Connection
   Review.
6. Review the public launcher/package state, typed target, direct or config-jump
   route, strict host-key state, full public algorithm/fingerprint when observed,
   public identity evidence, `session.launch`, production risk, 60-second approval,
   exact route-specific operation, and PTY input/output. Changed host keys show
   **BLOCKED** and cannot create a launch binding.
7. Choose **Allow once** (A or focused Enter), **Allow for session** (S), or
   **Deny** (D). Press `C` to copy the exact reviewed user-owned command without
   executing it or adding Enter. In the current nonactivated build, either Allow
   action shows **Protected security review is still pending** and starts nothing.
   Use Escape/Back to return.

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
| Enter | Open review from results; allow once only when Review/Allow once owns focus |
| `A` | Request Allow once from Connection Review |
| `S` | Request Allow for session from Connection Review |
| `D` | Deny and return from Connection Review |
| `C` | Copy the exact reviewed SSH command; never execute it or append Enter |
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
compact Connection/Safety/Launch review. Package and current executable
verification are explicitly pending, host-key handling stays owned by system
OpenSSH, and `session.launch` is the only requested capability. The three
decision actions are available, but execution remains independently disabled.

The literal editor owns separate host, optional user, and optional decimal-port
fields. URI, raw IPv6, jump, tunnel, wildcard, Unicode, whitespace, control,
bidirectional, option, and shell forms fail closed; rejected paste or IME text is
not partially inserted. Tab and Shift+Tab stay inside all three fields, Review,
and Cancel. While
the nested editor is open, Cancel replaces the redundant top-level close icon
so scaled small viewports keep one clear, non-overlapping dismissal action;
Escape performs the same cancellation. Cancel, owning-surface close, or
successful preparation clears the editor.

Exact inventory aliases, opaque identity/source references, executable digests,
and plan fingerprints are not copied into presentation state. Inventory
selection, route, catalog, metadata, runtime-state, or generation changes
discard or rebuild inventory preparation; an unrelated catalog refresh cannot
rewrite a transient literal review. A config route accepts only the first static
canonical comma-separated `ProxyJump` value, at most 8 hops/2 KiB, and produces
one exact `-J` argument. `none` remains direct; ProxyCommand, dynamic/ambiguous
routes, shell text, and excessive chains fail closed with path-free diagnostics.
No OpenSSH process, network connection, PTY, login, reconnect, persistence,
history, or recent-use write occurs.

This is an approval-check workflow, not an available Connect button. A fixed
diagnostic explains the blocked prerequisite without exposing private values.
Continue using the system client below; the manual path remains the recovery
path whenever managed SSH is disabled or unavailable.
## Review typed tunnels after activation

The nonactivated M5 review model adds one compact tunnel row to the Safety card.
It shows direction, exact public endpoints, session lifetime, OpenSSH ownership,
and planned/starting/ready/collision/failed/cancelled/closed state with an icon,
color, and text. Local and dynamic binds default to `127.0.0.1`.

A remote, non-loopback, or production tunnel always requires **Allow once**.
**Allow for session** is visibly disabled, omitted from focus, inert to pointer
input, and rejected if `S` is pressed. Any endpoint or route change invalidates
the review. These controls are source-complete but unreachable in production:
current builds start no managed SSH process or listener and change no SSH
configuration. Manual system OpenSSH remains the recovery path.

## What the managed lifecycle will do after activation

The source path can launch only an opaque fresh-review binding. No-tunnel direct
routes use 17 fixed options plus optional exact `-l user`/`-p port` fields and
one host; config-defined routes use the 15-option subset plus one exact `-J`
chain and one alias. Both keep all forwarding disabled. The separate tunnel
grammar uses `-F none`, fixed defensive options, exact typed `-L`/`-R`/`-D`
pairs, and one literal destination; it rejects configuration-dependent routes.
All paths disable multiplexing, local/remote commands, backgrounding, X11, agent,
and TUN forwarding. Automexia does not override OpenSSH key-exchange defaults or
weak-crypto warnings. OpenSSH still owns host-key prompts, authentication,
agent/certificate/hardware interaction, `known_hosts`, all terminal content,
and any approved forwarding sockets.

When a managed child ends, its actual status determines success, failure,
unavailable status, or cancellation. Route close is not success. The UI receives
fixed host/path-free notifications. Provider-neutral receipts can be queued to a
private store limited to 256 records and 2 MiB; they contain no destination,
terminal text, credential, path, environment, process ID, or executable identity.
Only an opaque inventory/source revision can support reconnect. Current inventory
must still match, and reconnect always returns to a fresh executable/host-trust
review and explicit approval—there is no automatic connection or command replay.

This contract is present for review and tests but unreachable in production until
package attestation, protected approvals, real native OpenSSH and forced child-
tree cleanup, resource, and accessibility gates pass.
## Review-only workspaces and broadcast

M6 workspace and recipe support is implemented internally but is not yet an
interactive product feature. Current builds may project a compact restore or
broadcast review in tests, but the Connection Hub has no activated control that
runs it.

A restore review shows the workspace, window/connection counts, exact target
rows, and a warning that automatic reconnect and interrupted-action resume are
off. A broadcast review shows `○ DISARMED` or `● ARMED` with matching text, icon,
and color; it requires the exact command and target list, an explicit arm action,
and a second production confirmation. It never presses Enter for you.

Until proposed ADR 0023 and the managed-SSH activation/native gates are accepted,
continue to arrange panes manually and type or paste commands into each intended
session. Do not assume a saved workspace reconnects, resumes a recipe, restores a
tunnel, or carries credentials. Schema-1 Connection Libraries are previewed in
memory and only advance after a reviewed CAS; imported workspace topology loses
its connection bindings and must be rebound locally.
## Provider authentication framework (current source boundary)

M7/D6.0 is complete as an internal provider-neutral framework, not as a current
cloud-login button. Automexia can validate and isolate a public provider context,
project it as available/refreshing/browser/device/MFA/ready/expired/offline/
denied/cancelled/stale/error, and require an exact visible one-time capability
review before a later adapter may ask the existing runner to start an official
CLI. Opening or filtering the Hub never performs a provider refresh or login.

The framework stores no token, password, client secret, browser/device code,
certificate, cookie, inherited environment, or provider cache. AWS CLI, Azure
CLI, Google Cloud CLI, `kubectl`/`oc`, and later organization tools keep
ownership of their own authentication, browser or device flow, multifactor
authentication, credentials, certificates, and caches. Automexia retains only a
bounded public in-memory observation and opaque configuration references.

D6.1/M8 now has an internal AWS source adapter, but still no active cloud-login
button. It can parse exact granted public profile hints, construct review-bound
IAM Identity Center PKCE/device and STS operations, describe an exact SSM plan,
and produce a non-mutating EKS dry-run intent. Execution remains false behind
the protected application runner, and no AWS process or network call happens on
Hub open, search, startup, or typing.

D6.2/M9 now has the same deliberately nonactivated boundary for Azure. Exact
granted public `az account` JSON may provide subscription, tenant, cloud, state,
and public identity-kind metadata. Review requests use `az login --tenant` with
CLI-owned Windows broker, browser, or device-code behavior and `az account show
--subscription`; they never run `az account set`. Bastion is AAD-only and bound
to the capsule subscription. AKS carries only an opaque reference to a future
M11-owned private transient file, never a user kubeconfig path.

Continue to authenticate with the official CLI in a shell today. Do not expect
the Hub to change an AWS default profile, Azure subscription, Google Cloud active
configuration, Kubernetes current context, or kubeconfig. Any later AWS or Azure
product flow must return to a fresh review and can be cancelled, revoked,
disabled, or removed without affecting ordinary terminal use. D6.3-D6.5
provider adapters and all real native/provider evidence remain separate work.
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
profile/recipe/preference document is under `connections`. The dormant managed
receipt store uses `connections/managed-receipts.v1.json` plus one validated
previous generation. Raw selected paths, typed literal hosts, destinations,
keys, passphrases, tokens, provider credentials, search text, terminal history,
environment, process IDs, and executable identity are not stored in receipts.
M7 provider-auth observations are memory-only public records; current builds
create no Automexia-owned provider-auth file, token cache, or browser state.
Each owned store is bounded, user-private, atomically replaced, and recoverable
only from its validated previous generation. No receipt file is written in
current builds because managed launch cannot start.

To disable the feature, close it and do not invoke the palette action; it does
no passive scanning or provider work. Removing the palette/runtime code is a
clean product rollback and leaves normal terminal/manual OpenSSH behavior
unchanged. Before deleting Automexia-owned connection metadata manually, stop
all Automexia processes and make a backup; never delete or rewrite OpenSSH
configuration, keys, agents, or `known_hosts` as a Hub recovery step.

## What remains planned

D5.2 still requires production activation, attested current executable/identity
observations, actual bounded public status execution, independent native PTYs,
tunnels, real OpenSSH/forced-cleanup/resource/accessibility evidence, and the
protected release gate. Typed user/port/config jumps, full host-trust explanation,
safe copy recovery, cancellation/reconnect/receipts source, and public identity
status parsing are complete locally but nonactivated. D6.0's provider-neutral capsule/authentication framework is complete locally,
D6.1 AWS and D6.2 Azure adapters are source-complete but nonactivated; D6.3-D6.5 still own Google Cloud, Kubernetes, OpenShift, Teleport, and OpenBao adapters. There is no current Hub remote-file browser,
credential vault, automatic provider login, cloud refresh, or provider command
execution.

See [Remote sessions and WSL](remote-and-wsl.md), [OpenSSH inventory](../SSH-INVENTORY.md),
and the [connectivity roadmap](../SSH-CONNECTIVITY-MULTI-ENVIRONMENT-MULTI-CLOUD-PLAN.md).
