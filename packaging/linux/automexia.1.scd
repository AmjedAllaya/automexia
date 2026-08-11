AUTOMEXIA(1)

# NAME

automexia - Automexia Terminal

# SYNOPSIS

*automexia* [options] [*-e* command ...]

# DESCRIPTION

Automexia Terminal is a hardware-accelerated terminal for modern development
workflows.

# OPTIONS

*-h, --help*

	Print command help.

*-V, --version*

	Print the Automexia version.

*-e, --command* _COMMAND ..._

	Execute a command and its arguments.

*-w, --working-dir* _DIRECTORY_

	Start the shell in a specific directory.

*--write-config* _[PATH]_

	Write the default TOML configuration to PATH or the Automexia config root.

*--enable-log-file*

	Write logs beneath the Automexia config root's logs directory.

# ENVIRONMENT

*AUTOMEXIA_CONFIG_HOME*

	Override the Automexia configuration root.

*AUTOMEXIA_LOG_LEVEL*

	Override the configured tracing level.

# FILES

See docs/CONFIGURATION.md for Windows, macOS, Linux, and BSD paths.

# BUGS

Report public defects at
https://github.com/AmjedAllaya/automexia-terminal/issues. Report security
vulnerabilities privately as described in SECURITY.md.
