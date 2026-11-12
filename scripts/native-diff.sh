#!/usr/bin/env bash
# The differential harness of the native RPG backend
# (docs/native/adr/0005): the same programs under the interpreter and
# under generated code, compared. Any difference, error or level that did
# not run fails the script.
#
#   1. Generation run: the whole suite (cargo test --no-fail-fast) and
#      scripts/demo.sh under the interpreter, with AS4TAINR_NATIVE_EMIT:
#      the per-test results are the reference, and every program they run
#      is generated.
#   2. Native build: the same code built with AS4TAINR_NATIVE_DIR.
#   3. Level 1: core/tests/native_diff.rs, the corpus run under both
#      engines in one process (storage, indicators, files, output). It
#      must run, not be skipped.
#   4. Level 2: the whole suite under AS4TAINR_RPG_ENGINE=auto: programs
#      with generated code run on it, the others in the interpreter. Every
#      test's result must be the same as in the generation run.
#      Then under =rust, to measure coverage: each program refused must
#      be one whose generated code does not exist (its source changes from
#      one process to the next, as tests that put their random library
#      name in the RPG text do), never a difference in behavior.
#   5. Level 3: scripts/demo.sh under =rust on a new database: the same
#      output as under the interpreter, matching the golden files.
#      tools/golden.py when captures exist in target/ibmi/golden.
#
# Env:
#   AS4TAINR_TEST_DATABASE_URL  PostgreSQL URL for the tests (required)
#   NATIVE_DIFF_OUT             directory for logs (default target/evidence/<date>-native-diff-<arch>)
#   CARGO_BUILD_JOBS            as usual (1 on small machines)
set -uo pipefail

here="$(cd "$(dirname "$0")/.." && pwd)"
cd "$here"
db="${AS4TAINR_TEST_DATABASE_URL:?set AS4TAINR_TEST_DATABASE_URL}"
arch="$(uname -m)"
out="${NATIVE_DIFF_OUT:-target/evidence/$(date -u +%F)-native-diff-$arch}"
emit="$here/target/rpg-native/diff-$arch"
mkdir -p "$out"
if [ -s "$out/status.tsv" ]; then
  echo "native-diff: $out has a recorded run; give another NATIVE_DIFF_OUT" >&2
  exit 2
fi
: > "$out/status.tsv"
rm -rf "$emit"
export CARGO_INCREMENTAL="${CARGO_INCREMENTAL:-0}"
fail=0

step() { # name status detail
  printf '%s\t%s\t%s\n' "$1" "$2" "$3" >> "$out/status.tsv"
  echo "native-diff: $1: $2 $3"
  [ "$2" = PASS ] || fail=1
}

results() { # log -> sorted "test NAME ... RESULT" lines
  grep -E '^test .* \.\.\. (ok|FAILED|ignored)$' "$1" | sort
}

# A new database for demo.sh, as upstream CI does.
newdb() {
  local name="$1" admin="${db%/*}/postgres"
  psql "$admin" -qc "DROP DATABASE IF EXISTS $name" > /dev/null &&
    psql "$admin" -qc "CREATE DATABASE $name" > /dev/null &&
    echo "${db%/*}/$name"
}

demo_out() { # log -> the programs' output (the setup lines differ by design;
  # today's date is normalized as demo.sh does for its golden files)
  grep -vE '^(Loaded |File .* already|CP[CF][0-9A-F]{4})' "$1" |
    sed -e 's/[[:space:]]*$//' -e 's#[ 0-9][0-9]/[0-9][0-9]/[0-9][0-9]#MM/DD/YY#g'
}

# 1. Generation run.
AS4TAINR_TEST_DATABASE_URL="$db" AS4TAINR_NATIVE_EMIT="$emit" AS4TAINR_RPG_ENGINE=interp \
  cargo test --locked --no-fail-fast > "$out/1-interp.log" 2>&1
echo "exit=$?" >> "$out/1-interp.log"
results "$out/1-interp.log" > "$out/1-interp.res"
url=$(newdb native_diff_demo_interp) || { step demo-interp FAIL "no database"; }
AS4TAINR_NATIVE_EMIT="$emit" AS4TAINR_RPG_ENGINE=interp AS4TAINR_DATABASE_URL="$url" \
  AS4TAINR_BIN=target/debug/as4tainr scripts/demo.sh > "$out/1-demo-interp.log" 2>&1
if [ $? -eq 0 ]; then step demo-interp PASS "$(grep -c '^PASS' "$out/1-demo-interp.log") cases"; else step demo-interp FAIL "see 1-demo-interp.log"; fi
n=$(find "$emit" -name '*.rs' 2>/dev/null | wc -l)
if [ "$n" -gt 0 ]; then step generate PASS "$n routines"; else step generate FAIL "nothing generated"; fi

# 2. Native build (the same target directory: only rpgnative and what
# depends on it is built again).
AS4TAINR_NATIVE_DIR="$emit" cargo build --locked > "$out/2-build.log" 2>&1 &&
  AS4TAINR_NATIVE_DIR="$emit" cargo test --locked --no-run >> "$out/2-build.log" 2>&1
if [ $? -eq 0 ]; then step build PASS "with $n routines"; else step build FAIL "see 2-build.log"; fi

# 3. Level 1.
AS4TAINR_NATIVE_DIR="$emit" AS4TAINR_TEST_DATABASE_URL="$db" \
  cargo test --locked -p as4tainr-core --test native_diff -- --nocapture > "$out/3-level1.log" 2>&1
r=$?
if [ $r -eq 0 ] && ! grep -q 'skipped:' "$out/3-level1.log" && grep -q '^test native_diff_corpus \.\.\. ok' "$out/3-level1.log"; then
  step level1 PASS "corpus identical under both engines"
else
  step level1 FAIL "see 3-level1.log"
fi

# 4. Level 2: auto (parity), then rust (coverage).
AS4TAINR_NATIVE_DIR="$emit" AS4TAINR_TEST_DATABASE_URL="$db" AS4TAINR_RPG_ENGINE=auto \
  AS4TAINR_NATIVE_REFUSALS="$here/$out/4-auto.refusals" \
  cargo test --locked --no-fail-fast > "$out/4-auto.log" 2>&1
echo "exit=$?" >> "$out/4-auto.log"
results "$out/4-auto.log" > "$out/4-auto.res"
if diff "$out/1-interp.res" "$out/4-auto.res" > "$out/4-auto.diff"; then
  step level2-auto PASS "$(wc -l < "$out/4-auto.res") tests, same results as the interpreter"
else
  step level2-auto FAIL "see 4-auto.diff"
fi
AS4TAINR_NATIVE_DIR="$emit" AS4TAINR_TEST_DATABASE_URL="$db" AS4TAINR_RPG_ENGINE=rust \
  AS4TAINR_NATIVE_REFUSALS="$here/$out/4-rust.refusals" \
  cargo test --locked --no-fail-fast > "$out/4-rust.log" 2>&1
echo "exit=$?" >> "$out/4-rust.log"
results "$out/4-rust.log" > "$out/4-rust.res"
total=$(wc -l < "$out/4-rust.res")
same=$(comm -12 "$out/1-interp.res" "$out/4-rust.res" | wc -l)
touch "$out/4-rust.refusals"
bad=0
while IFS=$'\t' read -r _engine program fp reason; do
  case "$reason" in
    "no native code for"*) [ -e "$emit/$fp.rs" ] && { echo "$program $fp: refused but generated" >> "$out/4-rust.unexplained"; bad=1; } ;;
    *) echo "$program $fp: $reason" >> "$out/4-rust.unexplained"; bad=1 ;;
  esac
done < "$out/4-rust.refusals"
refused=$(cut -f2 "$out/4-rust.refusals" | sort -u | wc -l)
if [ $bad -eq 0 ]; then
  step level2-rust PASS "$same of $total tests as under the interpreter; $refused programs refused, none generated (source differs by process)"
else
  step level2-rust FAIL "see 4-rust.unexplained"
fi

# 5. Level 3.
url=$(newdb native_diff_demo_rust) || { step demo-rust FAIL "no database"; }
AS4TAINR_RPG_ENGINE=rust AS4TAINR_DATABASE_URL="$url" AS4TAINR_BIN=target/debug/as4tainr \
  scripts/demo.sh > "$out/5-demo-rust.log" 2>&1
r=$?
if [ $r -eq 0 ] && diff <(demo_out "$out/1-demo-interp.log") <(demo_out "$out/5-demo-rust.log") > "$out/5-demo.diff"; then
  step level3-demo PASS "$(grep -c '^PASS' "$out/5-demo-rust.log") cases, the same output as the interpreter"
else
  step level3-demo FAIL "see 5-demo-rust.log, 5-demo.diff"
fi
if [ -d target/ibmi/golden ]; then
  AS4TAINR_RPG_ENGINE=rust AS4TAINR_BIN=target/debug/as4tainr python3 tools/golden.py target/ibmi/golden > "$out/5-golden.log" 2>&1
  if [ $? -eq 0 ]; then step level3-golden PASS "see 5-golden.log"; else step level3-golden FAIL "see 5-golden.log"; fi
else
  printf 'level3-golden\tNOT_RUN\tno captures in target/ibmi/golden\n' >> "$out/status.tsv"
  echo "native-diff: level3-golden: NOT_RUN (no captures in target/ibmi/golden)"
fi

(cd "$out" && sha256sum ./*.log ./*.res ./*.refusals 2>/dev/null) > "$out/logs.sha256"
echo "native-diff: logs in $out"
exit $fail
