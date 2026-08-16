@echo off
setlocal
if "%AUTOMEXIA_PLAIN_LS%"=="1" (
  dir %*
  exit /b %ERRORLEVEL%
)

where pwsh.exe >nul 2>nul
if not errorlevel 1 (
  pwsh.exe -NoLogo -NoProfile -File "%~dp0automexia-ls.ps1" %*
) else (
  powershell.exe -NoLogo -NoProfile -File "%~dp0automexia-ls.ps1" %*
)
exit /b %ERRORLEVEL%
