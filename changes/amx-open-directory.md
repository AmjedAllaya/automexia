Added explicit `amx open [directory]` with a destination preview, directory-only
validation, redacted errors and supervised Windows-backed WSL path resolution.
Existing file managers retain ownership; no tool installation, startup work or
persistent cache is added. See the directory-opening guide for platform limits.
Corrected Windows local-tool cleanup returning before descendant process handles
signaled. Bounded job accounting and pinned member waits now complement existing
termination ownership, with concurrent native and exact handle-recovery tests.
