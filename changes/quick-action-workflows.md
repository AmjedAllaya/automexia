# Visual Quick Actions and reviewed workflows

Add keyboard-first creation, editing, duplication and confirmed deletion using
the existing validated action store and asynchronous worker. Fix insertion while
the review palette owns focus, report clipboard/save failures, reject stale
reviews, and preserve conflicting drafts as copies.

Add explicit review and Run for bounded, ordered, shell-integrated workflows,
with full-command inspection, configurable completion/timeout, SSH scope
transitions, progress, pause, resume and cancellation. Failed or unknown command
status stops subsequent steps. Reviewed Windows commands follow ConPTY's active
input protocol; ordinary paste and insert-without-Enter keep their own semantics.
