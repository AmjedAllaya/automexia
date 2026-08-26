# Product vision

Status: canonical product-purpose and value authority. This page explains what
Automexia is trying to achieve; the [feature catalog](FEATURES.md) remains the
authority for what is available now, release-gated, internal, or planned.

## Purpose

Automexia is a flexible terminal that helps people make complex command-driven
work faster, simpler, and easier to control.

It brings shells, tools, files, and useful context into one organized workspace.
The goal is not to hide commands. The goal is to remove avoidable friction
around them: repeated setup, lost context, scattered sessions, hard-to-find
actions, and unnecessary switching between tools.

## Who Automexia is for

Automexia is not only for software developers. It is for anyone whose work can
benefit from commands, automation, or connected tools, including:

- developers and technical contributors;
- system administrators, operators, and infrastructure teams;
- researchers, analysts, and people processing data;
- creators working with media and command-line production tools;
- people building repeatable personal or team workflows;
- experienced terminal users and people who want a clearer way to learn one.

A person does not need to fit one role. The workspace should adapt as the task
changes.

## The value Automexia should provide

### Faster work

Keep related tasks together, reuse a working context, and reach common actions
without repeating setup or searching through unrelated windows.

### Simpler workflows

Make complex work easier to understand by showing the active task, location,
session, and relevant context. Provide clear choices instead of hiding behavior
behind automation.

### Practical flexibility

Support different shells, platforms, layouts, shortcuts, tools, and fields.
Let people begin with a simple terminal and add structure only when it helps.

### Visible control

Show what will happen before a sensitive or mutating action. Keep credentials
with their proper owners, keep local data local unless the user chooses
otherwise, and provide safe cancellation, recovery, disable, and rollback
paths.

### Safer production operations

Help operators keep the exact environment, current evidence, dependencies,
permissions, risk, impact, verification, and recovery visible while they work.
Make recurring questions—what changed, why this resource differs, where a
connection fails, which users are affected, and what remains unknown—quick to
answer without copying whole dashboards into the terminal. Suggestions should
reduce investigation and context switching without pretending there is always
one certain answer. Diagnosis and refusal are better than a confident unsafe
action, and every external change remains a human decision.

### Room to grow

Keep the terminal useful by itself while allowing separately reviewed workflow
extensions to add focused capabilities later. Growth should not make ordinary
typing, shell use, startup, or terminal output slower or less predictable.

## Experience principles

1. **Lead with the outcome.** Explain what a person gains before describing how
   it is implemented.
2. **Keep commands direct.** Shells and specialist tools retain their normal
   behavior; Automexia organizes and supports them.
3. **Reduce context switching.** Related sessions, tools, results, and context
   should be easy to keep together.
4. **Make state understandable.** Active scope, location, freshness, risk, and
   failure should be visible in plain language.
5. **Adapt without forcing complexity.** Keyboard use is complete, pointer use
   remains available, and advanced structure stays optional.
6. **Protect user control.** Discovery or suggestions never become hidden
   execution, network access, credential access, or persistent changes.
7. **Grow in replaceable parts.** Specialized workflows belong behind clear
   boundaries so they can be enabled, disabled, updated, or removed without
   weakening the terminal.
8. **Tell the truth about status.** Available behavior, release-gated work,
   disabled foundations, research, and future plans must never be presented as
   the same thing.
9. **Stay AI-capable, not AI-dependent.** The terminal and every domain
   extension must remain complete and useful without a model, provider account,
   network connection, or paid API. Model-assisted orchestration belongs in a
   separately installed extension and never receives hidden execution authority.

A future small task-specific model must follow the same principle: it belongs in
a separately reviewed optional model extension or adapter, not in core or hidden
inside a domain extension. It must prove that its narrow value, offline behavior,
model provenance, memory, storage, CPU/GPU use, cancellation, disable, uninstall,
and fallback justify the added cost. This is not permission to label ordinary
rules, parsing, search, or automation as AI.

## What Automexia provides today

The current v0.4 product focuses on the terminal workspace:

- windows, tabs, split panes, and pane-local tabs;
- familiar PowerShell, Command Prompt, WSL, Bash, and Zsh workflows;
- pane and visible-workspace search;
- keyboard-first navigation, selection, and command discovery;
- visible shell, path, command, and project context;
- local and terminal-protocol image viewing;
- configurable appearance, behavior, and shortcuts;
- explicit security boundaries and safe configuration fallback.

See the [feature catalog](FEATURES.md) and
[documentation home](index.md#what-is-available-today) for exact status.

## Delivery order

Product delivery should preserve a clear dependency order:

1. finish and publish the first stable terminal release;
2. prove the shared extension, workspace, process, and DevOps foundations;
3. deliver the separately gated read-only context, evidence graph,
   change/resource/comparison/network/user-impact investigations,
   situation-aware completion, and production preflight slices after their
   CP5/D6 foundations; add Incident Mode before any managed operation or
   diagnostic session, while Studio and neutral-workflow research may continue
   without runtime authority;
4. define and prove a small domain-neutral workflow contract that domain
   extensions can expose without depending on an editor or a model;
5. release the first useful Automation Studio slice for file editing and
   recovery;
6. release a dedicated video-editing extension only after that Studio slice is
   stable at its declared platform boundary.

Architecture research and feasibility prototypes may proceed earlier when they
add no production dependency, runtime authority, or release blocker. The
optional LLM Orchestration extension may start after the domain-neutral workflow
contract is proven; it is independent of Studio and video delivery and must not
block either. The ordering does not require every advanced Studio phase—language
intelligence, debugging, remote work, or collaboration—to finish before video
work can continue.

Automation Studio and future video work should reuse generic core-owned
workspace, file, task, progress, cancellation, recovery, and extension
lifecycle services. The video extension must not depend on editor-specific
webview, document-view, or language-server internals. This keeps both features
optional, independently releasable, and replaceable.

## Long-term direction

Automexia can become a home for many kinds of command-driven work without
turning every task into a developer workflow. Candidate domains include remote
operations, infrastructure, data processing, automation, media production, and
video workflows.

For production operations, the planned direction is a lightweight optional
DevOps/SRE layer that can show a pane-local production passport, use bounded
current evidence to explain state and scheduling, show recent changes and field
ownership, compare unhealthy resources with healthy peers, trace a network path,
summarize user impact, rank native commands, and preserve GitOps and just-in-time
access policy. Incident Mode can then organize hypotheses, time/log navigation,
handoff, verification and recovery. Managed changes, port forwards, probes and
debug sessions remain later, separately granted actions. The layer stays
deterministic and model-free by default, performs no provider work per keystroke,
and never turns a suggestion into hidden execution.
See the [Production Operations proposal](SITUATION-AWARE-PRODUCTION-OPERATIONS.md).
The experience should stay compact and reveal complexity only when requested:
extend the context already shown above the command, reuse the native suggestion
surface, keep evidence detail progressive, use one cancel-first review for a
mutation, and keep any managed-operation monitor nonmodal. The shortest safe path
must remain keyboard-efficient, understandable without internal terminology,
usable at narrow sizes and high scale, and honest about stale, conflicting, or
missing evidence.

Inserting or reviewing shell text does not make Automexia the execution owner;
the shell owns a later Enter. Only a separately activated managed-action path may
promise final revalidation, monitoring, verification, receipts, cancellation,
and recovery. Product language must keep that distinction visible.


For video work, the direction is not to pretend that terminal text alone is a
complete editor. A future, separately reviewed extension could combine
command-line media tools with focused visual controls, previews, timelines, and
repeatable processing steps. The specialist media tools would still perform the
actual transformations, while Automexia would help people organize, review, and
control the workflow.

A future LLM Orchestration extension may help a user compose reviewable workflows
from actions exposed by installed extensions. It is a separate optional product
surface, not a feature hidden inside the terminal, DevOps/SRE, Studio, or video
extensions. Models may propose typed plans, but core-owned policy, review,
approval, and action execution remain authoritative. Local and self-hosted use
must be possible without a required paid provider; remote providers, if added,
remain explicit adapters inside that extension.

This direction is research, not shipped behavior. It does not authorize a video
runtime, model, dependency, download, network service, or editing claim. Any
such capability must enter the [roadmap](ROADMAP.md), pass the
[build, wrap, or adopt boundary](BUILD-WRAP-ADOPT-ARCHITECTURE.md), receive any
required ADR, and satisfy its own security, resource, accessibility, platform,
testing, disable, and removal gates.

## How documentation should express the vision

Public-facing pages should:

- explain the user value before implementation details;
- use broad language such as “people,” “users,” and “command-driven work” when
  the subject is not specifically software development;
- use “developer” or “contributor” when the instructions truly concern coding,
  building, testing, or contributing to Automexia;
- give examples across software, operations, automation, data, and creative
  work where the current product genuinely supports them;
- separate current capabilities from future direction in the same section;
- move exact internals, limits, and evidence into the appropriate reference,
  architecture, testing, or roadmap owner.

The [brand guide](BRANDING.md#product-message-and-voice) defines the short-form
message and writing voice used by the README, website copy, release pages, and
other introductions.
