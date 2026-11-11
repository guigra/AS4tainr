# ADR 0003: Crates, the run-time interface and a static registry

Status: accepted by the user (2026-11-11), after an independent review

## Context

CLAUDE.native.md's first design: each generated program is its own crate; a `programs` crate depends on all of them and registers them in a safe table; `as4tainr` is rebuilt with it. No `unsafe`, no `dlopen`, no `rustc` in the production image.

Three facts make that literal design hard to use:

1. **Generated code needs `Machine`, which is in `as4tainr-core`.** If generated crates depend on `core` and `core` (or its binary) depends on them, that is a dependency cycle, which Cargo rejects. Avoiding the cycle needs a second binary that reuses `core/src/main.rs`. That is possible, since main.rs uses only `as4tainr_core::` paths, but its `main`, `run` and `Cli` are private. The second binary also does not help the next point.
2. **The upstream suite runs RPG inside test binaries, not through the `as4tainr` binary.** `core/tests/rla_test.rs` has about 137 RPG tests that call `rpg::call` and `commands::Job` directly. A registry that only a second binary fills would leave the best parity test (the whole upstream suite under the native engine) out of reach.
3. **A crate per program changes `Cargo.toml` and `Cargo.lock` at every generation**, and `--locked` then fails (section 5 requires it).

## Decision

The crates (`rpgrt` exists; the other three are new):

| Crate | Depends on | What |
| --- | --- | --- |
| `rpgrt` (exists) | anyhow, ethnum | adds `rpgrt::native`: the `Runtime` trait the generated code calls, `Flow`, the MONITOR stack types, and the native error type |
| `rpgcg` | rpgrt | the generator: an IR in (`rpgcg::ir`, built from `Code` by `core/src/rpg/native/lower.rs`), Rust source plus a source map out. Deterministic, no I/O |
| `native-tools` | core | `as4tainr-native-emit`: compiles the programs and modules of a set of libraries with emission on (needs the database) |
| `rpgnative` | rpgrt | the registry. Its `build.rs` reads `AS4TAINR_NATIVE_DIR`. When that is unset or empty, the registry is empty. Otherwise it writes `OUT_DIR/programs.rs`: one `#[path = "<dir>/<fingerprint>.rs"] mod r_<fingerprint>;` per generated file, in sorted order, and a sorted `static ROUTINES: &[(&str, RoutineFn)]` |

`as4tainr-core` depends on `rpgcg` (to generate, ADR 0004) and on `rpgnative` (to look up). There is no cycle: generated code sees only `rpgrt::native::Runtime`, a trait object, and `Machine` implements it inside `core`.

- Every binary and test that links `core` gets the registry: the `as4tainr` binary, `rla_test`, `system_cli`. Building with `AS4TAINR_NATIVE_DIR` set makes the whole upstream suite runnable under `AS4TAINR_RPG_ENGINE=rust` without editing a test.
- `Cargo.toml` and `Cargo.lock` never change when programs are generated. Only `rpgnative`'s build output does. `cargo:rerun-if-env-changed=AS4TAINR_NATIVE_DIR` and `rerun-if-changed=<dir>` rebuild that crate when the generated code changes, and with it everything that depends on it: `core`, the binary and the test binaries. ADR 0005 accounts for that build cost.
- Linking is static and safe. The table holds plain `fn` items: no `unsafe`, no `dlopen`, no new dependency.
- `rpgnative`'s crate root carries the section 5 `#![forbid(...)]` lints, so they apply to the generated code itself.
- Production: generate from the application's programs (ADR 0004, `AS4TAINR_NATIVE_EMIT`), then build the image with `AS4TAINR_NATIVE_DIR` pointing at the generated directory. The image has the binary only, no `rustc` and no `cargo`. A program created after that build runs in the interpreter in `auto` mode.
- The generated directory lives under `target/rpg-native/<build>/` and is never committed.

## Consequences

- This deviates from the guide's per-program crates and second binary. The guide should say "a registry crate built from generated modules", with the user's agreement.
- Every program's generated code is compiled into one crate. Stage 1 functions are small. If stage 2 makes that crate's build time a problem, the registry can be split into several crates later without changing the interface.
- `core/Cargo.toml` gets two dependency lines (`rpgcg`, `rpgnative`), like the existing `rpgrt` line.
