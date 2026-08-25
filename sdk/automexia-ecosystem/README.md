# Automexia ecosystem conformance source

Status: private, unpublished, and disabled by release policy. This directory is
not a public SDK or an activation path.

The accepted logical Component Model world is
`automexia:ecosystem/suggestion@1`. Its checked-in WIT source is
`../../wit/automexia-ecosystem-1.0.0/ecosystem.wit`, where the source world is
named `extension`. WIT package interfaces and worlds share one item namespace,
so the required imported `suggestion` interface and the world cannot both use
that source item name. The immutable acceptance receipt records this syntax
mapping without changing the accepted logical contract.

The current compatibility window is SDK major 1, minor 0 only. Components must
declare their exact imports, receive no default WASI, and pass the package,
provenance, capability, resource, privacy, lifecycle, and malicious-component
checks owned by `automexia-ecosystem` and `automexia-ecosystem-runtime`.

Release policy is explicit: runtime activation remains unavailable. Publishing, downloads, registry
metadata, generated language packages, and component execution also remain
unavailable until their separately recorded release, network, signing, native,
accessibility, security-review, and governance gates pass. The existing
first-party extensions and CP1-CP3 paths remain the fallback.
