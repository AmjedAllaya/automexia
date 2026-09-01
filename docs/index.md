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
| Review the proposed embedded script editor and DevOps/SRE extension design | [Automation Studio architecture](AUTOMATION-STUDIO-ARCHITECTURE.md), [testing and evidence contract](AUTOMATION-STUDIO-TESTING.md), [proposed ADR 0030](adr/0030-automation-studio-domain-extension-boundary.md) |
| Review the optional LLM Orchestration architecture | [LLM Orchestration specification](LLM-ORCHESTRATION-EXTENSION.md), [testing and evidence contract](LLM-ORCHESTRATION-TESTING.md), [proposed ADR 0033](adr/0033-optional-llm-orchestration-extension.md) |
| Review the proposed error-section and failed-command navigation design | [Semantic Diagnostic Navigator](SEMANTIC-DIAGNOSTIC-NAVIGATOR.md), [proposed ADR 0032](adr/0032-bounded-semantic-diagnostic-navigation.md) |
| Review the proposed production investigation, situation-aware completion, incident, and managed-operations design | [Production Operations specification](SITUATION-AWARE-PRODUCTION-OPERATIONS.md), [exact PO0 contracts and 2026 decisions](SITUATION-AWARE-PRODUCTION-OPERATIONS-CONTRACTS.md), [UX and implementation blueprint](SITUATION-AWARE-PRODUCTION-OPERATIONS-UX.md), [testing and evidence contract](SITUATION-AWARE-PRODUCTION-OPERATIONS-TESTING.md), [proposed ADR 0034](adr/0034-situation-aware-production-operations.md) |
| Run verification or understand release gates | [Testing overview](developer/testing-release.md), [complete testing and evidence contract](TESTING.md), [per-feature reinforcement plan](FEATURE-TEST-REINFORCEMENT.md) |
| Audit completed-output grouping and its validation incident | [Command-result surface assurance](COMMAND-RESULT-ASSURANCE.md) |
| Review UI branding implementation and exact U10 release evidence | [UI branding roadmap](UI-BRANDING-ROADMAP.md), [U10 assurance audit](research/U10-UI-BRANDING-ASSURANCE-AUDIT.md) |
| Configure or audit controlled F5 native OpenSSH evidence | [F5 native OpenSSH assurance](F5-NATIVE-OPENSSH-ASSURANCE.md), [D0/D3/D5 and stable-release completion audit](research/D0-D3-D5-MANAGED-SSH-AND-STABLE-RELEASE-AUDIT.md) |
| Audit stable-release blockers or restore hosted CI and repository protection | [Section 12 completion audit](research/STABLE-RELEASE-AND-REPOSITORY-BLOCKERS-AUDIT.md), [repository protection guide](../.github/BRANCH-PROTECTION.md), [ADR 0031](adr/0031-versioned-hosted-ci-and-repository-protection.md) |
| Review stabilization/S1/S2 status and external evidence | [Stabilization roadmap](STABILIZATION-ROADMAP.md), [S1 native/visual/resource/accessibility audit](research/S1-NATIVE-VISUAL-RESOURCE-ACCESSIBILITY-AUDIT.md), [S2 release-ratchet completion audit](research/S2-RELEASE-RATCHET-COMPLETION-AUDIT.md), [historical S1/S2 implementation audit](research/S1-S2-IMPLEMENTATION-AUDIT.md) |
| See future work / decision history | [Canonical roadmap](ROADMAP.md), [canonical decision index](DECISIONS.md), [condensed project overview](project/roadmap.md) |
| Review reconciled research and proposals | [Aligned research/proposal pack](../automexia_docs_repository_aligned/README.md) - non-authoritative until integrated into a canonical owner |
| Follow the detailed SSH and multi-cloud implementation phases | [SSH, connectivity, multi-environment, and multi-cloud plan](SSH-CONNECTIVITY-MULTI-ENVIRONMENT-MULTI-CLOUD-PLAN.md), [provider audit](MULTI-CLOUD-PROVIDERS-IMPLEMENTATION-AUDIT.md) |

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
| Semantic diagnostic and failed-command navigation | **DN0 design partially done; no action, detector, setting, or product UI exists** | [Proposed specification](SEMANTIC-DIAGNOSTIC-NAVIGATOR.md), [Roadmap](ROADMAP.md#post-v04-semantic-diagnostic-navigation-dn0-dn6) |
| Situation-aware production operations and investigation | **PO0 detailed proposal/checker only; passports, explain/change/compare/network/SLO views, situation-aware candidates, Incident Mode, managed operations/diagnostic sessions, runbooks, cross-environment comparison, and all PO1-PO8 runtime behavior are not implemented** | [Proposed specification](SITUATION-AWARE-PRODUCTION-OPERATIONS.md), [exact proposed contracts](SITUATION-AWARE-PRODUCTION-OPERATIONS-CONTRACTS.md), [detailed UX/build blueprint](SITUATION-AWARE-PRODUCTION-OPERATIONS-UX.md), [testing](SITUATION-AWARE-PRODUCTION-OPERATIONS-TESTING.md), [Roadmap](ROADMAP.md#post-v051-situation-aware-production-operations-po0-po8) |
| Static OpenSSH inventory, read-only Hub, and disabled direct-review preparation | **Implemented locally; release evidence gated; no launch authority** | [Connection Hub and SSH](user-guide/connection-hub-and-ssh.md) |
| Connection Hub records, pending selected-host review, and dry-run models | **Implemented locally; authority disabled** | [Remote connections](guide/remote-connections.md) |
| Recipes and multi-environment workspace review (M6) | **Hardened CLI and Hub review/editor surface implemented; execution disabled by D3/M5 gates** | [Connection automation](SSH-CONNECTION-AUTOMATION.md#m6-review-only-implementation), [workspace guide](user-guide/connection-hub-and-ssh.md#review-only-workspaces-and-broadcast), [stable-release audit](research/M6-F6-WORKSPACES-STABLE-RELEASE-AUDIT.md), [M6 roadmap](ROADMAP.md) |
| Managed OpenSSH sessions | **Source and assurance path implemented locally; production activation and real native evidence blocked** | [Remote connections](guide/remote-connections.md), [F5 assurance](F5-NATIVE-OPENSSH-ASSURANCE.md), [completion audit](research/D0-D3-D5-MANAGED-SSH-AND-STABLE-RELEASE-AUDIT.md), [Roadmap](ROADMAP.md) |
| AWS provider source contracts | **Stable local contract implemented, not activated; real provider/native/release evidence external** | [Connection Hub and SSH](user-guide/connection-hub-and-ssh.md), [M8 testing](TESTING.md#m8-aws-adapter-source-contracts), [stable provider gate](MULTI-CLOUD-PROVIDERS-TESTING.md#m8-m12-stable-release-semantic-and-hostile-input-gate) |
| Azure provider source contracts | **Stable local contract implemented, not activated; real provider/native/release evidence external** | [Connection Hub and SSH](user-guide/connection-hub-and-ssh.md), [M9 testing](TESTING.md#m9-azure-adapter-source-contracts), [stable provider gate](MULTI-CLOUD-PROVIDERS-TESTING.md#m8-m12-stable-release-semantic-and-hostile-input-gate) |
| Google Cloud provider source contracts | **Implemented internally, not activated** | [Provider testing](MULTI-CLOUD-PROVIDERS-TESTING.md#m10-google-cloud-adapter-source-contracts), [Provider audit](MULTI-CLOUD-PROVIDERS-IMPLEMENTATION-AUDIT.md) |
| Kubernetes/OpenShift source contracts | **Implemented internally, not activated** | [Provider testing](MULTI-CLOUD-PROVIDERS-TESTING.md#m11-kubernetes-and-openshift-source-contracts), [Provider audit](MULTI-CLOUD-PROVIDERS-IMPLEMENTATION-AUDIT.md) |
| Teleport adapter / OpenBao adapter | **Teleport implemented internally, not activated / OpenBao blocked on ADR 0024** | [Provider testing](MULTI-CLOUD-PROVIDERS-TESTING.md#m12-teleport-source-contracts), [Provider audit](MULTI-CLOUD-PROVIDERS-IMPLEMENTATION-AUDIT.md) |
| Public extension SDK, sandbox, and CP6 selected-input model suggestions | **Accepted source implementation complete locally; activation/downloads/provider calls disabled; native/release gates open** | [Safety boundary](ECOSYSTEM-PLATFORM.md), [testing](ECOSYSTEM-PLATFORM-TESTING.md), [implementation audit](research/D7-CP6-IMPLEMENTATION-AUDIT.md), [accepted ADR 0029](adr/0029-sandboxed-signed-ecosystem-boundary.md) |
| Optional LLM Orchestration extension | **LO0 documentation proposal only; LO1-LO5 and all runtime behavior not implemented** | [Specification](LLM-ORCHESTRATION-EXTENSION.md), [testing](LLM-ORCHESTRATION-TESTING.md), [proposed ADR 0033](adr/0033-optional-llm-orchestration-extension.md) |
| Embedded Automation Studio file editor, language servers, and DevOps/SRE script execution | **Architecture and test proposal only; no implementation or product UI** | [Architecture](AUTOMATION-STUDIO-ARCHITECTURE.md), [testing](AUTOMATION-STUDIO-TESTING.md), [proposed ADR 0030](adr/0030-automation-studio-domain-extension-boundary.md) |
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

The planned product order is the first stable terminal, shared extension and
DevOps foundations, a small neutral workflow contract, a minimal Studio release,
then a dedicated video-editing extension. Orchestration may proceed after the
neutral contract is proven, independently of Studio and video. Research may
overlap, but video remains a separate domain built on generic workspace and task
services rather than Studio-specific editor or LLM code.


## Documentation model

The set intentionally separates six kinds of information:

- **User Guide** pages teach practical use, choices, commands, shortcuts, and end-to-end workflows.
- **Guide** pages explain deeper product behavior and specialized workflows without becoming exact schema tables.
- **Reference** pages contain exact settings, bindings, commands, defaults, and limits.
- **Developer** pages explain architecture, security boundaries, testing, and release trust.
- **Project** pages summarize roadmap and decision history for readers. The complete machine-enforced status register and ADR set remain the canonical owners linked above; project summaries are not instructions for current product behavior.
- **Research/proposal pack** pages preserve evidence, alternatives, and candidate plans. They are inputs to canonical documentation and ADR review, not product instructions or implementation proof.

The original documentation mixed these roles heavily. The [source consolidation map](project/source-map.md) records that reorganization and the later restoration of detailed, machine-enforced owners; it is a provenance map, not a competing authority list.

## CP3.3 evidence

CP3.3 native imports and trusted workspace task bridges are part of the reviewed command-productivity architecture contract.
