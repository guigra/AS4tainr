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

## S2.47: pointer fields and %ADDR (rpgcg 0.1.0+stage2.47)

- **Pointer fields as operands.** Global or local pointer fields (`ir::Ty::Pointer`, 16 bytes in rpgrt::rpg::mem::Ptr's encoding) can be used in three places:
  - EVAL of a pointer, from another pointer field or from %ADDR;
  - EVAL of *NULL (EvalFig with `FigOp::Null`);
  - comparisons with each other or with *NULL (data::compare's pointer order).
- **Kept out.** Pointers are not numeric operands, not in MOVE, and not in the generic EVAL.
- **%ADDR.** %ADDR of a global field, element or data structure, and %ADDR(varying:*DATA) past the length prefix, are ptrs.rs `ptr_to` of the field's place. `Runtime::addr_of` writes it again: lent storage, the heap, the module's globals (its activation's) and the frame's (this call's). It differs in one place only: a buffer or offset that does not fit a pointer's 32-bit field gives *NULL, where `ptr_to` would cut the number. Storage never reaches that size.
- **Not lowered.** BASED fields read or written through a pointer (S2.48), pointer arithmetic, %ALLOC, %PADDR, %ADDR of a local or a parameter (*OMIT gives *NULL there).
- **Expected values.** From PUB400, tools/golden/native48.txt n48_ptr (NPTR). The tripwire now watches ptr_to.
- **room_for_based.** After a pointer is stored into a field with no index, the storage it addresses grows to the largest variable based on that pointer, as the interpreter's `assign` does (`Runtime::pointer_set`). That step is written again from interp.rs room_for_based and ptrs.rs addr, and the tripwire watches both and `assign`. native-diff's level 1 found it missing: a buffer was 2 bytes shorter under rust.
- **The differential harness.** A pointer's owner is the activation it was taken in, and it differs between two runs in one process. native-diff's level 1 found NPTR's P different only there (103, then 104). The observation now clears each global pointer's owner bytes (4 to 8), as it blanks the PSDS's run-dependent fields, so pointers are compared by kind, buffer and offset.

## S2.48: BASED fields (rpgcg 0.1.0+stage2.48)

- **Where a based field is.** A based field that is not an array, global or a procedure's local, is lowered with its pointer as an operand (`ir::Based { ptr, size }`). So is a subfield of a based data structure. Each read or write first loads the pointer, then calls `Runtime::based_at`. That is the interpreter's `based_place` (ptrs.rs) written again: `addr` of the pointer for the field's offset plus size, then the offset on.
- **Errors.** A pointer that addresses nothing gives MCH3601, and storage too short gives MCH0601. The error is raised at the read or write, as the interpreter's `check_buf` raises the pointer fault there. A *NULL pointer used in MONITOR gives status 222, as on IBM i.
- **The pointer.** It is found as `based_place` finds it, through the routine's names, so a procedure's own come first. It must be a scalar pointer that is not based itself, or the field is not lowered.
- **Not lowered.** Based arrays, %ADDR of a based field, whole based data structures, and pointer arithmetic (S2.49).
- **Expected values.** From PUB400, tools/golden/native49.txt n49_bas (the unit NBASED). The same batch captured %ABS nested 99 times (it compiles) and 100 times (RNF0640, severity 30), with lines split between tokens, and the nesting test now cites it. The tripwire watches based_place.

## S2.49: pointer arithmetic (rpgcg 0.1.0+stage2.49)

- **What is lowered.** A pointer value can be a pointer field or %ADDR, moved by numbers in turn: `p + n`, `p - n`, and chains of them, left to right (`ir::PtrVal::Move`). The left spine is walked with a loop, at most MAX_NESTING steps, and each number is a lowered numeric expression.
- **Arithmetic.** `rpgrt::native::ptr_move` writes ptrs.rs ptr_arith again:
  - the number must be whole, and is taken modulo 2^64 as the interpreter's `as i64` takes it;
  - an offset outside 0..=u32::MAX is MCH0601;
  - *NULL is MCH3601.
  The sum is checked where mem::Ptr::add adds unchecked. The tripwire watches ptr_arith.
- **An interpreter difference (fidelity.md).** IBM i moves *NULL plus a number with no error (PUB400, native50 n50_pa), and the interpreter raises MCH3601. The native engine does as the interpreter does. NPNULL checks the parity, and it is held for upstream with the other reports.
- **Not lowered.** A number plus a pointer, and a pointer less a pointer (a number).
- **Expected values.** From PUB400, tools/golden/native50.txt n50_pa (NPARITH, all but the *NULL case).

## S2.50: heap storage (rpgcg 0.1.0+stage2.50)

- **What is lowered:**
  - `%ALLOC(n)` and `%REALLOC(p: n)` as pointer values (`ir::PtrVal::Alloc`, `Realloc`), assigned as any pointer, with room_for_based;
  - DEALLOC and DEALLOC(N) of a pointer field (`ir::Lowered::Dealloc`), as ptrs.rs storage_op: the storage freed, then with (N) *NULL stored.
- **The runtime.** `Runtime::alloc`, `realloc` and `dealloc` write ptrs.rs alloc, realloc and dealloc again, over `addr_n`, with the same errors:
  - RNQ0425 for a size outside 1 to 16 776 704 (status 425 on IBM i);
  - RNQ0426 for storage not from the heap or freed twice (status 426).
  The tripwire watches the four functions.
- **Not lowered.** ALLOC and REALLOC in fixed form: the interpreter's parser does not take them, while IBM i runs them (fidelity.md, PUB400 n51_heap). They are held for upstream with the other reports. Also not lowered: DEALLOC with E or indicators, and %REALLOC of anything but a pointer field.
- **Expected values.** From PUB400, tools/golden/native51.txt n51_heap (the unit NHEAP, free form).
- **A limit.** As in the interpreter, the heap has no limit on the number of allocations (mem::Heap::alloc), only on each one's size.

## S2.51: pointer parameters and OPTIONS(*VARSIZE) (rpgcg 0.1.0+stage2.51)

- **Pointer parameters.** A parameter of a call (a procedure of the module, a program, a bound procedure) can be a pointer.
  - By reference, it takes a pointer field.
  - By value or CONST, it takes *NULL or a pointer value (`ir::CallArg::Ptr`): a field, %ADDR, pointer arithmetic, %ALLOC.
  The numbers in a pointer value now take their helper names from the argument's path (`ptr_val`'s `path`), so two arguments never share one.
- **OPTIONS(*VARSIZE).** It is lowered for a program or a bound procedure. A by-reference argument goes as the interpreter's `args_for` passes it, `loc_of`: with the argument's own size and type. `rpgrt::native::CallArg::Ref` carries the type now, and `call_proto` builds the Loc from it. For every other argument that type is the parameter's, so nothing else changes. A character field of any length is taken for a character parameter, by reference, and with CONST as `char_variable` decides. *VARSIZE for a procedure of the module is not lowered: its descriptor is the parameter's in `call_frame`.
- **Expected values.** From PUB400, tools/golden/native52.txt n52_parm (NPARM, the pointer parameters). The capture's QCMDEXC with *VARSIZE needs a job, which the unit harness does not give. The suite's own *VARSIZE tests run in jobs, and native-diff level 2 compares them. The tripwire watches char_variable.
- **room_for_based for VALUE parameters.** call_proc binds each VALUE or CONST value with `assign` once the frame is pushed, so a pointer passed by value makes room for the called procedure's based variables. `call_body` now does the same through `pointer_set`. native-diff's level 1 found it missing: NPARM's BUF was 2 bytes shorter under rust.
- **The WHY diagnostic** now names what keeps a field out (local, array, based, occurs, type) and what keeps a prototype's call out (options, pointer parameters, C or CEE functions, procedure pointers, overloads, return types).

## S2.52: date, time and timestamp fields (rpgcg 0.1.0+stage2.52)

- **The type.** A date, time or timestamp field is lowered with its type: `ir::Ty::Date` and `Time`, with the format's name and separator, and `Timestamp` with its fractional digits. Its value is the character data `data::load` gives, its *ISO text. A store converts to the field's format, as `data::store` does for the interpreter's assign.
- **Where it is lowered.** As an EVAL's target, with any character value. As the whole value of an EVAL (`char_value`). As a side of a comparison of operands: `data::compare`, as eval's comparison does.
- **Where it is not.** The interpreter treats temporal fields apart in places, so they are not lowered there (`is_temporal`):
  - in a character expression, because a date plus a time is a timestamp in eval;
  - in DSPLY, which shows them in their format (%CHAR);
  - in MOVE (move_temporal);
  - in %SUBST, and as call arguments.
  %CHAR, %DATE, %TIME, %DIFF and durations come later.
- **Interpreter differences (fidelity.md)**, held for upstream:
  - With CTL-OPT DATFMT(*EUR), IBM i takes date literals in *EUR form only, while the interpreter takes them in *ISO form only.
  - *LOVAL and *HIVAL assigned to a date are 0001-01-01 and 9999-12-31 on IBM i, while the interpreter fills the field with x'00' and x'FF' bytes. The native engine does as the interpreter does, and NDATEFIG checks the parity.
- **Expected values.** From PUB400, tools/golden/native53.txt n53_date (the unit NDATE, with no literals, so both compile it).

## S2.53: %CHAR and DSPLY of dates, times and timestamps (rpgcg 0.1.0+stage2.53)

- **%CHAR.** %CHAR of a date, time or timestamp field, with no format or with a special value (*YMD, *JUL, *ISO0...), is lowered as `ir::CharExpr::TemporalChar`. The format is the one bifs.rs bif_override picks: the explicit one, or else the field's, or else the control specification's DATFMT or TIMFMT. It is known when the code is generated, and it goes into the layout lines, so into the fingerprint.
- **The runtime.** `rpgrt::native::temporal_char` writes bif_override and bifs.rs `temporal` again: the value is parsed as `temporal` parses it, then shown in the format. A value that does not parse is %CHAR's generic form, the characters as they are. A timestamp is its text, or its digits only with *ISO0.
- **Not lowered.** A format that does not parse, or a second argument that is not a special value, stays in the interpreter. The interpreter raises its error for it.
- **DSPLY.** DSPLY of a date or time field is %CHAR of it, and of a timestamp field its value, as Stmt::Dsply does. The tripwire watches bif_override and temporal, and stmt_inner (DSPLY) already.
- **Expected values.** From PUB400, tools/golden/native54.txt n54_chr (NTCHAR). DSPLY of a field is not captured: in a batch job its message goes to QSYSOPR, a queue that PUB400's users share. NTDSP checks it for parity between the engines.

## S2.54: %DATE, %TIME and %TIMESTAMP (rpgcg 0.1.0+stage2.54)

- **What is lowered.** %DATE, %TIME and %TIMESTAMP, as the whole value of an EVAL (`ir::CharExpr::TemporalBif`). Their result is a date, time or timestamp, and eval adds a date and a time into a timestamp, so they are not lowered inside character expressions.
- **The forms.**
  - With no argument, or *SYS (and *JOB for %DATE), the bif gives the current date and time, as dates::now.
  - Otherwise the first argument's value: a date, time or timestamp field, a character expression or a numeric one.
  - A special value as the second argument is the format. Any other second argument is not looked at, as special_arg gives none.
  - The control specification's DATFMT or TIMFMT code is fixed when the code is generated, and goes into the layout lines.
- **The runtime.** `rpgrt::native::temporal_bif` writes bifs.rs bif_more's arms again, with the same errors:
  - an invalid value is the dates module's error, status 112 under MONITOR as on IBM i;
  - "%DATE of a time is not valid" and the like.
  The tripwire watches bif_more, temporal_arg and special_arg.
- **Interpreter differences (fidelity.md).** Held for upstream:
  - With no format, IBM i takes a character or numeric value in *ISO form, not DATFMT's (RNF7510 at compile time). The interpreter takes the DATFMT or TIMFMT form.
  - %DATE(*SYS) is not valid on IBM i (RNF0605), and the interpreter takes it as the current date.
  The native engine does as the interpreter does.
- **Expected values.** From PUB400, tools/golden/native55.txt n55_bif (the unit NTBIF). n55_crt_dft is the compile of the forms IBM i rejects.

## S2.55: %DIFF, %SUBDT and durations (rpgcg 0.1.0+stage2.55)

- **A temporal value.** It is one of these (`temporal_value`):
  - a date, time or timestamp field;
  - %DATE, %TIME or %TIMESTAMP;
  - one of them plus or minus a duration (%YEARS ... %MSECONDS of a number).
  The duration can be on either side of +, as bifs.rs date_arith takes it, and only on the right of -. The operands are evaluated in the order eval evaluates them (`dur_first`).
- **Where a temporal value is lowered:**
  - as the whole value of an EVAL;
  - as an argument of %DIFF and %SUBDT, which are numeric expressions (`NumExpr::TemporalDiff`, `TemporalSubdt`);
  - on both sides of a comparison, when not both are fields (`Cond::CmpExpr` of character sides, data::compare).
- **The runtime.** `rpgrt::native::temporal_diff`, `temporal_subdt` and `temporal_add` write bif_more's %DIFF and %SUBDT and date_arith again, over the dates module, with the same errors.
  - A duration's number is taken whole and modulo 2^64, as int_arg's `as i64`.
  - A duration that cannot be negated is an error where the interpreter wraps.
  The tripwire watches date_arith and as_ts. temporal_bif now takes the statement's path for its argument's helpers, so two bifs in one statement do not share one name.
- **Interpreter differences (fidelity.md).** Held for upstream:
  - IBM i does not compile a duration on the left of + (RNF7421, n56_crt_first).
  - A date past 9999-12-31 is status 113 on IBM i and 112 in the interpreter.
  The native engine does as the interpreter does, and NTDURF checks the parity.
- **Expected values.** From PUB400, tools/golden/native56.txt n56_dur (the unit NTDUR). NTBIF, NTDUR and NTDURF are fully native.

## S2.56: C runtime functions and CEE APIs through prototypes

- **What is lowered.** A call through a prototype to a C runtime function or a CEE API, which lowering refused until now. The prototype must have no EXTPGM, and its parameters no OPTIONS other than *VARSIZE and *NOPASS. The call goes through `call_proto` in call_named_inner's order:
  1. the module's own procedure, unless the EXTPROC is written in lower case and names a C function (`system_named`);
  2. a procedure the binder found;
  3. a CEE API (`Machine::cee_call`);
  4. a C function (`Machine::c_call`);
  5. otherwise CPD5D02.
- **The C arguments.** As call_named_inner passes them:
  - the prototype's arguments as args_for gives them;
  - a CONST argument passed by reference goes as its value, or as its address when it is an output buffer (`C_OUT_BUFFERS`);
  - arguments past the prototype's go as values (printf's list), lowered as character, numeric or pointer values.
- **Too many arguments.** Only a C function takes more arguments than its prototype has. Elsewhere it is args_for's error, which the native call raises too.
- **A phase B dependency.** c_call and cee_call are interpreter services, as call_program and call_bound are (the user's choice, 2026-11-15: use them now, write them again in rpgrt later). The tripwire already watches call_named_inner.
- **Expected values.** From PUB400, tools/golden/native57.txt n57_c (the unit NCFUNC), and the interpreter gives the same. IBM i's C runtime is EBCDIC: toupper(97) is 97 (x'61' is '/'), and 55 (x'37') is not a digit.

## S2.57: hexadecimal literals

- **What is lowered.** A hexadecimal literal (`x'00'`) is an operand, the character data of its bytes, as eval gives it (`Expr::Hex(s) => Value::Char(s)`). So it works wherever a character literal does: concatenations, comparisons, EVAL to a character field, %SUBST as a target.
- **Not lowered.** Its two other forms stay in the interpreter:
  - EVAL of a hexadecimal literal to an integer or unsigned field, where Stmt::Eval takes its bytes as an unsigned number. That needs a numeric operand, and lowering gives none for a hexadecimal literal.
  - An integer argument, which value_arg refuses for a hexadecimal literal.
- **Expected values.** From PUB400: NCFUNC's `x'00'` terminators (tools/golden/native57.txt n57_c: strlen gives 5 and atoi 123). NCFUNC now has 2 instructions in the interpreter, down from 4.

## Plan: record I/O on database files (S2.58 and on)

About 150 of the corpus's instructions still in the interpreter are file operations: READ 45, WRITE 33, CHAIN 21, EXCEPT 20, SETLL 8, OPEN 8, UPDATE 7, CLOSE 7. In the interpreter they go through file_op_inner, which leans on:
- key_values (KLIST, %KDS);
- move_input and record_of (fields to and from the record, null maps under ALWNULL);
- record_change (writes: authority, triggers, the journal);
- file_failed (status and INFDS);
- commitment control and record locks.

**The runtime library is shared, the orchestration is ours.** The record operations themselves are `rla` (setll, chain, read, read_equal, lock_after_read, record changes), a runtime module the interpreter also calls, not interpreter code. The native engine calls rla directly. What file_op_inner does around rla is written again in native_rt, under the tripwire, as phase A asks:
- the order of steps;
- the status and %EOF/%FOUND/%EQUAL;
- the result indicators;
- the error path.
Renaming the interpreter's statement dispatch is not an option: it would count as native and is not.

**Steps, each a block with its own PUB400 captures and section 7:**
1. **S2.58: reads on externally described database files**, with no result data structure and no field list:
   - SETLL and SETGT, with keys, *LOVAL/*HIVAL and *START/*END;
   - CHAIN, READ, READP, READE and READPE;
   - keys from fields and literals, and KLIST;
   - result indicators, and the E extender (file_failed's status);
   - move_input written again (fields from the record, ALWNULL rules).
2. **S2.59: WRITE, UPDATE and DELETE**: record_of, record_change's authority check, commitment control's `changed`.
3. **S2.60: OPEN and CLOSE** (USROPN), UNLOCK and FEOD, %KDS, and the result data structure and field list forms.
4. **Then printer files (WRITE, EXCEPT) and display files (EXFMT, READC, subfiles).** These are their own blocks, after the database ones.

Each step keeps the interpreter as the reference (native-diff), and records any difference from IBM i in fidelity.md, as before.

## S2.58: reads of database files

SETLL, SETGT, CHAIN, READ, READE, READP and READPE of a database file, and %EOF, %FOUND and %EQUAL, run in Rust. The work is split three ways.

**Generated code:**
- evaluates the key's values;
- calls `Runtime::file_read`;
- moves the record's fields into the program fields of the same names, as fill_fields does: a number parsed exactly, characters, or hexadecimal digits two to a byte, stored with the strict fit;
- calls `Runtime::file_done` with the result of the moves.

**`native_rt`** writes again errors.rs file_op (with file_failed, file_idx and opcode_name) and the reads of interp.rs file_op_inner, in their order:
- the error flag off, and the savepoint of a file under commitment control;
- the `rla` operation, with its record lock;
- fill_fields' null values: ALWNULL(*NO) gives CPF5035, *USRCTL sets the null indicators;
- after the moves: the status, %EOF/%FOUND/%EQUAL, the result indicators, the counters, the status code (00011, 00012) and the operation and record names.

An error, of the operation or of a move, ends the operation as file_op does and goes on to the error path. native_tripwire.rs watches the copied functions. `rla` is the runtime library both engines call.

**What lowering lets through:**
- an externally described DISK file of the main program (no file declared in a procedure and no LIKEFILE parameter, so a name finds the same file wherever it runs);
- no INFDS, so infds_update has nothing to do;
- no input specifications for the file, no result data structure and no field list;
- no E extender and no error indicator;
- keys of fields and literals, a KLIST of them, or *LOVAL, *HIVAL, *START or *END alone. A field or a literal cannot fail, so evaluating the key before the operation starts, rather than inside it as key_values does, makes no difference;
- record fields that are numbers, characters or hexadecimal, into numeric, character or varying character program fields.

Everything else stays in the interpreter, for later steps.

**Verified by:**
- The native unit NFREAD, under both engines. Its DDS file, its rows, its program and every expected line come from PUB400 (tools/golden/native58.txt n58_r).
- On IBM i, a READPE right after READE reached end of file is end of file too; as4tainr's `rla` reads the key's last record again in both engines (fidelity.md, interpreter defects), so NFREAD leaves that READPE out.
- The first capture found that IBM i refuses a key literal of another length than the key field (RNF7078). as4tainr accepts it: fidelity.md, upstream findings.

## S2.59: WRITE, UPDATE and DELETE of database files

WRITE, UPDATE and DELETE of a database file run in Rust, under the same conditions as S2.58's reads, with these additions:
- WRITE and UPDATE take no key;
- DELETE takes a key of fields and literals, or none (the record last read).

The work is split in the order of the interpreter's file_op.

**`Runtime::file_change_start`:**
- the error flag off, and the savepoint of a file under commitment control;
- the open check;
- the authority check: *CHANGE on the file, CPF9802.

**Generated code:** makes the record from the program fields of the same names, as record_of does:
- a number with the record field's decimal places (an error if it does not fit them);
- hexadecimal bytes as two capital digits;
- characters as text.

**`Runtime::file_change`:**
- the record, with record_of's null values under ALWNULL(*USRCTL);
- record_change: WRITE, UPDATE, or DELETE (of the record a key finds, CPF5013 if none);
- the record lock released after UPDATE and DELETE;
- the commitment definition told of the change;
- %ERROR's status, and file_op's end.

**Trigger programs.** A file with trigger programs goes to the interpreter's record_change, which calls them. This is an external program call, like the other program calls kept for now (the user's choice, 2026-11-16), to be written again in phase B.

**Watched.** native_tripwire.rs watches record_of, record_change, rla_change and fref_of.

**Verified by** the native unit NFWRITE, under both engines. Its DDS file, its program and its expected lines are those captured on PUB400 (tools/golden/native59.txt n59_w).

## S2.60: whole data structures and hexadecimal literals as call arguments

**What chose this step.** S2.59's corpus pass with AS4TAINR_NATIVE_WHY, made to name each option and argument, showed the calls kept in the interpreter. Most were calls of system APIs (user spaces, messages) with two forms:
- the error code structure, or a receiver, as a whole data structure;
- a hexadecimal literal as a CONST character argument.

**What lowering now lets through.** A whole data structure (global, not an array, not based, not of multiple occurrences) as the argument of a call of a program, or of a procedure that is not in the module, goes as the interpreter's loc_of gives it: its bytes, as a character field of its size.
- By reference, it goes as is when the parameter's type is the same, or when the parameter is OPTIONS(*VARSIZE) character.
- For CONST, it goes by reference when the sizes match (same_type_variable), and as its value otherwise.
- For a procedure of the module, a data structure argument is still not lowered (OpenRPG test325).

A hexadecimal literal for a CONST or VALUE character parameter is its bytes, as eval gives them. For an integer parameter, args_for makes it a number, and that is not lowered.

**Verified by** the native unit NCALLDS, under both engines. It is the program captured on PUB400 (tools/golden/native60.txt n60_api), with the user space in QGPL:
- QUSCRTUS, QUSCHGUS, QUSRTVUS and QUSDLTUS, with the error code structure;
- a receiver data structure;
- x'C1C2' as data;
- CPF9801 after the space is deleted.

## S2.61: the module's global fields in procedures

The corpus pass found names that lowering called unknown in procedures (MODE, CALLS, SEEN1...). They were the module's global fields: a procedure's symbol table (ProcInfo::syms) has its own names only. The interpreter's Machine::sym finds a name in the running procedure first, then among the program's.

Lowering now does the same (`sym`):
- a procedure's names first;
- then the module's globals, which `variable` places, or refuses, as it does in the main program (based fields, multiple-occurrence data structures).

A local of the same name as a global still hides it.

**Verified by** the native unit NGLOBAL, under both engines. The program captured on PUB400 (tools/golden/native61.txt n61_glb) has procedures that change a global counter, a packed total, a data structure subfield, array elements and a character field, and a local that hides a global.

## S2.62: indicators in concatenations

An indicator (*INxx, *INLR...) is character data of length 1 (ILE RPG Reference, "Indicator Data Type"). The interpreter's + concatenates it with characters or with another indicator, and adds it as a number to a number. Lowering now takes an indicator as a part of a concatenation (`cat_part`): its value is '1' or '0' (Value::Ind's chars, X'F1' or X'F0'). Every other part of a lowered concatenation is character data, so the + is always a concatenation. An indicator on its own, or added to a number, is not lowered.

This was the most frequent form left in the corpus pass: DSPLY ('ARITH SUB ' + %char(r) + ' ' + *in50 + *in51 + *in52) in NARITH, whose expected lines come from PUB400 (tools/golden/native3.txt). NARITH's coverage ceiling goes from 4 instructions in the interpreter to 0.

## S2.63: arrays of a procedure's own storage

An element of a procedure's local array, or a subfield of an element of its local data structure array, is now lowered as the interpreter finds it. Machine::sym places the local in the frame, then elem_off adds (index - 1) times the stride, with RNQ0121 for an index out of range. Generated code computes the element's offset and asks `rt.local` for the frame's buffer.

Not lowered:
- a local array that is a parameter: one passed by reference is the caller's storage, where elem_off also checks that the element is there (MCH0601);
- a based local;
- a local varying-dimension array.

**Verified by** the native unit NLOCARR, under both engines. Its program was captured on PUB400 (tools/golden/native63.txt n63_loc):
- a local array with literal and field indexes;
- a local data structure array;
- a recursive call whose own array stays apart from its caller's;
- an index out of range under MONITOR, which gives 00121.

## S2.64: multiple-occurrence data structures

A scalar subfield of a global multiple-occurrence data structure is now lowered in its current occurrence. Machine::sym adds (occurrence - 1) times the occurrence's size, so lowering makes it an element indexed by the current occurrence: `Operand::Occur`, which `Runtime::occurrence` gives. These are written again from ops.rs:
- **%OCCUR(ds)** reads the current occurrence (occurrence).
- **EVAL %OCCUR(ds) = n** sets it (`Runtime::set_occurrence`, ops.rs set_occurrence): a whole number from 1 to the occurrences, RNQ0122 otherwise.
- **OCCUR** sets the occurrence from factor 1, a number or another multiple-occurrence data structure's current occurrence, then puts it in the result field with the strict fit (occur_op). This is lowered only with no error indicator.

A subfield that is an array, a based or local one, and the whole data structure as a field stay in the interpreter.

**Verified by** the native unit NOCCUR, under both engines. The program captured on PUB400 (tools/golden/native64.txt n64_occ) writes the three occurrences, reads them through %OCCUR and OCCUR, totals them, and gets 00122 under MONITOR for occurrence 4. The first capture found that IBM i refuses a literal out of range there at compile time (RNF7571), which as4tainr accepts (fidelity.md, upstream findings).

## S2.65: %ADDR of a procedure's local fields

%ADDR of a procedure's local field (not based) is now lowered. ptrs.rs ptr_to of the place that Machine::sym gives the field is the frame's buffer, or the caller's storage for a parameter passed by reference. Generated code gets that place from `rt.local`, then `rt.addr_of`. An element of a local array (S2.63) is placed the same way. `pointer_set` already found the variables based on a local pointer among the frame's based fields (room_for_based).

**Verified by** the native unit NLOCADR, under both engines. The program captured on PUB400 (tools/golden/native65.txt n65_adr) does three things:
- changes a local through a based field;
- walks a local array with pointer arithmetic;
- writes into the caller's storage through the address of a parameter passed by reference.

## S2.66: OPTIONS(*STRING) for C functions

A pointer parameter of a C function with OPTIONS(*STRING), passed by value or CONST, now takes a character argument in lowered code. The interpreter does it in three places, and native_rt now does the same:
- **Lowering** passes the argument as its characters (args_for's eval of it is character data).
- **call_proto's C path** does what args_for does for *STRING: a copy of the characters with a null byte after, in a heap block, and a pointer to it as the argument.
- **call** frees the copies when the call ends, as call_named does: it marks temp_strings before the call and frees what was added after it.

A pointer argument for such a parameter still goes as a pointer. *STRING for a program or a bound procedure stays in the interpreter.

**Verified by** the native unit NSTRING, under both engines. The program captured on PUB400 (tools/golden/native66.txt n66_str) calls strlen, atoi, strcmp and strncmp with these arguments:
- a literal;
- a fixed field, trailing blanks included;
- %TRIM of that field;
- a varying field;
- a concatenation;
- a hexadecimal literal.

## Plan: embedded SQL (S2.67 and on)

EXEC SQL is the largest kind of instruction left in the interpreter: 92 in the corpus as of S2.63. The interpreter's exec_sql.rs (about 1,700 lines) resolves everything by name when it runs:
- host variables, read with eval and written with assign;
- the SQLCA (SQLCODE, SQLSTATE, SQLERRMC, SQLERRD);
- cursors, prepared statements, GET DIAGNOSTICS.

The statement itself runs on sql::Session, which is a runtime library, not the interpreter.

**Approach, as for files (S2.58):** lowering resolves the host variables named in the statement to operands. Generated code reads their values from storage before the statement, and stores what comes back after it. The runtime writes again what exec_sql.rs does around the Session call:
- the statement's kind;
- the binding of values;
- the SQLCODE and SQLSTATE for each outcome;
- the SQLCA's fields, stored by generated code as operands.

native_tripwire.rs watches each copied function. Session and sql.rs (translation, the as4tainr_sql login) are shared, never copied.

**Steps, each with its own PUB400 captures and section 7:**
1. **Statements with no host variables:** DDL, SET OPTION, COMMIT/ROLLBACK, INSERT/UPDATE/DELETE of literals. The SQLCA comes back set.
2. **SELECT INTO and VALUES INTO**, with host variables and indicator variables (one row: SQLCODE 100 for none, -811 for more than one).
3. **INSERT, UPDATE and DELETE with input host variables.**
4. **Cursors:** DECLARE, OPEN, FETCH INTO (one row, then several rows into a data structure array), CLOSE, positioned UPDATE and DELETE.
5. **GET DIAGNOSTICS, CALL, PREPARE/EXECUTE/EXECUTE IMMEDIATE, and SET RESULT SETS.**

Each step keeps the interpreter as the reference (native-diff), and records any difference from IBM i in fidelity.md.

## S2.67: %SCANR, %LEFT and %RIGHT

%SCANR of character expressions, with a start and a length that are numeric literals or fields, and %LEFT and %RIGHT of a character expression with such a length, are now lowered. They are bifs.rs's arms, written again in rpgrt (`scanr`, `left_right`): the same checks, the same RNQ0100 messages, and the arguments in the same order. One difference from the interpreter: a %SCANR start more than one past the end with no length is RNQ0100 there too, where the interpreter's default length subtracts past zero (fidelity.md).

**Verified by** the native unit NSCANR, under both engines. Its program was captured on PUB400 (tools/golden/native67.txt n67_str). %SCANR is called with and without a start and a length, with a pattern that is not found, and with a pattern longer than the part searched. %LEFT and %RIGHT take a field and an expression, with lengths of 0 and of the whole string. Under MONITOR, a length longer than the string and a %SCANR length past the end both give 00100.

## S2.68: %XFOOT, %MAXARR and %MINARR

%XFOOT, %MAXARR and %MINARR of a whole global array, with no start or number of elements, are now lowered. The array's elements are read as %LOOKUP reads them (S2.27). The bifs.rs arms are written again in rpgrt `arr_fold`:
- %XFOOT gives the sum of the elements' numbers;
- %MAXARR and %MINARR give the index of the first greatest or smallest element, as data::compare orders them.

With a start or a number of elements, these stay in the interpreter. There, as for %SCANR, the default number subtracts past zero for a start past the end.

**Verified by** the native unit NARRFN, under both engines. Its program was captured on PUB400 (tools/golden/native68.txt n68_arr) and covers:
- a packed array with decimals and negative values;
- ties, where the first counts;
- an integer array;
- a character array.

## S2.69: embedded SQL, step 1 (no host variables)

An EXEC SQL with no host variable is now lowered, as the plan's step 1 says. Lowering refuses, and the interpreter keeps:
- a statement with bind_host's `:name` outside literals;
- OPEN, FETCH, CLOSE, PREPARE, EXECUTE, CALL, GET and SET RESULT SETS;
- INSERT ... ROWS;
- a positioned UPDATE or DELETE.

**Runtime.** `Runtime::exec_sql` writes again exec_sql.rs, in its order:
- exec_sql up to the SQLCA: the outcome, an SQL error as one, and `last` for GET DIAGNOSTICS;
- sql_statement's start: the wait for other jobs' locks, and STACK_INFO;
- sql_statement's arms for these statements: SET OPTION, INCLUDE, WHENEVER, BEGIN and END, DECLARE CURSOR, COMMIT and ROLLBACK, the DDL through ddl.rs, and SQLCODE -84 for GRANT, REVOKE, LOCK and MERGE;
- the rest through sql_change: run_sql, sql_overrides (temporary views for file overrides) and session.

The text helpers words, find_word and sql_error are written again without indexing. sql::Session, ddl.rs and objlock.rs are shared.

**Generated code** stores the outcome into SQLCODE, SQLSTATE, SQLERRMC, SQLERRD(3) and SQLERRD(5) with the truncating fit, as exec_sql assigns them. Lowering resolves those names (the program's SQLCA) as operands.

**Upstream touch point.** The native runtime shares the interpreter's SQL state (session, prepared statements, last outcome, override views). exec_sql.rs's fields are now `pub(super)`, authorized by the user on 2026-11-18 (CLAUDE.native.md section 3), in a commit of its own.

**Verified by** the native unit NSQL1, under both engines. The program captured on PUB400 (tools/golden/native69.txt n69_sql) runs:
- an INSERT of three rows;
- an UPDATE of none (SQLCODE 100);
- a DELETE of two (SQLERRD(3));
- a missing table (-204, 42704);
- a DROP.

The capture's CREATE (+7905, the table not journaled) and COMMIT under COMMIT(*NONE) (-7007) are interpreter defects (fidelity.md), so NSQL1 does not check those two lines.

native_tripwire.rs watches the copied functions. It now also watches ops.rs's occurrence functions, which S2.64 wrote again.

## S2.70: embedded SQL, step 2 (host variables)

The plan's steps 2 and 3 in one block. These EXEC SQL statements with host variables are now lowered:
- SELECT, VALUES or WITH ... INTO;
- SET :host = expression;
- the statements exec_sql.rs runs through bind_host and sql_change (INSERT, UPDATE, DELETE, a SELECT with no INTO).

**Lowering** takes the statement apart as sql_statement and bind_host do:
- the INTO list (host_list) and the text without it;
- for SET, `VALUES (` around the expression;
- bind_host's scan: `:name`, an indicator as `:ind` or `INDICATOR :ind`, nothing inside quotes, the byte count of `INDICATOR :` added to a char index as bind_host does.

Each host variable is resolved as host_targets does: a data structure's subfields, qualified or not. The text goes to generated code in pieces around the inputs.

Lowering keeps in the interpreter:
- a LOB (`_LEN`/`_DATA`), a host variable not defined, and a data structure with an indicator;
- a field that is not character, varying or numeric (dates, times, timestamps, floats, based fields);
- an INTO target with an index, and an indicator with an index;
- the statements step 1 leaves out, and DDL and DECLARE, whose host variables exec_sql.rs does not bind.

**Generated code** makes each input's literal (`rpgrt::native::sql_literal`, literal and host_literal written again): `NULL` for a negative indicator, a fixed CHAR(n) as `CAST(... AS char(n))`. It passes the INTO targets' storage (`SqlSlot`) to `Runtime::exec_sql_hosts`. That runs:
- sql_statement's start (`sql_prologue_n`, now shared with step 1);
- the bound text;
- single_row (none: +100; more than one: the first row, then -811) and assign_hosts (a null: the indicator -1, or -305 with none; a value: the indicator 0, a number parsed exactly or -302, the strict fit or -302); or else sql_change.

The SQLCA is stored as in step 1.

**Verified by** the native unit NSQL2, under both engines. The program captured on PUB400 (tools/golden/native70.txt n70_sql) runs:
- a character and a numeric input;
- a data structure INTO;
- a null with `:ind` and a value with `INDICATOR :ind`;
- no row and more than one row;
- SET;
- UPDATE and DELETE with host variables (SQLERRD(3));
- an input indicator of -1.

Two of the capture's lines are interpreter defects (fidelity.md), so NSQL2 does not check them: the variable is not cleared with a null, and a CHAR host variable loses its blanks in UPPER.

native_tripwire.rs watches host_list, literal, host_literal, host_targets, bind_host, assign_hosts and single_row.

## S2.71: dynamic SQL and GET DIAGNOSTICS

The plan's step 5, except CALL and the cursors. These are now lowered:
- PREPARE name FROM :text, and EXECUTE IMMEDIATE :text, where the text is a character or varying field (statement_text);
- EXECUTE name [USING :host, ...]: using_values, with host_literal's literals per host variable and a data structure's fields joined with ", ";
- GET DIAGNOSTICS: ROW_COUNT, or CONDITION n with MESSAGE_TEXT, MESSAGE_LENGTH, RETURNED_SQLSTATE, DB2_RETURNED_SQLCODE, MESSAGE_ID or DB2_MESSAGE_ID, into the fields of each target (host_targets).

**Runtime.**
- `Runtime::exec_sql_dyn` runs sql_statement's start, then its arm. PREPARE keeps the trimmed text under the statement's name in the interpreter's `prepared`, which the two engines share. EXECUTE IMMEDIATE and EXECUTE run the text (for EXECUTE, the prepared one with fill_markers' literals; SQL0518 when it was not prepared) through sql_ddl, or else sql_change. The outcome is kept as the last.
- `Runtime::sql_diagnostics` gives the items' values from the last outcome (get_diagnostics: the message ID split from the text) and SQLCODE 0. The last is not changed.

**Lowering refuses** (the interpreter keeps them):
- GET DIAGNOSTICS with an item get_diagnostics does not support, whose -104 the interpreter gives;
- a statement text that is not a character operand;
- a host variable lowering cannot resolve, as in S2.70.

OPEN, FETCH and CLOSE needed the interpreter's `cursors` and its `OpenCursor`, which the first authorization left out; S2.72 does them after the second.

**Verified by** the native unit NSQL3, under both engines. The program captured on PUB400 (tools/golden/native71.txt n71_sql) runs:
- EXECUTE IMMEDIATE of DDL and DML;
- ROW_COUNT;
- PREPARE from a varying field and from a fixed one;
- EXECUTE USING a character and a numeric value, and an indicator of -1;
- a prepared DELETE twice (2 rows, then +100);
- CONDITION 1's items after a missing table (42704, -204, SQL0204).

Its CREATE line (+7905) is the interpreter defect of S2.69. In the capture each GET DIAGNOSTICS comes right after its statement, because the capture's own output is an SQL CALL.

native_tripwire.rs watches get_diagnostics, statement_text and using_values.

## S2.72: cursors

The plan's step 4, without the positioned UPDATE and DELETE. The user's second authorization of 2026-11-18 makes `SqlState.cursors` and `OpenCursor` and its fields `pub(super)`, in a commit of its own, so the two engines share open cursors.

**Lowered:**
- DECLARE ... CURSOR, now also with host variables in its query. sql_statement answers SQLCODE 0 before binding them; they are bound at OPEN.
- OPEN of a cursor declared in the program. For a declared query: its text in pieces around its host variables (bind_host), with the inputs made as in S2.70. For a prepared statement: the USING inputs (using_values).
- FETCH of one row: orientation NEXT, PRIOR, FIRST, LAST, BEFORE, AFTER, CURRENT, RELATIVE n or ABSOLUTE n, FROM, INTO targets as in S2.70, or no INTO.
- CLOSE.

**Runtime:**
- `Runtime::sql_open` and `sql_open_n` write again sql_open:
  - -502 when the cursor is already open;
  - the bound query, or the prepared text with fill_markers (SQL0518 when it was not prepared);
  - strip_cursor_clauses and with_rrn, also written again without indexing;
  - every row read at OPEN, with each row's record number when there is one table.
- `sql_fetch_n` writes again sql_fetch: -501 when the cursor is not open, the position clamped between -1 and the row count, BEFORE and AFTER ending there, +100 off the rows, no INTO giving 0, and assign_hosts. The position arithmetic saturates where sql_fetch's `as isize` would overflow.
- `sql_close` writes again the CLOSE arm (-501).
- All three run sql_statement's start first, and keep the outcome as the last.

**Lowering refuses** (the interpreter keeps them):
- a cursor the program does not declare (sql_open's -504);
- a multiple-row FETCH (ROWS), which is the next block with the multiple-row INSERT;
- INTO targets or indicators with an index.

**Verified by** the native unit NSQL4, under both engines. The program captured on PUB400 (tools/golden/native72.txt n72_sql) covers:
- a host variable bound at OPEN (changed after OPEN, so the rows do not change);
- FOR READ ONLY;
- a data structure INTO to the end (100, 02000);
- OPEN of an open cursor (-502), and FETCH and CLOSE of a closed one (-501);
- a scroll cursor with LAST, PRIOR, FIRST, RELATIVE 2, BEFORE, a FETCH with no INTO, and RELATIVE past the end;
- a prepared statement's cursor opened with USING;
- a null into a field with an indicator.

PUB400's precompiler took no `FETCH ABSOLUTE 2` (SQL0104), so the capture leaves it out.

native_tripwire.rs watches sql_open, sql_fetch, strip_cursor_clauses and with_rrn.

## S2.73: UPDATE and DELETE WHERE CURRENT OF

**Lowered:** an UPDATE or DELETE whose words include CURRENT OF (and CURRENT is a keyword at depth 0), as sql_statement tells it. Lowering takes it apart as positioned_change does:
- the cursor after CURRENT OF;
- the table (`UPDATE table`, `DELETE FROM table`);
- the text before WHERE, in pieces around its host variables, with the inputs as in S2.70.

**Runtime.** `Runtime::sql_positioned` (`sql_positioned_n`) writes again positioned_change:
- -501 when the cursor is not open;
- -510 when it is not over one table;
- -508 when it is not on a row;
- otherwise the bound text with `WHERE RRN(table) = n`, through sql_change, after sql_statement's start.

**Verified by** the native unit NSQL5, under both engines. The program captured on PUB400 (tools/golden/native73.txt n73_sql) runs:
- a cursor FOR UPDATE, with an UPDATE before the first FETCH (-508);
- an UPDATE of the second row with a host variable (1 row);
- a DELETE of the third;
- an UPDATE after the last row (-508);
- the rows left.

After CLOSE, IBM i gives -507. positioned_change gives -501, an interpreter defect (fidelity.md), so NSQL5 stops that line before it.

native_tripwire.rs watches positioned_change.

## S2.74: multiple-row FETCH and INSERT

**Lowered:**
- FETCH ... FOR n ROWS INTO :dsarray, as fetch_rows reads it: the count is the word before ROWS;
- INSERT INTO table n ROWS VALUES (:dsarray), as sql_insert_rows reads it: the text before the count, with no host variables here; VALUES or values.

The count is a number as written (Dec::parse) or a numeric host variable. The host structure array must be:
- global, with DIM, and so qualified;
- not based, not of multiple occurrences, not imported or exported.

Each subfield is a character, varying or numeric `HOST.SF`, at its first element's place, with the structure's size between elements.

**Runtime.**
- `Runtime::sql_fetch_rows` (`sql_fetch_rows_n`) writes again fetch_rows:
  - -221 when the count is more than the elements, and -501 when the cursor is not open;
  - the first row by the orientation, clamped between 0 and the row count;
  - +100 with the cursor after the last row, when there is none or the count is 0;
  - otherwise the rows into the elements in order, through assign_hosts' strict fit (an error stops it);
  - the cursor on the last row read, and SQLERRD(5) = 100 when that is the last row.
- `sql_insert_rows_n` writes again sql_insert_rows:
  - a count of 0 is SQLCODE 0, and more than the elements is -221;
  - otherwise each element up to the count becomes a row of literals (a fixed character field as CAST(... AS char(n))), through sql_change.

The element storage is read with `rpgrt::native::load` at each element's place.

**Verified by** the native unit NSQL6, under both engines. The program captured on PUB400 (tools/golden/native74.txt n74_sql) runs:
- INSERT of 3 rows (a literal count) and of 2 (a host variable), and of 5 into 4 elements (-221);
- FETCH FOR 2 ROWS three times over 5 rows (0/2/0, 0/2/0, 0/1/100, the second element keeping its value);
- FETCH past the end (100), and FETCH FIRST FOR 3 ROWS into 2 elements (-221).

On IBM i an INSERT of 0 rows is -221, but sql_insert_rows gives 0, an interpreter defect (fidelity.md); NSQL6 leaves that part of its line out. PUB400's precompiler wants a scroll cursor for FETCH FIRST (SQL0225).

native_tripwire.rs watches fetch_rows and sql_insert_rows.

## S2.75: CALL of SQL procedures

The last step of the SQL plan. The remaining embedded SQL refusals are the cases lowering keeps in the interpreter: LOBs, dates and times as host variables, data structures with indicators, and array elements as targets.

**Lowered:** CALL, taken apart as sql_call does, with sqlpl::parse_call (a parse with no state) at lowering time.
- An argument by name (`NAME => :hv`) is an input: `NAME => ` and the host literal.
- A positional host variable (`:hv`, `:hv :ind`, `:hv INDICATOR :ind`) is an input, and also where an OUT or INOUT parameter's value goes.
- Any other argument is text as written.
- A host variable must be a field. A data structure, an element with an index, or a name with `:` in it stays in the interpreter.

**Runtime.** `Runtime::sql_call` (`sql_call_n`) writes again sql_call's end:
- sqlpl::call_exprs, on the program's SQL session, with the job's library list, user, current library and the main connection;
- a failure as its condition (sqlpl::condition_of; for -438 its SQLSTATE too);
- then each parameter whose mode is not IN into its host variable, through assign_hosts (an error stops it).

It runs after sql_statement's start, and the outcome is kept as the last.

**Verified by** the native unit NSQL7, under both engines. The program captured on PUB400 (tools/golden/native75.txt n75_sql) calls:
- a procedure with IN, OUT and INOUT host variables (42, 'hi!');
- the same with a literal argument and an INOUT with an indicator ('hi!!', 0);
- a procedure that SIGNALs (-438, 75001).

Two interpreter defects (fidelity.md):
- A missing procedure gives -440 and 42884, where IBM i gives -204 and 42704, so NSQL7 leaves that part of its line out.
- as4tainr's compiler cannot take the capture's EXEC SQL CREATE PROCEDURE with a compound body, so NSQL7 makes the procedures with EXECUTE IMMEDIATE.

native_tripwire.rs watches sql_call.

## S2.76: a path with one index on its middle part

`ds_array_field` (S2.24, S2.63) took only `ds(i).field`. It now takes any path with one index on a part that is not the last. For example `list.users(i).name`: the indexed part is a LIKEDS subfield with DIM, whose subfields program.rs names LIST.USERS.NAME at the first element's place. The place is the field's, with that structure's size between elements, as interp.rs path_sym adds (i - 1) times it after its range check (status 121). A path with two indexes (`gv(s).m(1).parmNo`) stays in the interpreter.

**Verified by** the native unit NNEST, under both engines. The program captured on PUB400 (tools/golden/native76.txt n76_nested) does three things:
- assigns and reads `list.users(n).name` and `.days` with literal and field indexes, in arithmetic;
- runs a FOR loop over them;
- catches an index past the elements with MONITOR (ERROR 121).

## S2.77: a path with more than one index

`ds_array_field` now takes a path with an index on any number of parts but the last, for example `gv(s).m(i).parmNo`: a LIKEDS subfield with DIM inside a data structure array. Each indexed part must be a data structure with DIM in the same storage. The operand's index is a chain:
- `ir::Index::outer` holds the index further out on the path;
- `elem_at` applies the outer one first, its value read and its range checked before the inner one's, as interp.rs path_sym goes part by part from the left.

The base place is the field's own, which program.rs gives at every index's first element. A single index emits the same code as before.

**Verified by** the native unit NNEST2, under both engines. The program captured on PUB400 (tools/golden/native77.txt n77_nested2) does four things:
- writes and reads `gv(s).m(i)` subfields with literal and field indexes;
- uses an index that is itself such a path (`gv(s).m(gv(s).nm)`);
- reads a subfield never set (blanks: x'40404040' as an int(10), 1077952576);
- catches an inner index past its elements with MONITOR (ERROR 121).

## S2.78: EXCEPT to a printer file

**Lowered:** EXCEPT [name] with no error extender or indicators. Lowering looks at the records it may print:
- its exception records;
- when one fetches overflow, every heading, detail and total line of that printer.

All of them must be to PRINTER files. Each program field they name must be a character, varying or numeric field with no index; a whole data structure prints as its bytes, but is not lowered when cleared after printing. An edit word given by name must be a named constant of the program. Generated code passes each field's place for this call (`OutSlot`), so the runtime reads a field when it prints it, and clears it after printing when the field is blank after.

**Runtime.** It writes again interp.rs except, out_cond, conditions_met, print_record (fetch overflow), overflow_routine, overflow_line, overflow_ind, print_line, out_text and flush_printer, and printer.rs form_of and ensure_printer, without indexing and with checked arithmetic:
- out_text: literals, PAGE to PAGE7, UDATE and *DATE, character fields, edit words, edit codes, and numbers with no edit code (zoned digits);
- print_line: spacing and skipping, field ends, blank after, the overflow indicator, a flush every 50 lines;
- ensure_printer: OVRPRTF SPLFNAME and USRDTA, the job's spooled file or the print job's, the last spooled file noted for QSPRILSP;
- flush_printer: the spooled file, or the printed lines.

The printers, the overflow state and the printed lines are the interpreter's. Its pure helpers edit_word, edit_code_with, today_num and overprint are called as they are.

**Verified by** the native unit NEXCPT, under both engines. The program captured on PUB400 (tools/golden/native78.txt n78_except) prints:
- HDR with PAGE edited Z;
- DTL with a character field, edit codes J and Z, and a constant conditioned by an indicator, twice (on, then off);
- an unnamed record with space after 2;
- TOT with a field blank after and an edit word;
- DTL again, the cleared field now blank.

PASE system printed the spooled file on standard output. That shows no blank lines and adds PUB400's banner, so the unit compares the program's printed lines that have text, without the banner. Its program leaves out the capture's CLOSE and its reading of the spooled file with SQL (printer CLOSE is not lowered yet; the unit has no spooled file).

native_tripwire.rs watches the functions written again.

## S2.79: WRITE, OPEN, CLOSE and FEOD of printer files

**Lowered**, with no error extender or resulting indicators:
- WRITE of an externally described printer file's record format;
- OPEN, CLOSE and FEOD of a printer file.

Both are lowered for a file of the program, not of a procedure. The fields the format prints must be character, varying or numeric fields with no index; a date or time field is not lowered. The INDDS data structure is placed when the file names one. A program-described printer file's WRITE of a data structure stays in the interpreter.

**Runtime.**
- `printer_write_n` and `printer_op_n` write again errors.rs file_op for these operations: the error status off, a savepoint under commitment control, then file_op's end (`file_end_n`, S2.58).
- Inside them they write again printer.rs:
  - printer_write: the record format, the indicators from INDDS's bytes or *IN01-*IN99, PAGNBR's page after SKIPB and SPACEB, each printed field's text, then crate::prtf::layout as it is;
  - printer_text: DATE, TIME, PAGNBR, fields, edit words and codes, zoned digits;
  - print_ops;
  - printer_op: OPEN and FEOD flush the lines kept; CLOSE flushes them and closes the spooled file.
- They share S2.78's printer helpers.

**Verified by** the native unit NPRTW, under both engines. The program captured on PUB400 (tools/golden/native79.txt n79_write) prints a printer file made from the capture's DDS (NPRTWP.prtf):
- HDR with a constant and PAGNBR;
- DTL with a character field, EDTCDE(J) and (Z), and a constant conditioned by 50 (on, then off);
- TOT with SPACEB and an EDTWRD;
- then CLOSE.

The unit compares the printed lines that have text, as S2.78's does. NEXCPT now also has the capture's CLOSE.

native_tripwire.rs watches printer_op, printer_write, printer_text and print_ops.

## S2.80: IN and OUT of data areas

**Lowered:** IN and OUT ([*LOCK] field, or *DTAARA: every data area of the program in name order), with no error extender or indicators, as interp.rs's IN/OUT arm loops. Each field must be:
- a data area of the program (DEFINE *DTAARA or DTAARA);
- a character, varying or numeric field, or a data structure, with no index;
- when its DTAARA is *VAR:name, with a character field `name` that holds the data area's name.

**Runtime.** `data_area_n`, `dtaara_unlock_object_n` and `local_data_area_n` write again interp.rs data_area, dtaara_unlock_object and local_data_area:
- *LDA is the local data area's bytes; *GDA and *PDA are not available (RNX0402);
- any other data area is resolved on the library list;
- IN: *USE authority, IN *LOCK's *EXCLRD lock (MCH5802 after the wait), and the value parsed for a numeric field;
- OUT: *CHANGE authority, the value written, and the program's and the job's locks given back.

The program's held locks are the interpreter's (`dtaara_locks`), so its end and errors release them as before.

**Verified by** the native unit NDTAARA, under both engines. The program captured on PUB400 (tools/golden/native80.txt n80_dtaara) runs:
- a character data area: IN, IN *LOCK with OUT twice;
- a decimal one: IN *LOCK, add, OUT, IN;
- a fixed-form DTAARA(*VAR:name);
- IN of *DTAARA;
- the local data area as a data structure.

Three IBM i behaviors are not as4tainr's (fidelity.md):
- *DTAARA in free form: NDTAARA reads the two data areas one by one instead;
- DTAARA(name) in free form;
- OUT with no lock held.

native_tripwire.rs watches the three functions.

## S2.81: %EDITC's third argument

**Lowered:** %EDITC (S2.26) with a third argument: *ASTFILL, *CURSYM, or a literal. Another expression is not lowered.

**Runtime.** `rpgrt::native::editc_third` writes again interp.rs bif's EDITC arm on the edited text:
- *ASTFILL puts asterisks for the leading blanks;
- *CURSYM puts '$', and a literal its text without trailing blanks, right before the first character that is not blank.

**Verified by** the native unit NEDITC3, under both engines. The program captured on PUB400 (tools/golden/native81.txt n81_editc) runs edit codes J, 1, A, 3 and K with each third argument over 1234.50, -42.00 and 0. PUB400's compiler takes no asterisk fill or floating symbol with Z (RNF6068).

For zero with K and '+', IBM i gives all blanks, while the arm puts the '+' after them. That is an interpreter defect (fidelity.md); NEDITC3 leaves that part out.

native_tripwire.rs watches bif, which holds the arm.

## S2.82: date, time and timestamp fields of database files

Lowered file operations (S2.58, S2.59) now take a file whose date, time or timestamp fields go to program fields of those types. The move is the character one, as fill_fields' Value::str and record_of's characters make it: the record's *ISO text into the field with the strict fit, and back. data::store and decode, which both engines use, convert for the program field's format.

**Verified by** the native unit NDATEF, under both engines. The program captured on PUB400 (tools/golden/native82.txt n82_datefile) uses a table of int, date, time and timestamp columns:
- two records written;
- read in arrival order;
- the first updated with %MONTHS (January 31 to February 28), %HOURS and %SECONDS (to the next day);
- read again after SETLL *START.

## S2.83: fixed-form string operations

**Lowered:** SCAN, CHECK, CHECKR, SUBST, XLATE and CAT, with the P extender or none (no E). Their factors are taken apart as ops.rs split takes them: `a:b`, with *BLANKS first meaning none.
- A character factor is a character or hexadecimal literal, or a character or varying field with no index. A figurative constant or a number is not lowered: chars_of would size or edit it.
- A numeric factor is a number, or a numeric field with no index.
- SCAN, CHECK and CHECKR put into a numeric field, into a whole global numeric array (every position), or into nothing. SUBST, XLATE and CAT put into a character or varying field.

Fields with no index cannot fail when read, so taking them before the operation sets its LO indicator off makes no difference.

**Runtime.** `str_op_n` writes again string_op's arms for these operations, without indexing:
- SCAN's and CHECK's search, and CHECKR's from the right, with put_positions (an array gets every position, the rest zero);
- SUBST's piece, XLATE's translation and CAT's joining (factor 1's trailing blanks dropped when a number of blanks is given), with put_left (the result's current value, blanks first with P, the truncating fit);
- op_error's LO indicator, or RNQ0100.

**Verified by** the native unit NSTROPS, under both engines. The program captured on PUB400 (tools/golden/native83.txt n83_strops) runs:
- SCAN into a field (with EQ), into a whole array, with a start, with a length, not found, and with a zero length in a field (a literal 0 is RNF5218), its LO indicator on;
- CHECK into a field and into an array, and CHECKR;
- SUBST with and without P;
- XLATE;
- CAT with factor 1, with :1 blank, and with P and no factor 1.

native_tripwire.rs watches string_op, split, put_positions, put_left and op_error.

## S2.84: fixed-form bit and test operations

**Lowered:** TESTN, TESTZ, TESTB, BITON and BITOFF, with no extender, whose result is a character or varying field with no index. The bits of TESTB, BITON and BITOFF are a literal of bit numbers 0-7 (bit_mask's mask, made when lowering; another digit, the interpreter's error, is not lowered) or such a field, whose first byte is the mask.

**Runtime.** `bit_op_n` writes again ops.rs bit_op's arms, and interp.rs set_result_inds:
- TESTN: digits, a sign zone on the last digit; leading blanks low; all blanks equal;
- TESTZ: the first character's zone;
- TESTB: off high, mixed low, on equal;
- BITON and BITOFF: the one byte, stored with the strict fit, or the error for a longer field.

**Verified by** the native unit NBITOPS, under both engines. The program captured on PUB400 (tools/golden/native84.txt n84_bitops) runs:
- TESTN on '123', ' 12', '1A3', blanks and '12C';
- TESTZ on A, J, 1, & and -;
- BITON '0167', and TESTB with two and three bits;
- BITOFF '0', and TESTB of one bit (no low indicator: RNF5117);
- TESTB with the bits of a field, and BITOFF with a field.

native_tripwire.rs watches bit_op, bit_mask and set_result_inds.

## S2.85: fixed-form LOOKUP of arrays and XFOOT

**Lowered** (both of a global array that is not of varying dimension):
- LOOKUP of a character or numeric array, with no alternating array. A table (TAB...) is not lowered, because its current element is the interpreter's state. The search argument is a literal or a field with no index; the starting index is a numeric field, or none.
- XFOOT of a numeric array into a numeric field, with or without H.

**Runtime.** `array_op_n` writes again:
- ops.rs lookup: the starting index's range check (RNQ0121), the first equal element when EQ is asked for, else the nearest above (HI) or below (LO), the indicators, and the index set to the element found, or 1;
- array_op's XFOOT: the sum, the truncating fit with H, and interp.rs num_inds on the value stored.

**Verified by** the native unit NARROPS, under both engines. The program captured on PUB400 (tools/golden/native85.txt n85_arrops) runs:
- LOOKUP of a character array, with an index (found, from a later start, not found) and without one;
- LOOKUP of an ascending packed array with HI, LO, HI and EQ, and HI past every element (fields of the array's type: literals of another are RNF7021);
- XFOOT into fewer decimals, with its indicators, and XFOOT(H).

%CHAR of an indicator is lowered too, as its character '1' or '0' (bif's CHAR arm for a Value::Ind; the same value as an indicator in a concatenation, S2.62). NARROPS shows its indicators that way.

native_tripwire.rs watches array_op, lookup and num_inds.

## S2.86: fixed-form SORTA and MOVEA of character data

**Lowered:**
- SORTA of a whole global array that is not of varying dimension (character, varying or numeric), DESCEND from the program's sequence;
- MOVEA whose sides are global fixed-length character arrays (from an index field's element or the first) or fixed-length character fields, factor 2 also a character literal, with P or not. Numbers, whose zoned images move_image makes, are not lowered.

**Runtime.**
- `sorta_n` writes again ops.rs sorta: a stable sort by data::compare, reversed for DESCEND. For an array overlaying an array of a data structure (its stride is not its size, or it has a lead), whole overlaid elements move by this key.
- `movea_parts_n` and `movea_n` write again movea_parts and movea for character data: factor 2's bytes over the result's, left to right; P blanks the result first; an index outside its array is RNQ0121.

**Verified by** the native unit NSORTMV, under both engines. The program captured on PUB400 (tools/golden/native86.txt n86_sortmovea) runs:
- SORTA of a character array, of a DESCEND packed array, and of a key overlaying a data structure array (the whole rows move);
- MOVEA of an array into a field, of a field into an array from index 2, and of a literal with and without P.

native_tripwire.rs watches sorta, movea_parts and movea.

## S2.87: procedure pointers

**Lowered:**
- %PADDR of a literal or a name, resolved as interp.rs bif's PADDR arm resolves it: the prototype's EXTPROC, a named constant's value, or the name itself;
- calls through a prototype with EXTPROC(pointer), where the pointer is a scalar pointer field.

**Runtime.**
- `paddr_n` writes again the arm's search for where the procedure is: this module, with its service program and module when it is not exported; *MODULE for a module bound into a program; *PGM for a program's entry module; else what the binder found. The procedure pointer it makes is proc_id's.
- `procptr_target_n` writes again call_named_inner's EXTPROC(pointer) branch:
  - the pointer field read when the call is made (MCH3601 when it is not a valid procedure pointer, or not set);
  - a procedure of another program, module or service program called there through call_bound_in, the interpreter's call services (ADR 0007's external calls);
  - one of this activation going on as a call by its name (`call_proto_target_n`, call_proto's end split off).

**Verified by** the native unit NPROCPTR, under both engines. The program captured on PUB400 (tools/golden/native87.txt n87_procptr) runs:
- %PADDR by name and by literal;
- calls through the pointer as it changes (ADD 5, MUL 6);
- comparisons of procedure pointers;
- a call through a *NULL pointer caught by MONITOR.

A procedure this activation has is called as call_proc calls it: more arguments than it has parameters are passed through a prototype that lists more (it does not see them; %PARMS counts them), so `call_frame` checks the count only for a call by the procedure's own name (`checked`), as call_named_inner's args_for does. OSSILE's OSSPARMS (rla_test's rpg_features_found_by_the_ossile_corpus) calls a one-parameter procedure through a two-parameter pointer prototype, and failed in the first native-diff of S2.87 before this.

The call through *NULL is status 202 on IBM i but 222 in as4tainr, an interpreter defect (fidelity.md); NPROCPTR does not check that status.

## S2.88: %STR as a value

**Lowered:** %STR(p) and %STR(p: max), where p is a lowered pointer value (S2.47-S2.50: a pointer field, %ADDR, pointer arithmetic, %ALLOC) and max is a numeric expression, evaluated after the pointer as bif's STR arm does. %STR as an assignment target stays in the interpreter. %LEN of a character expression that is not a name (a function, a concatenation, a literal) is lowered too: interp.rs type_of gives such an expression no type, so bif's LEN arm takes its value's length.

**Runtime.** `Runtime::str_at` writes again ptrs.rs str_at: the pointer's place (`addr_n`), then the bytes up to the first null, within max (else 16 MB) and the storage's end.

**Verified by** the native unit NSTRPTR, under both engines. The program captured on PUB400 (tools/golden/native88.txt n88_str) takes %STR of a field holding 'Hello', a null, 'World' and a null:
- of its address;
- with a maximum of 3;
- of a pointer field moved past the first null;
- of a moved address with a maximum;
- %LEN of a %STR.

## S2.89: FOR-EACH over whole arrays

**Lowered:**
- FOR-EACH's start over a whole global array of fixed dimension;
- its step into a field that is not based. The step is lowered on its own, so a loop over another list (%LIST, %SPLIT) still steps natively.

**Runtime.**
- `each_array` writes again bifs.rs list_of for an array with no index: every element in order. It keeps them under run_code_at's EachInit key (the code, the start's instruction, the call depth), in the interpreter's own `each` state, so either engine can take the loop's next value.
- `each_take` is the EachNext arm's next value, or none at the end, which drops the state.
- Generated code stores the value with the strict fit, as the arm's assign does, or jumps to the loop's end.

**Verified by** the native unit NFOREACH2, under both engines. The program captured on PUB400 (tools/golden/native89.txt n89_foreach) runs:
- a packed array summed, its item keeping the last value;
- a character array into a shorter field (cut);
- LEAVE inside the loop;
- two loops nested.

## S2.90: ON-EXIT

Native calls of this module's procedures now take procedures with ON-EXIT. Before, lowering kept every call of such a procedure in the interpreter.

**Runtime.** `call_frame` writes again interp.rs call_proc's ON-EXIT:
- it runs however the body ended, before the call stack entry is popped;
- its indicator variable is on when the body ended abnormally;
- a RETURN there replaces the value of a normal end;
- an error there is the call's.

The exit routine runs through dispatch, so natively, since the native table has it (native/mod.rs push_proc).

**Verified by** the native unit NONEXIT2, under both engines. The program captured on PUB400 (tools/golden/native90.txt n90_onexit) runs:
- a normal end (trace "normal", value 5);
- an ON-EXIT RETURN replacing the value (99);
- a division by zero in the body, the ON-EXIT run with its indicator on ("abnormal"), and the caller's MONITOR catching status 202.

## S2.91: imported and exported fields

Fields and data structures defined with IMPORT or EXPORT are lowered like any other global. Before, ADR 0006's first rules kept them out.

**Why this is safe.** Their storage is the module's own buffers. The interpreter's share_in and share_out (bifs.rs) copy it to and from the program's shared copy, and nothing else reads that copy. They run only at the edges where a module's values can change hands:
- the program's start, and its end with or without LR;
- a call into another module or service program (call_bound_in);
- QCMDEXC's message ID (crt.rs).

The native runtime reaches all of these through the same Machine and the same services. So native code that reads and writes the buffers between those edges does what the interpreter does.

**Verified by** the native unit NDATAIMP, under both engines. It is a program of two modules (CRTRPGMOD and CRTPGM), as captured on PUB400 (tools/golden/native91.txt n91_data):
- the main module imports TOTAL, which the other module exports with INZ(100);
- the main module exports LABEL, which the other module imports;
- each side sees the other's changes across the calls: "100 x 105 mod5x 2 mod1y".

## S2.92: the review of the printer functions

An independent read-only review (codex) compared the printer functions of native_rt.rs (S2.78, S2.79) with the interpreter's. Three findings change the engine:
- **A printer file with an INFDS.** It stays with the interpreter (lower.rs `has_infds`), because file_end_n does not write infds_update again. Database reads already excluded it.
- **Blank after.** The native runtime writes the field's initial image over its storage, as the interpreter's CLEAR does (`out_clear_n`).
- **A named edit word constant.** It is lowered only when the procedure's name is the module's own constant (`std::ptr::eq` of the two lookups).

**Verified by** the native units NEXCPT2 and NPRTINF, under both engines, as captured on PUB400 (tools/golden/native92). The captures also showed five interpreter defects, recorded in fidelity.md and held for upstream. The units leave those parts out.

## S2.93: a procedure's own data structures as a whole

A procedure's local data structure, not based and not an array, is an operand as a whole: its bytes as Char(size), where Machine::sym places it. That is the frame's buffer, or the caller's storage for a parameter by reference; the runtime's `local` places it the same way. Before, only global data structures were operands. This lowers:
- EVAL with such a data structure as its target (interp.rs assign_value's Sym::Ds arm: data::store as Char(size));
- the data structure read as a value.

**Verified by** the native unit NLOCDS, under both engines, as captured on PUB400 (tools/golden/native93.txt n93_localds):
- a local qualified data structure given %SUBST of a buffer, with its subfields read and its whole read back into a character field;
- a LIKEDS parameter given a value whole, which the caller sees.

## S2.94: CLEAR

CLEAR is lowered in the forms interp.rs stmt's CLEAR arm takes through `sym`:
- **A field.** Every element, for an array, gets its type's initial image (`DType::initial`, made at lowering).
- **A data structure.** clear_ds writes each subfield in turn.
- **An indicator or *IN.** They are set off (`SetInd`).

Each area is a place, its image, its number of elements and their distance apart (`ir::ClearArea`). `rpgrt::native::fill` writes the areas, and a procedure local's place comes through the runtime's `local`.

**Stays with the interpreter:**
- a data structure with a data structure subfield, because clear_ds copies a LIKEDS DIM's first element to the others;
- an element of a data structure array;
- a record format;
- a based, multiple-occurrence or varying-dimension field;
- RESET.

**Verified by** the native unit NCLEAR, under both engines, as captured on PUB400 (tools/golden/native94.txt n94_clear). It clears:
- character, packed and varying fields, an int array, and a data structure with character, zoned and int subfields;
- a qualified data structure, an indicator and *IN;
- in a procedure, a local field and a local data structure.

## S2.95: RESET of globals

RESET of a global field (every element, for an array) or data structure is lowered as interp.rs reset_to_initial does it: its bytes are copied from the program's initial storage (`prog.globals`). The image is made at lowering and written as one CLEAR area (`ir::ClearArea`). Its bytes are part of the layout, so a changed INZ value changes the routine's key (ADR 0002).

**Stays with the interpreter:**
- a procedure's own names, whose initial storage is the frame's;
- based storage, multiple-occurrence data structures and varying-dimension arrays;
- images over 4096 bytes.

**Verified by** the native unit NRESET, under both engines, as captured on PUB400 (tools/golden/native95.txt n95_reset). After the values change, it resets:
- character and packed fields with INZ, and a field without INZ;
- an array with INZ, a data structure with INZ subfields, and a qualified one;
- from a procedure, a global field and the procedure's own local.

## S2.96: based arrays in whole-array operations (review)

A read-only review (codex) of bit_op_n, array_op_n, sorta_n, movea_n and paddr_n against ops.rs and interp.rs found a native bug. The whole-array forms let a based global array through:
- FOR-EACH (S2.89);
- SORTA (S2.86);
- fixed-form LOOKUP and XFOOT (S2.85);
- MOVEA (S2.86).

They placed it by its declared buffer, but the interpreter reaches based storage through its pointer, and gives MCH3601 when the pointer is null. Lowering now leaves based globals and multiple-occurrence subfields to the interpreter in those forms (lower.rs `global_moved`), as `variable` already did.

**Verified by** the native unit NBASARR, under both engines, as captured on PUB400 (tools/golden/native96.txt n96_based). Arrays based on a pointer to other arrays get XFOOT, LOOKUP, SORTA, FOR-EACH and MOVEA.

## S2.97: LIKEDS parameters by reference to this module's procedures

A call of this module's procedure with a data structure parameter (LIKEDS, not DIM) passed by reference, neither CONST nor VALUE, given a whole data structure of the same size, is lowered. The data structure can be global, qualified or a procedure's own.

The argument is args_for's `Arg::Ref(loc_of(a))`: the data structure's bytes as Char(size), which is S2.93's operand. call_frame aliases the parameter's buffer to the argument's place, as call_proc does, and the callee reaches its subfields through the runtime's `local`.

CONST and VALUE data structure parameters still stay with the interpreter, which may build a value or pass by reference (same_type_variable), as do LIKEDS DIM parameters.

**Verified by** the native unit NDSPARM, under both engines, as captured on PUB400 (tools/golden/native98.txt n98_dsparm). BUMP changes its parameter's subfields, and the caller sees them. It is given a global data structure (twice), a qualified one, and another procedure's local one.

## S2.98: a pointer minus a pointer

An EVAL or RETURN whose whole value is a pointer field minus a pointer field is lowered (`ir::NumExpr::PtrDiff`). `rpgrt::native::ptr_diff` writes again ptrs.rs ptr_arith's Sub arm: the distance between the offsets when both pointers address the same storage, and its error otherwise. A *NULL operand fails as the interpreter's path for it does. Inside a larger expression, the difference stays with the interpreter.

**Verified by** the native unit NPTRDIF, under both engines, as captured on PUB400 (tools/golden/native99.txt n99_ptrdiff):
- two pointers into the same field, both ways (5, -5);
- two into the same %ALLOC storage (7).

The capture's subtraction of pointers to two different fields is an interpreter defect (fidelity.md) and is left out.
