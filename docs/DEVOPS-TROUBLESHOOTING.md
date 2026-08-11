# DevOps HUD troubleshooting

This document covers the native `automexia.devops` extension. The extension does not install shell commands, run Docker/Kubernetes/cloud CLIs, or make network requests.

## `/market` says Remove, but no HUD is visible

`Remove` means the extension is activated. A HUD segment appears only after local context is discovered. v0.3.2 fixes two cases that could leave the HUD invisible even with an active extension:

1. discovery completion now schedules bounded redraw polling until the current terminal session's snapshot is published;
2. nested WSL sessions are detected from raw terminal metadata rather than the Windows parent-process environment.

At minimum, a conventional WSL title such as `user@host:/path` produces a WSL segment once discovery completes. If Docker Desktop has an existing `.docker` client configuration directory, the host fallback produces `Docker default` even when no explicit `currentContext` is present.

## What is discovered

- Windows host: `%USERPROFILE%\.docker`, `%USERPROFILE%\.kube`, `%USERPROFILE%\.aws`, `%USERPROFILE%\.azure`, `%APPDATA%\gcloud`, environment overrides and project-local state.
- WSL: distro/version comes from lightweight shell metadata; `/mnt/<drive>` is mapped directly to the Windows filesystem. Host Docker/Kubernetes/cloud context remains a fallback. Runtime discovery does not enumerate `\\wsl.localhost` or `\\wsl$`.
- Project: `.automexia-context.json`, Git HEAD and Terraform local workspace state.

The HUD reports configured/local context. It does not claim that Docker daemon, Kubernetes API or cloud credentials are currently reachable.

## Expected WSL behavior

After entering WSL, a shell that publishes a conventional terminal title should result in a segment such as:

```text
[Ubuntu vector icon] 24.04 | [Docker vector icon] default | [Git vector icon] feature/x
```

Only contexts that actually exist are included.

## Debug logging

If a session still shows no HUD, run the development executable with Rio/Automexia's current debug logging enabled and inspect the log for:

```text
Automexia DevOps discovery completed
```

The event records the terminal session id, raw terminal title and the discovered snapshot. This is application logging, not a custom shell command supplied by the extension.

On the current Rio-derived frontend, a Windows diagnostic run can be started from PowerShell with:

```powershell
$env:RIO_LOG_LEVEL = "debug"
& ".\target\debug\rio.exe" --enable-log-file
```

Remove the temporary environment setting afterwards if desired:

```powershell
Remove-Item Env:RIO_LOG_LEVEL
```


## Shell integration does not appear

On Windows run:

```powershell
.\INSTALL-SHELL-INTEGRATION-WINDOWS.ps1
.\CHECK-SHELL-INTEGRATION-WINDOWS.ps1
```

The checker verifies the LocalAppData PowerShell script and WSL Bash hook. Close all Automexia windows after reinstalling so the new environment and WSLENV values are inherited.

## Prompt marker appears as `??`

This is an encoding problem, not a WSL command or DevOps discovery problem. v0.3.8 transferred Bash/Zsh integration source through a Windows PowerShell native-command text pipeline. Windows PowerShell 5.1 may encode that pipeline with a legacy OEM code page; U+03BB (`lambda`) is not representable there and arrives in WSL as two literal question marks.

v0.3.12 fixes this at both boundaries:

- Bash/Zsh source contains only ASCII-safe UTF-8 byte escapes (`\\xCE\\xBB`) for the prompt marker;
- PowerShell constructs the marker from `[char]0x03BB`;
- the Windows installer base64-encodes UTF-8 payload bytes and WSL decodes them before `sh` writes the integration files.

After upgrading, rerun `INSTALL-SHELL-INTEGRATION-WINDOWS.ps1`, close every Automexia window, and start a new one. `CHECK-SHELL-INTEGRATION-WINDOWS.ps1` now verifies that the installed WSL hook contains the UTF-8-safe prompt marker.
