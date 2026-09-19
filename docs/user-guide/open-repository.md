# Open the current repository

```sh
amx repo
amx repo issues
amx repo --preview
amx repo issues --remote upstream --preview
```

The standalone executable uses `automexia repo` with identical arguments.
Git must already be installed. The command reads the first fetch URL of `origin`,
or the explicitly named `--remote`, using `git remote get-url`. There is no
fallback to a different remote. Git owns repository discovery and `insteadOf`
rewrites, so subdirectories use their containing repository.

Supported hosts are **GitHub.com and GitLab.com**. Accepted remotes are HTTPS,
`ssh://git@host/namespace/repository` and `git@host:namespace/repository`, with an
optional `.git` suffix. GitLab subgroup paths are supported up to 16 components.
GitHub paths must contain an owner and repository. Host-specific issue pages are
used; this does not create an issue or change its state.
Hostnames are case-insensitive, including SSH URLs; repository and subgroup
names keep their original casing.

`--preview` prints one JSON destination without opening a browser or contacting
the host. It reads local metadata only. The command does not authenticate, fetch,
push, install tools, read repository contents, or write Git configuration. Actual
opening uses the desktop's existing browser handler; the browser may require login.
This cannot prove the repository exists or that you have access to it.

## Safety and limits

Remote names are at most 128 ASCII bytes, start with a letter or number, and use
letters, digits, dots, slashes, underscores or hyphens. Remote URLs are limited
to 4096 bytes, individual namespace components to 255 bytes, and path components
to ASCII letters, digits, dots, underscores and hyphens. Dot/parent segments and
ambiguous forms are rejected. Standard HTTPS and SSH ports are accepted; custom
ports, embedded credentials, query strings, fragments, local paths, SSH host
aliases and unsupported/self-hosted domains are not silently reinterpreted.

If lookup fails, check your current repository, its ownership, installed Git and
the remote name. Automexia never changes Git's trusted-directory settings. On
Windows-backed WSL it runs installed guest Git through the existing bounded
Python 3.10+ bridge, using the guest current directory. It does not inspect an
unrelated Windows checkout. Native Linux/macOS use their installed native Git.

Git config includes and rewrites retain Git's own semantics. Local metadata on
mounted filesystems can still involve filesystem I/O; the child lookup has a
deadline and cancellation owner. The command is not a filesystem sandbox.
Preview destinations can reveal a private repository name: review before sharing.
Raw remotes and tool diagnostic contents are not printed on failure or saved.

For these simple links, Automexia uses Git and its existing browser adapter.
GitHub CLI's `gh browse --no-browser` performs an API existence check, so it is
not used as an offline URL formatter. Your installed `gh` remains unchanged.
