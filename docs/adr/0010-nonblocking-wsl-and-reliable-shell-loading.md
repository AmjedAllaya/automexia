# ADR 0010 — Non-blocking WSL discovery and reliable shell loading

Status: accepted in v0.3.8

## Context

Real Windows dogfooding exposed two failures in v0.3.7. PowerShell integration
was installed into guessed Documents paths, which fails when Documents is
redirected (for example OneDrive). WSL context discovery enumerated
`\\wsl.localhost` / `\\wsl$` provider roots to identify a distro; those filesystem
provider calls can block while WSL cold-starts.

## Decision

1. The interactive Automexia PowerShell launcher directly sources the installed
   LocalAppData integration after normal profile loading. The installer uses the
   engine-reported `$PROFILE.CurrentUserCurrentHost` only for nested-shell hooks.
2. WSL Bash/Zsh files are installed with one non-login `wsl.exe --exec sh`
   invocation.
3. Runtime extension discovery never enumerates WSL UNC provider roots.
4. Bash/Zsh publish distro/version once through OSC 1337 `SetUserVar`; Automexia
   snapshots only these selected values into generic `SessionFacts`.
5. WSL CWD discovery uses OSC title/OSC 7 metadata plus cheap `/mnt/<drive>`
   mapping; host-side Docker/Kubernetes/cloud state may be used as a fallback.

## Consequences

- entering WSL no longer competes with Automexia UNC-provider scans;
- redirected PowerShell profiles no longer prevent the default Automexia prompt;
- shell metadata remains optional and bounded;
- the terminal core still has no Docker/Kubernetes/cloud concepts;
- exact Linux-local config inspection that would require potentially blocking WSL
  filesystem access is deferred until a future explicit, timeout-capable broker.
