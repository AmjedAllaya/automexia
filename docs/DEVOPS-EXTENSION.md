# Automexia DevOps extension

Status: **first-party bundled extension, v0.3.13**

The DevOps extension is a native Automexia application feature. It provides useful operational context and display-only semantic output decoration without changing shell commands, PTY bytes, prompt configuration, or terminal parser state.

## User experience

On a fresh Automexia install the extension is enabled by default. It can be disabled or re-enabled from the application command palette through `/market`.

There are **no custom CLI commands** in this extension. In particular, the extension does not install or intercept `ax`, `kgp`, `kubectl`, `docker`, `aws`, `az`, `gcloud`, Terraform, PowerShell, Bash or Zsh commands. `/market` is Automexia application UI and is never sent to the PTY.

The extension contributes two independent features:

```text
terminal visible row ─────────► pure semantic classifier ─► display color hint
                                                        (never PTY mutation)

working directory/context ────► bounded extension worker ─► cached DevOpsSnapshot
                                                         └► native HUD renderer
```

### Native context HUD

The native HUD shows only contexts that can be determined from local configuration/environment state. Empty context categories remain hidden so the terminal does not display a permanent "no context" card.

Current categories:

- Kubernetes current context and namespace;
- active WSL session/distro when passively observable;
- Docker context;
- AWS profile/region;
- Azure subscription/profile;
- GCP project/configuration and region;
- Terraform workspace;
- Git branch;
- logical environment (`AUTOMEXIA_ENV`, `ENVIRONMENT`, `APP_ENV`, `NODE_ENV`);
- a red `PROD` indicator when clear production tokens are found.

The context graphics are rendered natively by Automexia rather than printed as prompt text. The optional shell integration supplies only standard OSC 7/133 prompt/session boundaries plus editor colors; it does not print Docker/Kubernetes/cloud labels or add command wrappers.

### Semantic prompt-row layout and icon policy

v0.3.12 keeps the permanent header removed but changes how the **live** prompt is positioned. Historical context can still follow OSC-133 prompt rows in scrollback, while the active context row is resolved from the current OSC-133 Prompt/PromptContinuation pair every frame, with cursor geometry used only as a first-paint fallback. Shell integration publishes a lightweight prompt-active flag, so the native row stays immediately above editable input through resize/fullscreen and never follows the cursor into command output.

The current working directory belongs to the shell prompt line, not the DevOps extension. PowerShell renders its current Windows location there; Bash/WSL uses `\w`; Zsh/macOS uses `%~`. The native context row is therefore reserved for environment/session information only.

The resulting rhythm matches the original Liquid Hacker direction while remaining terminal-native:

```text
[Ubuntu icon] 24.04  │  [Docker icon] default  │  [Git icon] feature/x  │  [User icon] amjed
/mnt/d/work/project λ version
version: command not found

[Ubuntu icon] 24.04  │  [Docker icon] default  │  [Git icon] feature/x  │  [User icon] amjed
/mnt/d/work/project λ _
```

Icons are drawn with Sugarloaf primitives, not Nerd Font/private-use codepoints. Values use the shared Automexia semantic palette. Prompt-context snapshots are bounded and keyed by session + the current layout's absolute row. Historical rows keep captured state while scrolling; the live prompt is allowed to adopt a new row key after reflow so resize/fullscreen does not create a false command boundary.

Automexia only overlays a semantic `Prompt` row when that row is blank. Third-party shells that use OSC 133 on a row containing their own visible prompt text are left untouched.


### Nested WSL sessions on Windows

A Windows terminal process cannot read environment-variable changes made inside a child WSL shell. v0.3.2 therefore does **not** pretend the parent process environment is the active Linux environment. Instead, Automexia captures generic session facts from the terminal snapshot, including the raw OSC/window title. When the child publishes a conventional WSL title such as:

```text
amjed@DESKTOP-2LR87FN:/mnt/d/workstation/project
```

the DevOps extension recognizes WSL passively and maps `/mnt/<drive>` paths directly to Windows paths. The shell publishes distro/version metadata once; the extension deliberately does **not** enumerate `\\wsl.localhost` or `\\wsl$` during refresh because those provider probes can stall a cold WSL startup. It never runs `wsl.exe`, `docker`, `kubectl`, or cloud CLIs from the renderer/extension worker.

Automexia does not enumerate or read WSL UNC provider files during runtime discovery. The prompt can still identify WSL from shell metadata, map `/mnt/<drive>` projects to the host filesystem, and fall back to configured host-side Docker/Kubernetes/cloud context.

### Configured context vs. connectivity

The prompt context represents **configured/local context**, not proof of live remote connectivity. The extension does not call Kubernetes, Docker or cloud APIs in order to paint the terminal. A displayed AWS profile, for example, means that profile/region was selected or locally configured; it does not promise that credentials are currently valid.

That distinction is intentional: renderer latency and terminal availability must not depend on a daemon, VPN, cloud API, cluster or network connection.

## Local discovery sources

The worker reads bounded local state only.

| Context | Sources |
|---|---|
| Kubernetes | `KUBECONFIG`, otherwise `~/.kube/config` |
| Docker | `DOCKER_CONTEXT`, `DOCKER_CONFIG/config.json`, existing Docker config root (`default` fallback) |
| AWS | `AWS_PROFILE`, `AWS_DEFAULT_PROFILE`, `AWS_REGION`, `AWS_DEFAULT_REGION`, `AWS_CONFIG_FILE`, otherwise `~/.aws/config` |
| Azure | `AZURE_SUBSCRIPTION`, `AZURE_SUBSCRIPTION_ID`, `AZURE_CONFIG_DIR/azureProfile.json`, otherwise `~/.azure/azureProfile.json` |
| GCP | `GOOGLE_CLOUD_PROJECT`, `CLOUDSDK_CORE_PROJECT`, `CLOUDSDK_CONFIG`; on Windows `%APPDATA%/gcloud`; otherwise `~/.config/gcloud` |
| Terraform | `TF_WORKSPACE`, otherwise `.terraform/environment` under the terminal CWD |
| Git | nearest `.git/HEAD`, including worktree-style `gitdir:` pointers |
| Automexia project | nearest `.automexia-context.json` plus legacy local Automexia context state |

Every file read is bounded to 4 MiB and the number of kubeconfig files is bounded. Display labels have control characters removed and are length-limited.

The nested project-context schema from the older Automexia terminal is supported read-only, including:

```json
{
  "project": "payment-api",
  "environment": "staging",
  "git": { "branch": "main" },
  "kubernetes": { "context": "aks-staging", "namespace": "payments" },
  "docker": { "context": "desktop-linux" },
  "cloud": { "provider": "azure", "profile": "Development Subscription", "region": "westeurope" },
  "terraform": { "workspace": "staging" }
}
```

Reading this file never applies or switches a context.

## Semantic output highlighting

Semantic highlighting is display-only and whole-row. It is applied only to text already being rebuilt. Explicit truecolor and non-neutral application colors remain authoritative; default foreground and neutral ANSI white/light-white are eligible for semantic normalization so shell-native error messages can share the Automexia error color.

Precedence is:

```text
selection / search / hint
        >
explicit non-neutral ANSI / application truecolor
        >
Automexia semantic color (default/white foreground is eligible)
        >
default terminal foreground
```

This means tools such as `kubecolor`, TUIs and applications that already color their own output keep their colors.

### Error / failure — theme red

Examples include:

```text
version: command not found
Error from server: Forbidden
connection refused
Unauthorized
permission denied
CrashLoopBackOff
ImagePullBackOff
ErrImagePull
OOMKilled
Evicted
Docker Exited (1)
Docker daemon errors
Rust error[E....] diagnostics
npm ERR! / npm error
PowerShell CategoryInfo / FullyQualifiedErrorId
merge conflict
build failed
panic / exception / traceback
```

The `command not found` case is explicitly covered because it is the failure shown in the reported Automexia screenshot.

### Warning / pending — theme yellow

Examples:

```text
WARNING / WARN
Pending
ContainerCreating
PodInitializing
NotReady
Retrying
BackOff
Degraded
Deprecated / deprecation
Docker Restarting
```

A Kubernetes `Running` row with `1/2` READY is yellow rather than green.

### Success / healthy — theme green

Examples:

```text
Succeeded
Completed
healthy
apply complete!
test result: ok
Finished `dev` / `test` / `release` profile
Docker Exited (0)
Docker Up ... (healthy)
Kubernetes 2/2 Running
```

### Informational logs/progress — theme cyan

Examples:

```text
INFO / NOTICE / MESSAGE
level=info
Compiling ...
Checking ...
Building ...
Downloading ...
Kubernetes NAME READY STATUS ... header
Docker CONTAINER ID ... STATUS ... header
```

### Debug/trace — theme blue

Examples:

```text
DEBUG / TRACE
Serilog-style INF / WRN / DBG / TRC
level=debug
level=trace
{"level":"debug", ...}
```

The classifier is intentionally conservative: unrecognized normal terminal text keeps the active theme foreground.

## Threading and performance

The renderer never opens these config files itself.

```text
renderer session-facts/refresh request
       │ try_send (capacity 1)
       ▼
automexia-extension-worker
       │ bounded local reads + sanitization
       ▼
bounded per-session cache (32 entries)
       │ session revision + global wake generation
       ▼
renderer atomic generation check + cached snapshot clone on this-session completion
```

Multiple windows/panes therefore keep separate context snapshots and completion acknowledgements. A busy worker queue does not block a frame; the renderer retries later. Failure to start the optional worker disables DevOps prompt context rather than preventing a shell from opening.

## Capabilities

The built-in declares only:

```text
filesystem.read
environment.read
terminal.output.read
ui.overlay
```

It deliberately does **not** request `process.spawn` or `network`.

## Source ownership

```text
frontends/rioterm/src/automexia/builtins/devops/
├── mod.rs        manifest, capability declaration, public boundary, icons
├── model.rs      renderer-neutral snapshot data
├── context.rs    background local discovery and sanitization
└── semantics.rs  pure terminal-row classifier

frontends/rioterm/src/renderer/devops_status.rs
└── native renderer adapter consuming only cached models/runtime calls
```

Do not merge these responsibilities back into one file or put context discovery in the renderer. Future third-party DevOps-like extensions should target the Automexia extension contracts/sandbox rather than importing terminal-engine internals.
