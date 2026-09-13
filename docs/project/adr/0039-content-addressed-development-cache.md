# ADR 0039: Content-addressed development cache lifecycle

The canonical decision is
[ADR 0039](../../adr/0039-content-addressed-development-cache.md). It keeps
immutable assurance tools shared and content-addressed, mutable build targets
worktree-local, cleanup bounded and dry-run by default, and the local pre-push
hook dormant while GitHub's free hosted CI remains automatic.
