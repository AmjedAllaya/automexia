$ErrorActionPreference = 'Stop'
# Match the Unix fixture's literal rows through the real ConsoleHost output path.
for ($index = 0; $index -lt 1024; $index++) {
    [Console]::Write(("FINAL-ROW-{0:d4} {1}`r`n" -f $index, ('0' * 60)))
}
[Console]::Write("FINAL-TAIL-END`r`n")
