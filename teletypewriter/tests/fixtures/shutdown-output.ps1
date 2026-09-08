$ErrorActionPreference = 'Stop'
# More than the native pipe plus the terminal's 64 KiB ring can retain.
# No profile, history, filesystem state or unbounded output is involved.
[Console]::Out.Write(('x' * (2 * 1024 * 1024)))
[Console]::Out.Flush()
