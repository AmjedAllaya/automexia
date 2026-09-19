#!/usr/bin/env bash
set -eu
stty -echo
trap 'stty echo' EXIT
printf '\033]133;A;aid=1\a\r\n\033]133;P;k=c;aid=1\a/example\r\n\033]133;P;k=c;aid=1\alambda \033]133;B\alist\r\n\033]133;C\a'
if [[ ${1:-} == listing-wide ]]; then
  EZA_COLORS='' LS_COLORS='' eza --icons=always --color=always --width=146 --grid --sort=name -- "$3"
elif [[ ${1:-} == listing ]]; then
  # The parent owns this isolated directory and its cleanup, even on assertion
  # failure. Do not inspect a real working directory or load a shell profile.
  EZA_COLORS='' LS_COLORS='' eza --icons=always --color=always --width=100 --grid --sort=name -- "$3"
elif [[ ${1:-} == columns ]]; then
  # One-cell BMP and supplementary private-use icons match branded listings;
  # the gaps are literal cells, not tabs or renderer-injected spacing.
  for index in {1..32}; do
    printf 'ROW-%02d  \033[36m\uf07b alpha-%02d\033[0m       \033[35m\U000f19fc scripts-%02d\033[0m       \uf15c document-%02d.txt\r\n' "$index" "$index" "$index" "$index"
  done
elif [[ ${1:-} == wide ]]; then
    for index in {1..32}; do
        printf 'ROW-%02d  -a---  2026-01-01 12:00:00  %04d  artifact-%02d-abcdefghijklmnopqrstuvwxyz0123456789.txt\r\n' "$index" "$index" "$index"
    done
else
    for index in {1..8}; do printf 'ROW-%02d  retained output\r\n' "$index"; done
fi
printf '\033]133;D;0\a\033]133;A;aid=2\a\r\n\033]133;P;k=c;aid=2\a/example\r\n\033]133;P;k=c;aid=2\alambda \033]133;B\a'
printf '\033]2;RESIZE-READY\a'
# A title-only response does not move the cursor or manufacture output rows.
steps=12
if [[ ${2:-} == scan ]]; then
    IFS= read -r -n 1 probe
    printf '\033]2;RESIZE-BASELINE\a'
    steps=48
fi
for ((step=0; step<steps; step++)); do
    IFS= read -r -n 1 probe
    printf '\033]2;RESIZE-ACK-%s\a' "$step"
done
read -r -n 1 probe
