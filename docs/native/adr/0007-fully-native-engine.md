# ADR 0007: A fully native engine, and native procedure calls

Status: accepted by the user, 2026-11-15 (option A); written 2026-11-15

## Context

The user's requirement (2026-11-15): `AS4TAINR_RPG_ENGINE=rust` must become **100% native**, so that the interpreter could be removed. Both engines stay for now ([PLAN.md](../PLAN.md)), but nothing in the native engine may depend on `interp.rs` in the end.

Today the native engine is not that:

- **Instructions.** Generated code runs every instruction that is not lowered through the interpreter (`rt.stmt`, `rt.truth`, `rt.exsr`... of stage 1). In the development corpus (native-diff and OpenRPG, rpgcg 0.1.0+stage2.28) these were, by count:

  | Instruction | Count |
  | --- | --- |
  | DSPLY | 936 |
  | EVAL not lowered | 544 |
  | CALLP | 192 |
  | ON-ERROR/ENDMON | 157 |
  | EXEC SQL | 92 |

  File operations (CHAIN, READ, WRITE...) and most of the others come after these.
- **Runtime.** `rpgrt::native::Runtime` is implemented by the interpreter's `Machine` (`native_rt.rs`). The program's storage, the PSDS, the open files, the call frames and the error state are the interpreter's.

## Decision

The work goes in two phases. Each step is checked against the interpreter by the differential suite (ADR 0005), as before.

**Phase A: lower everything.** Every instruction and expression gets generated code that does not call the interpreter to run it. A per-routine count of the instructions still dispatched (`rt.stmt` and the other stage 1 entries) is reported by native-diff and native-corpus, with a target of 0.

**Phase B: a runtime of its own.** A native runtime state in `rpgrt`. It holds storage, the PSDS, the INFDS, files on `rla`, the SQL session, workstation and spool output, and call frames. It uses core's services (`rla`, `sql`, `pgmmsg`, `workstn`, `spool`), not `interp.rs`. At the end, a program under `rust` runs without a `Machine`.

**Where the logic comes from: option A (the user's choice).** Behaviour that today exists only in `interp.rs` is written again in our own code. That covers procedure calls, file I/O glue, *PSSR and the PSDS, and assignment rules. Phase A puts it in `native_rt.rs` and `rpgrt`, and phase B in `rpgrt` alone. Upstream code is not moved or edited (CLAUDE.native.md, section 2). The cost is two implementations for a while. The differential suite keeps them equal.

A design that delegates to the interpreter is not an end state. The stage 1 entries are scaffolding, to be removed construct by construct.

## S2.35: native procedure calls (phase A)

**Scope of the first step.** It covers a CALLP, or a call in an expression, to a procedure of the same module (`prog.procs`) that has native code. The procedure must have:

- no file parameters, no `*OMIT` or `*NOPASS` argument, no OPTIONS(*TRIM, *CONVERT, *STRING, *VARSIZE);
- no pointer initializations (`ptr_inz`), no local files, no ON-EXIT;
- every argument an eligible operand (stage 2), with a return type that is eligible.

Anything else stays dispatched and is counted.

**What the generated code does.** It does what `call_named`, `args_for` and `call_proc` do, in their order:

1. **The call stack line.** `pgmmsg::stack_line(job, line)`.
2. **The arguments.** For each parameter in order:
   - **BYREF.** The argument's place (buffer, offset).
   - **CONST.** Not by value, and the argument a variable of the parameter's own type: its place, by reference. Otherwise a value.
   - **VALUE.** The value of the argument, evaluated in order.
   - Too many arguments, or a missing one, is the interpreter's error.
3. **The frame.**
   - The procedure's frame template is appended to storage.
   - Each BYREF parameter's frame buffer is aliased to the caller's place.
   - `passed` counts the arguments, for %PARMS.
   - The descriptor lengths (CEEDOD) are those of `call_proc`.
   - `pgmmsg::stack_push` with the program, library, module, the external procedure name and the activation group.
   - The routine name and the line are saved.
4. **The values.** Each VALUE and CONST value is stored into its parameter, `Fit::Strict` with no half-adjust.
5. **The body.** The callee's generated routine runs under its error guard (`guarded`).
6. **The return.**
   - The call stack entry is popped, the frame dropped, the storage truncated, and the routine name and line restored.
   - A return value is stored through the return type and loaded back. A procedure that returns nothing where a value was expected is the interpreter's error.
7. **After the call.**
   - An error out of the call is marked `called` (status 00202).
   - A normal end sets a program status below 1000 back to 0, with PSDS positions 11-15 (PUB400 verify34).

In phase A the frame is still built on the interpreter's storage, but by our code. The `Runtime` methods `enter_proc` and `leave_proc` are implemented in `native_rt.rs` and do not call `call_proc`. Phase B moves the frame into the native runtime.

**Fingerprints.** A routine that calls a procedure natively depends on how the procedure's parameters are laid out. The parameter list (name, type, BYREF, VALUE or CONST) joins the caller's layout lines, so a change to the prototype changes the caller's key (ADR 0002).

**Tests.** The tests use PUB400 captures for:

- BYREF changes seen by the caller, two parameters aliasing one variable, and a parameter forwarded to another call;
- VALUE copies that stay independent;
- CONST by reference and by temporary;
- calls nested in arithmetic and in return values (CLAUDE.native.md, section 6);
- %PARMS, an error in the callee inside the caller's MONITOR (status 00202), and %STATUS after a normal return.
