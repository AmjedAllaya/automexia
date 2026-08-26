# ADR 0024: OpenBao token-helper and SSH-certificate boundary

- Status: Proposed; implementation blocked pending protected acceptance
- Date: 2026-08-23

## Context

OpenBao authentication normally persists a token through a configurable token
helper. The helper is an executable that receives `get`, `store`, or `erase`
and exchanges token material through standard input or output. OpenBao's SSH
secrets engine returns a signed public certificate which must be used alongside
the user's private key. Integrating either surface would add credential-helper,
secret-stream, certificate-file, expiry, cleanup, and OpenSSH identity authority
not covered by the generic provider or Teleport boundaries.

The M12 roadmap explicitly requires a separate accepted security decision
before any OpenBao implementation. Adding a manifest, parser, command builder,
helper, file writer, status probe, or product UI before that review would bypass
the required authority gate.

## Proposed decision

If accepted, Automexia will wrap only the official `bao` CLI through the sole
application-owned exact-argument runner. It will not implement a token helper,
read helper standard output, receive a token on standard input, accept a token
argument or environment value, parse the default token file, import a private
key, or call the OpenBao API directly.

The integration will require an exact user-selected external token-helper
reference owned by OpenBao and will retain only a non-secret opaque reference.
Login must remain visibly interactive in the official CLI and suppress token
printing. A reviewed certificate request may write only a signed public SSH
certificate to an application-owned private transient file, with no-follow
creation, restrictive permissions, bounded size, atomic publication, explicit
expiry, deletion on revoke/disable/session end/shutdown, and crash-recovery
cleanup. The user's private key remains an opaque OpenSSH identity reference and
is never read, copied, logged, serialized, or supplied to OpenBao by Automexia.

Each request binds server reference, TLS policy, auth-method reference, mount,
role, public-key reference, principals, extensions, certificate TTL, session,
capsule revision, risk, exact executable identity, exact ordered arguments,
capabilities, deadline, and output limits. Secret-bearing flags and environment
names are forbidden. The review states that the external CLI/helper owns token
custody and that the transient public certificate will be removed.

OpenBao will be a separate disabled-by-default extension with independent
process, network, and exact transient-file capabilities. Disabling or
uninstalling it cancels work, revokes Automexia grants, removes only
Automexia-owned transient certificates, and never deletes OpenBao's token cache,
helper configuration, server data, or user keys.

## Alternatives

- Implement an Automexia token helper or keychain bridge: rejected because it
  makes Automexia a credential custodian and expands secret-stream authority.
- Read the default token file or `BAO_TOKEN`: rejected because it imports
  bearer credentials and risks logs, process environment, and cross-session
  leakage.
- Call the OpenBao HTTP API directly: rejected because it embeds TLS, auth,
  token, retry, and protocol ownership already held by the official CLI.
- Persist certificates in the user's SSH directory: rejected because it mutates
  user-owned identity state and makes cleanup/rollback ambiguous.
- Treat OpenBao as part of Teleport: rejected because their credential,
  certificate, cache, revocation, and uninstall authorities are independent.

## Required acceptance and verification

This ADR must be explicitly accepted through the repository's protected
security/capability review before production or test implementation begins.
Acceptance alone does not activate the feature.

Implementation evidence must include fake helper/CLI tests proving that no
token crosses the Automexia boundary, hostile output and certificate bounds,
exact argv/environment policy, TLS downgrade rejection, certificate expiry,
revocation, cancellation, crash cleanup, symlink/reparse resistance, disk-full
and read-only behavior, two-session isolation, redaction canaries, repeated
lifecycle resource checks, disable/uninstall independence, and controlled
native OpenBao/OpenSSH fixtures. Release activation additionally requires the
existing protected process/session boundary and platform evidence.

Primary references:
[OpenBao token helpers](https://openbao.org/docs/commands/token-helper/),
[OpenBao login](https://openbao.org/docs/commands/login/), and
[OpenBao signed SSH certificates](https://openbao.org/docs/secrets/ssh/signed-ssh-certificates/).
