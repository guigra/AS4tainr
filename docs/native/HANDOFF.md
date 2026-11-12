# Native RPG backend: handoff

Last session: 2026-11-11 to 2026-11-12. Branches, stacked: `native/runtime-extraction` (rpgrt), then `native/engine` with everything after it. Nothing has been pushed.

## What changed

The commits, in order:

1. **`ee42b1a`, the `rpgrt` crate.** The run-time modules, moved unchanged.
2. **`8c9b6fd`, the plan.** PLAN.md and ADRs 0001-0005, accepted by the user after an independent review.
3. **`96bd6a7`, stage 1.** The engine flag `AS4TAINR_RPG_ENGINE`, `rpgcg`, the `rpgnative` registry, `native_rt` and the differential harness.
4. **`b0315e1`, PUB400 tooling and harness fixes.**
   - `tools/golden/golden.sh` and captures `native1`/`native2`, which confirm four interpreter defects.
   - Harness fixes: the level-2 rust coverage check, and `init` in the bench.
   - `scripts/native-corpus.sh`.
5. **`769f0d5`, S2.1 and S2.2.** Conditions, FOR/DO control and numeric EVAL as Rust.
6. **`a711dd2`, S2.3 and S2.4.** Fixed-form ADD/SUB/MULT/Z-ADD/Z-SUB and character fields as Rust, with captures `native3`/`native4`.
7. **Working tree, not committed: S2.5a.** The dispatcher finds a routine's entry with pointer comparisons and no allocation (`NativeTable::find`, `first_missing`). It was being checked on the VM (`devrun4`) when the session ended.

## Tests run

- **Section 7 checks** on clean trees of 96bd6a7 and 769f0d5, on x86_64 and aarch64: PASS, except clippy. Clippy fails on upstream code only (hostcmd.rs, hostdb.rs), as upstream main and its CI do. Manifests are in `docs/native/evidence/`.
- **`native-diff.sh` on 769f0d5:** PASS on x86_64. On aarch64, one intermittent upstream test failed under `auto` (`verify22`).
- **Development runs of S2.3/S2.4:** `native-diff.sh` PASS, and 385/385 OpenRPG test programs identical under both engines.
- **Performance, NBENCH1:** -14.6% on x86_64 and -17.8% on aarch64 with stage 2 on 769f0d5.

## Environment

- **Machines.** This host (x86_64) and `jsr-test-arm` (aarch64). On the VM, work under `/home/ctl/data`, the 100 GB disk.
- **PostgreSQL** 16 in podman (`as4tainr-test-db`), with an empty `/QSYS.LIB` on both machines.
- **Here,** run long jobs with `CARGO_BUILD_JOBS=1` and `setsid nohup`, because the session kills background shells when memory runs low.
- **PUB400:** user GUIGRA26, library GUIGRA261, ssh alias `pub400` (key `~/.ssh/id_pub400`). See `docs/native/pub400.md`.
- **External corpora, never vendored:** `../OpenRPG/tests`, for parity only (its `.out` files are not IBM i output; do not read OpenRPG's `src/`), and `../OrderDesk`, an interactive application, not used yet.

## Failures you can reproduce

- **Clippy** `chunks_exact_to_as_chunks` at core/src/hostcmd.rs:163 and hostdb.rs:657: upstream.
- **Intermittent upstream tests when the suite runs in parallel:** `save_changed_objects` and `verify22_joblog_info_in_a_system_job`.
- **A panic in rpgrt/src/rpg/data.rs:184** with OpenRPG's `test194_fixed_cspec_err_move_float.rpgle`, in both engines: upstream code, moved unchanged.

## Limits

- Stage 2 lowers only global fields of the main program. File fields are excluded until their buffer numbering is stable: a one-line sort in program.rs:2682, which needs the user's approval. Procedures still run on stage 1.
- `native-tools` (production emission for a catalog) is not written.
- The upstream reports are pending: the four defects in fidelity.md and the findings above. The user decided to send them when the work is finished.
- The user's fork `origin` (guigra/AS4tainr) has a newer main. Rebase both branches on it (rebase, not merge), then run the checks again.

## Next step

1. Finish checking S2.5a (`devrun4` on the VM: native-diff and bench), then commit it.
2. S2.5b: EXSR and the subroutine's code without going back through `run_code_at`. After that, lower DIV and MVR (they need the remainder), then MOVE/MOVEL.
3. Ask the user to approve the program.rs sort, so that file fields can be lowered.
4. At the end: rebase on origin/main, run the checks again, then send the upstream reports.
