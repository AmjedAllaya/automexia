Added `amx repo` and `amx repo issues` for GitHub.com/GitLab.com, with explicit
remote selection and offline destination previews. Installed Git retains remote
discovery and rewrite semantics; no fetch, authentication, installation or Git
configuration write is requested. Unsafe or unsupported remote URLs fail closed.
Native shell and WSL metadata tests, URL policy regressions and a checked parser
benchmark cover the new route; actual browser visibility remains separate.
