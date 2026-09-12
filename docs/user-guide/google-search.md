# Search Google from a pane

For terminal-native file/text search and offline usage examples, see
[local tools](local-tools.md).

## More browser searches

Use the same session-local `amx` command to choose a destination:

```sh
amx search github rust terminal
amx search youtube kubernetes networking
amx search ddg linux permissions
amx search google rust tutorial
amx docs kubernetes deployment
```

GitHub searches repositories, not issues or code. `docs` searches Google with
an official-documentation site filter. Supported documentation tools are
`kubernetes`, `docker`, `rust`, `python`, `git` and `terraform`. Browser searches
open a browser; they do not fetch results into the terminal. Unknown names return
an error and never silently switch providers. `amx google` remains supported.

Preview without a browser or network request:

```sh
amx search github --print-url 'C++ & Rust'
amx docs kubernetes --print-url deployment
```

Put `--print-url` before the search terms. After terms start, later arguments are
literal query text. Use `amx search --help` or `amx docs --help` for the catalog.
All routes use the same 256-argument and 4096-UTF-8-byte limits; controls and empty
queries are rejected. Query terms become a single encoded parameter. Shell
quoting/history and browser privacy still apply; documentation terms are sent to
Google when opening, not directly to a Kubernetes cluster or another provider API.

## Google compatibility command

In a new Automexia session with shell integration enabled, enter:

```text
amx google rust async tutorial
```

Your default browser opens a Google search for `rust async tutorial`. This is
an explicit command, not an automatic search of terminal output. It does not
create another terminal window. The same command is available as
`automexia google <terms>` when the application executable is on your PATH.

## Quoting and preview

Your shell still owns quoting and expansion. Quote operators such as `&`, `|`,
`$` and `;` instead of letting the shell interpret them. For example, Bash,
Zsh, Fish and PowerShell accept:

```text
amx google 'C++ & Rust comparison'
```

In CMD use double quotes: `amx google "C++ & Rust comparison"`.
Arguments are joined with a space and encoded as one Google query; Unicode,
ampersands, plus signs and other URL characters do not become URL options.
To inspect the URL without opening a browser:

```text
amx google --print-url C++ tutorial
amx google -- -site:example.invalid rust
```

Put `--print-url` before the first search word. After the first word, all
remaining arguments belong to the query. `--` separates a leading search
operator from command options. Empty or whitespace-only searches, control
characters, more than 256 arguments, or more than 4096 UTF-8 bytes including
joining spaces are rejected before opening anything.

## Availability and privacy

- The session-local `amx` helper supports PowerShell, Bash, Zsh, Fish and
  interactive CMD. Open a new session after updating Automexia. Existing
  functions, aliases and executable commands named `amx` keep priority.
- Set `AUTOMEXIA_AMX=0` before integration loads to disable this helper. This
  does not remove an `amx` command you created yourself. No global profile or
  persistent alias is installed by this feature.
- The application supplies its executable through a session-only path hint.
  WSL receives a translated Windows path and uses the Windows default browser.
  Native Linux/BSD uses `xdg-open`; macOS uses `open`. These require a working
  desktop/browser association. A successful handoff is not proof a page loaded.
- CMD uses an interactive DOSKEY macro, not a batch-file forwarding shim.
  Installation paths containing unsafe CMD expansion characters disable only
  this helper; invoke the quoted application executable directly instead.
- Automexia does not keep a search log or send a separate search request.
  Your shell can retain command history; the browser and Google receive the
  query. Preview explicitly writes the full URL to terminal output. Do not
  search for secrets or private operational data.

If no browser opens, use `--print-url` and check the OS default browser.
Headless and remote shells do not automatically gain access to your local
desktop. This helper does not install a browser or change system settings.
