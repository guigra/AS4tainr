#!/usr/bin/env python3
"""Evidence manifests of the native backend (CLAUDE.native.md section 8).

For each commit of the native branch, docs/native/evidence/<date>-<commit>.json
records what validated it:
- each validation run whose snapshot is the commit's tree, or a later
  snapshot that contains it, checked with git;
- each step's exit status and the native-diff levels;
- the test counts, and the SHA-256 of every log.

The logs themselves stay outside Git, under target/evidence/vm/<machine>/<run>/,
copied from the machines that ran them. Nothing in a manifest is
interpreted: statuses are the runs' own exit codes and status.tsv rows.
What a commit says about a failure (a known upstream flake run again,
say) is quoted from its message, which is where that judgement was made.

A manifest already committed is never written again (section 8: a
recorded result is not overwritten).

Usage: scripts/native-manifest.py [--evidence target/evidence/vm]
                                  [--out docs/native/evidence] REV_RANGE
Example: scripts/native-manifest.py 5b6f0c1..HEAD
"""

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
from datetime import datetime, timezone

# What each machine is (CLAUDE.native.md section 8: platform).
MACHINES = {
    "local-x86_64": "x86_64, the development machine (2 CPUs, 3 GB)",
    "jsr-test-2": "x86_64 VM (2 CPUs, 3 GB)",
    "jsr-test-arm": "aarch64 VM, not emulated (2 CPUs, 3 GB)",
}

# The commands each kind of run executes, in order (vm-s7full.sh, the
# earlier s7 scripts, and the native-diff-only runs nd<N>).
S7_COMMANDS = [
    "cargo fmt --all -- --check",
    "cargo clippy --locked --all-targets -- -D warnings",
    "cargo clippy --locked --all-targets -- -D warnings -A clippy::chunks_exact_to_as_chunks",
    "cargo test --locked --no-fail-fast",
    "cargo test --locked --release --no-fail-fast",
    "cargo build --locked --release",
    "AS4TAINR_BIN=target/debug/as4tainr scripts/demo.sh (new database)",
]
ND_COMMAND = "AS4TAINR_RPG_ENGINE=rust scripts/native-diff.sh (new database)"

TREE_NOTE = (
    "the snapshot's tree plus the catalog::init lock fix "
    "(branch fix/init-lock-timeout, upstream issue #18), applied uncommitted "
    "so that the suite does not hang; nothing else"
)


def git(*args):
    return subprocess.run(
        ["git", *args], check=True, capture_output=True, text=True
    ).stdout


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def read_status(path):
    """status: one name=value line per step."""
    out = {}
    with open(path, encoding="utf-8", errors="replace") as f:
        for line in f:
            line = line.rstrip("\n")
            if "=" in line:
                k, v = line.split("=", 1)
                out.setdefault(k, v)
            elif line:
                out.setdefault("notes", []).append(line)
    return out


def read_tsv(path):
    rows = []
    with open(path, encoding="utf-8", errors="replace") as f:
        for line in f:
            parts = line.rstrip("\n").split("\t")
            if len(parts) >= 2:
                rows.append(
                    {"level": parts[0], "status": parts[1], "detail": "\t".join(parts[2:])}
                )
    return rows


def runs(evidence):
    """Every run directory with a status file naming its snapshot."""
    found = []
    for machine in sorted(os.listdir(evidence)):
        mdir = os.path.join(evidence, machine)
        if not os.path.isdir(mdir):
            continue
        for name in sorted(os.listdir(mdir)):
            d = os.path.join(mdir, name)
            st = os.path.join(d, "status")
            if not os.path.isfile(st):
                continue
            status = read_status(st)
            # The earlier s7 runs kept their snapshot beside the run, in
            # <run>.commit.
            side = d + ".commit"
            if "snapshot" not in status and os.path.isfile(side):
                with open(side, encoding="utf-8") as f:
                    status["snapshot"] = f.read().strip()
            snap = status.get("snapshot")
            if not snap:
                continue
            found.append(
                {
                    "machine": machine,
                    "name": name,
                    "dir": d,
                    "status": status,
                    "snapshot": snap,
                    "tree": git("rev-parse", snap + "^{tree}").strip(),
                    "finished": os.path.exists(os.path.join(d, "done")),
                }
            )
    return found


def snapshot_trees(snap, cache):
    """The trees of a snapshot and of every snapshot or commit before it."""
    if snap not in cache:
        out = git("rev-list", "--format=%T", "--max-count=400", snap)
        trees = [l for l in out.splitlines() if not l.startswith("commit ")]
        cache[snap] = trees
    return cache[snap]


def run_record(r, relation):
    d = r["dir"]
    st = dict(r["status"])
    notes = st.pop("notes", [])
    snap = st.pop("snapshot")
    kind = "native-diff only" if os.path.basename(d).startswith("nd") else "section 7"
    rec = {
        "machine": r["machine"],
        "platform": MACHINES.get(r["machine"], r["machine"]),
        "run": r["name"],
        "kind": kind,
        "snapshot": snap,
        "relation": relation,
        "tree": TREE_NOTE,
        "commands": ([] if kind == "native-diff only" else S7_COMMANDS)
        + ([ND_COMMAND] if "native-diff" in st else []),
        "exit_status": st,
        # Steps whose exit status is not 0, as recorded; the commit
        # message says why each was accepted.
        "nonzero_steps": sorted(k for k, v in st.items() if v.isdigit() and v != "0"),
    }
    if notes:
        rec["notes"] = notes
    done = os.path.join(d, "done")
    if os.path.exists(done):
        rec["finished_utc"] = datetime.fromtimestamp(
            os.path.getmtime(done), timezone.utc
        ).strftime("%Y-%m-%d %H:%M")
    else:
        rec["finished_utc"] = None
        rec["incomplete"] = "no done marker: the run did not finish"
    summary = os.path.join(d, "summary.txt")
    if os.path.isfile(summary):
        with open(summary, encoding="utf-8", errors="replace") as f:
            rec["test_results"] = [l.strip() for l in f if l.strip()]
    tsv = os.path.join(d, "native-diff", "status.tsv")
    if os.path.isfile(tsv):
        rec["native_diff"] = read_tsv(tsv)
    logs = {}
    for root, _, files in os.walk(d):
        for f in sorted(files):
            p = os.path.join(root, f)
            logs[os.path.relpath(p, d)] = sha256(p)
    rec["files_sha256"] = dict(sorted(logs.items()))
    rec["local_copy"] = os.path.relpath(d)
    return rec


def validation_text(body):
    """The commit message's paragraphs about its validation, quoted."""
    # From the first paragraph about it to the end, trailers left out:
    # what a commit says about a failure follows its first mention.
    paras = [p.strip() for p in body.split("\n\n") if p.strip()]
    pat = re.compile(r"(?i)section 7|validation|native-diff|level 2|s7f-|snapshot")
    start = next((i for i, p in enumerate(paras) if pat.search(p)), len(paras))
    trailer = re.compile(r"^(Signed-off-by|Co-Authored-By|Claude-Session):", re.M)
    keep = []
    for p in paras[start:]:
        lines = [l for l in p.splitlines() if not trailer.match(l)]
        if lines:
            keep.append("\n".join(lines))
    return keep


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--evidence", default="target/evidence/vm")
    ap.add_argument("--out", default="docs/native/evidence")
    ap.add_argument("range")
    a = ap.parse_args()

    all_runs = runs(a.evidence)
    cache = {}
    commits = git("rev-list", "--reverse", a.range).split()
    unvalidated = []
    for c in commits:
        tree = git("rev-parse", c + "^{tree}").strip()
        subject = git("log", "-1", "--format=%s", c).strip()
        body = git("log", "-1", "--format=%b", c)
        date = git("log", "-1", "--format=%ad", "--date=format:%Y-%m-%d", c).strip()
        # Runs whose snapshot is this tree; failing that, the first
        # snapshot (by finish time) that contains it.
        same = [r for r in all_runs if r["tree"] == tree]
        records = [run_record(r, "same tree as the commit") for r in same]
        # A complete validation has a native-diff result. On a machine
        # with no complete run of this tree (a run stopped or split, or
        # a machine that validated a later snapshot only), the first later
        # snapshot that contains it is taken.
        def complete(r):
            return r["finished"] and r["status"].get("native-diff", "").isdigit()

        covered = {r["machine"] for r in same if complete(r)}
        later = [
            r
            for r in all_runs
            if r not in same
            and r["machine"] not in covered
            and complete(r)
            and tree in snapshot_trees(r["snapshot"], cache)
        ]
        later.sort(key=lambda r: os.path.getmtime(os.path.join(r["dir"], "status")))
        seen = set()
        for r in later:
            if r["machine"] in seen:
                continue
            seen.add(r["machine"])
            records.append(
                run_record(r, "a later snapshot, whose history contains this commit's tree")
            )
        not_run = []
        for rec in records:
            for row in rec.get("native_diff", []):
                if row["status"] == "NOT_RUN":
                    not_run.append(f"{row['level']} ({rec['machine']}): {row['detail']}")
        not_run.append("5250 end-to-end tests (tools/e2e_5250.py): not part of these runs")
        manifest = {
            "topic": subject,
            "date": date,
            "branch": "native/engine",
            "commit": git("rev-parse", c).strip(),
            "tree": tree,
            "toolchain": "rust-toolchain.toml at the commit: "
            + " ".join(
                l.strip()
                for l in git("show", c + ":rust-toolchain.toml").splitlines()
                if l.startswith("channel")
            ),
            "validation_runs": records,
            "commit_message_on_validation": validation_text(body),
            "not_run": not_run,
            "generated_by": "scripts/native-manifest.py",
        }
        if not records:
            manifest["status"] = "NOT_DEMONSTRATED: no validation run found for this tree or a later one"
            unvalidated.append(c[:7] + " " + subject)
        os.makedirs(a.out, exist_ok=True)
        path = os.path.join(a.out, f"{date}-{c[:7]}.json")
        # A manifest already committed is a recorded result: never
        # written again (CLAUDE.native.md section 8); a new run gets a
        # manifest of its own.
        if git("ls-files", "--", path).strip():
            print(f"{path}: committed, left as it is")
            continue
        with open(path, "w", encoding="utf-8") as f:
            json.dump(manifest, f, indent=1, ensure_ascii=False)
            f.write("\n")
        print(f"{path}: {len(records)} run(s)")
    if unvalidated:
        print("NOT_DEMONSTRATED:", *unvalidated, sep="\n  ", file=sys.stderr)


if __name__ == "__main__":
    main()
