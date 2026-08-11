#!/usr/bin/env sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
CFG=${XDG_CONFIG_HOME:-$HOME/.config}/automexia
mkdir -p "$CFG"
cp "$ROOT/shell-integration/bash/automexia.bash" "$CFG/shell-integration.bash"
cp "$ROOT/shell-integration/zsh/automexia.zsh" "$CFG/shell-integration.zsh"
append_block() {
  file=$1
  body=$2
  touch "$file"
  grep -Fq '# >>> AUTOMEXIA SHELL INTEGRATION >>>' "$file" 2>/dev/null && return 0
  printf '\n# >>> AUTOMEXIA SHELL INTEGRATION >>>\n%s\n# <<< AUTOMEXIA SHELL INTEGRATION <<<\n' "$body" >> "$file"
}
append_block "$HOME/.bashrc" '[ -r "${XDG_CONFIG_HOME:-$HOME/.config}/automexia/shell-integration.bash" ] && . "${XDG_CONFIG_HOME:-$HOME/.config}/automexia/shell-integration.bash"'
append_block "$HOME/.zshrc" '[ -r "${XDG_CONFIG_HOME:-$HOME/.config}/automexia/shell-integration.zsh" ] && . "${XDG_CONFIG_HOME:-$HOME/.config}/automexia/shell-integration.zsh"'
printf '%s\n' 'Automexia Bash/Zsh shell integration installed. Restart your shell.'
