Fix conventional WSL kubeconfig discovery without falling back to the Windows
host's cluster. Bound guest filesystem reads in an application-owned helper,
replace order-sensitive YAML scanning, and show clean namespace-only labels with
a larger Kubernetes icon. Add cached, bounded local path hints for Bash, Zsh,
Fish and PowerShell, with explicit clearing and opt-out. Keep hints out of generic
serialization/debug output; reject native network/device paths. Add parser,
native shell/kubectl, helper lifecycle, VT, glyph raster and benchmark coverage.
Reduce badge-loading delays by preserving discovery across title-only changes,
showing known shell identity immediately and publishing initial Git context before
WSL Kubernetes reads finish. Preserve visible namespaces during periodic refresh.
Replace Zsh full-PATH command-table probes with targeted native executable lookup.
