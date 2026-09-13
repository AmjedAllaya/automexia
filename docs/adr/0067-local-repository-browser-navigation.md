# ADR 0067: Local repository browser navigation

Status: accepted for `amx repo` and `amx repo issues`.

## Placement and reuse

Repository navigation belongs to the existing explicit application CLI/broker,
beside browser, directory and editor handoffs. It does not belong to VT/input/
render core or need a new extension lifecycle. The installed Git client owns
repository discovery and remote-config interpretation. The existing native/WSL
process owner supplies cancellation and bounded capture; the desktop adapter
owns browser activation. No new dependency, worker, watcher, index or startup
activity is introduced. Generic Quick Action execution remains blocked.

Source inspection found no existing authoritative remote-to-web navigation
implementation in the application/model owners. Reimplementing Git config parsing
would duplicate includes, repository discovery and rewrite semantics. Conversely,
current GitHub CLI `runBrowse` invokes `api.RepoExists` for `--no-browser`; it is
not an offline URL formatter. Using it for these two links adds API/authentication
requirements without needed functionality. Wrap one installed Git lookup and
build the small host-specific policy; do not replace GitHub CLI's general work.

## Authority and limits

The only Git command is `git --no-pager remote get-url -- <remote>`. It reads the
first fetch remote URL and performs no remote Git operation. The default remote
is origin; a missing remote fails without heuristics. The guest bridge permits
only this exact Git operation and uses the existing leased process group. Native
capture uses the existing job/group cleanup and deadlines. Missing tools never
trigger installation. Git configuration remains read-only and retains its own
include/rewrite semantics; metadata lookup is not a filesystem sandbox.

`repository_open` bounds remote names at 128 bytes, URLs at 4096, path components
at 255 and GitLab nesting at 16. It validates original path segments before URL
normalization can hide dot segments. Only fixed GitHub.com/GitLab.com hosts map
to HTTPS; standard SSH/HTTPS remotes are supported, credentials and ambiguous
or unsupported routes are rejected. Error/debug output does not disclose raw
remote values or paths. No persistence, configuration migration or credential
custody is introduced. Remove the route to roll back; existing Git is untouched.

## Evidence

Unit matrices cover schemes, hosts, ports, credentials, path normalization,
Unicode/control rejection, exact issues routes and resource boundaries. Real
temporary repositories provide independent config and read-only oracles. Shipped
shell tests execute the actual CLI, and the WSL suite checks guest metadata
ownership. Mutation tests enforce the fixed argv, allowlist, preview, redaction,
URL limits, native dispatch and checked benchmark. Parsing timing is not Git,
filesystem or browser latency. Native desktop visibility, repository access and
unexecuted native platforms remain external, not inferred from offline previews.

## Sources

- [Git remote command](https://git-scm.com/docs/git-remote)
- [Git configuration](https://git-scm.com/docs/git-config)
- [GitHub CLI browse contract](https://cli.github.com/manual/gh_browse)
- [GitHub CLI browse implementation](https://github.com/cli/cli/blob/trunk/pkg/cmd/browse/browse.go)
- [GitHub repositories](https://docs.github.com/en/repositories/creating-and-managing-repositories/about-repositories)
- [GitLab issues](https://docs.gitlab.com/user/project/issues/)
