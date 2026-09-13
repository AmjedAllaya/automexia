Prevent relative or empty WSL PATH entries from selecting a project-local Python
interpreter before isolated tool startup. Keep explicit absolute tool locations
in order, reject invalid or excessive PATH values, and preserve existing process
leases and cancellation. Add real interpreter canaries, boundary tests and a
correctness-checked filter benchmark.

Accept mixed-case GitHub/GitLab hostnames in standard SSH repository remotes
without changing repository path casing or weakening URL restrictions. Add
native Git regressions and mutation checks for both validation boundaries.
