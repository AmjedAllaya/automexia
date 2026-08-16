#!/usr/bin/env bash
set -euo pipefail

output=${1:?output evidence path is required}
root=$(git rev-parse --show-toplevel)
commit=$(git -C "$root" rev-parse --verify HEAD)
source_date_epoch=$(git -C "$root" log -1 --format=%ct)
temporary=$(mktemp -d)
source_root="$temporary/source"
trap 'rm -rf -- "$temporary"' EXIT HUP INT TERM

hashes=()
durations=()
sizes=()
for run in 1 2; do
  rm -rf -- "$source_root"
  mkdir -p "$source_root"
  git -C "$root" archive --format=tar HEAD | tar -xf - -C "$source_root"
  started=$SECONDS
  (
    cd "$source_root"
    export CARGO_INCREMENTAL=0
    export SOURCE_DATE_EPOCH="$source_date_epoch"
    export TZ=UTC
    export LC_ALL=C.UTF-8
    export RUSTFLAGS="--remap-path-prefix=$source_root=/usr/src/automexia -C link-arg=-Wl,--build-id=sha1"
    cargo build --release --locked -p automexia-terminal --bin automexia
  )
  binary="$source_root/target/release/automexia"
  test -x "$binary"
  snapshot="$temporary/automexia-run-$run"
  cp "$binary" "$snapshot"
  hashes+=("$(sha256sum "$snapshot" | cut -d' ' -f1)")
  sizes+=("$(stat -c %s "$snapshot")")
  durations+=("$((SECONDS - started))")
done

if ! cmp -s "$temporary/automexia-run-1" "$temporary/automexia-run-2" ||
  [[ "${hashes[0]}" != "${hashes[1]}" || "${sizes[0]}" != "${sizes[1]}" ]]; then
  printf 'non-reproducible release binary: %s/%s bytes, %s/%s hashes\n' \
    "${sizes[0]}" "${sizes[1]}" "${hashes[0]}" "${hashes[1]}" >&2
  exit 1
fi

mkdir -p "$(dirname "$output")"
temporary_output="$output.tmp"
printf '{\n  "schema": 1,\n  "result": "pass",\n  "target": "x86_64-unknown-linux-gnu",\n  "commit": "%s",\n  "source_date_epoch": %s,\n  "sha256": "%s",\n  "size": %s,\n  "cold_build_seconds": [%s, %s]\n}\n' \
  "$commit" "$source_date_epoch" "${hashes[0]}" "${sizes[0]}" "${durations[0]}" "${durations[1]}" >"$temporary_output"
mv -f "$temporary_output" "$output"
printf 'PASS: two cold release builds are byte-identical (%s bytes, sha256 %s)\n' \
  "${sizes[0]}" "${hashes[0]}"
