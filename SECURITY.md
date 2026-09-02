# Security policy

## Supported versions

Security support follows the currently published release channels. Use only
packages whose checksums, provenance, and platform signatures match the official
release metadata.

## Report a vulnerability

Do not open a public issue for a vulnerability or include credentials, real
host data, private infrastructure, terminal history, or exploit details in a
public artifact. Use the project's private security-reporting channel and
include a minimal redacted reproduction, affected version, platform, impact,
and safe contact information.

## Security model

Automexia treats terminal output, control sequences, shell metadata,
configuration, paths, files, clipboard data, imported records, extension data,
and generated text as untrusted.

The application keeps one owner for each PTY, process tree, terminal state,
route, snapshot, persisted record, and UI surface. Optional behavior cannot
become a competing terminal/process owner.

## Process and PTY safety

Structured launches use a typed executable and exact argument array. They do not
use shell command concatenation, `sh -c`, `cmd /c`, PowerShell expression
evaluation, or implicit Enter.

Each session owns its child/PTY lifecycle, ordered input, resize, output,
cancellation, exit, descendant cleanup, and shutdown joining. A failed optional
feature cannot break the basic local shell path.

## Terminal-output safety

CSI, OSC, DCS, APC, hyperlinks, titles, clipboard requests, and image protocols
have explicit byte, dimension, nesting, cache, and lifetime limits. Malformed or
unsupported input fails safely.

Terminal output cannot launch a process, read arbitrary files, gain a
capability, or become terminal input without an explicit user action.

## Configuration and files

Configuration and persisted state are bounded, versioned, private where
required, validated before publication, and written with safe temporary-file,
flush, replacement, and recovery behavior. Invalid reload keeps
last-known-good state.

File features validate exact targets, links, identity replacement, type, size,
permissions, and stale generations. Errors and diagnostics are actionable but
redacted.

## Clipboard and insertion

Copy and paste are explicit and route-scoped. Paste never adds Enter. Overlays
own their input while open, so palette/search/dialog keys do not reach the PTY.

No public insertion or alias path expands credentials, reads hidden history,
or executes text automatically.

## System OpenSSH boundary

When a user manually runs system OpenSSH, OpenSSH and the operating system own
configuration, credentials, agents, host-key policy, authentication, proxy
behavior, and networking. Automexia owns only the local terminal session.

Public inventory reads only explicitly selected bounded local files, exposes
public metadata, performs no passive network/login work, and preserves
last-known-good state after invalid, linked, replaced, or revoked input.

## Extension boundary

Public extension infrastructure is deny-by-default. Optional code receives only
exact, reviewed, bounded capabilities and data. It receives no ambient terminal
content, history, clipboard, environment, filesystem, network, credential,
process, or PTY authority.

Results are validated and bound to package/component identity, scope, route or
resource, generation, lifetime, and revocation. Disable and uninstall cancel and
join work, remove only exact owned state, preserve user files, and leave the core
terminal usable.

Automexia v0.4 does not claim public third-party extension download,
distribution, marketplace activation, or component execution.

## Secrets and privacy

Credentials belong in platform or external credential stores. Persist opaque
references instead of secret material where possible.

Never include real usernames, hostnames, home paths, environment values, account
or tenant identifiers, private IPs/domains, tokens, cookies, credentials,
private history, or identifying provider output in source, tests, fixtures,
snapshots, logs, documentation, screenshots, changes, commits, or reports.

Use fictional stable values such as `alice`, `devbox`,
`example.invalid`, documented test-network addresses, and runtime-created
temporary paths.

## Dependencies and supply chain

Dependencies require license, provenance, maintenance, advisory, unsafe-code,
authority, feature, platform, MSRV, build, binary-size, startup, resource,
offline, cancellation, cleanup, rollback, and replacement review.

Release artifacts bind exact source, lockfiles, dependencies, checksums, SBOM,
provenance, signatures, and platform verification. Never disable TLS,
signature, policy, or scanner controls to make a release pass.

## Assurance

Security-sensitive boundaries use table-driven negative tests, property tests,
fuzzing, deterministic concurrency models, forbidden-side-effect assertions,
resource/lifecycle repetition, checker mutation tests, and native platform
evidence.

A passing test proves only the exact campaign and environment. Missing native,
hardware, signing, account, assistive-technology, or long-duration evidence is
reported as external.

See [Architecture](docs/ARCHITECTURE.md), [Testing](docs/TESTING.md), and
[Release trust](docs/RELEASE-TRUST.md).

## Private planning

Unreleased advanced features, integrations, commercial products, and their
security designs are maintained in ignored private documentation. They must not
be copied into public issues, ADRs, fixtures, or examples before an explicit
publication review.
