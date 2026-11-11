# Native RPG backend: status

Where the native backend (RPG compiled to Rust, see `CLAUDE.native.md`) stands. Four levels are kept apart: **implemented**, **validated locally**, **contrasted with IBM i (golden)** and **fit for production**. One level never implies the next.

## Phases

| Phase | What | Level |
| --- | --- | --- |
| 1. Runtime extraction | Workspace crate `rpgrt` with the run-time modules moved unchanged from `as4tainr-core`, re-exported from their old paths | Implemented. Validated locally on x86_64 and aarch64, except clippy, which upstream `main` fails too (see "Checks" below). No behavior change, so nothing to contrast with IBM i |
| 2. Engine flag | `AS4TAINR_RPG_ENGINE` (`interp`, `rust`, `auto`) in `core/src/rpg/native/` | Not started |
| 3. Code generator | `rpgcg`: `Program` in, Rust source and source map out | Not started |
| 4. Compilation and caching | Program key, generated crates under `target/rpg-native/<key>/`, the `programs` crate, `as4tainr build --native` | Not started |
| 5. Differential harness | `scripts/native-diff.sh` | Not started (the script does not exist yet) |

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

## Evidence

- `docs/native/evidence/2026-11-11-runtime-extraction.json`: first run (aarch64, plus x86_64 stopped for lack of memory)
- `docs/native/evidence/2026-11-11-runtime-extraction-run2.json`: second and third runs (x86_64 and aarch64)
