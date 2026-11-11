# Native RPG backend: handoff

Last session: 2026-11-11, phase 1 (runtime extraction), branch `native/runtime-extraction`.

## What changed

- New workspace crate `rpgrt`. It holds `rpg/dec.rs`, `rpg/data.rs`, `rpg/mem.rs`, `rpg/dates.rs` and `tn5250/ebcdic.rs`, moved with `git mv` from `core/src/` and byte-identical. `as4tainr-core` re-exports them from their old paths (`core/src/rpg/mod.rs`, `core/src/tn5250/mod.rs`). Details are in `STATUS.md`.
- `bifs.rs` and `ptrs.rs` were **not** moved. They are `impl Machine` submodules of the interpreter, and the interpreter's machine holds the SQL client, record-level access and the workstation. The coupling is described in `STATUS.md`; the user agreed to leave them where they are.
- `ebcdic.rs` was added to the move, with the user's agreement. It is a table with no imports that `data.rs` needs.
- Lints: `rpgrt/src/lib.rs` forbids `unsafe_code` and `unused_must_use` and denies section 5's clippy lints. The moved modules allow those lints where they are declared, because a forbidden lint cannot be allowed again for code that keeps upstream's lints. This deviates from the literal "forbid" in section 5, and the reason is written in `lib.rs`.
- `docs/native/STATUS.md`, this file, and two evidence manifests in `docs/native/evidence/`.

## Tests run

Section 7, on x86_64 (this machine, `CARGO_BUILD_JOBS=1`) and on aarch64 (`jsr-test-arm`). Both have PostgreSQL 16.15 in podman (container `as4tainr-test-db`). The full logs are in `target/evidence/`, and the manifests give their hashes.

- PASS on both machines: `fmt`, `test`, `test --release`, `build --release`, `demo.sh`.
- The branch and the base commit have the same 593 tests with the same result for each.
- FAIL on both: clippy. Its only errors are upstream's two in `hostcmd.rs`/`hostdb.rs`, and upstream CI fails on `main` the same way.
- NOT_RUN: `native-diff.sh`, which does not exist yet.

## Environment the suite needs

- **An empty `/QSYS.LIB`** on hosts where tests run as a non-root user with user namespaces. Without it, `rla_test::qsh_commands_see_qsys_lib` fails. Both test machines have it now (`sudo mkdir /QSYS.LIB`).
- **A new database for `demo.sh`**, as upstream CI uses (`createdb as4tainr_demo`). Data left by a previous run makes CUSTLIST and CUSTPRT differ.
- **`cargo test` with `--no-fail-fast`** when one test fails. Otherwise the binaries after `rla_test` do not run.
- **Low memory.** On this machine (3.8 GB) the Claude Code session stopped a check for lack of memory. Use `CARGO_BUILD_JOBS=1` and run the checks outside the session (`setsid nohup`).

## Failures you can reproduce

- clippy 0.1.99 `chunks_exact_to_as_chunks` at `core/src/hostcmd.rs:163` and `core/src/hostdb.rs:657`, on upstream `main` as well. This is for upstream to fix (the host servers).
- `rla_test::save_changed_objects` fails now and then: 1 of 6 runs, on x86_64 under load. It passes alone. The likely cause, which is not proven: SAVLIB's reference point is the server-wide `pg_snapshot_xmin` (`save.rs`), which a concurrent test's open transaction holds back. This is upstream code.

## Limits

- No `rust-toolchain.toml` yet (section 5 asks for one). It is its own topic, and it would not fix the clippy failure.
- The pure parts of `bifs.rs`/`ptrs.rs` that generated code will need are still inside `impl Machine`.
- Neither upstream issue above has been reported upstream. That is the user's decision.

## Next step

1. Phase 2: the engine flag `AS4TAINR_RPG_ENGINE` in `core/src/rpg/native/`.
2. Before phase 3: an ADR on getting the pure parts of `bifs.rs`/`ptrs.rs` out of `impl Machine`. That touches `interp.rs`, outside section 3's touch points, so it needs the user's approval.
