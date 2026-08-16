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

printf '%s\n' 'after-user-content' >>"$installer_home/.bashrc"
HOME="$installer_home" XDG_CONFIG_HOME="$installer_config" \
  sh "$root/shell-integration/uninstall-unix.sh" >/dev/null
grep -Fq 'after-user-content' "$installer_home/.bashrc"
! grep -Fq '# >>> AUTOMEXIA SHELL INTEGRATION >>>' "$installer_home/.bashrc"
[[ ! -e "$installer_config/automexia/shell-integration.bash" ]]
[[ ! -e "$installer_config/fish/conf.d/automexia.fish" ]]

echo 'PASS: Bash/Zsh/Fish sources parse, ShellCheck passes, install/repair/uninstall are exact and idempotent, and integration contracts pass'
