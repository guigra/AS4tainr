#!/usr/bin/env bash
# Capture golden outputs from a real IBM i (PUB400) over SSH: the Linux
# and macOS counterpart of golden.ps1, with the same lists and the same
# results. See docs/golden-outputs.md.
#
# Usage: tools/golden/golden.sh [-l LIST] [-o OUT] [-h HOST] [-t TRIES]
#                               [-T TIMEOUT] [-f FILES_DIR] [-F FILES_TO] [-n]
# -n prints the remote script and stops (nothing is sent).
#
# A list has one "name|CL command" per line (run with PASE `system`,
# stdout and stderr together), or "name|!shell line" (stderr lines
# prefixed "2> "); '#' lines are comments. Each result goes to
# OUT/<name>.txt, then "rc=N". The outputs hold IBM's message texts:
# they stay in target/ (never committed).
#
# Lists written for any PUB400 account use @LIB@ (a library of the
# account) and @HOME@ (its home directory), replaced from GOLDEN_LIB and
# GOLDEN_HOME; FILES_TO is then @HOME@/gld unless -F says otherwise.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
list="$here/basics.txt"
out="$here/../../target/ibmi/golden"
host=pub400
tries=3
timeout=120
files=""
files_to="${GOLDEN_HOME:+$GOLDEN_HOME/gld}"
files_to="${files_to:-/home/AS4TRAINR/gld}"
dry=0
while getopts "l:o:h:t:T:f:F:n" o; do
  case $o in
    l) list=$OPTARG ;; o) out=$OPTARG ;; h) host=$OPTARG ;; t) tries=$OPTARG ;;
    T) timeout=$OPTARG ;; f) files=$OPTARG ;; F) files_to=$OPTARG ;; n) dry=1 ;;
    *) sed -n '2,15p' "$0"; exit 2 ;;
  esac
done
mkdir -p "$out"
logs="$(dirname "$out")"
id="golden_$(od -An -N4 -tx1 /dev/urandom | tr -d ' \n')"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
mkdir "$work/$id"

# The remote script: one sh for all commands (PUB400 drops many new
# connections). Each command gets an empty stdin (`system` would read the
# rest of the script) and runs in the background under a watchdog that
# kills it after the timeout ("TIMEOUT after N s" in its output).
{
  if [ -n "$files" ]; then
    for f in "$files"/*; do
      [ -f "$f" ] || continue
      cp "$f" "$work/$id/"
      b=$(basename "$f")
      printf "mkdir -p '%s' && cp '/tmp/%s/%s' '%s/'\n" "$files_to" "$id" "$b" "$files_to"
      # A copied file is tagged CCSID 1208, which PUB400's compilers do not
      # open (RNS9339); a source that is plain ASCII is tagged 819 instead.
      if ! LC_ALL=C grep -q '[^ -~\t]' "$f"; then
        printf "setccsid 819 '%s/%s'\n" "$files_to" "$b"
      fi
    done
  fi
  n=0
  while IFS= read -r line || [ -n "$line" ]; do
    [[ "$line" =~ [^[:space:]] ]] || continue
    [[ "$line" =~ ^[[:space:]]*# ]] && continue
    name="${line%%|*}"; cmd="${line#*|}"
    name="$(echo "$name" | xargs)"; cmd="$(echo "$cmd" | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//')"
    if [[ "$cmd" == *@LIB@* || "$cmd" == *@HOME@* ]]; then
      : "${GOLDEN_LIB:?the list uses @LIB@: set GOLDEN_LIB}" "${GOLDEN_HOME:?the list uses @HOME@: set GOLDEN_HOME}"
      cmd="${cmd//@LIB@/$GOLDEN_LIB}"; cmd="${cmd//@HOME@/$GOLDEN_HOME}"
    fi
    n=$((n + 1))
    echo "echo '=====BEGIN $name'"
    if [ "${cmd:0:1}" = "!" ]; then
      echo "( ${cmd:1} ) </dev/null >/tmp/golden_out.\$\$ 2>/tmp/golden_err.\$\$ &"
    else
      q=${cmd//\'/\'\\\'\'}
      echo "( system '$q' ) </dev/null >/tmp/golden_out.\$\$ 2>&1 &"
    fi
    echo "p=\$!; rm -f /tmp/golden_to.\$\$; ( sleep $timeout; kill -9 \$p && echo x >/tmp/golden_to.\$\$ ) </dev/null >/dev/null 2>&1 &"
    echo "w=\$!; wait \$p; rc=\$?; kill \$w 2>/dev/null"
    echo "cat /tmp/golden_out.\$\$; [ -f /tmp/golden_to.\$\$ ] && echo 'TIMEOUT after $timeout s'; echo \"rc=\$rc\""
    [ "${cmd:0:1}" = "!" ] && echo "sed 's/^/2> /' /tmp/golden_err.\$\$"
    echo "rm -f /tmp/golden_out.\$\$ /tmp/golden_err.\$\$ /tmp/golden_to.\$\$"
    echo "echo '=====END $name'"
  done < "$list"
  echo "rm -rf /tmp/$id; echo '=====DONE'"
} > "$work/$id/golden.sh"
count=$(grep -c "^echo '=====BEGIN " "$work/$id/golden.sh" || true)
if [ $dry = 1 ]; then cat "$work/$id/golden.sh"; exit 0; fi

# One scp for the script and the files, one ssh for the run.
ok=0
for t in $(seq "$tries"); do
  if scp -r -q -o ConnectTimeout=30 "$work/$id" "$host:/tmp/" 2>/dev/null; then ok=1; break; fi
  echo "copying the script failed (try $t); retrying in 60 s"; sleep 60
done
[ $ok = 1 ] || { echo "could not reach the IBM i (ssh port); try again later" >&2; exit 1; }
echo "running $count commands"
done_ok=0
for t in $(seq "$tries"); do
  log="$logs/attempt$t"
  set +e
  ssh -o ConnectTimeout=30 -o ServerAliveInterval=30 "$host" "sh /tmp/$id/golden.sh" 2> "$log.err" |
    tee "$log.out" | while IFS= read -r l; do
      case "$l" in '=====BEGIN '*) echo "running ${l#=====BEGIN }" ;; 'TIMEOUT after'*) echo "  $l" ;; esac
    done
  rc=${PIPESTATUS[0]}
  set -e
  echo "exit=$rc" >> "$log.err"
  if grep -qx '=====DONE' "$log.out"; then done_ok=1; break; fi
  echo "attempt $t incomplete (see $log.out/.err); retrying in 60 s"; sleep 60
done
[ $done_ok = 1 ] || { echo "no complete run" >&2; exit 1; }
# Split the run into one file per command.
awk -v out="$out" '
  /^=====BEGIN / { name = substr($0, 12); f = out "/" name ".txt"; printf "" > f; next }
  /^=====END /   { close(f); print "saved " name; name = ""; next }
  name != ""     { print > f }
' "$log.out"
