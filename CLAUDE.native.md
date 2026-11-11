# CLAUDE.native.md

Guidance for AI coding sessions working on the **native RPG backend** in this fork of as4tainr: RPG compiled to Rust instead of interpreted.

This file adds to the repository's `CLAUDE.md`; it does not replace it. Read `CLAUDE.md` first. Its non-negotiable rules, layout, commands and conventions apply here in full.

**Order of precedence:**
1. The upstream `CLAUDE.md` and `CONTRIBUTING.md`.
2. This file.
3. General Rust practice.

When two rules overlap, follow the stricter one. The one exception: code that already exists upstream keeps upstream's style, so it stays easy to rebase.

## 1. Goal and scope

- Generate Rust from the checked `Program` that `core/src/rpg/` already builds (`source → parse → check → Program`), compile it, and run it in place of the interpreter.
- The upstream interpreter (`interp.rs`) stays as it is. It remains the default engine and the reference for differential tests.
- In scope: the engine flag, the runtime extraction, the code generator, compilation and caching, and the differential test harness.
- Out of scope unless the user asks: host servers (the upstream author is building them), changes to the RPG front end other than the minimal hooks listed in section 3, and new IBM i features.

## 2. Fork hygiene

The upstream author commits many times a day. Every change must stay easy to rebase and easy to offer upstream.

- Remotes: `upstream` is `kyleturley-cyber/AS4tainr`, `origin` is our fork. Before starting work, run `git fetch upstream && git rebase upstream/main` on the working branch. Never merge upstream into a branch.
- Use short-lived branches named `native/<topic>`, with one topic per branch.
- **Touch upstream files only where section 3 allows.** Put new code in new files. If a change seems to need edits elsewhere in upstream code, stop and ask.
- Never reformat, rename or move upstream code just for style. Do not "fix" upstream lints or warnings in files you are not otherwise changing.
- Never push, open a PR, or create or move a tag unless the user asks.

## 3. Architecture

### Engine flag

`AS4TAINR_RPG_ENGINE` takes three values. Any other value is an error that names the variable.

| Value | Behavior |
| --- | --- |
| `interp` | Default. Upstream behavior, unchanged. |
| `rust` | Use the native program. If none is available or valid, fail with an explicit error. |
| `auto` | Use the native program when it is available and its key matches. Otherwise use the interpreter and log the reason once per program. |

### Components

| Path | What |
| --- | --- |
| `core/src/rpg/native/` | The engine switch, lowering (`Code` → `rpgcg::ir`), fingerprints, emission, the per-program native table (ADR 0002, 0004) |
| `core/src/rpg/native_rt.rs` | Submodule of `interp`: `Machine` as `rpgrt::native::Runtime`, the dispatcher, error routing (`settle`), observation |
| `rpgrt/` (new workspace crate) | Runtime for generated code: storage (`mem`), exact decimals (`dec`), data formats, built-in functions, dates, I/O through `rla`, `workstn` and `spool`. See "Runtime" below |
| `rpgcg/` (new workspace crate) | Code generator: `rpgcg::ir` (lowered from the compiled `Program`'s routines) in, Rust source plus a source map out |
| `rpgnative/` (new workspace crate) | The registry: `build.rs` includes the generated modules of `AS4TAINR_NATIVE_DIR` and a static table (ADR 0003) |
| `native-tools/` (new workspace crate) | `as4tainr-native-emit`: generates the code for the programs of a set of libraries |
| `target/rpg-native/<build>/` | Generated modules (`<fingerprint>.rs`, `.map.json`). Never committed |
| `docs/native/` | Design (ADRs), `STATUS.md`, `HANDOFF.md`, `fidelity.md` (differences between the native and interpreter engines) |

### Allowed touch points in upstream code

- `core/src/rpg/mod.rs`: the call into `native/` at compile and activation time.
- The root `Cargo.toml`: workspace members, and `overflow-checks = true` for the generated code (`[profile.release.package.rpgnative]`; ADR 0004: not the whole release profile, which would change upstream's behavior).
- `core/Cargo.toml`: dependencies on the workspace crates.
- `core/src/rpg/interp.rs`: the `native_rt` submodule declaration and the one-statement hook at the start of `run_code_at` (authorized by the user, 2026-11-11; ADR 0004).
- `core/src/rpg/program.rs`: the `native` field of `Program` and its initial value (authorized by the user, 2026-11-11; ADR 0002).
- `core/src/lib.rs`: module declarations.
- Moving upstream runtime modules into `rpgrt`, with re-exports so that every upstream path keeps compiling. Done as a pure move with no behavior change, in its own commit.

### Runtime

- Extract, do not rewrite. Move upstream code into `rpgrt` unchanged and re-export it from its old paths. Every upstream test must pass unchanged after the move.
- Generated code must keep the interpreter's storage model: bytes in their IBM i formats (EBCDIC, zoned, packed, binary), with data structure subfields as overlays on those bytes. Do not give generated code a different data representation, because that is where fidelity is lost.
- Use upstream's `dec.rs` for decimals (up to 63 digits, with RPG's intermediate precision rules). Never use `f64`/`double`, and never `rust_decimal` or any other 28-digit type.
- A later specialization (for example `i64`/`i128` with a fixed scale when the precision fits) needs an ADR first. It must give results identical to `dec.rs` on the full differential suite.

### Loading native programs

- **No `unsafe`, and no dynamic loading (`dlopen`/`libloading`) without an accepted ADR.** Loading a `.so` needs `unsafe`, which our policy forbids (section 5).
- Static linking (ADR 0003): generated routines are modules of the `rpgnative` crate, included by its `build.rs` from `AS4TAINR_NATIVE_DIR`, and registered in a safe static table (fingerprint, entry function). Every binary and test that links core gets them; `Cargo.toml` and `Cargo.lock` never change when code is generated. Code is generated with `AS4TAINR_NATIVE_EMIT=<dir>` (or `as4tainr-native-emit`), then core is rebuilt with `AS4TAINR_NATIVE_DIR=<dir>`.
- In production, the image ships the binary with the programs already compiled. The production image contains no `rustc` and no `cargo`.
- A program created after that build runs in the interpreter (`auto` mode) until the next build.
- **Key (ADR 0002):** each routine is identified by the SHA-256 of its compiled code (every instruction's `Debug` text; in stage 2 also the layout it uses), with the `rpgcg`, `rpgrt` and rustc versions. Everything the source, `/COPY`, `/INCLUDE`, options, overrides and file and field definitions change reaches the compiled code, so this is stricter than a hash of those inputs. A program runs natively only when every routine's key matches.

### Code generation rules

- Generated code is deterministic. The same `Program` and the same generator must produce byte-identical Rust.
- Rust names are mapped from RPG names deterministically, with Rust keywords escaped and no collisions. The mapping is written into the source map.
- Every generated statement carries a comment with its RPG source line, so errors and listings point back to RPG. The compiled code keeps the line number only, not the member and sequence number; the source map (`.map.json`) carries the program and routine as well.
- A construct the generator does not support is a compile-time diagnostic that names the RPG line. It must never become empty code, a zero, a default value or a comment. In `auto` mode the interpreter then runs the program; in `rust` mode compilation fails.
- RPG run-time errors use upstream's error path: the RNQ inquiry ID, turned into the RNX escape ID by `rpg::errors::message_id`. The program status data structure, `*PSSR`, MONITOR and indicators behave exactly as in the interpreter.
- Parameters keep upstream's semantics: BYREF shares storage, including aliasing and forwarding. VALUE is an independent copy. CONST is read-only, and an attempt to write through it is rejected at compile time.
- GOTO, TAG, CABxx and the RPG cycle are generated as an explicit state machine (`loop { match pc { … } }`). In stage 1 (ADR 0001) every routine is one, a faithful compilation of `code::Ins`; stage 2 generates structured Rust for routines that need no state machine.
- Do not suppress warnings globally to hide generator defects. Any local `#[allow]` must say why, and must not touch the lints in section 5.

## 4. Clean room

The upstream clean-room rules apply in full (`CLAUDE.md` rule 1, `CONTRIBUTING.md`). In addition:

- The backend implements no new IBM i behavior. It must reproduce exactly what the interpreter does. If the interpreter is wrong, report it as an upstream bug with a golden capture. Do not fix it only in the native engine.
- A change that does need IBM i behavior (for example a missing built-in function) needs one of two sources: IBM's public documentation (URL and topic) or a PUB400 capture made with `tools/golden/`. Record the source in the test, using this format:
  `// Source: IBM Docs, "<topic>", IBM i 7.5, <URL>, read YYYY-MM-DD` or
  `// Source: golden capture tools/golden/<id> (PUB400, IBM i 7.5, YYYY-MM-DD)`
- Never use IBM binaries, disassembly or decompilation (that includes `javap` on IBM jars), or material from third-party products such as Fortra's. Never write code "from memory" of proprietary software.
- Message texts are always written fresh. Run `tools/golden.py GOLDEN_DIR --audit core/src rpgrt rpgcg` before every PR.

## 5. Rust rules

### Every new crate root and every generated crate starts with

```rust
#![forbid(unsafe_code, unused_must_use)]
#![forbid(clippy::unwrap_used, clippy::expect_used, clippy::panic,
          clippy::todo, clippy::unimplemented,
          clippy::indexing_slicing, clippy::string_slice,
          clippy::arithmetic_side_effects)]
```

These lints apply to `rpgcg`, to `core/src/rpg/native/`, to generated code, and to new code added to `rpgrt`. Code moved unchanged from upstream into `rpgrt` keeps upstream's lints; do not rewrite it to satisfy ours.

### Errors

- Foreseeable failures return `Result` with a typed error and are propagated with `?`. That covers data, conversions, overflow, division by zero, SQL, I/O and authority.
- Never use `unwrap`, `expect`, `panic!`, `todo!` or `unimplemented!` where a program or its input can reach them.
- Replace `RefCell::borrow`/`borrow_mut` with `try_borrow`. Never write `lock().unwrap()`.
- Integer and decimal arithmetic is checked, including division by zero and negating the minimum value. Overflow produces the RPG error, never a panic.
- Use `get` instead of indexing, and check conversions with `try_from` instead of truncating `as` casts.
- Writing output (DSPLY, diagnostics) can fail, so handle the error. Never write `println!` followed by `unwrap`.
- Inputs, recursion depth, memory and loops have explicit limits. No cache, retry or pool is unbounded.
- `catch_unwind` and `panic = "abort"` are not error handling.
- Passing these checks proves only that the checks pass. Never claim that code cannot panic.

### Text, time and SQL

- Use CCSID and EBCDIC explicitly per field and per object, through upstream's conversion code. No silent character substitution.
- Keep bytes, characters and RPG positions apart. Never slice UTF-8 at an arbitrary byte index.
- SQL follows upstream's conventions: identifiers only through `catalog::ident()`, values always bound as parameters, user SQL only through the `as4tainr_sql` login.
- No secrets in Git, in logs or in evidence. Error messages carry context but never credentials.

### Dependencies and builds

- Keep `Cargo.lock` and validate with `--locked`. Never delete the lockfile to fix a build.
- Every new dependency needs, in the PR description: its purpose, license (it must be compatible with Apache-2.0), maintenance status, the features enabled, and any advisories. Prefer no new dependency.
- Pin the toolchain with a `rust-toolchain.toml` (a new file, so it does not conflict upstream). Never use `target-cpu=native` for artifacts you distribute.
- Keep `CARGO_TARGET_DIR` separate per platform. Check that every artifact you claim was built actually exists, and record its SHA-256.

### Performance

- Measure before you optimize: run the same program on both engines and record throughput, p50/p95/p99, CPU, memory, allocations and database round trips.
- Never trade correctness or checks for speed. Every optimization keeps the full differential suite green.
- Results from an emulated platform (for example ARM64 under Podman) are not evidence for any other platform.

## 6. Tests

### Expected values

Expected values come from IBM documentation or golden captures, never from running the new code. The interpreter is the reference for *parity* between engines, not for *truth*: it can share a defect with the backend.

### Differential harness

`scripts/native-diff.sh` runs every program in `samples/qrpglesrc/`, every `scripts/demo.sh` case and every golden test under both `interp` and `rust`. It compares:

- DSPLY output and spooled files;
- records in the files the program uses;
- final indicators and the program status data structure;
- message IDs and exit status;
- for the cases that capture it, the final storage of global variables.

Any difference is a failure.

### Native unit tests

Cover at least:

- BYREF changes visible to the caller;
- two parameters aliasing the same variable;
- forwarding a parameter to another call;
- VALUE copies that stay independent;
- CONST, including a rejected attempt to forward it as a writable parameter;
- calls nested in arithmetic and in return values;
- precision and overflow at the edges of packed and zoned types;
- division by zero;
- data structure overlays;
- a GOTO/TAG state machine.

### Never

- Weaken, skip or edit a test so that it passes. If a test looks wrong, explain why and stop.
- Mark a test as passed when it was skipped or did not run.

## 7. Checks before every commit

Run all of these, in the order listed:

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
AS4TAINR_TEST_DATABASE_URL=postgres://as4tainr:as4tainr@127.0.0.1/as4tainr cargo test --locked
cargo test --locked --release
cargo build --locked --release
AS4TAINR_BIN=target/debug/as4tainr scripts/demo.sh
AS4TAINR_RPG_ENGINE=rust scripts/native-diff.sh
```

- The upstream suite must pass unchanged with `AS4TAINR_RPG_ENGINE=interp`, and with the flag unset.
- When a change touches screens, jobs or locks, also run the 5250 end-to-end tests listed in `CLAUDE.md`.
- When a change touches decimals, the database or protocols, a human reviews it line by line.

## 8. Evidence

- Each run is recorded with a status: `PASS`, `FAIL`, `BLOCKED` (give the cause), `NOT_RUN` (give the cause) or `NOT_DEMONSTRATED`. `RUNNING` is not `PASS`.
- Each acceptance writes a small manifest, `docs/native/evidence/<date>-<topic>.json`, holding: commit, toolchain, platform, the commands, exit statuses, test counts, and the SHA-256 of the artifacts and logs.
- Full logs stay outside Git (`target/evidence/`); the manifest refers to them by hash. Never overwrite a recorded result: a new run is a new manifest.
- Never pipe output through `head`, `tail` or `grep` in a way that hides a failure. Record full stdout and stderr.
- Keep four levels apart: implemented, validated locally, contrasted with IBM i (golden), and fit for production. One level never implies the next.

## 9. Commits

- Sign off every commit (`git commit -s`), and add the AI co-author lines the session gives you.
- Write commit messages in English, in upstream's style. The first line says what changes in the engine or which IBM i behavior is now reproduced, and which golden or differential cases verify it. The body gives the detail.
- Commits are small and cover one topic. A runtime move is always a commit on its own, with no behavior change.
- Update `docs/native/STATUS.md` and `docs/native/fidelity.md` in the same commit as the code they describe.

## 10. Sessions

- Start each session by reading, in this order:
  1. `CLAUDE.md`;
  2. this file;
  3. `docs/native/STATUS.md` and `docs/native/HANDOFF.md`;
  4. `git status`, and `git log upstream/main..HEAD`.
- Work on one task per session.
- End each session by updating `docs/native/HANDOFF.md` with: what changed, the tests run and their results, failures you can reproduce, limits, and the next step.

## 11. Stop and ask when

- a change needs an upstream file outside the touch points in section 3;
- the only way forward is `unsafe`, dynamic loading, or a new dependency;
- a test would have to change its expected value to pass;
- the native and interpreter engines differ and the right behavior is unclear: report it with a golden capture, and do not choose silently;
- there is no public source for an IBM i behavior;
- a rebase on upstream conflicts with files you were not working on.
