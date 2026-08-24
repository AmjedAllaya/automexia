# Remote sessions and WSL

> **Connection Hub is read-only and release-gated.** v0.5 M1 source builds can
review selected OpenSSH inventory and public favorites/tags, but cannot launch a
managed session. For the exact workflow, use **[Connection Hub and
SSH](connection-hub-and-ssh.md)**.

Automexia can host the real shells and system tools used for remote work. The
important distinction is between **using OpenSSH/WSL normally today**, the
**read-only inventory Hub**, and the still-planned managed launch path.

## Current status

| Capability | Status | What you should do |
|---|---|---|
| Run system `ssh`, `scp`, `sftp`, provider CLIs, Kubernetes tools in the shell | **Available now** | Use them normally; they keep their own semantics/auth/config |
| WSL-hosted shell sessions and Automexia shell/context integration | **Available now** | Use the configured/selected WSL launch path and normal Linux tools |
| Static bounded OpenSSH inventory and read-only Hub | **Implemented locally; release evidence gated** | Review inventory/favorites/tags, but use the shell to connect |
| Connection Hub review/record/planner models | **Implemented locally; authority disabled** | The product modal may review state; Connect/Login remain unavailable |
| First-party managed OpenSSH session launch | **Planned** | Continue using system OpenSSH in the shell |
| Multi-cloud provider adapters/managed auth | **Planned** | Continue using official/provider CLIs and their normal auth flows |

## The recommended remote approach today: system OpenSSH

Use OpenSSH from the real shell:

```text
ssh host-alias
```

If you do not have an alias in your OpenSSH config:

```text
ssh user@example.com
```

Custom port:

```text
ssh -p 2222 user@example.com
```

Jump/bastion host:

```text
ssh -J bastion target
```

Automexia does not replace OpenSSH's config precedence, key selection, agent, certificates, hardware-token behavior, host-key policy, or protocol implementation. That is a feature, not a limitation: mature security behavior remains in the system tool that owns it.

Use `scp`/`sftp` the same way when those system tools fit the job. Automexia is the terminal host, not a secret vault or alternate SSH stack.

## Use an OpenSSH config alias for repeatable hosts

For hosts you access frequently, let OpenSSH own the alias in its normal config and connect with:

```text
ssh my-host
```

This is better than creating a global terminal alias containing sensitive connection detail because OpenSSH already understands host-specific keys, proxy jumps, ports, certificates, and first-value-wins configuration semantics.

Automexia's internal static inventory deliberately does **not** fully evaluate OpenSSH config. It avoids `Match exec`, `ProxyCommand` execution, dynamic shell expansion, `ssh -G`, DNS, sockets, and other behaviors that would turn background discovery into hidden process/network authority.

## Use splits for local + remote visibility

A useful current workflow is:

1. Keep a local shell in one pane.
2. Create a fresh or cloned split.
3. Run `ssh <host>` in the second pane.
4. Use `Alt+Arrow` / `Cmd+Alt+Arrow` to move between local and remote panes.
5. Create a window-level tab if the remote work is a separate operational context rather than part of the same task.

Choose a **fresh split** if you want a clean default shell before connecting. Choose a **clone split** if you want the same local launch profile/directory before starting the remote command.

A cloned Automexia pane does not duplicate an already-running SSH connection's live process state. For raw SSH started as a child command, explicitly run the SSH command in the new session.

## Port forwarding remains an OpenSSH responsibility

When you intentionally need a local forward, use OpenSSH's own syntax, for example:

```text
ssh -L 8080:localhost:80 host-alias
```

Review the binding/target/host carefully and use your organization's normal policies. Automexia does not silently create or replay tunnels in v0.4.

## WSL: treat it as its own Linux environment

On Windows, a WSL session is a Windows-launched Linux environment. It has its own shell, filesystem, tools, aliases, profiles, trust state, and package manager.

Practical rules:

- Keep Linux build-heavy repositories on the WSL-native filesystem (for example `~/src/project`) rather than `/mnt/c/...` when performance matters.
- Keep native Windows/MSVC work on NTFS.
- Do not assume a persistent alias/profile installed on Windows automatically exists in WSL, or vice versa.
- Automexia session context can identify the actual WSL distribution/user/directory when the integration is active.
- Normal Automexia launch does not start WSL merely to provision profile files.
- With the Automexia Windows profile, `Ctrl+V` and `Ctrl+Shift+V` paste into the selected WSL pane through
  Automexia's normal filtered/bracketed clipboard path. If a WSL application
  needs the original `Ctrl+V` control input, bind that chord to
  `ReceiveChar` as documented in [Shortcuts](shortcuts.md#clipboard-and-selection).

For Automexia's own source-development workflow, [Troubleshooting](../guide/troubleshooting.md#windows-and-wsl-development-workflow) contains the supported dual-native layout and safeguards.

## Raw SSH vs WSL vs future managed connections

| Situation | Best approach |
|---|---|
| Need a Linux development environment on the same Windows machine | WSL session |
| Need an interactive remote Unix/Linux host now | System `ssh` in the shell |
| Need a known OpenSSH alias/proxy/key configuration | OpenSSH config + `ssh alias` |
| Need cloud-provider authentication now | Official provider CLI in the shell |
| Need structured discovery/review/launch receipts in Automexia | **Planned Connection Hub**, not current v0.4 |
| Need Automexia to hold passwords/private keys | Not a current design goal; use established credential/agent tooling |

## What the planned Connection Hub is supposed to change

The future managed workflow is designed around:

1. bounded discovery;
2. selecting a connection record;
3. explaining public effective values/source state;
4. reviewing executable identity and exact arguments;
5. approving only the capability needed for that launch;
6. launching through one application-owned broker while system OpenSSH remains protocol/config authority;
7. keeping session/provider context isolated by owning session.

Those models exist internally, but the final process/network authority is intentionally disabled until the release/security gates are satisfied. Therefore this User Guide does not provide fake “click Connect” instructions for a surface that is not shipped.

## Remote workflow safety

- Treat remote terminal output, hostnames, provider metadata, and copied commands as untrusted input.
- Review host-key prompts; do not look for an Automexia “accept all” bypass.
- Keep credentials in established agents/keychains/provider tools rather than command text, Quick Actions, logs, or generated aliases.
- Do not make a destructive remote command automatically retry just because a connection dropped.
- Use explicit shell/OpenSSH commands today; wait for the managed broker before assuming Automexia owns process/network authorization.

For the architectural/status detail, see [Remote connections](../guide/remote-connections.md).
