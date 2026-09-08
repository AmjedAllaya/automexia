@echo off
setlocal
echo ]133;A;aid=1
echo ]133;P;k=c;aid=1/example
echo ]133;P;k=c;aid=1lambda ]133;Blist
<nul set /p "=]133;C"
if "%~1"=="wide" (
    for %%n in (01 02 03 04 05 06 07 08 09 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31 32) do echo ROW-%%n  -a---  2026-01-01 12:00:00  00%%n  artifact-%%n-abcdefghijklmnopqrstuvwxyz0123456789.txt
) else (
    for %%n in (01 02 03 04 05 06 07 08) do echo ROW-%%n  retained output
)
echo ]133;D;0]133;A;aid=2
echo ]133;P;k=c;aid=2/example
<nul set /p "=]133;P;k=c;aid=2lambda ]133;B]2;RESIZE-READY"
rem PAUSE reads a single non-echoing key. SET /P plus Enter adds a native newline
rem and can scroll during repaint, so it is only used by the explicit editor case.
for /l %%n in (0,1,11) do (
    if "%~2"=="line" (set /p "probe=") else (pause >nul)
    <nul set /p "=]2;RESIZE-ACK-%%n"
)
rem Exit is a separate line-input handshake, after all viewport assertions.
rem Unlike PAUSE, this accepts a release already buffered by the parent.
set /p "probe="
exit 0
