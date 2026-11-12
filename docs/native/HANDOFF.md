# Native RPG backend: handoff

Last session: 2026-11-11 to 2026-11-12. Branches, stacked on `origin/main` (f20febb, the same as `upstream/main`): `native/runtime-extraction` (e394b4e), then `native/engine` with everything after it. Nothing has been pushed.

## What changed

On `native/engine`, after the rebase:

1. **224a02b CLAUDE.native.md.** It existed only in the old local main and is kept here.
2. **e394b4e `rpgrt`.**
3. **f059ccd the plan and ADRs 0001-0005.**
4. **2805d6a stage 1.** The engine flag, generator, registry, runtime adapter and harness.
5. **6ed533a PUB400 tooling.** `tools/golden/golden.sh` and captures native1-2, which confirm four interpreter defects, plus harness fixes.
6. **35f4a8b, S2.1-S2.2.** Conditions, FOR/DO control, numeric EVAL.
7. **5aeb0d6, S2.3-S2.4.** Fixed-form ADD/SUB/MULT/Z-ADD/Z-SUB and character fields.
8. **c039cdb, evidence of stages 1-2.**
9. **6520c2a, S2.5a.** The dispatcher finds native code with no allocation.
10. **5790720, S2.6.** File fields, after the one-line sort in program.rs the user approved.
11. **4f8a89c, S2.7.** DIV and MVR.
12. **e6b7f5e, S2.8.** Lowered code inside procedures: locals, parameters by reference, the `native_ok` guard.
13. **This commit:** the evidence of 4f8a89c on both platforms, STATUS.md and this file.

The map from the old hashes to the new ones is in `docs/native/evidence/2026-11-12-rebased.json`.

## Tests run

- **4f8a89c,** on clean trees, on x86_64 and aarch64: the section 7 checks and `native-diff.sh` PASS on both. Clippy is the exception, and fails only on upstream code: hostcmd.rs, hostdb.rs and hostifs.rs, as upstream's CI does.
- **S2.8, on 498cdfe:** formal evidence on both platforms, in `docs/native/evidence/2026-11-12-s28.json`. Everything passes except clippy (upstream). On x86_64 the test suites failed once, on a database an interrupted run had left catalog rows in. They passed on a new database.

## Environment

- **Machines.** This host (x86_64) and `jsr-test-arm` (aarch64). On the VM, work under `/home/ctl/data`, the 100 GB disk.
- **PostgreSQL** 16 in podman (`as4tainr-test-db`), with an empty `/QSYS.LIB` on both machines.
- **Here,** run long jobs with `CARGO_BUILD_JOBS=1` and `setsid nohup`: the session kills background shells when memory runs low. Wait loops inside the session can be killed too; the jobs themselves survive.
- **PUB400:** user GUIGRA26, library GUIGRA261, ssh alias `pub400`. See `docs/native/pub400.md`.
- **External corpora, never vendored:** `../OpenRPG/tests` (parity only; do not read its `src/`) and `../OrderDesk` (interactive, not used yet).

## Failures you can reproduce

- **Clippy** `chunks_exact_to_as_chunks` in the upstream host servers: hostcmd.rs:367, hostdb.rs:657, hostifs.rs:217.
- **Intermittent upstream tests when the suite runs in parallel:** `save_changed_objects` and `verify22_joblog_info_in_a_system_job`.
- **Catalog rows in library QTEMP (literally) after an interrupted run.** They make `change_a_physical_file_with_a_logical_file_over_it` fail with CPF5813 on that database, and `qtemp::cleanup` at init does not remove them. Use a new database for evidence runs.
- **A panic in rpgrt/src/rpg/data.rs:184** with OpenRPG's `test194_fixed_cspec_err_move_float.rpgle`, in both engines: upstream code, moved unchanged.

## Limits

- Not lowered yet: BIFs, MOVE/MOVEL, file and SQL operations, calls, arrays, data structures as a whole, based storage, varying fields, dates, floats. These are runtime calls into the interpreter, as in stage 1.
- `native-tools` (production emission for a catalog) is not written.
- The upstream reports are pending, by the user's decision, until the work is finished:
  - the four defects in fidelity.md;
  - the panic in data.rs;
  - the intermittent tests;
  - clippy in the host servers;
  - the program.rs sort, which could be offered upstream as a determinism fix.

## Next step

2. Stage 2 goes on: the commonest BIFs (%LEN, %TRIM, %SUBST, %CHAR), using the `precision` pattern of asking the interpreter for static facts once; then MOVE/MOVEL; then arrays with constant indexes.
3. `native-tools`, and a production-style build of an application (OrderDesk) under `rust`.
4. At the end: rebase on origin/main again, run the checks, then send the upstream reports.
