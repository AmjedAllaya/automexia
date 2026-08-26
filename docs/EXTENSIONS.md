# Extensions

Extensions are how Automexia can adapt to different kinds of work without
turning the core terminal into a large collection of features that every user
must carry.

The terminal should remain useful on its own. A person can then add a focused
extension for infrastructure, automation, data, research, media, or another
field only when it improves their workflow.

## Current status

Automexia v0.4 does **not** provide a public extension marketplace, a public
software development kit (SDK), or a user-facing install command. The
repository contains extension contracts,
first-party DevOps provider foundations, and a disabled package/sandbox
foundation, but release activation and public downloads remain blocked.

| Part | Status today | What that means for a user |
|---|---|---|
| Core terminal workspace | **Available now** | Windows, tabs, panes, shells, search, configuration, image support, and normal command-line tools work without an extension. |
| First-party extension contracts and runtime boundaries | **Internal / contributor-facing** | The source has reviewed interfaces and limits, but this is not a public third-party SDK promise. |
| DevOps provider extension foundations | **Implemented internally, not activated** | Source contracts and cached review models exist; live provider authentication, network work, and managed execution are not public v0.4 features. |
| Local signed extension-bundle verification and disabled storage | **Implemented locally / release-gated** | The source can verify and store a reviewed bundle in a disabled state, but no public install or activation workflow is released. |
| Public extension browsing, download, enable, update, and removal | **Planned** | Do not follow unofficial instructions that claim a current public marketplace. |
| Automation Studio, video editing, and large language model (LLM) orchestration | **Planned proposals** | Their documents define direction and safety boundaries; no public runtime is shipped. |

The [feature catalog](FEATURES.md) is the authority when a status changes.

## Why Automexia uses extensions

Different people need very different tools. A Kubernetes operator, a data
analyst, and a video editor may all use a terminal, but they should not have to
load the same provider clients, language tools, media libraries, or background
services.

Focused extensions should provide four benefits:

1. **A smaller core.** Normal typing, terminal output, startup, and shell use do
   not depend on optional domain packages.
2. **Clear ownership.** The tool or provider that already owns authentication,
   commands, and data remains the authority.
3. **Simple choices.** Users add only the fields and workflows they need.
4. **Safe removal.** Disabling or removing one extension should not damage the
   terminal, another extension, or the user's external tools.

## First-party foundations in the repository

The source tree currently separates these DevOps provider foundations:

| Extension source | Intended responsibility | Current user boundary |
|---|---|---|
| OpenSSH | Review SSH inventory and prepare explicit connection work | Use system `ssh` for real connections today; managed launch remains gated. |
| AWS | Parse bounded public context and prepare exact AWS Command Line Interface (CLI) operations | No live AWS login, credential-cache read, network call, or managed execution is activated. |
| Azure | Parse bounded public context and prepare exact Azure CLI operations | No live Azure login, browser flow, token-cache read, or managed execution is activated. |
| Google Cloud | Parse bounded named context and prepare exact `gcloud` operations | No live Google login, credential database access, or managed execution is activated. |
| Kubernetes | Validate selected configuration copies and prepare exact client operations | No cluster connection, credential plugin execution, global context change, or managed command is activated. |
| OpenShift | Validate selected public context and prepare exact `oc` operations | No cluster login, browser flow, or managed command is activated. |
| Teleport | Parse bounded public status and prepare exact Teleport operations | No agent, certificate cache, multi-factor authentication (MFA), browser, network, or managed execution authority is activated. |

These packages are separated so each provider can evolve, be reviewed, and be
removed without becoming part of the terminal engine. The
[Connection Hub and SSH guide](user-guide/connection-hub-and-ssh.md) explains
which read-only and review surfaces exist in source builds and which operations
remain unavailable.

## What you can do today

You can use the real tools you already trust inside Automexia:

```text
ssh my-host
kubectl get pods
aws sts get-caller-identity
az account show
gcloud config list
```

Those commands remain owned by your shell and the installed tools. Automexia
does not change their syntax or silently take over their credentials. Use panes
and tabs to keep related sessions visible, and use the context shown above the
command line to reduce mistakes.

Source builds also contain review-oriented Quick Action, connection, and
provider models at different assurance levels. Read their status labels before
using them. **Implemented locally** does not mean **available in every stable
package**, and **implemented internally** does not mean the final operation can
run.

## What is deliberately unavailable today

Automexia v0.4 does not offer a supported way to:

- download an extension from a public marketplace;
- install or enable an arbitrary third-party component;
- grant an extension general access to files, networks, processes, credentials,
  terminal history, clipboard content, or the active shell;
- let a provider extension change a global cloud or Kubernetes context;
- let an extension press Enter or execute a suggested shell command silently;
- install an LLM provider inside the terminal core or a domain extension.

These are honest product boundaries, not missing steps from this guide.

## The planned installation experience

When public extension support is ready, the intended user path is:

1. Open **Extensions** from the command palette or application navigation.
2. Search by task or field, such as Kubernetes, automation, or media.
3. Review the publisher, version, supported Automexia versions, platforms,
   download size, signature, requested capabilities, and data behavior.
4. Install the package in a disabled state.
5. Enable only the capabilities needed for the selected workflow.
6. See the new commands, views, or context in the places where they are useful.
7. Update, disable, revoke, or remove the extension without changing unrelated
   terminal, shell, SSH, cloud, or project state.

That experience is planned. No current screenshot, placeholder action, or
internal state model should be interpreted as a released marketplace.

## What an extension should feel like

A good Automexia extension should be easy to understand before it is enabled:

- its purpose fits in one short sentence;
- its controls appear only when they are relevant;
- empty, loading, stale, unavailable, and error states use plain language;
- it shows the exact account, cluster, project, file, or environment involved;
- sensitive changes remain reviewable and cancellable;
- background work is bounded and stops when the related session or extension
  is closed;
- disabling it restores the ordinary terminal workflow immediately;
- removing it does not remove user-owned files or external provider state.

An extension may make work simpler, but it must not hide who owns an action or
what will happen.

## Production and DevOps extensions

The long-term DevOps and site reliability engineering (SRE) experience is meant
to bring useful context close to the command without replacing native tools. It
may help users:

- keep account, region, cluster, namespace, identity, and production status
  visible;
- compare unhealthy resources with healthy peers;
- understand recent changes and configuration ownership;
- rank useful native commands using fresh evidence;
- review impact, policy, GitOps workflow, and recovery before a sensitive operation;
- organize an incident without turning suggestions into automatic execution.

That situation-aware Production Operations layer is planned, not a v0.4
feature. Its exact scope and Kubernetes rollout example are in the
[Production Operations specification](SITUATION-AWARE-PRODUCTION-OPERATIONS.md).

## Automation Studio and creative extensions

The proposed Automation Studio is an optional editing workspace for scripts,
configuration, and automation files. It is planned as a separate extension or
add-on inside the Automexia window, not as a requirement for the terminal or
the DevOps/SRE extension.

Future media and video extensions should reuse general workspace, task,
progress, cancellation, file, and recovery services. They should not depend on
DevOps code, editor internals, or an LLM. The current project does not claim a
built-in editor or video-editing runtime. See the
[Automation Studio proposal](AUTOMATION-STUDIO-ARCHITECTURE.md) and
[product vision](PRODUCT-VISION.md#long-term-direction).

## AI and model extensions

Automexia is designed to remain complete and useful without an AI model, paid
application programming interface (API), provider account, or network connection.

A future LLM Orchestration extension may turn a goal into a reviewable workflow
across enabled domain actions. It will be separately installed and will not be
embedded in the core terminal, DevOps/SRE extension, Automation Studio, or a
video extension. A model may propose a plan; it does not receive direct shell,
provider, credential, or execution authority.

Small task-specific models must follow the same rule: optional, narrow,
resource-bounded, easy to disable, and backed by a useful non-model fallback.
See the [LLM Orchestration proposal](LLM-ORCHESTRATION-EXTENSION.md).

## Keeping Automexia lightweight

Optional capabilities must stay away from the terminal's typing, output,
resize, rendering, and startup hot paths. Extensions should use bounded workers,
queues, caches, history, storage, timeouts, and concurrency. Obsolete work must
be cancelled, and disabled extensions must not keep hidden processes or network
connections alive.

This approach matters on ordinary laptops and smaller computers, but it does
not create a support claim for every device. Automexia still needs a desktop
operating system, a windowing environment, terminal-session support, and
suitable graphics.
See [Platform support](PLATFORMS.md) and the hardware answer in the
[FAQ](FAQ.md#can-automexia-run-on-a-raspberry-pi-smartphone-nintendo-switch-or-arduino).

## For extension contributors

The public SDK is not released. Contributors working on first-party source
must follow:

- the [architecture contract](ARCHITECTURE.md#core-and-extension-ownership);
- [ADR 0003](adr/0003-extension-capability-and-threading.md);
- the [build, wrap, or adopt boundary](BUILD-WRAP-ADOPT-ARCHITECTURE.md);
- the [ecosystem safety boundary](ECOSYSTEM-PLATFORM.md);
- the [testing guide](TESTING.md).

Do not present an internal Rust crate, model, placeholder marketplace view, or
disabled package store as a supported third-party extension API.

## Find the right next page

| Need | Read |
|---|---|
| See what users can run today | [Feature catalog](FEATURES.md) |
| Use normal commands and shell tools | [Commands and shell workflows](user-guide/commands-and-shell.md) |
| Review SSH and provider status | [Connection Hub and SSH](user-guide/connection-hub-and-ssh.md) |
| Understand package verification and disabled activation | [Ecosystem platform](ECOSYSTEM-PLATFORM.md) |
| Understand long-term domain flexibility | [Product vision](PRODUCT-VISION.md) |
| Check delivery order | [Roadmap](ROADMAP.md) |
