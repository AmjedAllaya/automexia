@echo off
rem Automexia Command Prompt integration. This file is installed with the two
rem base64 placeholders below resolved for the current Windows account.
if /I not "%TERM_PROGRAM%"=="Automexia" if not "%AUTOMEXIA_SHELL_INTEGRATION%"=="1" goto :eof
if defined AUTOMEXIA_CMD_INTEGRATION_LOADED goto :eof

set "AUTOMEXIA_CMD_INTEGRATION_LOADED=1"
set "AUTOMEXIA_SHELL_INTEGRATION=1"
set "TERM_PROGRAM=Automexia"
set "COLORTERM=truecolor"
set "AUTOMEXIA_CMD_ROOT=%~dp0"

rem CMD reads batch source using its active console code page. UTF-8 keeps the
rem lambda prompt and Unicode file names deterministic in modern ConPTY hosts.
chcp 65001 >nul

rem Obtain ESC without embedding a control byte in the tracked source file.
for /F "delims=#" %%E in ('"prompt #$E# & for %%E in (1) do rem"') do set "AUTOMEXIA_ESC=%%E"

rem Publish clone-safe shell identity before the first prompt. Empty WSL fields
rem deliberately clear metadata left by a nested WSL session.
<nul set /p "=%AUTOMEXIA_ESC%]1337;SetUserVar=automexia_shell_name=Q01E%AUTOMEXIA_ESC%\%AUTOMEXIA_ESC%]1337;SetUserVar=automexia_shell_user=__AUTOMEXIA_CMD_USER_BASE64__%AUTOMEXIA_ESC%\%AUTOMEXIA_ESC%]1337;SetUserVar=automexia_shell_path=__AUTOMEXIA_CMD_PATH_BASE64__%AUTOMEXIA_ESC%\%AUTOMEXIA_ESC%]1337;SetUserVar=automexia_distro=%AUTOMEXIA_ESC%\%AUTOMEXIA_ESC%]1337;SetUserVar=automexia_os_version=%AUTOMEXIA_ESC%\%AUTOMEXIA_ESC%]1337;SetUserVar=automexia_shell=MQ==%AUTOMEXIA_ESC%\"

rem CMD expands $P every time it paints a prompt, so OSC 7, the window title,
rem and the complete semantic path update immediately after every `cd`. The
rem first blank row and path row are terminal-owned; CMD owns only lambda/input.
set "PROMPT=$E]2;CMD - $P$E\$E]7;file:///$P$E\$E]1337;SetUserVar=automexia_prompt_active=MQ==$E\$E]133;A$E\$S$_$E]133;P;k=c$E\$E[38;2;98;176;255m$P$E[0m$_$E]133;P;k=c$E\$E[38;2;45;212;191mλ$E[0m$S$E]133;B$E\"

rem CMD has no object formatting system. These interactive DOSKEY macros call
rem a display-only PowerShell helper and leave built-in DIR and cmd /c intact.
if not "%AUTOMEXIA_PLAIN_LS%"=="1" (
  doskey ls=call "%~dp0automexia-ls.cmd" $*
  doskey ll=call "%~dp0automexia-ls.cmd" -la $*
)

set "AUTOMEXIA_ESC="
