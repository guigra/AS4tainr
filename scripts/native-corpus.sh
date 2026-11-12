#!/usr/bin/env bash
# An external corpus for the native RPG backend's differential harness
# (docs/native/adr/0005): every batch RPG source of a directory that
# declares no files runs with `as4tainr call` under the interpreter
# (generating its code), then, in a build with that code, under
# AS4TAINR_RPG_ENGINE=rust. Standard output and exit status must be the
# same. The corpus is read where it is and never copied into the
# repository (it may not be ours to vendor).
#
# This is parity, not truth: a corpus's own expected outputs come from
# another implementation, not from IBM i, and are not used.
#
# Env:
#   AS4TAINR_TEST_DATABASE_URL  PostgreSQL URL (required; a database of its own is made)
#   NATIVE_CORPUS               directory of .rpgle sources (default ../OpenRPG/tests)
#   NATIVE_CORPUS_OUT           log directory (default target/evidence/<date>-native-corpus-<arch>)
#   NATIVE_CORPUS_TIMEOUT       seconds per call (default 60)
set -uo pipefail
here="$(cd "$(dirname "$0")/.." && pwd)"; cd "$here"
db="${AS4TAINR_TEST_DATABASE_URL:?set AS4TAINR_TEST_DATABASE_URL}"
corpus="${NATIVE_CORPUS:-$here/../OpenRPG/tests}"
arch="$(uname -m)"
out="${NATIVE_CORPUS_OUT:-target/evidence/$(date -u +%F)-native-corpus-$arch}"
case "$out" in /*) ;; *) out="$here/$out" ;; esac
limit="${NATIVE_CORPUS_TIMEOUT:-60}"
emit="$here/target/rpg-native/corpus-$arch"
[ -d "$corpus" ] || { echo "native-corpus: no corpus at $corpus" >&2; exit 2; }
mkdir -p "$out/interp" "$out/rust"
if [ -s "$out/summary.tsv" ]; then echo "native-corpus: $out has a recorded run" >&2; exit 2; fi
rm -rf "$emit"
export CARGO_INCREMENTAL="${CARGO_INCREMENTAL:-0}"
admin="${db%/*}/postgres"
psql "$admin" -qc "DROP DATABASE IF EXISTS native_corpus" >/dev/null &&
  psql "$admin" -qc "CREATE DATABASE native_corpus" >/dev/null || { echo "no database" >&2; exit 2; }
url="${db%/*}/native_corpus"

# The programs: batch sources that declare no files (F specs or DCL-F),
# named as `as4tainr call` finds them in AS4TAINR_PGM_PATH.
list="$out/programs.txt"
for f in "$corpus"/*.rpgle; do
  [ -f "$f" ] || continue
  grep -qiE '^\s*dcl-f\b|^.{5}[Ff][A-Za-z]' "$f" && continue
  b=$(basename "$f" .rpgle)
  [ ${#b} -le 10 ] || continue
  echo "$b"
done | sort > "$list"
echo "native-corpus: $(wc -l < "$list") programs from $corpus"

run_all() { # engine bin dir
  local engine=$1 bin=$2 dir=$3
  while read -r p; do
    AS4TAINR_RPG_ENGINE=$engine AS4TAINR_PGM_PATH="$corpus" AS4TAINR_DATABASE_URL="$url" \
      timeout "$limit" "$bin" call "$p" > "$dir/$p.out" 2> "$dir/$p.err" < /dev/null
    echo $? > "$dir/$p.rc"
  done < "$list"
}

cargo build --locked > "$out/build-interp.log" 2>&1 || { echo "build failed" >&2; exit 1; }
target/debug/as4tainr --database-url "$url" init > "$out/init.log" 2>&1
AS4TAINR_NATIVE_EMIT="$emit" run_all interp target/debug/as4tainr "$out/interp"
AS4TAINR_NATIVE_DIR="$emit" cargo build --locked > "$out/build-native.log" 2>&1 || { echo "native build failed" >&2; exit 1; }
run_all rust target/debug/as4tainr "$out/rust"

: > "$out/summary.tsv"
same=0; differ=0; refused=0
while read -r p; do
  ri=$(cat "$out/interp/$p.rc"); rr=$(cat "$out/rust/$p.rc")
  if grep -q 'AS4TAINR_RPG_ENGINE=rust: program' "$out/rust/$p.err"; then
    s=REFUSED; refused=$((refused + 1))
  elif [ "$ri" = "$rr" ] && cmp -s "$out/interp/$p.out" "$out/rust/$p.out"; then
    s=SAME; same=$((same + 1))
  else
    s=DIFFER; differ=$((differ + 1))
  fi
  printf '%s\t%s\t%s\t%s\n' "$p" "$s" "$ri" "$rr" >> "$out/summary.tsv"
done < "$list"
echo "native-corpus: $same same, $differ differ, $refused refused (no native code) of $(wc -l < "$list")"
(cd "$out" && find . -type f ! -name logs.sha256 -print0 | sort -z | xargs -0 sha256sum) > "$out/logs.sha256"
[ "$differ" -eq 0 ]
