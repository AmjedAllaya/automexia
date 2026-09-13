# Build, wrap, and adopt architecture

## Status and purpose

This page documents the dependency decisions needed to maintain current public
source. It does not publish future product, extension, provider, or commercial
plans.

Automexia uses three approaches:

- **Build** small product-specific state and policy that must remain under the
  terminal's direct control.
- **Wrap** operating-system or installed-tool behavior through a narrow typed
  adapter.
- **Adopt** a maintained library or protocol when implementing it again would
  add risk without creating product-specific value.

## Current ownership

Automexia builds:

- terminal and workspace state owned by this repository;
- typed configuration and migration rules;
- keyboard action mapping;
- bounded UI models;
- capability-free command and connection review records;
- cancellation, generation, cleanup, and recovery policy.

Automexia wraps:

- Unix PTYs and Windows ConPTY;
- native windows, clipboard, file selection, and credential-store references;
- installed shells and command-line tools;
- system OpenSSH for actual SSH behavior;
- platform packaging and signing tools.

Automexia adopts:

- Rust crates already approved by repository dependency policy;
- documented terminal protocols;
- maintained Unicode, image, font, window, serialization, and platform
  libraries where their licenses and resource behavior fit;
- external tools for protocols, authentication, and credential custody they
  already own.

## Dependency direction

```text
terminal engines and capability-free models
                 |
                 v
desktop application composition and capability brokers
                 |
                 v
narrow platform, shell, file, process, and optional-extension adapters
                 |
                 v
operating system and external tools
```

Lower layers do not depend on the desktop application or on vendor-specific
services. Terminal input, PTY, resize, parsing, rendering, and startup hot paths
must not wait for filesystem, provider, credential, network, or extension work.

## Process and credential boundary

Structured actions use a typed executable plus exact argument array. They do not
construct a shell command, invoke `sh -c`, `cmd /c`, or PowerShell expression
evaluation.

Passwords, tokens, private keys, cookies, and provider credential caches stay
with the operating system, agent, credential store, or installed tool that owns
them. Automexia stores an opaque reference where integration needs one.

## Extension boundary

Current extension foundations are optional and least-authority:

- terminal engines expose no direct provider or network handle;
- the application owns capability review and brokered I/O;
- extensions remain disabled until their package, lifecycle, capability,
  resource, native, and release gates pass;
- disabling or removing an extension must preserve terminal operation;
- extension failures cannot block input, PTY teardown, or renderer progress.

This public page does not describe unreleased extension products or marketplace
plans.

## Dependency review

Before adding or materially changing a dependency, record:

- official source, version, enabled features, and license;
- maintenance, provenance, advisories, and transitive dependencies;
- supported Rust toolchain and platforms;
- binary size, build time, startup, memory, CPU, storage, and network impact;
- unsafe code and capability footprint;
- cancellation, timeouts, limits, cleanup, offline behavior, and failure mode;
- migration, rollback, disable, uninstall, and replacement;
- why the standard library or existing project infrastructure is insufficient.

## Verification

Changes must preserve:

- acyclic owned-crate dependencies;
- one application composition root;
- exact argument vectors and no shell evaluation;
- credential and provider ownership outside terminal engines;
- bounded queues, caches, storage, retries, time, and concurrency;
- stale-generation rejection and publish-before-wake;
- cancellation and joined cleanup;
- unchanged terminal hot-path performance;
- native evidence for every claimed platform;
- accurate documentation and release status.

See [Architecture](ARCHITECTURE.md), [Security](../SECURITY.md),
[Testing](TESTING.md), and [ADR 0020](adr/0020-hybrid-build-wrap-adopt-boundary.md).
