@echo off
rem Automexia Command Prompt integration. This file is installed with the two
rem base64 placeholders below resolved for the current Windows account.
if /I not "%TERM_PROGRAM%"=="Automexia" if not "%AUTOMEXIA_SHELL_INTEGRATION%"=="1" goto :eof

set "AUTOMEXIA_CMD_INTEGRATION_LOADED=1"
set "AUTOMEXIA_SHELL_INTEGRATION=1"
set "TERM_PROGRAM=Automexia"
set "COLORTERM=truecolor"
rem Preserve existing macros and executables. Rust excludes expansion characters
rem from this macro target; query text is parsed once by the user's native CMD.
if "%AUTOMEXIA_AMX%"=="0" goto :automexia_amx_done
if not defined AUTOMEXIA_CLI_CMD goto :automexia_amx_done
doskey /macros | findstr /B /I /L "amx=" >nul 2>nul
if not errorlevel 1 goto :automexia_amx_done
where amx >nul 2>nul
if errorlevel 1 doskey amx="%AUTOMEXIA_CLI_CMD%" $*
:automexia_amx_done
set "AUTOMEXIA_CMD_ROOT=%~dp0"
if not defined AUTOMEXIA_CMD_USER_BASE64 set "AUTOMEXIA_CMD_USER_BASE64=__AUTOMEXIA_CMD_USER_BASE64__"
if not defined AUTOMEXIA_CMD_PATH_BASE64 set "AUTOMEXIA_CMD_PATH_BASE64=__AUTOMEXIA_CMD_PATH_BASE64__"

rem Keep this tracked and installed batch strictly ASCII. The launcher selects
rem UTF-8 before CMD parses this file and passes the Unicode prompt glyph through
rem the process command line, avoiding BOM/active-code-page parser corruption.
if not defined AUTOMEXIA_CMD_PROMPT_GLYPH set "AUTOMEXIA_CMD_PROMPT_GLYPH=>"

rem Obtain ESC without embedding a control byte in the tracked source file.
for /F "delims=#" %%E in ('"prompt #$E# & for %%E in (1) do rem"') do set "AUTOMEXIA_ESC=%%E"

rem Cache clone-safe shell identity once and embed it into PROMPT below. CMD's
rem prompt is repainted after every command, so this also restores CMD metadata
rem after a nested shell exits. Empty WSL fields deliberately clear stale data.
rem A bare OSC 133 D closes the preceding A/B region without claiming an exit
rem code CMD cannot expose through PROMPT. The terminal renders it neutrally.
set "AUTOMEXIA_CMD_IDENTITY=%AUTOMEXIA_ESC%]1337;SetUserVar=automexia_shell_name=Q01E%AUTOMEXIA_ESC%\%AUTOMEXIA_ESC%]1337;SetUserVar=automexia_shell_user=%AUTOMEXIA_CMD_USER_BASE64%%AUTOMEXIA_ESC%\%AUTOMEXIA_ESC%]1337;SetUserVar=automexia_shell_path=%AUTOMEXIA_CMD_PATH_BASE64%%AUTOMEXIA_ESC%\%AUTOMEXIA_ESC%]1337;SetUserVar=automexia_distro=%AUTOMEXIA_ESC%\%AUTOMEXIA_ESC%]1337;SetUserVar=automexia_os_version=%AUTOMEXIA_ESC%\%AUTOMEXIA_ESC%]1337;SetUserVar=automexia_shell=MQ==%AUTOMEXIA_ESC%\"
set "AUTOMEXIA_CMD_DONE=%AUTOMEXIA_ESC%]133;D%AUTOMEXIA_ESC%\"
<nul set /p "=%AUTOMEXIA_CMD_IDENTITY%"

rem CMD expands $P every time it paints a prompt, so OSC 7, the window title,
rem and the complete semantic path update immediately after every `cd`. The
rem first blank row and path row are terminal-owned; CMD owns only lambda/input.
set "PROMPT=%AUTOMEXIA_CMD_IDENTITY%%AUTOMEXIA_CMD_DONE%$E]2;CMD - $P$E\$E]7;file:///$P$E\$E]1337;SetUserVar=automexia_prompt_active=MQ==$E\$E]133;A$E\$S$_$E]133;P;k=c$E\$E[38;2;98;176;255m$P$E[0m$_$E]133;P;k=c$E\$E[38;2;45;212;191m%AUTOMEXIA_CMD_PROMPT_GLYPH%$E[0m$S$E]133;B$E\"

rem CMD has no object formatting system. These interactive DOSKEY macros call
rem a display-only PowerShell helper and leave built-in DIR and cmd /c intact.
if not "%AUTOMEXIA_PLAIN_LS%"=="1" (
  doskey ls=call "%~dp0automexia-ls.cmd" $*
  doskey ll=call "%~dp0automexia-ls.cmd" -la $*
)

set "AUTOMEXIA_ESC="
set "AUTOMEXIA_CMD_IDENTITY="
set "AUTOMEXIA_CMD_DONE="
set "AUTOMEXIA_CMD_PROMPT_GLYPH="
rem CP3.1 aliases are verified once as a complete immutable generation, then
rem DOSKEY loads the exact reviewed macro file. No per-alias process is created.
set "AUTOMEXIA_ALIAS_CONFIG_ROOT=%AUTOMEXIA_CONFIG_HOME%"
if not defined AUTOMEXIA_ALIAS_CONFIG_ROOT set "AUTOMEXIA_ALIAS_CONFIG_ROOT=%LOCALAPPDATA%\Automexia\Terminal"
set "AUTOMEXIA_ALIAS_STATE=FAILED"
set "AUTOMEXIA_ALIAS_GENERATION="
set "AUTOMEXIA_ALIAS_FILE="
set "AUTOMEXIA_ALIAS_NAMES="
set "AUTOMEXIA_ALIAS_RESULT="
set "AUTOMEXIA_ALIAS_POWERSHELL=powershell.exe"
where pwsh.exe >nul 2>nul && set "AUTOMEXIA_ALIAS_POWERSHELL=pwsh.exe"
for /f "usebackq delims=" %%R in (`%AUTOMEXIA_ALIAS_POWERSHELL% -NoLogo -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "%~dp0automexia-alias-loader.ps1" -ConfigRoot "%AUTOMEXIA_ALIAS_CONFIG_ROOT%"`) do set "AUTOMEXIA_ALIAS_RESULT=%%R"
for /f "tokens=1-4 delims=|" %%A in ("%AUTOMEXIA_ALIAS_RESULT%") do (
  set "AUTOMEXIA_ALIAS_STATE=%%A"
  set "AUTOMEXIA_ALIAS_GENERATION=%%B"
  set "AUTOMEXIA_ALIAS_FILE=%%C"
  set "AUTOMEXIA_ALIAS_NAMES=%%D"
)
if /I "%AUTOMEXIA_ALIAS_STATE%"=="READY" doskey /macrofile="%AUTOMEXIA_ALIAS_FILE%"

doskey /macros | findstr /B /I /L "automexia_aliases_health=" >nul 2>nul
if errorlevel 1 where automexia_aliases_health >nul 2>nul
if errorlevel 1 doskey automexia_aliases_health=echo state=%AUTOMEXIA_ALIAS_STATE% generation=%AUTOMEXIA_ALIAS_GENERATION% names=%AUTOMEXIA_ALIAS_NAMES%
doskey /macros | findstr /B /I /L "automexia_aliases_reload=" >nul 2>nul
if errorlevel 1 where automexia_aliases_reload >nul 2>nul
if errorlevel 1 doskey automexia_aliases_reload=echo Open a new CMD session to activate a different verified alias generation safely.

set "AUTOMEXIA_ALIAS_RESULT="
set "AUTOMEXIA_ALIAS_POWERSHELL="
