# Frequently asked questions

## What is Automexia?

Automexia is a keyboard-first terminal for local shells, command-line tools, and
ordinary system OpenSSH workflows.

## Which features are public?

Only capabilities listed in [the public feature catalog](FEATURES.md) and their
linked user guides are public. Source names, fixtures, tests, disabled modules,
or compatibility paths do not announce a feature.

## Does Automexia replace my shell?

No. PowerShell, CMD, Bash, Zsh, Fish, and other supported shells keep command
editing, history, quoting, completion, aliases, expansion, pipelines, and
execution.

## Can I run normal command-line tools?

Yes. Run Git, Cargo, Python, FFmpeg, database clients, cloud CLIs, Kubernetes
clients, REPLs, TUIs, and scripts with their normal shell syntax. Their
credentials, configuration, network behavior, and command semantics remain
owned by those tools.

## Does Automexia press Enter for me?

No. Paste, copy, selection, navigation, alias generation, and any public
insertion path do not add implicit Enter.

## How does OpenSSH work?

Run the system `ssh`, `scp`, or `sftp` command in a pane. OpenSSH and the
operating system own host keys, credentials, agents, authentication,
configuration, and networking. Automexia owns the local terminal session.

## Does Automexia store SSH passwords or keys?

No public terminal or inventory workflow is a credential vault. Keep credentials
with OpenSSH, the operating system, a hardware device, or another appropriate
external owner.

## What does explicit OpenSSH inventory do?

It reads only bounded local files that the user explicitly selects and reviews,
shows public metadata, and performs no passive connection, login, or credential
work. Revocation and invalid input preserve user files and safe
last-known-good behavior.

## How is configuration handled?

Configuration is bounded and transactionally validated. Invalid reload keeps the
last-known-good state. Starter generation and migration do not overwrite
existing source files.

## Does normal launch change shell profiles?

No. Supported integration is session-local by default. Persistent install,
repair, or removal is explicit and affects only Automexia-owned content.

## Which platforms are supported?

See [Platforms](PLATFORMS.md) and the current release page. Source portability,
cross-compilation, and native release evidence are different claims.

## Is Automexia accessible?

Accessibility is a release contract covering keyboard access, focus,
semantics, contrast, reduced motion, scaling, Unicode/IME, and native assistive
technology evidence. See [Accessibility](ACCESSIBILITY.md).

## Are there public extensions or a marketplace?

Automexia v0.4 does not claim public third-party extension download,
installation, marketplace activation, or component execution. The public
extension page documents only capability and lifecycle safety.

## Does the public terminal require AI or a paid API?

No.

## Where are advanced and commercial plans?

They are intentionally maintained in ignored private documentation and are not
summarized publicly. Public pages must not tease, name, or link those plans
before an explicit publication decision.

## How do I report a problem?

Use the issue template for ordinary bugs and the private security channel for
vulnerabilities. Redact credentials, real host/account data, shell history,
machine-local paths, and private planning material.
