# ADR 0005: The differential harness

Status: accepted by the user (2026-11-11), after an independent review

## What is compared

Section 6 of CLAUDE.native.md asks for the same programs run under `interp` and `rust`, comparing:

- DSPLY output and spooled files;
- records in the files the programs use;
- final indicators and the program status data structure;
- message IDs and exit status;
- for the cases that capture it, the final storage of global variables.

No existing path shows indicators or storage at the end of a program. Tests see only DSPLY lines, printed lines, messages, `params` and file contents.

## Three levels

**1. The corpus diff test.** A new integration test, `core/tests/native_diff.rs`, which is a new file. Its corpus:

- `samples/qrpglesrc`;
- `samples/apps/contacts`;
- the 23 sources in `core/tests/rpg/`;
- the 76 RPG sources in `tools/golden/`;
- the native unit-test programs (below).

For each program, in one process and in a library of its own with a fixed name:

1. Create the files and data the program needs.
2. Run it under `with_engine(Interp)` and take an observation.
3. Restore the data.
4. Run it under `with_engine(Rust)` and take another observation.
5. Require the two observations to be identical.

An observation (`native::observe`) records:

- DSPLY lines;
- printed lines and spooled files;
- the error's message ID;
- every global buffer, as bytes (SHA-256 plus the first differing offset on a mismatch);
- the indicators, sorted;
- the PSDS bytes;
- `params`;
- the program's files' records, ordered by relative record number.

Everything runs in one process with fixed names, so nothing random enters the comparison.

Global state is kept apart. Each engine's run has its own job key (`PID`-style key plus engine), so that `PROGRAMS` (a program left active without LR is reactivated and skips *INZSR, cycle.rs:378-383), `BOUND`, `ile::ACTIVATIONS`, `exec_sql::KEPT`, `cee::HEAPS`, `crt::FDS` and the pgmmsg stacks never carry one run's state into the other. `interp::end_programs` runs between the two.

The job name stays out of the observation. Time is masked:
- the PSDS's date and time positions (errors.rs:290-300);
- the job's date and the run's start time.

A program whose globals depend on the clock (`%DATE()`, `%TIME()`, `%TIMESTAMP()`, `*DATE`, `TIME`) is marked in the corpus list. Its outputs are still compared after the same masking, and its storage is compared outside the fields named in the list. Each such entry gives its reason. The test does not enter the suite until its programs have been generated (two-phase build, below). Before that, it is skipped and says so, and a skipped run never counts as passed (section 6).

**2. The whole upstream suite under both engines.**

1. Run `cargo test --no-fail-fast` with `AS4TAINR_RPG_ENGINE=interp` and `AS4TAINR_NATIVE_EMIT=dir`, on the normal build.
2. Build again with `AS4TAINR_NATIVE_DIR=dir`.
3. Run `cargo test --no-fail-fast` with `AS4TAINR_RPG_ENGINE=rust`.

Every test's result must be the same in both runs. Under `rust`, a program without native code fails, so coverage is checked too. The upstream tests have their own expected values, from IBM documentation and captures, so this level checks truth as well as parity.

**3. The scripts.** Under both engines:

- `scripts/demo.sh`, on a new database each time;
- `tools/golden.py`, when captures exist in `target/ibmi/golden`; otherwise NOT_RUN with that cause.

The outputs must be the same, and both must match the golden files.

**Build cost.** Changing `AS4TAINR_NATIVE_DIR` rebuilds `rpgnative`, then `core`, the binary and every test binary. That is about 6 minutes and more per build on these 2-CPU machines. Level 2 is two test runs plus one rebuild. Phase N4 budgets for it: x86_64 and aarch64 in parallel, one each, and separate `CARGO_TARGET_DIR`s for the emitting build and the native build, so the first is never invalidated. If one crate holding every generated routine compiles too slowly, the registry is split by fingerprint prefix (ADR 0003).

`scripts/native-diff.sh` runs the three levels in order. It writes `target/evidence/<date>-native-diff/` and a manifest (section 8), and fails on any difference, any error, or any level that did not run.

## Native unit tests (section 6)

They are RPG programs in `core/tests/native/`, one per item:

- BYREF changes seen by the caller;
- two parameters that alias one variable;
- a parameter forwarded to another call;
- independent VALUE copies;
- CONST, including a rejected attempt to forward it as writable;
- calls inside arithmetic and in return values;
- precision and overflow at the edges of packed and zoned fields;
- division by zero;
- data structure overlays;
- a GOTO/TAG state machine.

Also, for stage 1:

- MONITOR nesting with handlers and GOTO out of a monitored body;
- CABxx in and out of subroutines;
- ENDSR return points in the cycle;
- *PSSR with GOTO;
- FOR-EACH left early;
- RETURN from a subroutine;
- ON-EXIT after an error;
- recursion.

Each program DSPLYs its results. Each expected value comes from IBM's documentation and is cited in the comment format of CLAUDE.native.md section 4, or from a PUB400 capture. It is never taken from a run of either engine. The corpus diff test then also requires the two engines to agree.

## Generator tests (rpgcg, no database)

- **Determinism:** the same IR gives byte-identical source.
- One golden file of generated source per `Ins` kind, under `rpgcg/tests/`. Its expected text is written by hand from the design, not copied from the generator's output.
- Name mapping is deterministic and escapes Rust keywords; the source map holds one entry per instruction.
- The generated source compiles under the section 5 lints (a test crate built from the goldens).

## Tripwires against upstream drift

- The lowering matches `Ins` with no wildcard arm, so a new variant breaks the build.
- A test hashes the text of `run_code_at` and errors.rs's routing functions (`include_str!` of the sources, cut at the function headers with `split_once`, not by slicing: `clippy::string_slice` is forbidden), and compares the hash with a recorded value. When upstream changes them, the test fails, and `native_rt`'s copy of the routing must be reviewed before the recorded hash is updated, with the reason in the commit.
