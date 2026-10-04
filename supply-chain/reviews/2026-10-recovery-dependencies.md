# Recovery protection dependency review

Recovery uses platform credential storage and authenticated encryption on its
existing background worker. It introduces no custom cryptographic primitive.
The exact registry versions recorded in `audits.toml` received automated source
review of checksummed archives, including production code, unsafe boundaries,
manifests and relevant tests. These records are not human or formal cryptographic
audits. Their notes retain caller, hardware, platform and provider assumptions.

The reviewed registry additions are aead 0.6.1, aes 0.9.3, block-padding 0.4.2,
cbc 0.2.1, chacha20poly1305 0.11.0, cipher 0.5.2, cmov 0.5.4, cpubits 0.1.1,
ctutils 0.4.2, hkdf 0.13.0, hmac 0.13.0, num 0.4.3, num-bigint 0.4.8,
num-complex 0.4.6, num-integer 0.1.47, num-iter 0.1.46, num-rational 0.4.2,
poly1305 0.9.1, secret-service 5.2.0, security-framework-sys 2.17.0 and
universal-hash 0.6.1. Existing dependencies retain their prior policy.

InOut is a separately reviewed local source adaptation, described in its
[provenance record](../../third-party/inout/UPSTREAM.md). No registry audit or
exemption is added for that modified package. Cargo Vet's path-dependency policy
is paired with a mandatory complete-file fingerprint check, rejection of registry
substitution, and workspace regression tests. Its registry dependencies remain
subject to Cargo Vet. Removing the override, changing a reviewed file or adding
an unreviewed source file fails the recovery architecture gate. Updating it
requires reviewing the new source and rerunning both Miri borrow models.

Protection tests exercise real encryption and reject tampering and mismatched
keys. Windows uses native user-scoped DPAPI. Unix cryptographic tests replace
only the key-store adapter; native Secret Service and macOS Keychain behavior
remain separate platform evidence. The macOS adapter preserves native ACLs and
uses process-local dialog suppression without updating or deleting credentials.
