#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$root"
target_root=${CARGO_TARGET_DIR:-target}
mkdir -p "$target_root"

# librio is private in v0.4, but its curated header and exported symbols still
# form one ABI. Compile the consumer as strict C11, then link with C++ because
# the static library contains simdutf's C++ runtime dependency.
cargo build -p librio --locked
cc -std=c11 -Wall -Wextra -Werror -I librio/include \
  -c librio/ctest/main.c -o "$target_root/librio-c-api-smoke.o"
c++ "$target_root/librio-c-api-smoke.o" "$target_root/debug/liblibrio.a" \
  -ldl -lpthread -lm -lrt -lutil -o "$target_root/librio-c-api-smoke"
timeout 20s "$target_root/librio-c-api-smoke"

echo 'PASS: librio curated C header, exported symbols, mouse input, PTY, and render pull ABI'
