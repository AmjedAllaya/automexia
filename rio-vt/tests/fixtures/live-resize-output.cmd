@echo off
setlocal
echo ]133;A;aid=1
echo ]133;P;k=c;aid=1/example
echo ]133;P;k=c;aid=1lambda ]133;Blist
<nul set /p "=]133;C"
for %%n in (01 02 03 04 05 06 07 08) do echo ROW-%%n  retained output
echo ]133;D;0]133;A;aid=2
echo ]133;P;k=c;aid=2/example
<nul set /p "=]133;P;k=c;aid=2lambda ]133;B]2;RESIZE-READY"
rem Title-only acknowledgments leave the command output unchanged.
for /l %%n in (0,1,11) do (
    set /p "probe="
    <nul set /p "=]2;RESIZE-ACK-%%n"
)
rem Let the reader verify the final resize before releasing the native child.
set /p "probe="
exit 0
