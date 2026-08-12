#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$root"

find "$root" \
  \( -path "$root/.git" -o -path "$root/target" -o -path "$root/.cargo-packager" \) \
  -prune -o -type f \( -name '*.sh' -o -name '*.bash' \) -print0 |
  while IFS= read -r -d '' source; do
    bash -n "$source"
    shellcheck "$source"
  done

find "$root" \
  \( -path "$root/.git" -o -path "$root/target" -o -path "$root/.cargo-packager" \) \
  -prune -o -type f -name '*.zsh' -print0 |
  while IFS= read -r -d '' source; do
    zsh -n "$source"
  done

bash tools/ci/test_shell_integration.sh
zsh tools/ci/test_zsh_integration.zsh

echo 'PASS: all repository Bash/Zsh sources parse, ShellCheck passes, and integration contracts pass'
