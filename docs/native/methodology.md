# Native RPG engine: how it is built, tested and validated

This document describes how the native engine is built and verified. That engine compiles RPG to Rust instead of interpreting it.

It sets out what the process guarantees, and also what it does not. It describes what is actually done, with the real file and script names, so that anyone can check it.

State when written (2026-11-19):
- branch `native/engine`, stage 2 at S2.123;
- 95.8% of the suite's instructions run natively;
- 119 native unit tests;
- 117 PUB400 capture lists;
- a manifest for every commit since 2026-11-15.

## 1. Rules that are not negotiable

The rules are written in `CLAUDE.md` (upstream's) and `CLAUDE.native.md` (the native engine's). The ones the guarantees rest on are these:

1. **Clean room.** Only IBM's public documentation and behavior observed on a real IBM i (PUB400) are used. No IBM binaries, no disassembly, no code "from memory". Message texts are written fresh.
2. **The interpreter is the reference for parity, not for truth.** The native engine must do exactly what upstream's interpreter does. When the interpreter is wrong compared with IBM i, the native engine does not fix it on its own. The defect is recorded in `docs/native/fidelity.md` with its PUB400 capture and held for upstream.
3. **Expected values come from IBM, never from running the new code.** Every test carries its source: `// Source: golden capture tools/golden/nativeNNN ... (PUB400, IBM i 7.5, date)`, or the IBM Docs page.
4. **Tests are never weakened.** If a test looks wrong, the reason is explained and work stops; the test is not edited to pass.
5. **No floating point for business values.** Packed and zoned decimals go through `dec.rs`, exact decimal arithmetic up to 63 digits. RPG `float` fields, which are IEEE on IBM i, follow the interpreter's model (ADR 0008, accepted).
6. **What is not supported is never filled in.** A construct the generator cannot translate is left to the interpreter (`auto` mode), or fails the compile (`rust` mode). It never becomes empty code, a zero or a comment.
7. **Strict Rust rules** in all new code and in generated code:
   - `forbid(unsafe_code)`;
   - no `unwrap`, `expect`, `panic` or unchecked indexing;
   - checked arithmetic.

## 2. The architecture that makes verification possible

- **Three modes** through `AS4TAINR_RPG_ENGINE`:
  - `interp`: upstream's behavior, and the default;
  - `auto`: native when valid generated code exists, the interpreter otherwise;
  - `rust`: native only, with an explicit error when code is missing.
- **Content-addressed routines (ADR 0002).** Each routine is keyed by the SHA-256 of its compiled code, with the `rpgcg`, `rpgrt` and `rustc` versions.
  - Any change to the source, `/COPY`, options, overrides or file definitions changes the key.
  - A program runs natively only when every key matches.
- **Static linking (ADR 0003).** Generated routines are compiled into the binary, with no dynamic loading and no `unsafe`.
- **Deterministic generation.** The same program and the same generator give byte-identical Rust.
- **Tripwire** (`core/tests/native_tripwire.rs`): it records a hash of the text of the interpreter functions the native engine reproduces.
  - When upstream changes one of them, the test fails.
  - The native code is then reviewed against the change, and the new hash is recorded with the reason in the commit.
  - An upstream change cannot leave the native engine out of step without anyone noticing.

## 3. The loop for each block

Each block (S2.x) covers one RPG construct and always follows the same steps.

### 3.1 Choose what is missing, from data

- `scripts/native-coverage.sh` counts which of the suite's instructions still run in the interpreter, by kind.
- A diagnostic pass (`AS4TAINR_NATIVE_WHY`) tells, for each instruction that is not translated, why not. For example: "the start of `%SUBST` is an expression".
- The block chosen is the one that unblocks the most instructions.

### 3.2 Read the interpreter and the documentation

- The exact interpreter function that performs the operation is read: its order of evaluation, its errors, its edge cases.
- The code says which function it reproduces. For example, "as interp.rs's SUBST arm: the string, then the start, then the length".

### 3.3 Capture what IBM i really does, on PUB400

- A small RPG program (`tools/golden/nativeNNN/*.rpgle`) exercises the construct, including its error cases under MONITOR.
- It is sent with a list (`tools/golden/nativeNNN.txt`) through `tools/golden/golden.sh`, which compiles and runs it on PUB400 over SSH.
- **One SSH connection at a time and one batch per block**, so as not to strain a free shared system.
- The program is run locally under the interpreter first, so as not to spend a request on source that does not compile. That has still happened a few times, and is recorded.
- IBM's outputs, which contain IBM message texts, stay in `target/ibmi/golden/` and **are never committed**. Only the program and the list are.

### 3.4 Implement

1. **Lowering** (`core/src/rpg/native/lower.rs`): from the compiled program to the generator's intermediate form (`rpgcg::ir`).
2. **Emission** (`rpgcg/src/lib.rs`): from that form to Rust. Every generated statement carries its RPG line.
3. **Runtime** (`rpgrt/src/native.rs`, `core/src/rpg/native_rt.rs`): it reuses the interpreter's own functions wherever it can, instead of rewriting them.
4. Every change raises the generator's version (`+stage2.NNN`), which changes every key.

### 3.5 Native unit test

- `core/tests/native/NXXX.rpgle` is added to `core/tests/native_units.rs`. It is the captured program, with DSPLY instead of writing to the IFS.
- The expected value is IBM's, from the capture, with its `Source:` comment.
- When the interpreter and IBM i disagree, the test asserts only what they agree on, and the difference goes to `fidelity.md`.

### 3.6 Local checks of the block (local x86_64 machine)

In this order:
1. `cargo clippy --all-targets -D warnings` of our code;
2. the unit tests under the interpreter, generating code (`AS4TAINR_NATIVE_EMIT`);
3. the same tests in `rust` mode on the generated code (`AS4TAINR_NATIVE_DIR`);
4. `clippy -D warnings` of the **generated** code (`-p rpgnative`), with the section 5 lints;
5. the tripwire.

All five must pass. For example, the generated-code clippy found two helper functions with the same name in S2.123. It was fixed before validation.

### 3.7 Snapshot and full validation on a VM

1. A Git snapshot of the working tree (`commit-tree` with a temporary index) is chained to the previous one. A later snapshot contains the earlier ones, so its validation covers them all.
2. It is copied to a VM, where `vm-s7full.sh` runs the whole of `CLAUDE.native.md` section 7 on new PostgreSQL databases:
   - `cargo fmt --check`;
   - `clippy` (see section 9, point 3);
   - `test` in debug and `test --release` (upstream's whole suite and the native one);
   - `build --release`;
   - `scripts/demo.sh`;
   - `scripts/native-diff.sh`.
3. Each step writes its exit status to `status` and its full log. At the end, the SHA-256 of every log is recorded (`logs.sha256`).

### 3.8 Commit only after validation

- The commit is made **after** the full validation passes.
- The snapshot's starting tree is checked to be the last commit's tree.
- The commit is signed off (`-s`) and carries the co-author lines.
- Its message says which IBM i behavior it reproduces, which capture verifies it, and the validation's result, including any failure and its cause.
- It is pushed to our fork (`origin native/engine`). Never to the upstream repository unless the user asks.
- Its evidence manifest is then written (section 7).

## 4. The differential harness (`scripts/native-diff.sh`, ADR 0005)

It compares the two engines on the same programs, and any difference fails the validation.

| Level | What it does |
| --- | --- |
| reference | The whole suite under the interpreter, generating code for every program it runs. Its per-test results are the reference. |
| demo-interp | `scripts/demo.sh` under the interpreter. |
| generate, build, generated-lints | Compiles the generated code and runs clippy on it with `-D warnings`. |
| coverage | Information only: the share of instructions that are native. |
| level1 | A corpus of programs under both engines in one process: identical storage, indicators, files and output. |
| level2-auto | The whole suite in `auto` mode. Every test must give exactly the reference's result. |
| level2-rust | The whole suite in `rust` mode. It counts the tests that agree, and every program refused must have an explained cause (no generated code), never a difference in behavior. |
| level3-demo | The demo in `rust` mode, with the same output as under the interpreter. |
| level3-golden | Comparison with `tools/golden.py` captures; `NOT_RUN` today because the VMs have no captures of those lists. |

**Determinism (S2.121).** Some upstream tests put a library name made from the process ID into their RPG source. The source then changed between the run that generates the code and the run that executes it, so 23 programs could not run natively.

The harness now runs each test binary as process 1 of its own PID namespace (`unshare -cpf`, as the ordinary user), with its tests one at a time. Upstream's tests are not touched.

## 5. The machines, and why more than one architecture

| Machine | Architecture | Use |
| --- | --- | --- |
| Local | x86_64, 2 CPUs, 3 GB | Development, block checks, diagnostic passes, PUB400 captures |
| `jsr-test-2` | x86_64, 2 CPUs, 3 GB | Full validations |
| `jsr-test-arm` | ARM64 (aarch64), real, 2 CPUs, 3 GB | Full validations |

- Each machine runs PostgreSQL in a container, with a new database for every validation.
- The two VMs work in parallel on different snapshots: while one validates block N, the other validates block N+1.
- Validating on real ARM64 hardware, not emulated, shows that the generated code and the runtime do not depend on the architecture. `CLAUDE.native.md` does not accept emulated results as evidence for another platform.
- To avoid waiting, two blocks are sometimes validated in one run. Each is still its own commit.

## 6. Independent reviews with Codex

- **Parity reviews per block.** Codex runs read-only (`codex exec -s read-only`, high reasoning effort) on the new functions. It is asked to compare each one with the interpreter function it reproduces and look for differences. Lists of about six functions per review finish; longer ones end unfinished.
- **Real defects it found, all fixed with a test:**
  - the argument count of a call through a prototype (S2.87);
  - a printer file's INFDS;
  - based arrays in whole-array operations;
  - `*NEXT` read or passed by reference;
  - LIKEDS through an EXTPROC of another size;
  - three float details: unary minus precision, the order of the return type, calls inside expressions;
  - a display file CHAIN key evaluated too early.
- **Rule:** a finding is confirmed by reading the code before anything is done about it. When Codex is right about the interpreter but not about the native engine, the finding goes to `fidelity.md`.
- **A full audit of the repository** (2026-11-19: security, performance and IBM i fidelity) gave 37 findings.
  - Each one is marked CONFIRMED (checked in the code) or PLAUSIBLE (not reproduced).
  - No code has been changed for them yet. The plan is to fix them on `fix/` branches once the native engine is done, and to report what affects upstream; security findings privately.

## 7. Evidence: what is kept, and where

- **For every commit**, a manifest: `docs/native/evidence/<date>-<commit>.json`.
  - `scripts/native-manifest.py` writes it mechanically from the validation runs. It checks with Git which run validated the commit: one whose snapshot has the same tree as the commit, or a later snapshot whose history contains it.
  - A run stopped by hand carries an `ABORTED` file saying why, and the script skips it. On 2026-11-20 two runs on the ARM machine shared one work directory, and both were stopped and marked this way.
  - It records:
    - the commit and its tree;
    - the toolchain (`rust-toolchain.toml`);
    - each run's machine and platform, and its commands;
    - every step's exit status, and the steps that were not 0;
    - the native-diff levels;
    - the test counts;
    - the SHA-256 of every log and file of the run.
  - It also quotes the commit message's paragraphs on its validation. That is where each non-zero step is explained, for instance a known upstream flake run again.
  - It interprets nothing.
- **The logs themselves** stay outside Git, under `target/evidence/vm/<machine>/<run>/`, copied from the machines that produced them. The manifest's hashes prove which logs they are. Logs are recorded whole: nothing is piped through `head` or `grep` when it is saved.
- **For every test:** the `Source:` comment with the capture or the IBM page, and the capture list in Git.
- **For every decision:** the ADRs in `docs/native/adr/` (0001-0008), with their status. ADR 0008, on floats, was explicitly accepted by the user.
- **For every difference from IBM i:** `docs/native/fidelity.md`, with its capture.
- **State and handover:** `docs/native/STATUS.md`, updated in the same commit as the code, and `docs/native/HANDOFF.md`.
- **Honest statuses:**
  - `RUNNING` is not `PASS`;
  - what did not run is recorded as `NOT_RUN` with its cause;
  - four levels are kept apart: implemented, validated locally, contrasted with IBM i, and fit for production. One never implies the next.

## 8. How failures are handled

- **A native engine failure** is fixed before the commit. The snapshot is made again and the validation repeated.
- **A known flaky upstream test**, such as the CHGLP race in QTEMP (CPF3219), PostgreSQL's "tuple concurrently updated" in catalog grants, or the CRTDUPOBJ race:
  - it is checked that the test runs no RPG;
  - it is run again on a new database under both engines;
  - the result goes into the commit message, and so into the manifest.
  - The test is not edited and the failure is not hidden: the manifest still shows the step's non-zero exit.
- **An interpreter defect against IBM i:** the native engine reproduces it, to keep parity, and it is recorded in `fidelity.md` with the capture for upstream. For example, `WRITE file.format` on a qualified file.
- **Our own mistakes** are recorded too. Some examples:
  - a false statement in S2.94's commit message, corrected in S2.96's without rewriting history;
  - PUB400 captures that did not compile;
  - the name collision in S2.123.

## 9. What this process does **not** guarantee

1. **Parity is not truth.** If the interpreter and the native engine share a defect, the differential harness does not see it. PUB400 captures cover that, but only for the cases captured.
2. **Coverage of the suite is not coverage of RPG.** The 95.8% is measured on the suite's and the demo's programs. A real program may use constructs that still go to the interpreter (`auto` mode), or that fail in `rust` mode.
3. **Upstream's full clippy fails** on upstream code, on the `clippy::chunks_exact_to_as_chunks` lint, which this fork does not change. The fix was offered upstream as PR #24. Validation therefore counts `clippy-ours`, which allows only that lint; the manifests record both exit statuses.
4. **The tree validated carries a patch the commit does not:** the `catalog::init` lock fix from branch `fix/init-lock-timeout` (upstream issue #18), which keeps the suite from hanging on the VMs. It is pending upstream, and every manifest says so.
5. **Evidence before 2026-11-15** is in 11 hand-written manifests. From that date on, the manifests are generated, and their logs are kept in `target/evidence/vm/`, outside Git. A reader without that directory can check the hashes only against copies of the logs obtained from us.
6. **`level3-golden` does not run** on the VMs, because they have no `tools/golden.py` captures. The 5250 end-to-end tests are not part of these runs either.
7. **There are no recent performance measurements.** Section 5 requires measuring before optimizing, and nothing has been optimized.
8. **Fitness for production is not demonstrated.** The level reached is "validated locally and on two architectures, contrasted with IBM i for the cases captured".
