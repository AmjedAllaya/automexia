# SSH integration library

The private library supplies pure contracts to Automexia's explicit `+ssh`
wrapper. System OpenSSH owns authentication, configuration, host trust, agents,
proxies and networking. The library itself never executes a process.

## Usage

Run `automexia +ssh --shell bash -- host-alias` for an integrated interactive
session. On Windows, use the installed console launcher: `amx +ssh --shell bash
-- host-alias`. It waits for the application and preserves console input and
output. Explicit shell choices are `bash`, `zsh`, `fish`, `powershell` (Windows
PowerShell) and `pwsh` (PowerShell 7). Choose the actual remote account shell.
Unknown shells, non-interactive operations and incompatible effective
configuration preserve native SSH in automatic mode. If effective configuration
cannot be read or classified, automatic mode stops before connecting;
`--integration off` is the explicit native escape hatch. See the
[CLI reference](CLI-REFERENCE.md#ssh-integration-planning).

`automexia ssh-integration status|inspect` remains read-only. Inspection consumes
declared assumptions; it does not evaluate configuration, contact a server or
authorize a managed launch. The explicit wrapper evaluates bounded `ssh -G`
output on demand; user-configured OpenSSH `Match exec` behavior remains native.

## Contracts

Adapters preserve native prompt builders and profiles, add prompt/input and
command-status boundaries, and reserve a separate context row. Capabilities are
negotiated: unavailable encoders, conflicting shell state or missing input hooks
may reduce them. Bash requires version 5.1 or newer. Fish naturally reads its
configuration in the non-interactive account shell and the new interactive
child; an interactive-only profile hook runs in the interactive child.

Enhanced PTYs use the portable `xterm-256color` terminal type. No custom terminfo
database or remote package is installed. Local settings still control rendering,
tables, output colors and status presentation. Each shell's native input editor
retains ownership of editing and native syntax colors.

Temporary startup files do not alter remote profiles or the user's umask.
Remote CWD and user labels are bounded display facts; neither becomes local
filesystem or provider authority. Local Git/Kubernetes/cloud discovery is
suspended during a remote scope. Without the optional helper, remote context
reflects explicit remote environment hints. Remote pane cloning requires a new
explicit connection.

## Temporary discovery helper

Add `--helper-upload /path/to/automexia-ssh-helper` before `--` to upload a
compatible helper for this session. Select a trusted Automexia helper built for
the **remote** operating system and architecture; the local executable is not
necessarily compatible. There is no automatic download or permanent remote
installation. Bash and Zsh use the POSIX helper; PowerShell 5 and 7 use the
Windows helper. PowerShell helper uploads to Unix and Fish helper uploads are
rejected. Ordinary Fish integration remains supported: its builtin IO cannot
provide the nonblocking request channel required by this helper on Fish 3.7.
POSIX staging requires Bash 3.2 or newer even when the interactive shell is Zsh;
it does not change the selected session shell or install any prerequisites.

The helper reuses Automexia's passive Git, Kubernetes, Docker, Terraform and
cloud configuration detector. It reads public context, never runs provider
commands or Kubernetes authentication plugins. Directory and selector changes
invalidate older results; idle configuration changes refresh every three
seconds. On a **remote macOS** helper, completed discovery results appear at
the next shell prompt; background discovery never writes into command output.
Linux and Windows helpers retain idle presentation refresh. Discovery has a
1.5-second deadline and does not run in prompt or terminal rendering code.
A missing, malformed or timed-out result clears the
affected presentation rather than displaying local machine context.

Upload uses two OpenSSH invocations and may authenticate twice. The first has a
120-second deadline, disables PTY, forwarding and local commands, and transfers a
bounded immutable copy (at most 64 MiB). The private temporary copy must match
its size, SHA-256 and helper protocol before it can be selected for the session.
The remote protocol probe has its own five-second deadline, so cancelling the
local connection cannot leave a stalled probe running indefinitely.
These integrity checks do not establish the publisher's identity: the caller
must choose a trusted executable. Ordinary SSH authentication and host trust
remain unchanged.

Normal session retirement removes only its exact owned files and directory;
profiles are never edited. Interrupted staging, a failed second connection or a
forcefully terminated remote process can leave temporary files. The wrapper
reports unconfirmed cleanup without starting another connection. Removing the
option restores the environment-hint integration without discovery.

## Architecture

The application reuses its system-tool resolver and process lifetime owner. WSL
tool routing retains the selected guest's SSH configuration and agent. Native
passthrough preserves exact arguments and exit/signal behavior; enhanced mode
uses inherited terminal streams rather than creating another PTY.

`--force-tty` explicitly requests a remote PTY when the caller's streams are
redirected. It does not make redirected local streams into a terminal. In
particular, calling the Windows executable from a WSL terminal can lose native
input and resize behavior across that process bridge. Use the Linux executable
inside WSL for a native interactive session. Default redirected invocations
remain native SSH passthrough.

The VT owns the versioned `AMXSCOPE1` lifecycle. A scope begins with a digest of
a locally held random preimage plus advisory generation/shell identity. Only
the matching local end can restore outer metadata; nesting is bounded to eight.
Automatic native fallback for a classified interactive session also isolates
metadata, with an unknown shell and no enhanced readiness or Quick Actions.
Native control, tunnel and remote-command operations, and explicit integration
off, retain passthrough behavior without injecting scope frames. Interactive
fallback owns only the SSH leader so persistent OpenSSH masters survive normal
exit; enhanced mode owns a Unix process group or a Windows job. Unix group
cleanup does not contain a process that deliberately detaches into another
session or process group.
Reset and full metadata buffers cannot remove isolation. If the wrapper is
forcibly killed before cleanup, the pane stays isolated; open a new terminal to
recover. Readiness and paths use `AMXSSH1` and `AMXSSHCWD1` and remain untrusted.
No persisted schema is migrated.

The helper is an application binary, not an effectful expansion of the pure SSH
library. It uses the existing CLI process owner for the shell and one isolated
scanner at a time. The scanner receives only a complete allowlisted public
snapshot, never the inherited environment. `AMXREQ1`, `AMXSSHREV1` and
`AMXSSHCTX2` bind requests and results to their scope and revision. An empty
revision revokes helper facts; stale results cannot restore them. Unix uses a
bounded inherited nonblocking socket. The shared zero-dependency
`automexia-terminal-protocol` codec lets the VT retain validated revision
ordering through nested scopes and coalesced renders without depending on the
SSH execution or connectivity domains. Windows uses a private local named pipe
and checks the exact child PID. The publisher owns one bounded writer and
completes accepted metadata frames; a partial OSC must not consume subsequent
command output. [Darwin's terminal writer](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/kern/tty.c)
can interleave writes under backpressure, so
the macOS helper sends frames through a private nonblocking stream instead.
A one-byte reverse datagram acknowledges each fully queued frame before the
foreground prompt can read it. The prompt acknowledges consumption on the
same private stream: only one unconsumed frame and one replaceable pending
result are retained, so idle refreshes cannot fill a queue with stale context.
Each prompt consumes at most four complete frames of at most 6 KiB and emits
only the latest valid metadata frame. No
prompt-time subprocess, filesystem access, discovery or blocking read is added.
The existing revision validator rejects stale results. Shutdown retires the
shell and scanner before cancelling the
writer. Unconfirmed child retirement retains its exact process owner, prevents
replacement, and ends the helper with failure. Startup files remain if shell
retirement cannot be confirmed.

See [ADR 0086](adr/0086-nonexecuting-ssh-planning-boundary.md). The managed launch
gate stays disabled. The wrapper does not alter reviewed bindings, add a
transport or introduce another process supervisor.

## Verification

Run `python tools/ci/check_ssh_library.py` for focused model, CLI and policy
checks. On Unix, `--shell-only --require-posix-shells` exercises actual
Rust-generated Bash, Zsh and Fish startup through controlling PTYs.
Resource-only tests without an exporter
are not proof of generated bootstrap behavior. Additional shell and isolated
SSH runtime suites live beside that runner under `tools/ci`.

On Windows, `--shell-only --require-windows-powershells` requires both Windows
PowerShell and PowerShell 7 and tests their actual generated adapters. These are
interpreter and hook checks; they do not replace an SSH-server or terminal-input
test.

`python tools/ci/test_ssh_helper_native.py` builds the helper and runs its
PowerShell 5/7 sessions through the existing bundled-ConPTY fixture on Windows.
It requires both shells and checks discovery, prompt interruption, resize,
exit status and cleanup, including inherited Ctrl+C settings. CI and the full
Windows SSH runner require this fixture. It proves local helper/console behavior,
not an end-to-end Windows OpenSSH-server connection.

The Windows `inline_pipeline_native_ssh_conpty_preserves_remote_terminal_contract`
fixture exercises the installed console launcher with bundled ConPTY and actual
OpenSSH against the same disposable Linux server. Set
`AUTOMEXIA_SSH_NATIVE_CONFIG` to the fixture's generated, strictly pinned client
configuration and `AUTOMEXIA_SSH_NATIVE_BINARY` to the sibling `amx.exe`; run
the ignored test only during that fixture's bounded lease. It checks metadata
refresh, command failure, Ctrl+C, remote terminal dimensions, native exit status
and restoration of local metadata. It does not establish Windows-server support.

For opt-in end-to-end Linux SSH checks, build the disposable image from
`tests/fixtures/ssh/Dockerfile`, record its immutable ID, and run the generated
fixtures against the current application:

```sh
docker build --iidfile target/ssh-runtime-image.id -f tests/fixtures/ssh/Dockerfile tests/fixtures/ssh
cargo build --locked -p automexia-terminal --bin automexia
python tools/ci/test_ssh_wrapper_runtime.py --image "$(cat target/ssh-runtime-image.id)" \
  --fixture bash=target/ssh-library-checks/actual-rust-generator.fixture \
  --fixture zsh=target/ssh-library-checks/zsh.fixture \
  --fixture fish=target/ssh-library-checks/fish.fixture \
  --application target/debug/automexia
cargo build --locked -p automexia-terminal --bin automexia-ssh-helper
python tools/ci/test_ssh_helper_runtime.py --image "$(cat target/ssh-runtime-image.id)" \
  --application target/debug/automexia --helper target/debug/automexia-ssh-helper
```

Run the POSIX shell checks above first to generate those fixtures. The runtime
test uses temporary keys, strict generated host-key pinning and a loopback-only
published port, and removes its container, network and keys. The image remains
available for another run. Its build downloads signed distribution packages;
no SSH service or account is installed on the host.

Model, parser and shell tests do not prove native Windows SSH servers, native
macOS, authentication methods or compositor behavior. Each needs its own runtime
evidence. `cargo ready` is the repository contributor gate. Windows and WSL
build directories must remain separate.
