# Native RPG backend: plan

Goal: a generator that turns every RPG program the interpreter runs into Rust, statically linked, selected with `AS4TAINR_RPG_ENGINE`, and compared with the interpreter by a differential harness.

Decisions: ADR 0001 (two stages), 0002 (content-addressed routines), 0003 (crates and static registry), 0004 (flag and hooks), 0005 (differential harness), in `docs/native/adr/`.

## Architecture at a glance

```
source ──compile_ovr──▶ Program ─┬─▶ Machine (interpreter) ──run_code_at(code)──┐
  (upstream, unchanged)          │                                              │ hook (ADR 0004)
                                 │                                              ▼
                                 │                          native_rt::dispatch: engine? fingerprint
                                 │                          → rpgnative::ROUTINES → generated fn
                                 │                                              │
                                 └─▶ native::activated (EMIT): lower.rs        │ calls back through
                                       Code ─▶ rpgcg::ir ─▶ rpgcg ─▶ <fp>.rs   │ rpgrt::native::Runtime
                                                                  │             ▼ (Machine adapter)
                              AS4TAINR_NATIVE_DIR ─▶ rpgnative/build.rs ─▶ static ROUTINES table
```

Crates:
- `rpgrt`: the runtime, with the `rpgrt::native` interface;
- `rpgcg`: the generator (depends on rpgrt);
- `rpgnative`: the registry of generated modules (depends on rpgrt);
- `native-tools`: the emit tool (depends on core).

New code in core:
- `core/src/rpg/native/`: the flag, lowering, emission and the `Slot`/`NativeTable` types;
- `core/src/rpg/native_rt.rs`: the `interp` submodule that adapts `Machine`: the dispatcher, `Runtime`, `settle`, `observe`.

The dispatcher finds each routine's generated function through `Program.native` (ADR 0002).

## Stage 1: what the generated code looks like

For a routine whose `Code` is:

```
0 Monitor{handlers:[([],3)], end_body:2, end:4}
1 Stmt(...)  line 12, cond [50]
2 EndMonitor{end:4}
3 EndMonitor{end:4}
```

the generator writes, roughly:

```rust
// Routine fingerprint 3f2a…; program ORDENT, subroutine CHKQTY (map: 3f2a….map.json)
static MONITORS: &[Monitor] = &[Monitor { at: 0, handlers: &[(&[], 3)], end_body: 2, end: 4 }];
static LABELS: &[(&str, usize)] = &[];
pub fn run(rt: &mut dyn Runtime, start: usize) -> NResult<Flow> {
    let info = RoutineInfo { monitors: MONITORS, labels: LABELS };
    let mut mon = MonitorStack::new();
    let mut pc = start;
    loop {
        let step: NResult<Next> = match pc {
            // line 0: MONITOR
            0 => { rt.line(0); mon.push(&info, 0); Ok(Next::To(1)) }
            // line 12: EVAL ... (conditioned)
            1 => { if !rt.cond(1) { pc = 2; continue; } rt.line(1); rt.stmt(1).map(|()| Next::To(2)) }
            // line 14: ON-ERROR
            2 => { rt.line(2); mon.pop(); Ok(Next::To(4)) }
            3 => { rt.line(3); mon.pop(); Ok(Next::To(4)) }
            _ => return Ok(Flow::Normal),
        };
        match rt.settle(pc, step, &info, &mut mon) {
            Settled::Go(t) => pc = t,
            Settled::Exit(r) => return r,
        }
    }
}
```

- Instruction kinds, jump targets, the next pc, conditioning (present or not), the MONITOR table and the TAG table are fixed in the generated code. Next pcs are literals, so there is no arithmetic.
- Every stage-1 routine is a `loop { match pc }` state machine. **This deviates from section 3**, which asks for a state machine "only in procedures that need one" and structured Rust for structured code. Stage 1 is a faithful compilation of `code::Ins`, where structured operations are already jumps, and one shape for every routine keeps parity simple. Stage 2 generates structured Rust for routines without GOTO, TAG, CABxx or cycle return points (phase N6, first item).
- `rt.settle` is the error routing of `run_code_at`. It handles the pointer-fault check (`take_fault`), the Return, LeaveSr, Goto and ReturnPoint signals, `passes_by`, monitor handler matching (`on_error_takes`, `MSG:`), `note_error`, and the `At{line}` context. It is in `native_rt.rs` and written to match `run_code_at` line for line. A tripwire test (ADR 0005) watches the original. The details it must keep, from the review:
  - `take_fault` runs after every step, but not for signals.
  - The monitor search pops the groups that do not contain `pc`, then stops at the first group that does, even when none of its handlers matches.
  - A matching handler: `note_error`, then a jump to the handler, without popping the group.
  - `line == 0`: the error goes out without `At`.
  - GotoSignal: search the whole `Code` for the label, then `retain(to < end_body)`.
  - `exsr` restores `line` afterwards.
  - Returning a whole DS array returns its bytes.
  - CAB with `out` raises GotoSignal before it looks at `to`.
  - ENDSR blanks its field before it signals.
- `Runtime` (in `rpgrt::native`): `cond`, `line`, `stmt`, `truth`, `loop_past`, `loop_step`, `each_init`, `each_next`, `exsr`, `ret`, `leave_sr`, `cab`, `goto_out`, `end_sr`, `settle`. Each method takes the pc. The adapter holds the `&Code`, so it reaches the AST, and the FOR-EACH key keeps `code.as_ptr()` as the interpreter's does.

## Phases

Each phase is one or more small commits on its own `native/<topic>` branch, rebased on upstream, with the section 7 checks, evidence (section 8), STATUS.md and HANDOFF.md updated. "Done" means its acceptance criteria hold and the evidence is recorded.

| Phase | Content | Acceptance |
| --- | --- | --- |
| **N0** Plan | These documents; review; the user's approval of the open points below | User approval |
| **N1** Flag and hooks | `rust-toolchain.toml` (1.99.1); `core/src/rpg/native/{mod,engine}.rs` (flag, `with_engine` carried into `spawn_blocking`, `Slot`/`NativeTable`, `annotate`); the `Program.native` field; the interp.rs submodule and the `run_code_at` hook with a dispatcher; `rpgrt::native` types; the `rpgnative` crate with `build.rs` and an empty registry | Upstream suite unchanged with the flag unset and with `interp`. Under `rust` a program fails at its first routine entry, before any calculation, with an error naming the variable, the program and the routine. CRTPGM, CRTSRVPGM and bound resolution still work under `rust`. `auto` runs the interpreter and logs once. A bad value is an error naming the variable. Tests for each case |
| **N2** Generator | `rpgcg` (IR, generation, source map, names); `core/src/rpg/native/lower.rs` (`Code` → IR, exhaustive over `Ins`); fingerprints (ring SHA-256); emission (`AS4TAINR_NATIVE_EMIT`) | rpgcg tests: determinism, a hand-written golden per `Ins` kind, names, source map; generated goldens compile under section 5's lints. Emission is atomic and idempotent |
| **N3** Runtime adapter | `native_rt.rs`: `Runtime` for `Machine`, `settle`, the dispatcher with its bounded validated cache; the registry lookup; tripwire tests | The native unit-test programs (ADR 0005) pass under `rust` with their documented expected values. The same programs give identical observations under both engines |
| **N4** Differential harness | `core/tests/native_diff.rs` (corpus), observation, `scripts/native-diff.sh` (levels 1-3), evidence manifest | Level 1: all corpus programs identical. Level 2: the whole upstream suite has the same per-test results under `interp` and `rust`, every program native. Level 3: demo identical and golden |
| **N5** Build tooling and baseline | `native-tools` (`as4tainr-native-emit --lib …`), `scripts/native-build.sh`, docs for production images (the binary only); first performance baseline (section 5: throughput, p50/p95/p99, CPU, memory, allocations, DB round trips) on both machines | A production-style build of the samples runs under `rust` with no `cargo`/`rustc` present; baseline recorded |
| **N6…** Stage 2 | Lowering, one construct per step (order below) | Each step: section 7 checks plus native-diff green; fingerprint covers the new dependencies; performance before/after recorded; fidelity.md updated |

### Stage 2 order (first estimate, revised by measurement)

1. Conditioning indicators and indicator conditions (direct access to the indicator state through narrow `Runtime` accessors).
2. Comparisons of fields with literals and fields (JumpIfNot, LoopTest, CAB) for character and numeric types.
3. Numeric EVAL and fixed-form arithmetic (ADD, SUB, MULT, DIV/MVR, Z-ADD, Z-SUB) on packed, zoned and binary fields. This needs the precision rules extracted into `rpgrt`, which is an upstream change needing approval.
4. Character assignment, MOVE/MOVEL, data structure overlays (byte copies over `mem`).
5. Pure built-in functions (string and numeric, then dates over `rpgrt::dates`).
6. Loops (FOR/DO) with lowered index arithmetic.
7. Calls between procedures of the same program with BYREF, VALUE and CONST (frames over `mem`).

File I/O, workstation, printer, SQL, system APIs and dynamic calls stay as `Runtime` calls into `Machine`'s services. They are dominated by the database and the screen, and duplicating them would add risk without speed. To retire the interpreter (below), those calls must still stop passing ASTs: the services take the values the generated code has evaluated.

## Definition of "complete"

- **Coverage:** every program the interpreter runs runs under `rust`. That comes from stage 1, and level 2 of the harness proves it.
- **Parity:** no difference on the three harness levels.
- **Code generation:** every construct in the stage 2 list lowered. The constructs left as runtime calls are listed in `docs/native/fidelity.md` with the reason.

## Final phase: retiring the interpreter (the user's goal, 2026-11-11)

Once the native engine is proven, the interpreter is to be removed. "Removed" means that the evaluation of the AST and `run_code_at` go:

- `Machine::eval`, `stmt_inner`, `run_code_at` and the dispatcher.

`Machine`'s run-time services stay, as the library generated code calls:

- files through `rla`, the workstation, printers, embedded SQL;
- calls, activation groups, system APIs and the C and CEE APIs;
- the PSDS and INFDS, *PSSR and INFSR.

Prerequisites, each its own decision when the time comes:

1. **Stage 2 complete.** No generated code calls `Runtime::stmt`, `truth` or any other method that takes an AST. The services take values. That means the services' signatures move from AST arguments to evaluated values, which is an upstream refactoring of interp.rs.
2. **Programs created at run time.** CRTBNDRPG followed by a CALL, the test suite's inline sources, applications that generate and compile RPG at run time (rpgle-repl, `AS4TAINR_DUMP_SOURCE`). Without the interpreter, and with no `rustc`, `cargo` or `dlopen` in production (sections 3 and 5), these cannot run until the binary is rebuilt. A solution needs an ADR. Options to weigh then: keep a reduced interpreter only for programs created after the build, a sanctioned loading mechanism, or a build service.
3. **A parity reference.** The interpreter is today's reference for parity. Before removing it:
   - record each harness observation as a committed baseline;
   - keep the upstream suite's expected values and the golden captures as the truth.
4. **Upstream.** Upstream keeps its interpreter, so removing it turns the fork into a divergent one, and rebasing on upstream stops being routine. This needs the user's decision on how to follow upstream after that point.
5. **The definition of complete above becomes:** every construct lowered, no AST evaluation left, and the services called only with values.

## Risks

| Risk | Mitigation |
| --- | --- |
| `settle` drifts from `run_code_at` after an upstream change | Tripwire hash test; the routing has unit tests through the native unit-test programs; level 2 runs the whole suite |
| `Debug` text changes with a rustc upgrade | rustc version in the fingerprint; pinned toolchain; failures are fallbacks, never wrong code |
| Cost of fingerprints and lookup | Fingerprints are computed once per compile and shared by the clones (`Program.native`); the lookup is pointer comparisons within the live `Program`. Measured in N3/N5 |
| Stack depth: generated frames add to the interpreter's recursion (procedures up to 1000 frames; `spawn_blocking` threads) | Native unit test with deep recursion under both engines; measure the stack per level in N3 |
| `rpgnative` build time with many programs | Stage 1 functions are small; split the registry into several crates if needed (ADR 0003) |
| The machines have 2 CPUs and 3.8 GB | `CARGO_BUILD_JOBS=1`, checks outside the session; the two machines in parallel (one per platform) |
| Upstream commits many times a day | Small hooks; rebase before each phase; tripwires |
| Pre-existing upstream failures (clippy in the host servers, an intermittent `save_changed_objects`) | Recorded as such in each manifest, compared against `main`, never hidden |

## Review

An independent review (Fable 5.1, 2026-11-11) checked the design against the code. It found the hook location, `settle`'s feasibility and the registry crate sound. It also found two blockers and five major issues, all now addressed:

- **Blockers:**
  - the test override did not cross `spawn_blocking`;
  - failing in `compile_ovr` would have broken CRTPGM, CRTSRVPGM and bound resolution under `rust`.
- **Major issues:**
  - per-entry validation and per-CALL fingerprinting cost (now the `Program.native` field);
  - global state and clock in the corpus test;
  - the undeclared state-machine deviation;
  - `observe` needing `Machine`'s private fields (now in `native_rt`);
  - build cost.

## Open points for the user

1. Approve the two upstream changes outside the touch points (ADR 0004):
   - the `run_code_at` hook: one statement in interp.rs. Without it, no native code can run;
   - the `Program.native` field: two lines in program.rs. Without it, the dispatcher needs a deep comparison per routine entry, which is slower than the interpreter.
2. Agree to the deviations from CLAUDE.native.md, and to updating the guide:
   - routines keyed by the compiled code's fingerprint, not a hash of the inputs (ADR 0002);
   - one registry crate built from generated modules, instead of a crate per program and a second binary (ADR 0003);
   - `overflow-checks` only for the generated code (ADR 0004);
   - a state machine for every routine in stage 1 (above);
   - source lines without member and sequence number.
3. New crates: `rpgcg`, `rpgnative`, `native-tools`. No new external dependencies.
4. Pin the toolchain to 1.99.1 (`rust-toolchain.toml`), the version upstream CI uses today.
