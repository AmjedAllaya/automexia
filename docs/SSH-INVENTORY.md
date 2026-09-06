# OpenSSH inventory

The public inventory reads bounded public metadata from local OpenSSH
configuration files that the user explicitly selects and reviews.

## Why Automexia does not resolve effective SSH configuration

Inventory does not connect, authenticate, launch OpenSSH, read credentials,
contact a network, modify OpenSSH configuration, or scan the user profile.

## Security and resource contract

Selected files and includes are untrusted. The implementation bounds total
files, bytes, include depth, records, labels, and refresh work. It rejects or
safely handles malformed input, cycles, duplicates, links, identity replacement,
permission failure, oversized data, control characters, and stale generations.

## Using the inventory package

### Records

Records contain only the public fields needed to identify an inventory entry.
Passwords, private keys, tokens, command history, terminal output, and hidden
environment values are never stored.

## Refresh, revoke, and recovery

Refresh work is cancellable and publishes only a complete current generation.
Failure preserves last-known-good state and reports a redacted error. Revoking a
selected source removes its records from the next accepted snapshot without
changing the source file.

## Verification

Tests cover missing/empty/valid/malformed inputs, nested includes, duplicates,
limits, links, replacement, concurrent refresh, cancellation, stale results,
private-data canaries, restart, disable, revoke, and cleanup.

See [OpenSSH and remote shells](user-guide/connection-hub-and-ssh.md) and
[Native OpenSSH assurance](F5-NATIVE-OPENSSH-ASSURANCE.md).
