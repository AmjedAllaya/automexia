# ADR 0063: Explicit browser search routing

Status: Accepted

## Decision

Extend the existing application CLI owner from ADR 0062 with `search` and `docs`.
The single query validator preserves argument/UTF-8 limits and the existing
`google` spelling. A bounded static provider catalog builds fixed HTTPS URLs
using the existing URL encoder; the application-owned default-handler adapter
dispatches exactly once after validation. Unknown providers fail without fallback.

`search` selects Google, GitHub repositories, YouTube or DuckDuckGo. `docs` uses
Google's separate site filter for a selected official documentation host/path;
its help and success message identify Google. User query text cannot replace a
query parameter, origin or fragment. Search engines still control returned pages;
the filter is a search request, not a guarantee or an authorization to execute
their contents. Endpoint catalogs also supply provider descriptions in CLI help.

This is application launch behavior, not terminal VT parsing or a network-backed
provider integration. A new extension would duplicate the existing CLI/native
dispatch owner without introducing a separate capability. No new crate,
dependency, API client, background service, startup lookup or persisted query is
introduced. General Quick Action exact-launch restrictions are unchanged.

## Interaction and failure behavior

Native shells retain editing, history, Tab ownership and quoting. `--print-url`
before query terms is an explicit offline preview; it never opens the browser or
writes configuration. Empty/unknown/oversized/control-bearing inputs fail before
dispatch. Errors and debug formatting omit query contents. Browser and shell
history policies remain external. Browser handoff success does not prove page
loading; optional search failure cannot affect an existing terminal session.

Windows uses the shared COM-balanced shell handler; WSL wrappers forward exact
arguments to the translated application executable. Native Unix uses its existing
desktop adapter. No platform shell expressions or implicit Enter are introduced.
Reverting the two new CLI variants and catalog reverses this increment without
data migration. Existing packaging carries the application and session adapters.

## Verification

Library tests independently assert exact provider URLs, repository type, official
site scope, Unicode, query injection, limits and preview/dispatch isolation. The
post-build native harness executes all routes and verifies shell forwarding on
each available supported shell. Correctness-checked URL benchmarks measure
construction only, not network/browser latency. Source-contract mutation tests
guard routing and actual post-build verification. Unexecuted native desktop
handlers, browser rendering and interactive CMD remain external validation.

## Primary references

- [GitHub repository search](https://docs.github.com/en/search-github/searching-on-github/searching-for-repositories)
- [DuckDuckGo URL parameters](https://duckduckgo.com/duckduckgo-help-pages/settings/params)
- [YouTube search behavior](https://support.google.com/youtube/answer/16090438)
- [Google advanced site filtering](https://support.google.com/websearch/answer/35890)
- [Google advanced search form](https://www.google.com/advanced_search)
- [Existing launch and query ownership](0062-explicit-google-search-command.md)
