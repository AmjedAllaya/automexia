# Frequently asked questions

## What is Automexia?

Automexia is a keyboard-first terminal for local shells, command-line tools, and
ordinary system OpenSSH workflows.

## Which features are public?

Check [Features](FEATURES.md) and its linked guides. Source modules and tests
alone do not establish availability.

## Does Automexia replace my shell?

No. Your shell controls editing, history, quoting, completion, aliases,
expansion, pipelines, and execution.

## Can I run normal command-line tools?

Yes. Run Git, Python, database clients, and other programs with normal shell
syntax. Each tool keeps its own credentials, configuration, networking, and
command behavior.

## Does Automexia press Enter for me?

Paste and **Insert without Enter** do not submit commands; you press Enter to
run them. A [reviewed workflow](user-guide/quick-actions.md#build-a-workflow) can
submit its approved steps after you explicitly choose **Run workflow**.

## How does OpenSSH work?

Run system `ssh`, `scp`, or `sftp` in a pane. OpenSSH and the operating system
manage authentication, keys, agents, configuration, and networking. Automexia
manages the local terminal session.

## Does Automexia store SSH passwords or keys?

Terminal and inventory workflows are not credential vaults. Keep credentials
with OpenSSH, the operating system, hardware, or an appropriate external store.

## What does explicit OpenSSH inventory do?

It reads bounded local files you select and review, then displays public metadata.
It does not connect, log in, or manage credentials. Invalid input or revocation
preserves user files and the last working state.

## How is configuration handled?

Configuration is checked against limits before changes apply. Invalid reloads
keep the last working settings. Starter generation does not overwrite files;
migration preserves the originals. See [Configuration](CONFIGURATION.md).

## Does normal launch change shell profiles?

No. Integration is session-local by default. Persistent install, repair, and
removal require explicit action and affect only Automexia-owned content.

## Which platforms are supported?

See [Platforms](PLATFORMS.md) and the current release page. Source portability,
cross-compilation, and native release evidence are different claims.

## Is Automexia accessible?

Accessibility requirements cover keyboard access, focus, semantics, contrast,
reduced motion, scaling, Unicode/IME, and native assistive-technology evidence.
See [Accessibility](ACCESSIBILITY.md) for status and limits.

## Are there public extensions or a marketplace?

Automexia v0.4 does not claim public third-party downloads, installation,
marketplace activation, or component execution. [Extensions](EXTENSIONS.md)
describes current capability and lifecycle boundaries.

## Does the public terminal require AI or a paid API?

No.

## Where are advanced and commercial plans?

They remain private. See [Private documentation policy](PRIVATE-DOCUMENTATION-POLICY.md).

## How do I report a problem?

Use issue templates for bugs and the [private security route](../SECURITY.md)
for vulnerabilities. Remove credentials, host/account details, history, local
paths, and private planning material before sharing.
