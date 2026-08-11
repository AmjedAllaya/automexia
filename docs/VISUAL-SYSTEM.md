# Automexia unified visual system

Status: **accepted in v0.3.7; UTF-8 shell-transfer hardening in v0.3.9; live cursor-anchored prompt reliability in v0.3.12**

Automexia owns one cross-platform visual language. The terminal renderer, semantic decorators, DevOps context UI and optional shell integrations share the same role palette instead of letting each shell define an unrelated appearance.

## Layers

```text
applications / shells
        │ ANSI/named colors + explicit RGB
        ▼
Automexia unified palette
        │
        ├── renderer named colors
        ├── semantic Error/Warning/Success/Info/Debug roles
        ├── navigation / selection / search / cursor colors
        └── extension chrome

optional shell integration
        ├── PowerShell PSReadLine syntax roles
        ├── Bash prompt + OSC 7/133 metadata
        └── Zsh/macOS prompt + OSC 7/133 metadata
```

The core palette remaps the standard named/ANSI color vocabulary. Explicit application truecolor remains authoritative so TUIs and applications that deliberately own their colors are not destructively recolored.

## Default palette

| Role | Color |
|---|---|
| Background | `#04100D` |
| Foreground | `#EEF7F2` |
| Error / red | `#FF6F91` |
| Success / green | `#7CFFB2` |
| Warning / yellow | `#FFD166` |
| Primary blue | `#48A7FF` |
| Purple | `#B58CFF` |
| Info / cyan | `#61E7FF` |
| Muted | `#5D7A70` / `#90AEBE` |
| Cursor | `#7CFFB2` |

`AUTOMEXIA_UNIFIED_COLORS=0` disables the palette override for troubleshooting or advanced users who explicitly want application-specific named colors.

## Prompt/header language

The DevOps extension renders a native vector-icon context line on the blank OSC-133 semantic prompt row immediately above each editable command. It uses `icon + value` segments only: Ubuntu `24.04`, Docker `default`, Git `<branch>`, Kubernetes `<context>/<namespace>`, cloud `<profile>/<region>`, Terraform `<workspace>`, user `<name>`, environment `<name>`, and `PROD` when applicable. The working directory is intentionally rendered by the shell on the editable command line, not by DevOps. The context line scrolls with the command instead of living in fixed window chrome.

Native icons are drawn by Sugarloaf primitives and therefore do not depend on Nerd Font/private-use glyphs.

The optional shell integration creates the two-row semantic prompt surface and keeps command editing consistent with it. It does not install wrapper commands. It emits OSC 7/133 metadata and configures the shell's prompt/editor colors. PowerShell uses PSReadLine syntax roles; Bash and Zsh use the same lambda prompt and leave editable command text in Automexia magenta, resetting before child output. This makes the command-input rhythm consistent without wrapping or intercepting commands.

## Architecture rule

Shell-specific code is installation/profile glue only. `automexia-core`, VT, PTY and GPU backends must never import PowerShell/Bash/Zsh logic. The renderer knows generic colors and extension contributions, not shell implementation details.

## Live prompt reliability (v0.3.12)

The active DevOps row is resolved from current OSC-133 prompt semantics every frame, with cursor geometry used only before the semantic pair is available and is shown only while shell-published prompt lifecycle metadata reports editable input. Resize/fullscreen therefore cannot orphan the live row, and it cannot follow the cursor into running-command output. OS/version and user identity render synchronously with zero filesystem IO. Docker/Git/Kubernetes/cloud discovery remains asynchronous and enriches the same live row. The shell renders its current directory in the normal prompt line before `λ`. Historical context remains bounded.
