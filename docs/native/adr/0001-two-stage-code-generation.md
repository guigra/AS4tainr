# ADR 0001: Code generation in two stages

Status: accepted by the user (2026-11-11)

## Context

The interpreter runs a checked `Program` in two layers:

- **Control flow.** Each routine is a `code::Code` (`Vec<I>`), built by `code::build` from the calculations. The routines are the main detail and total calculations, the subroutines, and each procedure's body and ON-EXIT. `Machine::run_code_at` (interp.rs) runs it. It holds a program counter and a stack of MONITOR groups, and handles conditioning indicators, jumps, loops, EXSR, CABxx, RETURN, LEAVESR, ENDSR return points and GOTO out of a subroutine. It also routes every error: to the innermost matching ON-ERROR handler, or out of the routine with the source line attached.
- **Statements and expressions.** `Machine::stmt_e`/`stmt` and `Machine::eval` handle them, on the AST, over the program's storage (`mem::Mem`: bytes in IBM i formats). This is about 10,000 lines of opcodes, built-in functions, file I/O through `rla`, workstation, printer, embedded SQL, calls, and system and C APIs.

The goal is a generator that runs every program the interpreter runs, with identical results, and can be switched and compared with the interpreter.

## Decision

Two stages.

**Stage 1: compiled control flow.** For every `Code`, the generator emits one Rust function. Each instruction becomes a `match` arm (`loop { match pc { … } }`), and the kind of instruction, its jump targets, whether it has conditioning indicators, and its source line are fixed in the generated code. The function calls a narrow run-time interface (`rpgrt::native::Runtime`, ADR 0003) for:

- each statement;
- each expression the instruction evaluates (a condition, a loop limit, a CAB comparison);
- the conditioning-indicator test;
- the error routing.

`Machine` implements that interface, reusing exactly the code the interpreter runs. Every program is covered at once, and parity rests on shared semantics, not on a reimplementation.

**Stage 2: lowering, one construct at a time.** Calls into `Runtime` are replaced by direct Rust over `rpgrt` (`dec`, `data`, `mem`, `dates`) where the semantics can be expressed without the interpreter. Candidates are conditioning indicators, conditions on indicators and simple comparisons, numeric EVAL and the fixed-form arithmetic opcodes, MOVE/MOVEL, and pure built-in functions. Each construct lands on its own:

- with the full differential suite green (ADR 0005);
- with its fingerprint covering everything it now depends on (ADR 0002);
- with performance measured before and after (CLAUDE.native.md section 5).

When a lowering needs interpreter logic that only exists as a private `Machine` method (for example the precision rules `num_precision` and `to_rule_scale`, or `divide`), it first needs a pure-move extraction of that logic into `rpgrt`. That touches `interp.rs` beyond the touch points, so each one needs the user's approval.

## Consequences

- Stage 1 brings no meaningful speed. Its value is the whole pipeline: generation, keys, linking, switching and the differential harness. It proves all of it on 100% of programs before any semantics are reimplemented.
- **Stage 1 is not a stub.** The control flow, including MONITOR stacks, GOTO/TAG state machines and signals, really is compiled. A wrong jump target or a wrong monitor nesting shows up as a difference.
- **No construct is unsupported in stage 1.** Every `Ins` variant is generated. The lowering's `match` over `Ins` has no wildcard arm, so a variant added upstream breaks the build of `core/src/rpg/native/` until the generator handles it.
- Stage 2 is open-ended. "Complete" means every construct lowered. That is a long series of verified steps, and the intended order is in `docs/native/PLAN.md`.
