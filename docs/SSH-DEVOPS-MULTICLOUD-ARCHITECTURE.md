# SSH, DevOps, and multi-cloud architecture

Status: public architecture summary. Some source foundations exist, while live
provider and production activation remains release-gated.

## Product boundary

Automexia keeps terminal fundamentals in the core and puts SSH, cloud,
Kubernetes, OpenShift, infrastructure, and production workflows in optional
first-party extensions. The application owns one capability broker for
privileged operations so extensions do not launch processes, read credentials,
or contact providers through hidden paths.

## Main responsibilities

The core owns terminal sessions, panes, rendering, input, scrollback, common
configuration, and stable capability contracts. The DevOps/SRE extension owns
environment context, connection profiles, provider-neutral operational models,
and user-facing workflows. Provider adapters own their native authentication,
discovery, and command mapping.

Each pane keeps an isolated environment context. A connection or provider action
must name its account, region, cluster, namespace, host, and other relevant
targets explicitly. Credentials stay with native tools, platform credential
stores, agents, or approved vaults; Automexia stores opaque references whenever
possible.

## Remote access

The preferred SSH baseline is the maintained system OpenSSH client. Automexia
builds a typed request, shows the target and security-relevant options, and asks
the application broker to launch the exact executable and argument list. Host
trust remains explicit. Tunnels, routes, certificates, and organization access
are optional capabilities, not silent defaults.

## Multi-cloud behavior

AWS, Azure, Google Cloud, Kubernetes, OpenShift, Teleport, and secret-management
integrations remain independently enabled adapters. Discovery is explicit or
cached, bounded, cancellable, and never runs on each keystroke. Losing one
provider must not break the terminal or another environment.

## Resource and safety principles

- no provider work on terminal input, PTY, resize, renderer, or startup hot paths;
- bounded workers, queues, cache entries, retained records, retries, timeouts,
  network requests, child processes, and persisted data;
- generation and pane isolation with stale-result rejection;
- typed executable and argument arrays, no command-string evaluation;
- clear review for identity, target, authority, risk, and environment changes;
- disable, uninstall, offline fallback, and native-tool recovery remain usable.

Detailed unreleased APIs, provider playbooks, internal limits, and delivery
sequences are maintained outside the public repository until implementation
review.

See [Terminal-first operations](TERMINAL-FIRST-OPERATIONS.md),
[SSH connection automation](SSH-CONNECTION-AUTOMATION.md), and
[Extensions](EXTENSIONS.md).
