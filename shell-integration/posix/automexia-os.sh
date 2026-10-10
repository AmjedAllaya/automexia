#!/bin/sh
# Cached display identity only. Never source/evaluate os-release as shell code.
# Optional exact file/kernel arguments exercise this same parser in fixtures.
# Output is two data lines: OS name and version. No host/user information.
export LC_ALL=C
kernel=${2:-$(uname -s 2>/dev/null)}
case "$kernel" in
  Darwin) fallback=macOS ;;
  Linux|'') fallback=Linux ;;
  CYGWIN*|MSYS*|MINGW*) fallback=Windows ;;
  *) fallback=$kernel ;;
esac
file=${1:-/etc/os-release}
if [ "$#" -eq 0 ] && [ ! -e "$file" ]; then file=/usr/lib/os-release; fi
# os-release is Linux metadata; do not mistake a package manager's file on macOS
# for the host OS. Regular-file checks also exclude devices and named pipes.
if [ "$kernel" != Linux ] || [ ! -f "$file" ] || [ ! -r "$file" ]; then
  printf '%s\n\n' "$fallback"
  exit 0
fi
# Read at most 64 KiB + 1, even for malformed files. Reject a truncated record
# rather than interpreting its prefix. awk parses assignments, never executes them.
dd if="$file" bs=65537 count=1 2>/dev/null | awk -v fallback="$fallback" '
function value(raw,    q,c,n,i,out,nextc) {
  sub(/^[ \t]+/, "", raw); sub(/[ \t]+$/, "", raw)
  if (length(raw) > 1024 || raw ~ /[[:cntrl:]]/) return ""
  q = substr(raw,1,1); n = length(raw); out = ""
  if (q == "\047" || q == "\042") {
    if (n < 2 || substr(raw,n,1) != q) return ""
    for (i=2; i<n; i++) {
      c=substr(raw,i,1)
      if (c == q) return ""
      if (q == "\042" && c == "\\") {
        if (++i >= n) return ""
        nextc=substr(raw,i,1)
        if (nextc != "\\" && nextc != "\042" && nextc != "$" && nextc != "`") out=out c
        c=nextc
      }
      out=out c
    }
  } else {
    if (raw ~ /[ \t\047\042\\]/) return ""
    out=raw
  }
  if (length(out) > 256) return ""
  return out
}
{
  bytes += length($0) + 1
  if (bytes > 65536 || NR > 2048) { invalid=1; exit }
  if ($0 !~ /^(NAME|PRETTY_NAME|ID|VERSION_ID)=/) next
  key=$0; sub(/=.*/, "", key)
  raw=$0; sub(/^[^=]*=/, "", raw)
  fields[key]=value(raw)
}
END {
  name=fallback; version=""
  if (!invalid) {
    if (fields["NAME"] != "") name=fields["NAME"]
    else if (fields["PRETTY_NAME"] != "") name=fields["PRETTY_NAME"]
    else if (fields["ID"] ~ /^[a-z0-9._-]+$/) name=fields["ID"]
    version=fields["VERSION_ID"]
  }
  print name; print version
}'
