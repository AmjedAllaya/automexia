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
    normalized=$(mktemp "${TMPDIR:-/tmp}/automexia-shellcheck.XXXXXX.sh")
    sed 's/\r$//' "$source" >"$normalized"
    bash -n "$normalized"
    # Retained upstream demo utilities intentionally declare a few variables
    # for copy/paste output and keep license comments before their shebang.
    shellcheck --severity=warning -e SC1128,SC2034,SC2046 -s bash "$normalized"
    rm -f "$normalized"
  done

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
  "$root/shell-integration/posix/automexia-eza-filter.pl" \
  "$installer_config/automexia/automexia-eza-filter.pl"
[[ $(grep -Fc '# >>> AUTOMEXIA SHELL INTEGRATION >>>' "$installer_home/.bashrc") -eq 1 ]]
[[ $(grep -Fc '# >>> AUTOMEXIA SHELL INTEGRATION >>>' "$installer_home/.zshrc") -eq 1 ]]

# A second run must be a no-op, while an altered installed file must be repaired
# from the repository-owned source without duplicating either profile marker.
HOME="$installer_home" XDG_CONFIG_HOME="$installer_config" \
  sh "$root/shell-integration/install-unix.sh" --quiet
printf '\nlocally altered\n' >>"$installer_config/automexia/shell-integration.bash"
HOME="$installer_home" XDG_CONFIG_HOME="$installer_config" \
  sh "$root/shell-integration/install-unix.sh" --quiet
cmp -s \
  "$root/shell-integration/bash/automexia.bash" \
  "$installer_config/automexia/shell-integration.bash"
[[ $(grep -Fc '# >>> AUTOMEXIA SHELL INTEGRATION >>>' "$installer_home/.bashrc") -eq 1 ]]
[[ -s "$installer_config/automexia/install-state-unix.sha256" ]]

bash tools/ci/test_shell_integration.sh
zsh tools/ci/test_zsh_integration.zsh

echo 'PASS: all repository Bash/Zsh sources parse, ShellCheck passes, auto-install idempotency holds, and integration contracts pass'
