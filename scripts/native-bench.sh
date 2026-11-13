#!/usr/bin/env bash
# A first performance baseline of the native RPG backend
# (CLAUDE.native.md section 5): the same program under the interpreter
# and under generated code, in a release build that has the program's
# generated code. Records wall time (p50/p95/p99 over RUNS calls), CPU
# time and maximum resident memory per call. Allocations and database
# round trips are not measured yet (NOT_DEMONSTRATED).
#
# Env: AS4TAINR_DATABASE_URL (required), RUNS (default 20),
#      NATIVE_BENCH_PGM (a program of samples/native, default NBENCH1),
#      NATIVE_BENCH_OUT (default target/evidence/<date>-native-bench-<arch>).
set -uo pipefail
here="$(cd "$(dirname "$0")/.." && pwd)"; cd "$here"
db="${AS4TAINR_DATABASE_URL:?set AS4TAINR_DATABASE_URL}"
runs="${RUNS:-20}"; arch="$(uname -m)"
out="${NATIVE_BENCH_OUT:-target/evidence/$(date -u +%F)-native-bench-$arch}"
emit="$here/target/rpg-native/bench-$arch"
mkdir -p "$out"; rm -rf "$emit"
export AS4TAINR_PGM_PATH="$here/samples/native"
pgm="${NATIVE_BENCH_PGM:-NBENCH1}"
# Generate the program's code with the interpreter, then build release with it.
cargo build --locked --release > "$out/build-interp.log" 2>&1 || { echo "build failed"; exit 1; }
target/release/as4tainr --database-url "$db" init > "$out/init.log" 2>&1 || { echo "init failed"; exit 1; }
AS4TAINR_NATIVE_EMIT="$emit" AS4TAINR_DATABASE_URL="$db" target/release/as4tainr call $pgm > "$out/emit.log" 2>&1 || { echo "emit run failed"; exit 1; }
AS4TAINR_NATIVE_DIR="$emit" cargo build --locked --release > "$out/build-native.log" 2>&1 || { echo "native build failed"; exit 1; }
bin=target/release/as4tainr
sha256sum "$bin" > "$out/binary.sha256"
python3 -I - "$out" "$bin" "$pgm" "$runs" "$db" <<'PY'
import sys, os, json, time, statistics, subprocess, pathlib
out, binary, pgm, runs, db = pathlib.Path(sys.argv[1]), sys.argv[2], sys.argv[3], int(sys.argv[4]), sys.argv[5]
res, outputs = {}, {}
for engine in ("interp", "rust"):
    env = dict(os.environ, AS4TAINR_RPG_ENGINE=engine, AS4TAINR_DATABASE_URL=db)
    # One call first, not measured (connections, caches); its output is kept.
    first = subprocess.run([binary, "call", pgm], env=env, capture_output=True, text=True)
    if first.returncode != 0:
        sys.exit(f"{engine}: the call failed: {first.stderr}")
    outputs[engine] = first.stdout
    (out / f"{engine}.output").write_text(first.stdout)
    wall, cpu, rss = [], [], []
    for _ in range(runs):
        t0 = time.perf_counter()
        child = subprocess.Popen([binary, "call", pgm], env=env,
                                 stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        _, status, use = os.wait4(child.pid, 0)  # this child's own usage
        t1 = time.perf_counter()
        child.returncode = os.waitstatus_to_exitcode(status)
        if child.returncode != 0:
            sys.exit(f"{engine}: a call failed")
        wall.append(t1 - t0)
        cpu.append(use.ru_utime + use.ru_stime)
        rss.append(use.ru_maxrss)  # KB on Linux
    wall.sort()
    q = lambda p: round(wall[min(len(wall) - 1, int(round(p * (len(wall) - 1))))], 4)
    res[engine] = {"runs": runs, "wall_p50_s": q(0.50), "wall_p95_s": q(0.95), "wall_p99_s": q(0.99),
                   "cpu_mean_s": round(statistics.mean(cpu), 4), "max_rss_kb": max(rss),
                   "allocations": "NOT_DEMONSTRATED", "db_round_trips": "NOT_DEMONSTRATED"}
if outputs["interp"] != outputs["rust"]:
    sys.exit("the engines' output differs")
(out / "summary.json").write_text(json.dumps(res, indent=2) + "\n")
print(json.dumps(res, indent=2))
PY
