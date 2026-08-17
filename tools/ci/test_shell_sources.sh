#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$root"

find "$root" \
  \( -path "$root/.git" -o -path "$root/target" -o -path "$root/.cargo-packager" \) \
  -prune -o -type f \( -name '*.sh' -o -name '*.bash' \) -print0 |
  while IFS= read -r -d '' source; do
    # Git guarantees LF in committed sources, but an older Windows checkout
    # can retain CRLF in its existing working tree after .gitattributes changes.
    # Validate the canonical text without mutating contributor files.
    # BSD/macOS mktemp requires the XXXXXX suffix at the end of the template.
    # ShellCheck receives the language explicitly, so this file does not need
    # a .sh suffix.
    normalized=$(mktemp "${TMPDIR:-/tmp}/automexia-shellcheck.XXXXXX")
    sed 's/\r$//' "$source" >"$normalized"
    bash -n "$normalized"
    # Retained upstream demo utilities intentionally declare a few variables
    # for copy/paste output and keep license comments before their shebang.
    shellcheck --severity=warning -e SC1128,SC2034,SC2046 -s bash "$normalized"
    rm -f "$normalized"
  done

if command -v fish >/dev/null 2>&1; then
  find "$root/shell-integration" -type f -name '*.fish' -print0 |
    while IFS= read -r -d '' source; do
      fish --no-execute "$source"
    done
else
  printf '%s\n' 'EXTERNAL: Fish syntax/runtime validation requires the native CI Fish package.'
fi

find "$root" \
  \( -path "$root/.git" -o -path "$root/target" -o -path "$root/.cargo-packager" \) \
  -prune -o -type f -name '*.zsh' -print0 |
  while IFS= read -r -d '' source; do
    sed 's/\r$//' "$source" | zsh -n
  done

installer_home=$(mktemp -d)
trap 'rm -rf "$installer_home"' EXIT
installer_config="$installer_home/config"
HOME="$installer_home" XDG_CONFIG_HOME="$installer_config" \
  sh "$root/shell-integration/install-unix.sh" --quiet
cmp -s \
  "$root/shell-integration/bash/automexia.bash" \
  "$installer_config/automexia/shell-integration.bash"
cmp -s \
  "$root/shell-integration/zsh/automexia.zsh" \
  "$installer_config/automexia/shell-integration.zsh"
cmp -s \
  "$root/shell-integration/completion/bash/automexia-completion.bash" \
  "$installer_config/automexia/automexia-completion.bash"
cmp -s \
  "$root/shell-integration/completion/zsh/automexia-completion.zsh" \
  "$installer_config/automexia/automexia-completion.zsh"
cmp -s \
  "$root/shell-integration/fish/automexia.fish" \
  "$installer_config/fish/conf.d/automexia.fish"
cmp -s \
  "$root/shell-integration/completion/fish/automexia-completion.fish" \
  "$installer_config/fish/conf.d/automexia-completion.fish"
cmp -s \
  "$root/shell-integration/posix/automexia-eza-filter.pl" \
  "$installer_config/automexia/automexia-eza-filter.pl"
[[ $(grep -Fc '# >>> AUTOMEXIA SHELL INTEGRATION >>>' "$installer_home/.bashrc") -eq 1 ]]
[[ $(grep -Fc '# >>> AUTOMEXIA SHELL INTEGRATION >>>' "$installer_home/.zshrc") -eq 1 ]]

# A valid-but-stale managed block must be repaired in place.
awk '
  $0 == "# >>> AUTOMEXIA SHELL INTEGRATION >>>" { print; getline; print "stale-owned-source-line"; next }
  { print }
' "$installer_home/.bashrc" >"$installer_home/.bashrc.stale"
mv "$installer_home/.bashrc.stale" "$installer_home/.bashrc"
HOME="$installer_home" XDG_CONFIG_HOME="$installer_config" \
  sh "$root/shell-integration/install-unix.sh" --quiet
grep -Fqx '[ -r "${AUTOMEXIA_CONFIG_HOME:-${XDG_CONFIG_HOME:-$HOME/.config}/automexia}/shell-integration.bash" ] && . "${AUTOMEXIA_CONFIG_HOME:-${XDG_CONFIG_HOME:-$HOME/.config}/automexia}/shell-integration.bash"' "$installer_home/.bashrc"
! grep -Fq 'stale-owned-source-line' "$installer_home/.bashrc"
[[ $(grep -Fc '# >>> AUTOMEXIA SHELL INTEGRATION >>>' "$installer_home/.bashrc") -eq 1 ]]

# A second run must be a no-op, while an altered installed file must be repaired
# from the repository-owned source without duplicating either profile marker.
HOME="$installer_home" XDG_CONFIG_HOME="$installer_config" \
  sh "$root/shell-integration/install-unix.sh" --quiet
printf '\nlocally altered\n' >>"$installer_config/automexia/shell-integration.bash"
printf '\nlocally altered\n' >>"$installer_config/fish/conf.d/automexia-completion.fish"
HOME="$installer_home" XDG_CONFIG_HOME="$installer_config" \
  sh "$root/shell-integration/install-unix.sh" --quiet
cmp -s \
  "$root/shell-integration/bash/automexia.bash" \
  "$installer_config/automexia/shell-integration.bash"
cmp -s \
  "$root/shell-integration/completion/fish/automexia-completion.fish" \
  "$installer_config/fish/conf.d/automexia-completion.fish"
[[ $(grep -Fc '# >>> AUTOMEXIA SHELL INTEGRATION >>>' "$installer_home/.bashrc") -eq 1 ]]
[[ -s "$installer_config/automexia/install-state-unix.sha256" ]]

bash tools/ci/test_shell_integration.sh
zsh tools/ci/test_zsh_integration.zsh
if command -v fish >/dev/null 2>&1; then
  fish tools/ci/test_fish_integration.fish
fi

mkdir -p "$installer_config/automexia/actions"
printf '%s\n' 'schema_version = 1' 'revision = 7' \
  >"$installer_config/automexia/actions/actions.toml"
python3 "$root/tools/ci/create_cp31_alias_fixture.py" \
  --config-root "$installer_config/automexia" --alias aut --value uninstall \
  >/dev/null

printf '%s\n' 'after-user-content' >>"$installer_home/.bashrc"
HOME="$installer_home" XDG_CONFIG_HOME="$installer_config" \
  sh "$root/shell-integration/uninstall-unix.sh" >/dev/null
grep -Fq 'after-user-content' "$installer_home/.bashrc"
! grep -Fq '# >>> AUTOMEXIA SHELL INTEGRATION >>>' "$installer_home/.bashrc"
[[ ! -e "$installer_config/automexia/shell-integration.bash" ]]
[[ ! -e "$installer_config/fish/conf.d/automexia.fish" ]]
[[ ! -e "$installer_config/automexia/generated/aliases" ]]
grep -Fqx 'revision = 7' "$installer_config/automexia/actions/actions.toml"

# Simulate Darwin deterministically and prove the canonical application-support
# location stays aligned with the frontend and xtask.
mac_home=$(mktemp -d)
mac_bin=$(mktemp -d)
printf '%s\n' '#!/bin/sh' 'printf "%s\\n" Darwin' >"$mac_bin/uname"
chmod 0755 "$mac_bin/uname"
HOME="$mac_home" PATH="$mac_bin:$PATH" \
  sh "$root/shell-integration/install-unix.sh" --quiet
mac_config="$mac_home/Library/Application Support/io.github.AmjedAllaya.AutomexiaTerminal"
cmp -s "$root/shell-integration/bash/automexia.bash" "$mac_config/shell-integration.bash"
grep -Fqx '[ -r "${AUTOMEXIA_CONFIG_HOME:-$HOME/Library/Application Support/io.github.AmjedAllaya.AutomexiaTerminal}/shell-integration.bash" ] && . "${AUTOMEXIA_CONFIG_HOME:-$HOME/Library/Application Support/io.github.AmjedAllaya.AutomexiaTerminal}/shell-integration.bash"' "$mac_home/.bashrc"
HOME="$mac_home" PATH="$mac_bin:$PATH" \
  sh "$root/shell-integration/uninstall-unix.sh" >/dev/null
[[ ! -e "$mac_config/shell-integration.bash" ]]
! grep -Fq '# >>> AUTOMEXIA SHELL INTEGRATION >>>' "$mac_home/.bashrc"
rm -rf "$mac_home" "$mac_bin"

# Persistence roots are an authority boundary. Relative roots and linked
# managed parents must fail before profile or external state changes.
relative_home=$(mktemp -d)
if (cd "$relative_home" && HOME="$relative_home" AUTOMEXIA_CONFIG_HOME=relative-root \
    sh "$root/shell-integration/install-unix.sh" --quiet >/dev/null 2>&1); then
  echo 'relative Automexia config root was accepted' >&2
  exit 1
fi
[[ ! -e "$relative_home/relative-root" ]]
rm -rf "$relative_home"

hostile_home=$(mktemp -d)
hostile_config="$hostile_home/config/automexia"
mkdir -p "$hostile_home/config"
chmod 700 "$hostile_home/config"
python3 "$root/tools/ci/create_cp31_alias_fixture.py" \
  --config-root "$hostile_config" --alias badt --value guarded >/dev/null
touch "$hostile_config/generated/aliases/unexpected-user-file"
printf '%s\n' \
  '# >>> AUTOMEXIA SHELL INTEGRATION >>>' \
  'owned-source-line' \
  '# <<< AUTOMEXIA SHELL INTEGRATION <<<' \
  'user-content' >"$hostile_home/.bashrc"
if HOME="$hostile_home" XDG_CONFIG_HOME="$hostile_home/config" \
    sh "$root/shell-integration/uninstall-unix.sh" >/dev/null 2>&1; then
  echo 'uninstall accepted an unexpected generated-alias entry' >&2
  exit 1
fi
grep -Fqx '# >>> AUTOMEXIA SHELL INTEGRATION >>>' "$hostile_home/.bashrc"
grep -Fqx 'user-content' "$hostile_home/.bashrc"
[[ -f "$hostile_config/generated/aliases/current" ]]
rm -rf "$hostile_home"

linked_home=$(mktemp -d)
linked_outside=$(mktemp -d)
printf '%s\n' 'outside-sentinel' >"$linked_outside/sentinel"
ln -s "$linked_outside" "$linked_home/config-link"
printf '%s\n' \
  '# >>> AUTOMEXIA SHELL INTEGRATION >>>' \
  'owned-source-line' \
  '# <<< AUTOMEXIA SHELL INTEGRATION <<<' \
  'user-content' >"$linked_home/.bashrc"
if HOME="$linked_home" AUTOMEXIA_CONFIG_HOME="$linked_home/config-link" \
    sh "$root/shell-integration/uninstall-unix.sh" >/dev/null 2>&1; then
  echo 'uninstall accepted a linked Automexia config root' >&2
  exit 1
fi
grep -Fqx 'outside-sentinel' "$linked_outside/sentinel"
grep -Fqx '# >>> AUTOMEXIA SHELL INTEGRATION >>>' "$linked_home/.bashrc"
grep -Fqx 'user-content' "$linked_home/.bashrc"
rm -rf "$linked_home" "$linked_outside"

echo 'PASS: Bash/Zsh/Fish sources parse; installs use platform-canonical absolute roots; repair/uninstall are exact, idempotent, and link-safe; integration contracts pass'
