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
<nul set /p "=]133;P;k=c;aid=2lambda ]133;B"
rem PAUSE flushes buffered input, racing the parent's resize acknowledgment.
rem Keep CMD output and line-editor coverage; use a non-echoing buffered reader
rem for the no-redraw acknowledgment protocol shared with PowerShell.
if "%~2"=="line" goto line_probe
if "%~2"=="buffered" (
    powershell.exe -NoLogo -NoProfile -NonInteractive -File "%~dp0live-resize-ack.ps1" -CumulativeReceipt
) else (
    powershell.exe -NoLogo -NoProfile -NonInteractive -File "%~dp0live-resize-ack.ps1"
)
if errorlevel 1 exit /b 1
goto exit_probe
:line_probe
<nul set /p "=]2;RESIZE-READY"
for /l %%n in (0,1,11) do (
    set /p "probe="
    <nul set /p "=]2;RESIZE-ACK-%%n"
)
:exit_probe
rem Exit is a separate buffered line-input handshake after viewport assertions.
set /p "probe="
exit 0
