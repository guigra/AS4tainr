#!/usr/bin/env bash
# How far the generated code of a directory is from a fully native engine
# (docs/native/adr/0007, phase A): every instruction of every routine,
# from the source maps (<fingerprint>.map.json), by how it runs:
#
#   native    its code never calls the interpreter;
#   fallback  lowered, but run by the interpreter when the runtime says
#             the context is not one the lowered code handles (native_ok);
#   interp    always run by the interpreter (stage 1).
#
# Phase A ends when "fallback" and "interp" are 0. Every routine also
# still settles errors through the interpreter (Runtime::settle); that is
# counted as one line, not per instruction.
#
#   scripts/native-coverage.sh DIR [OUT]
#
# Prints the totals and the instructions not yet native by kind and
# statement; with OUT, also writes them there as TSV.
set -u
dir=${1:?usage: native-coverage.sh DIR [OUT]}
out=${2:-}
python3 -I - "$dir" "$out" <<'PY'
import collections, glob, json, os, sys
d, out = sys.argv[1], sys.argv[2]
via = collections.Counter()
todo = collections.Counter()
routines = 0
old = 0
for p in sorted(glob.glob(os.path.join(d, "*.map.json"))):
    with open(p, encoding="utf-8") as f:
        m = json.load(f)
    routines += 1
    for i in m.get("instructions", []):
        v = i.get("via")
        if v is None:
            old += 1
            v = "unknown"
        via[v] += 1
        if v != "native":
            todo[(v, i.get("kind", ""), i.get("what", "").split(" ")[0])] += 1
total = sum(via.values())
print(f"native-coverage: {routines} routines, {total} instructions")
for k in ("native", "fallback", "interp", "unknown"):
    if via[k]:
        pct = 100.0 * via[k] / total if total else 0.0
        print(f"  {k:9} {via[k]:7}  {pct:5.1f}%")
if old:
    print(f"  ({old} instructions from source maps without \"via\": generated before rpgcg 0.1.0+stage2.34.1)")
print(f"  settle    {routines} routines (errors routed through the interpreter)")
print("not native yet, by kind and statement:")
for (v, k, w), n in todo.most_common(40):
    print(f"  {n:6}  {v:8} {k:14} {w}")
if out:
    with open(out, "w", encoding="utf-8") as f:
        f.write("via\tkind\twhat\tcount\n")
        for (v, k, w), n in sorted(todo.items()):
            f.write(f"{v}\t{k}\t{w}\t{n}\n")
        for k in ("native", "fallback", "interp", "unknown"):
            f.write(f"{k}\t*\t*\t{via[k]}\n")
PY
