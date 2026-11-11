# ADR 0004: The engine flag, generation and the hooks in upstream code

Status: accepted by the user (2026-11-11), after an independent review

## The flag

`AS4TAINR_RPG_ENGINE`, read once per process (`OnceLock`).

| Value | Behavior |
| --- | --- |
| unset or `interp` | Upstream behavior. The dispatcher returns at once and nothing else runs, apart from filling `Program.native` (ADR 0002), which happens only when an engine other than `interp` or emission is asked for |
| `rust` | Every routine of the program must have native code. The check is made at the program's first `run_code_at` entry, before any calculation runs. Otherwise the error names the variable, the program, the routine and its first source line |
| `auto` | Native code when every routine of the program has it. Otherwise the whole program runs in the interpreter, and the reason goes to standard error once per program and process (a bounded set) |
| anything else | An error at the first `run_code_at` entry that names the variable and its valid values |

- The decision is per program (per `Program`, from its `NativeTable`), so a program never mixes engines. A called program decides again for itself.
- **Errors are raised at the first entry, not in `compile_ovr`.** `compile_with` → `compile_ovr` also compiles programs that never run: `ile::module_info` (ile.rs:233, used by CRTPGM and CRTSRVPGM), `usrspc.rs:692`, `seed_bound_modules` (bifs.rs:1094), and the export searches in `resolve_bound` (interp.rs:7373) and `call_procedure` (mod.rs:958). Failing there would break those commands.
- The old upstream value `native` (from before Jariko was removed, `fe21ca1`) is not read by any code now. It gets the same error, with a hint that it is no longer used. `docs/rpg-engine.md:109-126` still mentions it.
- **Test override.** `native::with_engine(engine, f)` is thread-local. RPG runs on a `spawn_blocking` thread (mod.rs:807, 929), so `call_any` and `call_procedure` capture the override before `spawn_blocking` and set it again inside, as they already do for QTEMP (`qtemp::current()` / `ThreadScope`, mod.rs:803-809). Calls between programs inside a `Machine` stay on that thread. Nothing outside tests sets it.

## Generation

`AS4TAINR_NATIVE_EMIT=<dir>`. When `compile_ovr` fills `Program.native` on a cache miss, it also lowers each routine to `rpgcg::ir` and generates `<dir>/<fingerprint>.rs`, with `<fingerprint>.map.json` beside it: the program, the routine, and each instruction's source line. Section 3 asks for "member and sequence number", but `I.line` holds only the line; that limit is recorded in `fidelity.md`.

- Files are written to a temporary name and renamed into place. A file that already exists is skipped. This happens once per compile, so not on every CALL.
- Emission works under any engine.
- `native-tools` (`as4tainr-native-emit`; a binary crate that depends on core and needs the database) compiles every RPG program and module in a set of libraries.

## Hooks in upstream code

| File | Change | Status |
| --- | --- | --- |
| `core/src/rpg/interp.rs` | `#[path = "native_rt.rs"] pub(crate) mod native_rt;`. The new file implements `rpgrt::native::Runtime` for `Machine`, the dispatcher and `observe`. As a child of `interp` it reaches `Machine`'s private items (`bufs`, `inds`, `frames`, the PSDS), `Flow`, and errors.rs's `pub(super)` functions | authorized |
| `core/src/rpg/interp.rs` | First statement of `run_code_at`: `if let Some(r) = native_rt::dispatch(self, code, start) { return r; }` | **needs approval** |
| `core/src/rpg/program.rs` | Field `pub native: native::Slot` on `Program` and `native: Default::default()` in `bind` (ADR 0002) | **needs approval** |
| `core/src/rpg/mod.rs` | `compile_ovr` miss path: `native::annotate(&mut p)` before the cache insert. `call_any` / `call_procedure`: carry the engine override into `spawn_blocking`. `call_any` after the run: `native_rt::observe(&m)` when `AS4TAINR_NATIVE_OBSERVE` is set. `pub mod native;` | touch point (section 3) |
| `Cargo.toml` | Members `rpgcg`, `rpgnative`, `native-tools`; `[profile.release.package.rpgnative] overflow-checks = true` | touch point |
| `core/Cargo.toml` | Dependencies `rpgcg`, `rpgnative` | as for `rpgrt` |

- **The `run_code_at` hook.** Every RPG routine runs through `run_code_at`. Its callers are EXSR (interp.rs:4119), *PSSR/INFSR (errors.rs:861), `guarded` (errors.rs:886), *INZSR and the cycle's detail and total calculations (cycle.rs:416, 501, 538), and ON-EXIT (interp.rs:7988); procedure bodies go through `guarded`. The review checked that nothing else executes `Ins`. Restarts after a GOTO from an error subroutine come in with a non-zero `start` and a new monitor stack, which `run(rt, start)` reproduces.
- **`overflow-checks`** is scoped to the generated code. Turning it on for the whole release profile would change upstream's release behavior.

## Consequences

- With the flag unset, upstream behavior is unchanged; the upstream suite checks that.
- A rebase can conflict only on these few lines.
