# Native RPG backend: status

Where the native backend (RPG compiled to Rust, see `CLAUDE.native.md`) stands. Four levels are kept apart: **implemented**, **validated locally**, **contrasted with IBM i (golden)** and **fit for production**. One level never implies the next.

## Phases

The plan is in `PLAN.md` and the decisions in `adr/0001`-`0005`, all accepted by the user on 2026-11-11 after an independent review.

| Phase | What | Level |
| --- | --- | --- |
| Runtime extraction | Workspace crate `rpgrt` with the run-time modules moved unchanged from `as4tainr-core`, re-exported from their old paths | Implemented. Validated locally on x86_64 and aarch64, except clippy, which upstream `main` fails too (see "Checks" below). No behavior change, so nothing to contrast with IBM i |
| N0 Plan | `PLAN.md`, ADRs 0001-0005, CLAUDE.native.md brought in line | Done (user approval 2026-11-11) |
| N1 Flag and hooks | `AS4TAINR_RPG_ENGINE`, the hooks, `rpgrt::native`, the `rpgnative` registry | Implemented; validated locally on x86_64 and aarch64 (manifests below) |
| N2 Generator | `rpgcg`, lowering, fingerprints, emission | Implemented; validated locally |
| N3 Runtime adapter | `native_rt`: `Runtime` for `Machine`, `settle`, the dispatcher | Implemented; validated locally |
| N4 Differential harness | `core/tests/native_diff.rs`, `scripts/native-diff.sh`, `scripts/native-corpus.sh` (external corpora) | Implemented; validated locally |
| N5 Build tooling and baseline | `native-tools`, production builds, performance baseline | Baseline recorded (`scripts/native-bench.sh`, manifest of 769f0d5). `native-tools` (`as4tainr-native-emit`) is implemented: a release build with CONTACTS' generated code ran CONTCNT under `rust` with no cargo or rustc on PATH, same output as `interp` (docs/native/production.md; development check, no manifest yet) |
| N6… Stage 2 | Lowering, one construct per step | S2.1-S2.30 implemented (ADR 0006): conditions, FOR/DO control, numeric EVAL, fixed-form arithmetic incl. DIV/MVR, character fields, file fields, procedures (locals, parameters), %TRIM/%TRIML/%TRIMR, %LEN, %SUBST, fixed-form MOVE/MOVEL, %SCAN, %XLATE, conditions with functions, %CHAR of numbers, %INT, %DEC, array elements with literal indexes, qualified subfields, `/`, %REM, %DIV, varying fields and their %LEN, array elements with field indexes, figurative constants, %REPLACE, whole data structures, %SIZE and %ELEM, %EDITC, %LOOKUP, %SCANRPL, %EDITW, data structure array subfields (S2.10-S2.17 validated in development runs on aarch64: native-diff PASS, 639 tests the same under auto, OpenRPG 385/385; S2.18-S2.20 by the native units and native_diff level 1 on x86_64; S2.21 on aarch64: native-diff PASS, 649 tests the same, OpenRPG 385/385, 5250 end-to-end 165 checks the same; S2.22-S2.24 by the native units and native_diff level 1 on x86_64; S2.23-S2.28 on aarch64 (VM, development run): native-diff PASS, 656 tests the same under auto, 388 routines, 5250 end-to-end 165 checks the same, OpenRPG FAIL 384/385: test396 (P0269) panics in the interpreter's debug build at bifs.rs:573, the %SCANRPL defect in fidelity.md, and the native engine prints the test's expected output; S2.25 on aarch64 (Mac M1, podman): native units 34/34 under interp and rust, native-diff PASS, 654 tests the same under auto, 385 routines, with the upstream catalog::init lock fix applied uncommitted (evidence/2026-11-14-s225-aarch64-mac.json), not yet on x86_64; S2.26-S2.28 native units PASS on aarch64; S2.26-S2.30 on aarch64 (Mac M1, podman, at 8e212bd): native units 39/39 under interp and rust, native-diff PASS, 658 tests the same under auto, 390 routines, cargo test --release 657 passed 0 failed, release build PASS, with the upstream catalog::init lock fix applied uncommitted (evidence/2026-11-14-s230-aarch64-mac.json)); the dispatcher allocates nothing per entry |
| Final | Retiring the interpreter (the user's goal; prerequisites in PLAN.md) | Not started |

## Phase 1: the `rpgrt` crate

Moved with `git mv`, byte for byte (no line of these files changed):

| Old path (`as4tainr-core`) | New path | Still reachable as |
| --- | --- | --- |
| `core/src/rpg/dec.rs` | `rpgrt/src/rpg/dec.rs` | `as4tainr_core::rpg::dec` (`pub use rpgrt::rpg::dec`) |
| `core/src/rpg/data.rs` | `rpgrt/src/rpg/data.rs` | `as4tainr_core::rpg::data` |
| `core/src/rpg/mem.rs` | `rpgrt/src/rpg/mem.rs` | `as4tainr_core::rpg::mem` |
| `core/src/rpg/dates.rs` | `rpgrt/src/rpg/dates.rs` | `as4tainr_core::rpg::dates` |
| `core/src/tn5250/ebcdic.rs` | `rpgrt/src/tn5250/ebcdic.rs` | `as4tainr_core::tn5250::ebcdic` (`pub use rpgrt::tn5250::ebcdic`) |

The modules keep their paths inside `rpgrt` (`rpg::…`, `tn5250::…`), so the references between them (`super::dec`, `crate::rpg::dates`, `crate::tn5250::ebcdic`) resolve without editing them. `rpgrt` depends only on `anyhow` and `ethnum`, both already in `Cargo.lock` at the same versions; the lockfile gains only the `rpgrt` package.

`ebcdic.rs` is the generated CCSID 37 table with no imports. `data.rs` needs it; it lived under `tn5250` by accident, not by coupling.

### Not moved: `bifs.rs` and `ptrs.rs`

Both were asked for and are **not** runtime: they are private submodules of the interpreter (`interp.rs`: `mod bifs;`, `mod ptrs;`) made of `impl Machine` methods. `bifs.rs` evaluates AST expressions (`self.eval`, `self.special_arg`), reads open files' state (`self.files`, `self.io`) and calls the compiler (`compile_with`). `ptrs.rs` maps pointers onto the machine's buffers (`self.bufs`, `self.frame_owner`). `Machine` holds a `tokio_postgres::Client`, a `tokio::runtime::Handle`, `rla::OpenFile` and `workstn::Workstation`, so moving them would take SQL sessions, record-level access and the 5250 into the runtime.

Generated code will need their pure parts (date arithmetic, `%STR` over bytes...). Getting them out of `impl Machine` is a refactoring of `interp.rs`, outside the touch points of `CLAUDE.native.md` section 3: it needs the user's approval and probably an ADR.

### Lints

`rpgrt/src/lib.rs` forbids `unsafe_code` and `unused_must_use` for the whole crate. Section 5's clippy lints are **denied** at the crate root, not forbidden. A forbidden lint cannot be allowed again lower down, and code moved unchanged keeps upstream's lints (section 5; the user's instructions for phase 1). The moved modules allow those lints where they are declared (`rpgrt/src/rpg.rs`, `rpgrt/src/tn5250.rs`), with a comment saying why. Any new module in `rpgrt` must not allow them.

### Upstream files touched

- `Cargo.toml`: `rpgrt` added to the workspace members.
- `core/Cargo.toml`: the dependency on `rpgrt` (needed for the re-exports).
- `core/src/rpg/mod.rs`: `pub mod data/dates/dec/mem` became `pub use rpgrt::rpg::…`.
- `core/src/tn5250/mod.rs`: `pub mod ebcdic` became `pub use rpgrt::tn5250::ebcdic`.
- `Cargo.lock`: the `rpgrt` package.

### Tests

The unit tests in the moved files moved with them: they now run in `rpgrt`'s test binary, with the same names (`rpg::dec::tests::…`). On the branch and on its base the suite has the same 593 tests, and every test has the same result: `as4tainr_core`'s unit tests go from 340 to 326 and `rpgrt`'s 14 pass.

### Checks (CLAUDE.native.md section 7)

Run on this commit's code on x86_64 (2 CPUs, `CARGO_BUILD_JOBS=1`) and on the VM `jsr-test-arm` (aarch64, native). Both have PostgreSQL 16.15, and both have an empty `/QSYS.LIB` (see below).

| Check | x86_64 | aarch64 |
| --- | --- | --- |
| `cargo fmt --all -- --check` | PASS | PASS |
| `cargo clippy --locked --all-targets -- -D warnings` | FAIL (upstream) | FAIL (upstream) |
| `cargo test --locked` (with the database) | PASS (run 3; run 2 had one intermittent upstream failure) | PASS |
| `cargo test --locked --release` (with the database) | PASS | PASS |
| `cargo build --locked --release` | PASS | PASS |
| `scripts/demo.sh` | PASS | PASS (on a new database, as upstream CI) |
| `scripts/native-diff.sh` | NOT_RUN: does not exist yet | NOT_RUN |

Notes:

- **clippy.** `clippy::chunks_exact_to_as_chunks` fires at `core/src/hostcmd.rs:163` and `core/src/hostdb.rs:657`. Those lines are upstream host server code from `29f23f6` and `e0ac4d8`. Upstream's own CI fails the same way on `main` with rustc 1.99.1, the same toolchain as its last green run, so pinning the toolchain would not help. Allowing that lint alone, the branch has no other warning in any target. Section 2 says not to fix it here.
- **`/QSYS.LIB`.** `rla_test::qsh_commands_see_qsys_lib` needs the mountpoint `/QSYS.LIB`. QSH tries `mkdir -p /QSYS.LIB` in an unprivileged user namespace (`qshfs.rs`), which fails for a non-root user. as4tainr's container runs as root and makes it (docs/qshell.md). The test hosts got an empty `/QSYS.LIB` made with `sudo mkdir`. No test or code changed.
- **`save_changed_objects`.** It failed once out of six runs (x86_64 run 2) and then passed alone and in the full suite. This is an intermittent upstream failure in `save.rs`. The likely cause, which is not proven: SAVLIB's reference point is the server-wide `pg_snapshot_xmin`, and a concurrent test's open transaction holds it back.
- **`demo.sh`** keeps the sample data a previous run left in the same database and then differs. Upstream CI runs it on a new database, and so do these runs.

## Stage 1 (N1-N4)

- **Engine flag.** `AS4TAINR_RPG_ENGINE` is read once per process. Tests choose an engine per thread with `native::scope`/`with_engine`, carried into `spawn_blocking`.
  - With the flag unset or `interp`, the dispatcher returns at once.
  - Under `rust`, a program without generated code for every routine ends before its first calculation. The error names the variable, the program, the routine and its fingerprint. Its *PSSR does not run.
  - Under `auto`, such a program runs in the interpreter and is named once on standard error.
- **Generation.** `AS4TAINR_NATIVE_EMIT=<dir>` writes `<fingerprint>.rs` and `.map.json` for every routine of every program compiled (`compile_ovr`). `rpgcg` makes a `loop { match pc }` per routine; its comments give the RPG line of each instruction.
- **Registry.** Building with `AS4TAINR_NATIVE_DIR=<dir>` includes the generated modules in `rpgnative`, under section 5's forbidden lints. `Cargo.lock` does not change.
- **Runtime.** `core/src/rpg/native_rt.rs` is `Machine` as `rpgrt::native::Runtime`: each method is the matching arm of `run_code_at`, and `settle` is its error routing. `core/tests/native_tripwire.rs` fails if upstream changes `run_code_at` or the error functions it copies.
- **Tests:**
  - `core/tests/native_engine.rs`: the flag;
  - `core/tests/native_units.rs`: 9 RPG programs with expected values from IBM Docs, run on whichever engine the process uses;
  - `core/tests/native_diff.rs`: the level-1 corpus;
  - the unit tests of `rpgcg`, `rpgnative` and `rpgrt::native`.
- **Harness.** `scripts/native-diff.sh` runs the three levels of ADR 0005. With `auto`, every test's result must be the same as under the interpreter. With `rust`, coverage is measured: every refusal must be a program whose generated code never existed (sources that put the test's random library name in the RPG text).
- **Upstream changes.** 5 lines in interp.rs, 3 in program.rs, a few in rpg/mod.rs (emission, the engine carried into `spawn_blocking`, observation), and the workspace and core manifests. All are authorized (CLAUDE.native.md section 3).
- **Interpreter defects found**, upstream and not fixed here: nested MONITOR, writes to CONST parameters, `OVERLAY(:*NEXT)`. They are in `fidelity.md`.

## Stage 2 and results so far

- **Interactive programs.** `scripts/native-e2e.sh` runs upstream's 5250 end-to-end tests (`tools/e2e_5250.py`, through the browser terminal with Playwright). They run once under the interpreter, generating code, and once under `rust`. First run, aarch64, S2.13 development tree, 2026-11-12: 165 checks passed under both engines, with the same output and 0 programs refused (15 routines generated).

- **Parity.** On commit 4f8a89c (rebased on the current upstream, f20febb), `native-diff.sh` passes every level on x86_64 and on aarch64: 626 tests with the same results under `auto`, and the corpus identical. In the development runs of S2.3 to S2.10, the 385 programs of OpenRPG's test corpus that run without files give the same output and exit status under both engines.
- **Speed.** NBENCH1 (`samples/native/NBENCH1.rpgle`), release build, p50 over 20 calls:

  | Platform | Interpreter | Native | Change | Commit |
  | --- | --- | --- | --- | --- |
  | x86_64 | 3.97 s | 3.07 s | -22.6% | 4f8a89c (S2.1-S2.7) |
  | aarch64 | 2.01 s | 1.66 s | -17.8% | 4f8a89c (S2.1-S2.7) |
  | aarch64 | 2.00 s | 1.60 s | -20.4% | S2.8, development run |
  | aarch64 | 2.03 s | 1.61 s | -20.5% | 498cdfe (S2.8), evidence run |
  | x86_64 | 3.83 s | 3.31 s | -13.5% | 498cdfe (S2.8), evidence run; the host was short of memory |
  | aarch64 | 2.00 s | 1.62 s | -18.9% | S2.9, development run |
  | aarch64 | 2.01 s | 1.61 s | -20.1% | S2.10, development run |
  | aarch64 | 2.06 s | 1.63 s | -20.8% | S2.10 + S2.11, development run |
  | aarch64 | 2.08 s | 1.66 s | -19.8% | S2.12, development run |
  | aarch64 | 2.07 s | 1.64 s | -20.8% | S2.13, development run |
  | aarch64 | 1.99 s | 1.63 s | -18.3% | S2.14, development run |
  | aarch64 | 2.00 s | 1.63 s | -18.2% | S2.15, development run |
  | aarch64 | 1.99 s | 1.49 s | -25.0% | S2.17, development run |
  | x86_64 | 3.58 s | 2.50 s | -30.1% | 0b9d585 (S2.20, rebased on dd47e28), evidence run |
  | aarch64 | 2.02 s | 1.46 s | -28.0% | 0b9d585, development run |
  | aarch64 | 2.04 s | 1.45 s | -28.9% | S2.21, development run |
  | aarch64 | 2.10 s | 1.47 s | -30.0% | S2.23-S2.28, development run |

  NBENCH2 (`samples/native/NBENCH2.rpgle`) is written with what stage 2 lowers: packed arithmetic, array elements, qualified subfields, %SUBST, %SCAN, %TRIM, %CHAR and conditions. Release build, p50 over 20 calls, same output on both engines:

  | Platform | Interpreter | Native | Change | Commit |
  | --- | --- | --- | --- | --- |
  | x86_64 | 4.97 s | 2.67 s | -46.3% | fac4084 + NBENCH2, development run |
  | aarch64 | 2.62 s | 1.29 s | -50.7% | S2.17, development run |
  | aarch64 | 2.62 s | 1.32 s | -49.5% | S2.21, development run |
  | aarch64 | 2.69 s | 1.33 s | -50.4% | S2.23-S2.28, development run |

  Allocations and database round trips are not measured yet.
- **IBM i contrast.** Expected values of the native unit programs come from IBM Docs and from PUB400 captures (`tools/golden/native1-5.txt`, IBM i 7.5, 2026-11-12). Those captures confirmed four interpreter defects (fidelity.md), to be reported upstream when this work is finished.

## Evidence

- `docs/native/evidence/2026-11-11-runtime-extraction.json`: first run (aarch64, plus x86_64 stopped for lack of memory)
- `docs/native/evidence/2026-11-11-runtime-extraction-run2.json`: second and third runs (x86_64 and aarch64)
- `docs/native/evidence/2026-11-12-stage1.json`: stage 1 on 96bd6a7, x86_64 and aarch64 (level-2 rust coverage NOT_DEMONSTRATED, bench FAIL: harness defects fixed in b0315e1)
- `docs/native/evidence/2026-11-12-stage2.json`: stages 1 and 2 (S2.1, S2.2) on 769f0d5, x86_64 and aarch64, with the first performance baseline
- `docs/native/evidence/2026-11-12-rebased.json`: stage 1 and S2.1-S2.7 on 4f8a89c, rebased on origin/main; x86_64 and aarch64 all PASS except clippy (upstream); the map of old to new commit hashes
- `docs/native/evidence/2026-11-12-s28.json`: S2.8 on 498cdfe, x86_64 and aarch64. Everything passes except clippy (upstream). The x86_64 tests failed on a test database with leftover catalog rows, and passed on a new database (the run x86_64-cleandb)
- `docs/native/evidence/2026-11-14-rebased2.json`: S2.1-S2.20 on 0b9d585, rebased on upstream dd47e28, x86_64 and aarch64. native-diff PASS on both (648 tests the same under auto). Every failure has its cause recorded, with a rerun that passes: the clippy finding is upstream's; save_changed_objects is intermittent; two upstream tests hung under the parallel release run on x86_64; the aarch64 release build ran out of memory with 2 jobs
