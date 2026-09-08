# Strengthen Kubernetes import assurance

Kubernetes import tests now verify that their credential-redaction sentinels
actually occur in the input and check each inline credential independently.
Coverage includes public and merged metadata, debug output and rejected-document
diagnostics. Parser benchmarks validate their results before measuring empty,
comment-heavy, dense credential, oversized and malformed inputs.

This changes test and benchmark evidence, not provider activation or runtime
permissions. Native cluster and desktop validation remain separate requirements.
