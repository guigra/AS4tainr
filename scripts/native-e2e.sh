#!/usr/bin/env bash
# The 5250 end-to-end tests under both RPG engines (docs/native/adr/0005,
# interactive programs): tools/e2e_5250.py driven through the browser
# terminal (tools/e2e_5250_web.py, Playwright), as CI runs it with the
# tn5250 client.
#
#   1. Interpreter run: a new database with the samples, the 5250 server
#      with AS4TAINR_NATIVE_EMIT, the tests. Every RPG program the
#      sessions run is generated.
#   2. Native build: the same code built with AS4TAINR_NATIVE_DIR.
#   3. Native run: a new database, the server under AS4TAINR_RPG_ENGINE=rust,
#      the same tests. Both runs must pass with the same output, and each
#      program refused must be one with no generated code (a source that
#      differs by run).
#
# Environment: AS4TAINR_TEST_DATABASE_URL (any database of the server: new
# ones are made next to it), PYTHON (with playwright; default python3),
# PLAYWRIGHT_BROWSERS_PATH, NATIVE_E2E_OUT (default
# target/evidence/<date>-native-e2e).
set -u
cd "$(dirname "$0")/.."
here=$PWD
db=${AS4TAINR_TEST_DATABASE_URL:?set AS4TAINR_TEST_DATABASE_URL}
py=${PYTHON:-python3}
out=${NATIVE_E2E_OUT:-target/evidence/$(date -u +%F)-native-e2e}
case $out in /*) ;; *) out=$here/$out ;; esac
mkdir -p "$out"
: > "$out/status.tsv"
emit=$out/emit
rm -rf "$emit"
mkdir -p "$emit"
fail=0
step() { # name status detail
  printf '%s\t%s\t%s\n' "$1" "$2" "$3" >> "$out/status.tsv"
  echo "native-e2e: $1: $2 $3"
  [ "$2" = PASS ] || fail=1
}
newdb() {
  local name="$1" admin="${db%/*}/postgres"
  psql "$admin" -qc "DROP DATABASE IF EXISTS $name" > /dev/null &&
    psql "$admin" -qc "CREATE DATABASE $name" > /dev/null &&
    echo "${db%/*}/$name"
}
wait_port() { # port: until it accepts connections (60 s)
  for _ in $(seq 120); do
    (exec 3<> "/dev/tcp/127.0.0.1/$1") 2> /dev/null && return 0
    sleep 0.5
  done
  return 1
}

# One run: engine, binary, label. Server on 2323 (5250) and 5250 (web).
run() {
  local engine=$1 bin=$2 label=$3 url pid r
  url=$(newdb "native_e2e_$label") || { step "$label" FAIL "no database"; return; }
  AS4TAINR_DATABASE_URL="$url" AS4TAINR_BIN="$bin" scripts/samples.sh > "$out/$label-samples.log" 2>&1 ||
    { step "$label" FAIL "samples.sh failed, see $label-samples.log"; return; }
  env AS4TAINR_DATABASE_URL="$url" AS4TAINR_WAITRCD=5 AS4TAINR_PGM_PATH="$here/samples/qrpglesrc" \
    AS4TAINR_RPG_ENGINE="$engine" ${4:+AS4TAINR_NATIVE_EMIT="$emit"} \
    AS4TAINR_NATIVE_REFUSALS="$out/$label.refusals" \
    "$bin" tn5250 --listen 127.0.0.1:2323 --web-listen 127.0.0.1:5250 > "$out/$label-server.log" 2>&1 &
  pid=$!
  if ! wait_port 2323 || ! wait_port 5250; then
    kill "$pid" 2> /dev/null
    step "$label" FAIL "the server did not start, see $label-server.log"
    return
  fi
  WEB=http://127.0.0.1:5250 "$py" tools/e2e_5250_web.py tools/e2e_5250.py 127.0.0.1:2323 > "$out/$label.log" 2>&1
  r=$?
  kill "$pid" 2> /dev/null
  wait "$pid" 2> /dev/null
  if [ $r -eq 0 ]; then
    step "$label" PASS "$(grep -c '^ok ' "$out/$label.log") checks"
  else
    step "$label" FAIL "exit $r, see $label.log"
  fi
}

# 1. Interpreter run, generating.
run interp target/debug/as4tainr interp emit
n=$(find "$emit" -name '*.rs' | wc -l)
echo "native-e2e: generated $n routines"

# 2. Native build.
if CARGO_TARGET_DIR=target/native-e2e AS4TAINR_NATIVE_DIR="$emit" cargo build --locked > "$out/build.log" 2>&1; then
  step build PASS "with $n routines"
else
  step build FAIL "see build.log"
fi

# 3. Native run.
touch "$out/rust.refusals"
run rust target/native-e2e/debug/as4tainr rust
if diff "$out/interp.log" "$out/rust.log" > "$out/e2e.diff"; then
  step same-output PASS "the two runs printed the same"
else
  step same-output FAIL "see e2e.diff"
fi
bad=0
while IFS=$'\t' read -r _engine program fp reason; do
  case "$reason" in
    "no native code for"*) [ -e "$emit/$fp.rs" ] && { echo "$program $fp: refused but generated" >> "$out/rust.unexplained"; bad=1; } ;;
    *) echo "$program $fp: $reason" >> "$out/rust.unexplained"; bad=1 ;;
  esac
done < "$out/rust.refusals"
if [ $bad -eq 0 ]; then
  step refusals PASS "$(cut -f2 "$out/rust.refusals" | sort -u | wc -l) programs refused, none generated"
else
  step refusals FAIL "see rust.unexplained"
fi
(cd "$out" && sha256sum ./*.log ./*.refusals 2> /dev/null) > "$out/logs.sha256"
echo "native-e2e: logs in $out"
exit $fail
