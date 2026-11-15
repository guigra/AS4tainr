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

## S2.35 as implemented (rpgcg 0.1.0+stage2.35)

- `Runtime::call` (rpgrt) and its implementation in `native_rt.rs`, `call` and `call_frame`, are the fork's own code. They follow `call_named` and `call_proc` step by step for the procedures lowering lets through, and call neither of them.
- The callee's routine runs through the native dispatcher. Under `auto` with no generated code for it, the interpreter runs the routine, as it runs any routine there.
- An error from the callee's code is noted as `guarded` and `recover` note it for a procedure with no *PSSR (`note_error`). That error routing is still the interpreter's, the same dependency every routine has through `settle`. It is a later phase A step.
- Lowering (`lower.rs`, `call_of`):
  - CALLP statements, and calls in numeric and character expressions, of procedures of the module;
  - the exclusions above;
  - BYREF needs a field or array element of the parameter's own type;
  - CONST goes by reference for a field of the parameter's type with no index (the interpreter's `same_type_variable`), and by value otherwise;
  - VALUE goes by value, numeric for a numeric parameter and character for a character one.
- `Runtime::precision` follows a path into a call's arguments, and starts from a CALLP's expression.
- The caller's layout gets a `call` line with each parameter's name, type and kind, so a change to the procedure's interface changes the caller's key.
- Coverage of the native units: interp 534 → 511. CALLP and the EVALs with calls are lowered (fallback only when `native_ok` is false).

## S2.36: DSPLY with no response field (rpgcg 0.1.0+stage2.36)

The interpreter's `Stmt::Dsply` with no response field (its batch path) puts `text_of(message)` into the job's messages, with trailing blanks removed. `text_of` gives:
- a character value as text (`data::a`);
- a number as `char_of_num`: %CHAR's form with a period, ignoring DECEDIT.

Generated code builds the same text from an eligible character or numeric expression. That uses `rpgrt::native::char_of_num(d, '.', false)`, written again for S2.13 and equal to `char_of_num_edit`. `Runtime::dsply` stores it. Not lowered: a response field (the interactive path), and float, date, time and indicator messages, which are not eligible expressions.

Coverage of the native units after S2.36: native 18.1%, fallback 73.7%, interp 8.2%. Almost everything is now lowered; what keeps it from being native is the `native_ok` fallback. That is the next phase A step.

## S2.37: no interpreter fallback in lowered code (rpgcg 0.1.0+stage2.37)

Since S2.8, lowered code asked `native_ok` and let the interpreter run the instruction outside the routine's own context. That context is a main procedure routine with no procedure frame, or a procedure's routine under its own frame. The only way the interpreter runs a routine elsewhere is a subprocedure's EXSR of a main procedure subroutine (`Ins::ExSr` looks there after the procedure's own subroutines). IBM i does not compile that: RNF7030 and RNF7018 at severity 30 (PUB400 capture tools/golden/native45; fidelity.md).

So the context is checked once, when the dispatcher enters a routine:
- **The routine's own context.** Generated code runs, and every lowered instruction in it is native only (`via` "native").
- **Any other context.** The routine is refused as a whole. The interpreter runs it under `auto`, with the refusal noted. Under `rust` it is an error that names the reason.

rpgcg's own tests of lowered code now check that no fallback is generated: no `native_ok`, no `rt.stmt` or `rt.truth` for a lowered instruction. Before, they checked that the fallback was there, which was the earlier design.

Coverage of the native units: native 91.8%, interp 8.2% (94 instructions: EVAL forms not lowered yet, RETURN, DSPLY of other types, CABxx, FOR-EACH). Every routine still settles errors through the interpreter.

## S2.38: RETURN (rpgcg 0.1.0+stage2.38)

The interpreter's RETURN evaluates its value, if it has one, and raises `ReturnSignal`. `settle` turns that into the routine's end with `Flow::Return(value)`, whatever MONITOR groups are open. Lowered code evaluates an eligible numeric or character value and ends the routine with `Flow::Return` itself. Not lowered: a whole data structure array, and values of other types. `settle`'s pointer-fault check does not apply, because eligible operands hold no pointers.

Coverage of the native units: native 93.1%, interp 6.9%.

## S2.39: indicators (rpgcg 0.1.0+stage2.39)

- **EVAL of an indicator.** Stmt::Eval evaluates the value, and `assign` does `set_ind(n, val.truth()?)`. Lowered for `*ON`, `*OFF` and an eligible condition, which gives `Value::Ind`.
- **SETON and SETOFF.** Stmt::SetInd calls `set_ind` for each indicator.
- **An indicator as a condition, and NOT.** In eval, `Expr::Ind` is `Value::Ind(self.ind(n))`, and NOT is `Value::Ind(!v.truth()?)`.
- **Our own code.** `Runtime::ind` and `Runtime::set_ind` (native_rt.rs) are written again from `Machine::ind` and `set_ind`. That includes the overflow indicator rule (the file's overflow lines due again) and `ind_name`.
- **Not lowered.** `*IN(i)` and `*IN` as an array.
- **Expected values.** `core/tests/native/NIND.rpgle`, captured on PUB400: tools/golden/native46.txt n46_ind. The capture also showed that IBM i does not compile an indicator used in a condition but set nowhere (fidelity.md).
- **Coverage of the native units.** Native 97.2%, interp 2.8%.
- **The `AS4TAINR_NATIVE_WHY=<file>` diagnostic.** It writes the shape of every EVAL that is not lowered. On the OpenRPG corpus at S2.38, indicator targets came first (260 of 583).

## S2.40: EXSR, LEAVESR and GOTO out of a subroutine (rpgcg 0.1.0+stage2.40)

`Runtime::exsr` (native_rt.rs, the fork's code) looks up the subroutine as the interpreter's `Ins::ExSr` does and keeps the routine name and the line. It now runs the subroutine's generated code through the native dispatcher. Only under `auto`, with no generated code, does the interpreter run it, as for any routine; `run_code` is the same dispatch first. LEAVESR and a GOTO out of a subroutine raise the interpreter's signals, which `settle` handles. None of the three evaluates anything with the interpreter, so the source maps count them as native. CABxx and ENDSR with a return point still evaluate their operands with the interpreter.

On the OpenRPG corpus at S2.39, before this step: native 79.3%, interp 20.7% (EVAL 310, DSPLY 151, conditions 82, MOVE 53, EXSR 36, CALL 20, CALLP 19).

## Errors stay anyhow (the user, 2026-11-15)

CLAUDE.native.md section 5 asks for typed errors. `rpgrt::native` (`NResult<T> = anyhow::Result<T>`) and `native_rt.rs` keep `anyhow`, as ADR 0003 had it. An RPG run-time error travels as the interpreter's message with its RNQ/RNX ID (`rpg::errors::message_id`, `status_code`), and `settle` and the interpreter's handlers read it there. The user chose to keep this as a documented exception, option 2 of the 2026-11-15 audit, over a native error type converted at the interpreter's edge. rpgcg keeps its typed `Error`. Tests may use `unwrap` (also the user's decision).

## S2.41: chains with no recursion, and IBM i's nesting limit (rpgcg 0.1.0+stage2.41)

S2.40.1 had bounded lowering's recursion with a depth cut-off of 64 that left deeper expressions to the interpreter. That was not native. Now:

- **Chains are lists.** A chain down the left operands is walked with a loop and generated as straight-line code: +, - and * (`NumExpr::Chain`), concatenation (`CharExpr::Chain`), and the same AND or OR (`Cond::Chain`).
  - Numbers: a running value, then each operation with its own precision, asked of the runtime at its node's path as every Bin's helper does.
  - Characters: the parts appended in order.
  - Conditions: evaluated in order, AND stopping at the first false and OR at the first true, as the interpreter's short circuit.
  - The order and the precision are the interpreter's. Long paths are written as runs (`rpgrt::native::path_rle`), and helper names use `z<n>l`/`z<n>r` for runs, so code size stays linear in the chain's length.
- **IBM i's nesting limit.** The ILE RPG Reference sets "Levels of nesting in expressions: Maximum of 100" (Appendix A. RPG IV Restrictions, IBM i 7.4 and 7.5). A chain counts as one level. An instruction nested more than 100 levels makes the generator refuse its routine (`ir::Ins::refused`, `rpgcg::Error::Refused`) with a diagnostic naming the line. Under `auto` the interpreter runs the program; under `rust` it is an error. Either way the program is not run half natively. Recursion is now only into nested levels, at most 100 deep.
- **A resource limit.** At most 2048 expression nodes per instruction, counted with an explicit stack (`lower::MAX_NODES`). It bounds paths and code size.
- **Stack size.** The as4tainr binary gives its runtime 64 MiB thread stacks (`main.rs`). The two tests that compile deep or long expressions run with the same stack: NCHAIN, and the nesting test in native_coverage.rs. The interpreter evaluates and checks expressions recursively, and #[tokio::test]'s 2 MiB is too small for it.
- **Tests.**
  - NCHAIN: a sum of 200 terms, a concatenation of 150 parts, AND and OR of 120 conditions, and 100 mixed terms. The expected values come from IBM's documented precedence and exact integer arithmetic.
  - native_coverage.rs: 100 levels are generated, 101 are refused. Each unit's count of interpreter instructions may not rise (a lowering that stops applying shows there, not in a parity test).
- **The interpreter, for the record (fidelity.md).** It compiles expressions nested past 100 levels. A source with 20 000 nested parentheses overflows the parser's stack and aborts the process, even with 64 MiB.

## S2.42: named constants in expressions (rpgcg 0.1.0+stage2.41)

A named constant (`dcl-c DIGITS '0123456789'`) of character or numeric value is the operand of its value. The interpreter's eval gives `Sym::Const(c) => c`, so it is the same operand as the literal it stands for. Other constant values (a date, for example) are not lowered. NCHECK, which uses one, now has no instruction left with the interpreter, and its coverage ceiling is 0. The constant's value goes into the routine's layout lines: the compiled code names the constant, not its value, so two programs with the same calculations and a different constant (one included from another source) would otherwise share a fingerprint and the same generated code. native-diff found this through rla_test include_search_order_follows_source_kind, and NCONSTA and NCONSTB now test it.

## S2.43: the error path for program errors (rpgcg 0.1.0+stage2.41)

`settle` and the native call's body note and route an error with the fork's own code. These are written again, step by step:
- from errors.rs: `note_error`, `psds_error`, `passes_by` and the status data structure helpers `ds_put`, `ds_get`, `ds_text` and `ds_zoned`;
- from interp.rs: `sender`.

That code sets `%STATUS` and `%ERROR`, and the PSDS's previous and current status, statement, routine, exception ID and data. The exception message goes to the handling procedure's call message queue, with its sender, the RPG run-time's logged sender for RNX messages, and the job log.

A file error (status 1000 and up) still goes to the interpreter's `note_error`. Its PSDS file fields come from the interpreter's file state (`infds_feedback`), and phase A's file operations replace that state.

native_tripwire.rs now watches every interpreter function the native runtime writes again since S2.35: calls, DSPLY's text, `%CHAR` of numbers, indicators, `run_code`, and the error path. Its hash was recorded after a review on 2026-11-15. Upstream's errors.rs and bifs.rs are unchanged, and interp.rs differs only by the authorized hook.

## S2.44: %SUBST as the target of an EVAL (rpgcg 0.1.0+stage2.44)

The interpreter evaluates the value first (Stmt::Eval). Then its `assign_value` "SUBST" arm:
1. loads the field and evaluates the start and the length (with no length, to the end);
2. raises RNQ0100 for a start of 0 or a part past the end;
3. stores the value as a character field of the part's length;
4. stores the field again.

Generated code does the same in that order, on an eligible character field with a character value; `rpgrt::native::subst_target` is the arm's middle part.

The interpreter turns a negative start or length into an unsigned number. That gives RNQ0100 in a release build and a panic in a debug build. Here it is RNQ0100, never a panic.

The start and length are expressions of the target, not of the value, so their precision paths start with `rpgrt::native::TARGET_PATH`, and `Runtime::precision` follows them into the EVAL's target.

Expected values come from IBM's documentation ("%SUBST Used as the Result of an Assignment", IBM i 7.5): its example `%SUBST(A:3:4) = '****'`, any expression for the start and length, no length meaning to the end, and a run-time error for a length past the end. PUB400 was unreachable, so there was no new capture. NSUBT; coverage ceiling 0.

## S2.45: varying-dimension arrays (rpgcg 0.1.0+stage2.45)

DIM(*AUTO:n) and DIM(*VAR:n) arrays, global, with elements of an eligible type:
- **Reading an element** (the interpreter's eval of an indexed name): the index must be 1 to the elements in use, then within the dimension. `rpgrt::native::elem_in_use` does this.
- **Writing an element** (the interpreter's assign): an index past the elements in use grows a DIM(*AUTO) array to it, within the dimension, before the store. Otherwise it is RNQ0121. `rpgrt::native::elem_grow` does this, and the count is set through `Runtime::set_elem_count`.
- **A literal index** cannot be placed at compile time here, so it takes the run-time path.
- **`%ELEM(arr)` as a value** is the elements in use (bifs.rs bif_override "ELEM").
- **`%ELEM(arr) = n`** follows assign_value's "ELEM" arm: a count outside 0 to the dimension is RNQ0121, and the elements from those in use to n get their initial value (`rpgrt::native::set_elems`).
- **The array's kind** goes into the layout lines.
- **Not lowered:** `*NEXT`, local arrays, data structure arrays.

Expected values come from PUB400 (tools/golden/native47b.txt n47_var): growth, `*NEXT`, indexes past the elements in use, `%ELEM` read and set, and initial values after shrinking and growing again. On IBM i an index past a DIM(*AUTO) array's dimension, and `%ELEM` past it, are status 00124. The interpreter gives 00121, so NVARDIM leaves those two cases out (fidelity.md). The capture's first version showed that IBM i also rejects such constants at compile time (RNF0530, RNF7563). NSUBT2 adds the %SUBST-target cases the documentation does not give (S2.44).

## Calls to other programs and service programs (the user, 2026-11-15)

The OpenRPG and native-diff corpora's remaining CALLPs are almost all to prototypes: programs (EXTPGM) and procedures of service programs (181 of 190, AS4TAINR_NATIVE_WHY). The user chose this approach:
- **Now (phase A).** The fork's own code prepares the arguments and the call natively: the prototype's parameters, by reference and by value, as `call_named` and `args_for` do. The program-to-program call itself, `Machine::call_program` and `call_bound_in`, is used as a system service. It resolves the program in the library list, compiles or caches it, handles activation groups, and copies parameters back. It evaluates no RPG, and the program called runs its own generated code.
- **The final goal (phase B).** That service is written again in rpgrt with no Machine, like files, SQL and spool.

Until then, the source maps count such a call as native, with a phase B dependency on the call service, recorded here.

## S2.46: calls through prototypes (rpgcg 0.1.0+stage2.45)

A CALLP, or a call in an expression, through a prototype is lowered when the prototype is one of these:
- a program (EXTPGM named by a literal or a constant);
- a procedure bound into the program, from another module or a service program;
- this module's procedure under another name (EXTPROC).

Procedure pointers, overloads, CEE APIs, C runtime functions and an EXTPGM named by a variable are not lowered.

Lowering takes the prototype's parameters with S2.35's rules. `Runtime::call` (native_rt.rs) picks the path, as call_named_inner does:
1. a procedure of the module by that name goes through S2.35's frame;
2. otherwise `call_proto` runs:
   - for a program, each value becomes a copy in its parameter's type, then the call goes through `call_program`;
   - otherwise the named procedure is this module's (S2.35's frame), or bound (`call_bound`), or else it is CPD5D02.

`call_program` and `call_bound` are the program call service. They are a phase B dependency (the user's decision above). The prototype goes into the caller's layout lines. native_tripwire.rs watches call_named_inner.

The native units have no calls between programs. The upstream suite's cross-program and service program tests check this under native-diff level 2.
