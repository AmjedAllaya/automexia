# Automexia Terminal documentation

Automexia helps people make complex command-driven work faster, simpler, and
easier to control. If this is your first visit, start with
[Install Automexia](INSTALLATION.md), continue with
[Getting started](GETTING-STARTED.md), and then use the
[Automexia User Guide](user-guide/index.md) for the task you want to complete.
The [product vision](PRODUCT-VISION.md) explains the wider direction.

The documentation is organized around **reader intent**, not implementation
phases. Reference pages own exact syntax, developer pages own architecture and
assurance, and project pages own future work and decision history.

> **Status rule:** current behavior says **Available now**; repository features that exist but still need release evidence say **Implemented locally / release-gated**; non-authoritative foundations say **Implemented internally, not activated**; future work says **Planned**. Roadmap status never overrides user-facing instructions.

## New user path

You do not need to read the documentation from beginning to end. Use this
short path to reach a useful first workspace:

1. [Install from source](INSTALLATION.md) using the instructions for your host.
2. Complete the [first-session tutorial](GETTING-STARTED.md).
3. Take the [15-minute tour](user-guide/index.md#a-15-minute-tour).
4. Choose a task page or [workflow recipe](user-guide/recipes.md).

## Start here

| I want to… | Read |
|---|---|
| Install Automexia or prepare a source checkout | **[Install Automexia](INSTALLATION.md)** |
| Open the first session and learn the basic controls | **[Getting started](GETTING-STARTED.md)** |
| Understand what Automexia is building and why | [Product vision](PRODUCT-VISION.md) |
| **Learn how to use Automexia** | **[Complete User Guide](user-guide/index.md)** |
| Understand current and planned extensions | [Extensions](EXTENSIONS.md) |
| Get a short answer to a common question | [Frequently asked questions](FAQ.md) |
| Launch a session, directory, or shell | [Start and launch sessions](user-guide/start-and-launch.md) |
| Decide between windows, tabs, local tabs, fresh splits, and cloned splits | [Workspaces, tabs, and panes](user-guide/workspace.md) |
| Learn command lines and which command surface to use | [Commands and shell workflows](user-guide/commands-and-shell.md) |
| Learn the practical keyboard/mouse controls | [Shortcuts and input](user-guide/shortcuts.md) |
| Use completion, Quick Actions, aliases, and workspace tasks | [Command productivity](user-guide/productivity.md) |
| Review the exact Quick Action and alias architecture | [DevOps alias specification](DEVOPS-ALIASES.md) |
| Customize shell, font, window, theme, navigation, and bindings | [Configuration and customization](user-guide/customization.md) |
| Configure SSH or find Connection Hub | **[Connection Hub and SSH](user-guide/connection-hub-and-ssh.md)** |
| Work with images, listings, semantic output, or WSL | [Files and images](user-guide/files-and-images.md), [Remote and WSL](user-guide/remote-and-wsl.md) |
| Follow practical project, operations, automation, and media examples | [Workflow recipes](user-guide/recipes.md) |
| Manually validate every implemented feature from a clean machine | [Complete implemented-feature manual testing guide](MANUAL-FEATURE-TESTING.md) |
| Build Automexia from source | [Contributor getting started](guide/getting-started.md) |
| Diagnose a problem | [Troubleshooting](guide/troubleshooting.md) |
| Look up exact CLI syntax | [CLI reference](reference/cli.md) |
| Look up every shortcut/action | [Keyboard reference](reference/keyboard.md) |
| Configure or inspect Ghostty-compatible bindings | [Ghostty compatibility](GHOSTTY-KEYBOARD-COMPATIBILITY.md), [generated bindings](generated/ghostty-1.3-keybindings.md), [generated actions](generated/ghostty-1.3-actions.md) |
| Look up every config key/default | [Configuration reference](reference/configuration.md) |
| Understand the technical design | [Architecture overview](developer/architecture.md), [detailed architecture contract](ARCHITECTURE.md) |
| Audit where every implemented feature belongs | [Implemented feature ownership audit](FEATURE-OWNERSHIP-AUDIT.md), [ADR 0035](adr/0035-core-domain-and-optional-extension-ownership.md) |
| Understand the planned in-terminal script editor | [Automation Studio summary](AUTOMATION-STUDIO-ARCHITECTURE.md), [assurance summary](AUTOMATION-STUDIO-TESTING.md), [proposed ADR 0030](adr/0030-automation-studio-domain-extension-boundary.md) |
| Understand the separate optional LLM Orchestration extension | [LLM Orchestration summary](LLM-ORCHESTRATION-EXTENSION.md), [assurance summary](LLM-ORCHESTRATION-TESTING.md), [proposed ADR 0033](adr/0033-optional-llm-orchestration-extension.md) |
| Understand planned error-section navigation | [Semantic Diagnostic Navigator summary](SEMANTIC-DIAGNOSTIC-NAVIGATOR.md), [proposed ADR 0032](adr/0032-bounded-semantic-diagnostic-navigation.md) |
| Understand planned production investigation and situation-aware guidance | [Production Operations summary](SITUATION-AWARE-PRODUCTION-OPERATIONS.md), [public contract principles](SITUATION-AWARE-PRODUCTION-OPERATIONS-CONTRACTS.md), [experience summary](SITUATION-AWARE-PRODUCTION-OPERATIONS-UX.md), [assurance summary](SITUATION-AWARE-PRODUCTION-OPERATIONS-TESTING.md), [proposed ADR 0034](adr/0034-situation-aware-production-operations.md) |
| Run verification or understand release gates | [Testing overview](developer/testing-release.md), [CI implementation audit](CI-ASSURANCE.md), [complete testing and evidence contract](TESTING.md), [per-feature reinforcement plan](FEATURE-TEST-REINFORCEMENT.md) |
| Run the GitHub-Free local security and release profiles | [Testing and assurance](TESTING.md#nightly-and-release-depth), [GitHub-Free private setup](../.github/FREE-PRIVATE-PRODUCTION-SETUP.md#91-local-assurance-before-a-push) |
| Understand Linux Early Access packaging and public downloads | [Public release distribution](PUBLIC-RELEASE-DISTRIBUTION.md), [release trust](RELEASE-TRUST.md), [ADR 0037](adr/0037-public-binary-release-distribution.md) |
| Audit completed-output grouping and its validation incident | [Command-result surface assurance](COMMAND-RESULT-ASSURANCE.md) |
| Review UI branding implementation and exact U10 release evidence | [UI branding roadmap](UI-BRANDING-ROADMAP.md), [U10 assurance audit](research/U10-UI-BRANDING-ASSURANCE-AUDIT.md) |
| Configure or audit controlled F5 native OpenSSH evidence | [F5 native OpenSSH assurance](F5-NATIVE-OPENSSH-ASSURANCE.md) |
| Audit stable-release blockers or restore hosted CI and repository protection | [Section 12 completion audit](research/STABLE-RELEASE-AND-REPOSITORY-BLOCKERS-AUDIT.md), [repository protection guide](../.github/BRANCH-PROTECTION.md), [ADR 0031](adr/0031-versioned-hosted-ci-and-repository-protection.md) |
| Review stabilization status and external evidence | [Stabilization direction](STABILIZATION-ROADMAP.md), [S1 native/visual/resource/accessibility audit](research/S1-NATIVE-VISUAL-RESOURCE-ACCESSIBILITY-AUDIT.md), [S2 release-ratchet completion audit](research/S2-RELEASE-RATCHET-COMPLETION-AUDIT.md), [historical S1/S2 implementation audit](research/S1-S2-IMPLEMENTATION-AUDIT.md) |
| See future direction / decision history | [Public roadmap](ROADMAP.md), [canonical decision index](DECISIONS.md), [condensed project overview](project/roadmap.md) |
| Understand what is public and what stays local | [Public/private documentation policy](PRIVATE-DOCUMENTATION-POLICY.md) |
| Understand SSH and multi-cloud direction | [SSH and multi-cloud delivery summary](SSH-CONNECTIVITY-MULTI-ENVIRONMENT-MULTI-CLOUD-PLAN.md), [provider status summary](MULTI-CLOUD-PROVIDERS-IMPLEMENTATION-AUDIT.md) |

## What is available today

Automexia v0.4 helps people keep command-driven work organized without taking
control away from their shells and tools. On Windows, Linux, and macOS it
provides windows, tabs, split panes, pane-local tabs, search, visible context,
keyboard navigation, configurable behavior, and local or terminal-protocol
image viewing. Invalid configuration leaves the last working setup active, and
migration from Rio is explicit and non-destructive.

The repository also contains substantial v0.5 foundations. Native shell completion and the CP2/CP3 Quick Action/alias pipeline are implemented locally, while S1 visual/model tooling and the S2 release ratchet are source-complete at their boundaries. Stable release still depends on hosted native/accessibility evidence, approved visual matrices, and 30 consecutive controlled performance days. The v0.5 read-only Connection Hub is implemented locally with process/network authority deliberately disabled; native macOS/Linux and controlled accessibility evidence remain release-gated. Managed SSH, multi-cloud provider authentication, public extensions, model
suggestions, and LLM workflow orchestration are **not** shipped v0.4 behavior.

Release work has two independent lanes: **v0.4 release closure** proves the
existing terminal product on its declared native, security, visual,
accessibility, packaging, and performance gates; **later protected feature
work** independently governs managed SSH, providers, credentials, ecosystem
runtime, CP6 model suggestions, and optional LLM orchestration authority.

| Area | Product status | Where to read |
|---|---|---|
| Terminal core, panes, tabs, selection, prompt context | **Available now** | [Terminal experience](guide/terminal-experience.md) |
| Configuration, themes, key bindings | **Available now** | [Configuration](reference/configuration.md), [Keyboard](reference/keyboard.md) |
| Session-only shell integration | **Available now** | [Shell and command productivity](guide/shell-productivity.md) |
| Native shell completion | **Implemented locally; release evidence still gated** | [Shell and command productivity](guide/shell-productivity.md) |
| Typed Quick Actions and opt-in aliases | **Implemented locally; release evidence still gated** | [Shell and command productivity](guide/shell-productivity.md) |
| Provider-aware Quick Actions (CP4) | **Product-integrated and nonactivating; provider refresh/execution not activated** | [Commands and shell](user-guide/commands-and-shell.md#provider-aware-quick-actions), [CP4 audit](PROVIDER-AWARE-QUICK-ACTIONS-IMPLEMENTATION-AUDIT.md), [testing](TESTING.md#m13-provider-aware-quick-actions) |
| Optional Automexia autocomplete surface | **Accepted source work partial overall; CP5.1-CP5.4 source/local models and CP5.5 inert bridge source done, preview disabled, native release gates open** | [CP5 testing](CP5-SUGGESTION-TESTING.md), [CP5 implementation audit](research/CP51-CP56-IMPLEMENTATION-AUDIT.md), [accepted ADR 0025](adr/0025-authenticated-native-editor-suggestion-bridge.md), [CP5.0 research](research/CP5-AUTOCOMPLETE-RESEARCH.md) |
| Semantic diagnostic and failed-command navigation | **Planned; no general action, detector, setting, or product UI exists** | [Public summary](SEMANTIC-DIAGNOSTIC-NAVIGATOR.md), [Roadmap](ROADMAP.md) |
| Situation-aware production operations and investigation | **Planned; no production-guidance runtime or managed operation exists** | [Public summary](SITUATION-AWARE-PRODUCTION-OPERATIONS.md), [contract principles](SITUATION-AWARE-PRODUCTION-OPERATIONS-CONTRACTS.md), [experience](SITUATION-AWARE-PRODUCTION-OPERATIONS-UX.md), [testing](SITUATION-AWARE-PRODUCTION-OPERATIONS-TESTING.md), [Roadmap](ROADMAP.md) |
| Static OpenSSH inventory, read-only Hub, and disabled direct-review preparation | **Implemented locally; release evidence gated; no launch authority** | [Connection Hub and SSH](user-guide/connection-hub-and-ssh.md) |
| Connection Hub records, pending selected-host review, and dry-run models | **Implemented locally; authority disabled** | [Remote connections](guide/remote-connections.md) |
| Recipes and multi-environment workspace review | **CLI and Hub review/editor foundations implemented; execution remains gated** | [Connection automation](SSH-CONNECTION-AUTOMATION.md), [workspace guide](user-guide/connection-hub-and-ssh.md#review-only-workspaces-and-broadcast), [Roadmap](ROADMAP.md) |
| Managed OpenSSH sessions | **Planned** | [Remote connections](guide/remote-connections.md), [Roadmap](ROADMAP.md) |
| AWS provider source contracts | **Implemented internally, not activated** | [Connection Hub and SSH](user-guide/connection-hub-and-ssh.md), [M8 testing](TESTING.md#m8-aws-adapter-source-contracts) |
| Azure provider source contracts | **Implemented internally, not activated** | [Connection Hub and SSH](user-guide/connection-hub-and-ssh.md), [M9 testing](TESTING.md#m9-azure-adapter-source-contracts) |
| Google Cloud provider source contracts | **Implemented internally, not activated** | [Provider testing](MULTI-CLOUD-PROVIDERS-TESTING.md#m10-google-cloud-adapter-source-contracts), [Provider audit](MULTI-CLOUD-PROVIDERS-IMPLEMENTATION-AUDIT.md) |
| Kubernetes/OpenShift source contracts | **Implemented internally, not activated** | [Provider testing](MULTI-CLOUD-PROVIDERS-TESTING.md#m11-kubernetes-and-openshift-source-contracts), [Provider audit](MULTI-CLOUD-PROVIDERS-IMPLEMENTATION-AUDIT.md) |
| Teleport adapter / OpenBao adapter | **Teleport implemented internally, not activated / OpenBao blocked on ADR 0024** | [Provider testing](MULTI-CLOUD-PROVIDERS-TESTING.md#m12-teleport-source-contracts), [Provider audit](MULTI-CLOUD-PROVIDERS-IMPLEMENTATION-AUDIT.md) |
| Public extension SDK, sandbox, and CP6 selected-input model suggestions | **Accepted source implementation complete locally; activation/downloads/provider calls disabled; native/release gates open** | [Safety boundary](ECOSYSTEM-PLATFORM.md), [testing](ECOSYSTEM-PLATFORM-TESTING.md), [implementation audit](research/D7-CP6-IMPLEMENTATION-AUDIT.md), [accepted ADR 0029](adr/0029-sandboxed-signed-ecosystem-boundary.md) |
| Optional LLM Orchestration extension | **Planned; no runtime behavior is implemented** | [Public summary](LLM-ORCHESTRATION-EXTENSION.md), [testing](LLM-ORCHESTRATION-TESTING.md), [proposed ADR 0033](adr/0033-optional-llm-orchestration-extension.md) |
| Embedded Automation Studio file editor, language services, and DevOps/SRE script execution | **Planned after the first stable release; no implementation or product UI** | [Public summary](AUTOMATION-STUDIO-ARCHITECTURE.md), [testing](AUTOMATION-STUDIO-TESTING.md), [proposed ADR 0030](adr/0030-automation-studio-domain-extension-boundary.md) |
| Specialized workflow extensions, including media and video | **Research direction only; no product runtime or editing claim** | [Product vision](PRODUCT-VISION.md#long-term-direction), [Roadmap](ROADMAP.md#future-specialized-workflow-domains) |

## Where Automexia is heading

Automexia's direction is broader than software development. The same terminal
workspace can support operations, automation, data, and creative work by helping
people keep tools, context, steps, and results together.

Future focused extensions may add experiences for domains such as media
processing and video editing. A separately proposed Automation Studio would
provide an embedded, optional foundation for scripts and configuration while a
DevOps/SRE extension would remain independently usable from the terminal. These
features must remain optional and separately reviewed; the current product does
not claim a built-in file or video editor. A separately proposed LLM
Orchestration extension may later compose reviewable workflows across enabled
domain actions, but Automexia remains useful without a model, account, network
connection or paid API. It is not part of the core, DevOps/SRE, Studio or video
extensions, and no orchestration runtime exists today. The
[product vision](PRODUCT-VISION.md) owns this value direction, while the
[roadmap](ROADMAP.md) owns sequencing and the [feature catalog](FEATURES.md)
owns availability.

The broad order is the first stable terminal, mature DevOps and extension
foundations, diagnostic and production-awareness workflows, an optional
Automation Studio release, and then dedicated creative extensions such as video
editing. Optional LLM Orchestration can evolve independently after the shared
capability and review boundaries are proven. Research may overlap, but no domain
becomes a hidden dependency of another.


## Documentation model

The set intentionally separates six kinds of public information:

- **User Guide** pages teach practical use, choices, commands, shortcuts, and end-to-end workflows.
- **Guide** pages explain deeper product behavior and specialized workflows without becoming exact schema tables.
- **Reference** pages contain exact settings, bindings, commands, defaults, and limits.
- **Developer** pages explain architecture, security boundaries, testing, and release trust.
- **Project** pages summarize high-level roadmap and decision history for readers; they are not instructions for current product behavior.
- **Public proposals** explain user value and safety boundaries without exposing exact unreleased algorithms, provider recipes, state machines, or execution ledgers.

The original documentation mixed these roles heavily. The [source consolidation map](project/source-map.md) records that reorganization, while the [public/private documentation policy](PRIVATE-DOCUMENTATION-POLICY.md) defines what must stay out of version control.

## CP3.3 evidence

CP3.3 native imports and trusted workspace task bridges are part of the reviewed command-productivity architecture contract.
