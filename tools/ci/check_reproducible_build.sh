#!/usr/bin/env bash
set -euo pipefail

output=${1:?output evidence path is required}
root=$(git rev-parse --show-toplevel)
target=$(rustc -vV | sed -n 's/^host: //p')
case "$target" in
  x86_64-unknown-linux-gnu|aarch64-unknown-linux-gnu) ;;
  *) echo 'Linux reproducibility requires a native supported Linux host.' >&2; exit 1 ;;
esac
options=()
if [[ -n "${2:-}" ]]; then options+=(--reference "$2"); fi
python3 "$root/tools/ci/reproducible_build.py" "$output" --target "$target" "${options[@]}"
