#!/usr/bin/env bash
set -euo pipefail
# More than a normal worker read batch, using only fictional fixed-width rows.
for ((index=0; index<1024; index++)); do
    printf 'FINAL-ROW-%04d %060d\r\n' "$index" 0
done
printf 'FINAL-TAIL-END\r\n'
