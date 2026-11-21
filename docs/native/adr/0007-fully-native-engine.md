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

## S2.99: calls returning a pointer

A call that returns a pointer is a pointer value (`ir::PtrVal::Call`): the value it returns, a pointer or *NULL, as eval gives it. It is lowered as `call_of` lowers the calls that return numbers:
- a procedure of this module;
- a procedure bound into the program;
- a C function.

Like any other pointer value, it can be an EVAL's value, the base of pointer arithmetic, or an argument.

**Verified by** the native unit NRETPTR, under both engines, as captured on PUB400 (tools/golden/native100.txt n100_retptr):
- a procedure returning `%ADDR(buf) + n`, read with %STR, and pointer arithmetic on the call's result;
- the C function strstr on null-terminated buffers: a match, and *NULL when there is none.

## S2.100: indicator fields as targets and conditions

Indicator fields (`dcl-s x ind`, subfields of that type) get a type of their own, `ir::Ty::Ind`: one byte, '1' or '0', whose value is `Value::Ind`. They enter lowering only through a narrow door, `ind_field`: not an array, not based, not in a multiple-occurrence data structure. `variable` still leaves them out, so no other form treats them as a number or as characters. Two forms take them:
- **EVAL of an indicator field** to *ON, *OFF or a condition (`ir::Lowered::EvalIndVar`): assign's data::store of DType::Ind, the value's truth;
- **an indicator field as a condition** (`ir::Cond::IndVar`): eval's Value::Ind, its truth, in IF, DOW, DOU and under NOT.

Comparing an indicator field, %CHAR of one, or an indicator field in a concatenation or as an argument still stays with the interpreter.

**Verified by** the native unit NINDFLD, under both engines, as captured on PUB400 (tools/golden/native101.txt n101_indfld). It uses a standalone field, a qualified subfield and a procedure's own, given *ON, *OFF, a comparison and another indicator field, in IF, NOT, DOW and DOU.

## S2.101: indicator parameters passed by value

An indicator parameter passed by value takes a condition as its argument (`ir::CallArg::Ind`): args_for's eval of the argument, a Value::Ind, which call_frame stores into the parameter as call_proc assigns it. This holds for an indicator field, *INxx, a comparison, NOT and the like. By reference and CONST indicator parameters still stay with the interpreter.

**Verified by** S2.100's native unit NINDFLD, as captured on PUB400 (tools/golden/native101 n101_indfld). Its YN procedure takes an indicator field by value five times, and those calls were the unit's last interpreted statements.

## S2.102: %LEN as a target

`%LEN(v) = n` of a varying field with no index that is not based is lowered (`ir::Lowered::SetVarLen`). The value comes first. Then `rpgrt::native::set_var_len` writes again assign_value's "LEN" arm: the range check from 0 to the maximum (RNQ0100), then two big-endian bytes for the length, `n as u16`.

**Verified by** the native unit NLENSET, under both engines, as captured on PUB400 (tools/golden/native102.txt n102_lenset). The capture showed two interpreter defects (fidelity.md), which the unit leaves out: the characters a longer length shows, and the error's status.

## S2.103: *NEXT

`arr(*NEXT) = value` of a global varying-dimension array is lowered (`ir::Index::next`). As interp.rs assign_value does, the index is the element after those in use, and the array grows to it, whatever its kind (`elem_grow` with growth allowed; RNQ0121 past its dimension). *NEXT is written only, and only as an EVAL's target (`target_var`). A codex review of 2026-11-19 found that the first version, in `variable`, also took `a(*NEXT)` read or passed by reference. The interpreter fails those as an undefined name, so they stay with it.

**Verified by** the existing native unit NVARDIM (S2.45), whose `au(*next) = 9` line ("NEXT 3 9") was its last interpreted statement.

## S2.104: indicator fields as characters

An indicator field is character data, '1' or '0' (`ir::CharExpr::IndVar`), in the two places where an *INxx indicator already was:
- %CHAR of it (bif's CHAR of a Value::Ind);
- a part of a concatenation (eval's + of an indicator next to characters).

**Verified by** the native unit NINDCHR, under both engines, as captured on PUB400 (tools/golden/native104.txt n104_indchar). It covers %CHAR of indicator fields, and indicator fields concatenated with text, with *IN42, with each other and with a qualified subfield, and a procedure's own.

## S2.105: indicator fields compared

A comparison with an indicator field on one side, and another indicator field, *ON or *OFF on the other, is lowered as a `Cond::Cmp`. The comparison is eval's: the field's Value::Ind against the other side's value, by data::compare (`rpgrt::native::cmp`). `ir::FigOp` gets On and Off, which only this comparison builds; `fig_op` still leaves *ON and *OFF out of every other form.

**Verified by** the native unit NINDCMP, under both engines, as captured on PUB400 (tools/golden/native105.txt n105_indcmp). It compares with *ON and *OFF on either side and with another indicator field (= and <>), in IF and DOW, and a procedure's own field.

## S2.106: float fields as EVAL targets (ADR 0008, part 1)

**Floats.** EVAL of a float field (4 or 8 bytes; standalone, subfield or a procedure's own; not an array, not based) to a decimal value is lowered. This is ADR 0008's first part, accepted by the user on 2026-11-19. The value is eval's, and assign's data::store of DType::Float converts it (`ir::Ty::Float`).

`variable` still leaves float fields out, so no float is an operand of a decimal expression; the interpreter would have done that arithmetic in float. Float operands, %FLOAT and float text come in later parts.

**Also in this block: the review fix to S2.97.** A codex review found a gap in the LIKEDS by reference call. The procedure interface's parameter must now be the prototype's own type: call_frame makes the descriptor from the interface's parameter, call_proc from the argument's. fidelity.md has the details.

**Verified by** the native unit NFLTEVL, under both engines, as captured on PUB400 (tools/golden/native107.txt n107_floateval):
- a product, a division, an integer field and a negative literal, into float(8) and float(4);
- a procedure's own float(8).

The float(4) values' %CHAR text is an interpreter defect (fidelity.md), and the unit leaves it out.

## S2.107: %CHAR of float fields (ADR 0008, part 2)

%CHAR of a float field is lowered (`ir::CharExpr::FloatChar`). `rpgrt::native::float_char` writes again bif's CHAR arm for a float: data::float_text of the field's float_value (interp.rs float_value, written again as `rpgrt::native::float_value`), with the field's bytes.

native_tripwire.rs now also watches the interpreter functions written again since S2.94: clear_ds, reset_to_initial, assign_value and float_value.

**Verified by** S2.106's native unit NFLTEVL (tools/golden/native107), whose %CHAR of float fields was the unit's last interpreted part. Its float(4) text is an interpreter defect, common to both engines and left out by the unit.

## S2.108: float arithmetic (ADR 0008, part 3)

An EVAL of a numeric or float field to an expression with a float operand is lowered (`ir::Lowered::EvalValue`, `ir::FloatExpr`). What counts as float is decided as interp.rs is_float decides it (`is_float_static`): a float field, a call returning a float, %FLOAT, + or - of one, and arithmetic with one.

Each part follows eval:
- a part with no float in it is a decimal expression, with its own precision rules;
- a float field is its loaded value;
- %FLOAT of a numeric value is `rpgrt::native::float_of`;
- + and - of a float is `float_neg`, under which a float stays a float;
- +, -, * and / are `float_bin`, eval's float arm: both sides as f64, the operation, MCH1211 for a division by 0.0, and the result a decimal when it fits, else the float.

assign's data::store then stores the value. ** and %FLOAT of character data still stay with the interpreter.

**Verified by** the native unit NFLTARI, under both engines, as captured on PUB400 (tools/golden/native108.txt n108_floatarith). It covers:
- a float times 2, a decimal divided by a float, minus a float, and %FLOAT(3) / 7 into float(4);
- float results into packed and integer fields;
- a procedure's own float;
- a float division by zero caught by MONITOR (status 102).

## S2.109: float comparisons (ADR 0008, part 4)

A comparison with a float operand on either side, by is_float, is lowered (`ir::Cond::FloatCmp`). `rpgrt::native::float_cmp` writes again eval's float arm for =, <>, <, <=, > and >=: both sides' values as f64 (float_value), then compared. The sides are float expressions (S2.108).

**Verified by** the native unit NFLTCMP, under both engines, as captured on PUB400 (tools/golden/native109.txt n109_floatcmp). It compares:
- a float with a literal and with a packed field;
- %FLOAT(1) / 3 with a float;
- in a compound condition, a DOW, and a procedure's own float.

## S2.110: %FLOAT of character data and %ADDR of a float (ADR 0008, part 5)

**%FLOAT of a character expression** (`ir::FloatExpr::FloatOfChars`). `rpgrt::native::float_of_chars` writes again bif's FLOAT arm for Value::Char: a number as %DEC reads it (char_number, also written again), else the text read in float form with a comma as the point, else char_number's RNQ0105. The result is a decimal when it fits.

**%ADDR of a float field.** It takes the field's place, as for any field.

**Not here: DSPLY of a float field.** A PUB400 capture of it through PASE system ended in RNX9998, so IBM i's value is unknown, and it stays with the interpreter.

**Verified by** the native unit NFLTCHR, under both engines, as captured on PUB400 (tools/golden/native110.txt n110_floatchar):
- %FLOAT of '12.5', of '1.5E3' and of a varying '2,5';
- %FLOAT of 'abc' caught by MONITOR (status 105);
- %ADDR of a float read back through a based float.

## S2.111: the review of the float code

A read-only codex review of S2.106-S2.110 found three differences from the interpreter. All three now follow it:
- **Unary + and -.** A decimal part under + or - of a float expression had no precision, because the runtime's `precision` could not step into `Expr::Unary`. It now can, and eval's rule applies.
- **The return type of a call.** `is_float_static` now asks for a call's return type as is_float does: the prototype's first, then the procedure's.
- **Calls in float expressions.** A float expression with a call in it stays with the interpreter (`has_call`). eval picks %FLOAT's path and the kind of + by the value the call returns, and a bound procedure may return its own type.

**Verified by** the native unit NFLTUNA, under both engines, as captured on PUB400 (tools/golden/native111.txt n111_floatunary). The capture shows the interpreter's 30-digit intermediate precision differing from IBM i (fidelity.md), and the unit leaves that value out.

## S2.112: %INT and %DEC of characters, %ERROR, %STATUS of a file

Three built-in functions are lowered:
- **%INT, %INTH, %DEC and %DECH of a character expression** (`ir::NumExpr::CharDec`). `rpgrt::native::char_dec` is those bif arms for a Value::Char: char_number of its text, then the usual rescale.
- **%ERROR** (`FileBit::Error`). It is the last status's error flag, as a condition and as %CHAR of it (`ir::CharExpr::CondChar`).
- **%STATUS of a file** (`ir::NumExpr::FileCode`, `Runtime::file_code`). It is that file's status code, found as interp.rs file_status finds it.

**Verified by** the native unit NCHRCNV, under both engines, as captured on PUB400 (tools/golden/native112.txt n112_charconv):
- %INT('123'), %INTH('12.5'), %DEC('-3.25':5:2) and %INT of a character field;
- %INT('abc') caught by MONITOR (105);
- %ERROR after TEST(DE) of a bad date.

The capture's second OPEN(E) of an open printer is an interpreter defect (fidelity.md), and the unit leaves it out.

## S2.113: %SQRT

%SQRT of a numeric expression is lowered (`ir::NumExpr::Sqrt`). `rpgrt::native::sqrt_bif` writes again bifs.rs's "SQRT" arm: RNQ0101 for a negative value, else ops.rs dec_sqrt with 15 decimal places, normalized. dec_sqrt is written again with checked arithmetic, and native_tripwire.rs now watches it.

**Verified by** the native unit NSQRT, under both engines, as captured on PUB400 (tools/golden/native113.txt n113_sqrt):
- %SQRT of 16, of 2 and of a packed field into packed(15:13);
- %SQRT(-1) caught by MONITOR (101).

%CHAR(%SQRT(10)) shows that the interpreter keeps 15 decimal places where IBM i keeps 29 (fidelity.md), and the unit leaves it out.

## S2.114: display files

WRITE, UPDATE, EXFMT, READ, READC and CHAIN of a display file (WORKSTN) are lowered (`ir::Lowered::Workstn`, `Runtime::workstn`). As for printer files (S2.79), the program fields of the file's formats come as places (`OutSlot`), together with each subfile's record number field and the INDDS.

The runtime writes again errors.rs file_op around interp.rs workstn_op, and workstn_op's helpers: ws_values, ws_fill, ws_inds, ws_set_ind, ws_interact and message_subfile. native_tripwire.rs watches them.

The display itself stays where it was: workstn.rs's state (frame, write, sfl_write, sfl_update, sfl_readc, sfl_chain, read_frame, apply) and the job's display channel, as the interpreter uses them.

**Stays with the interpreter:**
- OPEN and CLOSE;
- the E extender;
- a display file with an INFDS;
- a program with files of procedures or LIKEFILE parameters.

A subfile CHAIN's key is a numeric field, passed as its place, which the runtime reads where workstn_op evaluates the key: after the open and format checks. A codex review found that the first version read it before the operation started.

**Verified by** rla_test's display tests, whose expected values come from the ILE RPG Reference and DDS for display files. A stand-in display answers each screen. They run under both engines in the differential suite, and their display operations are now native:
- rpg_exfmt_with_a_display (CUSTINQ: EXFMT, the response indicators, *INKx, ERRMSG under *IN30);
- rpg_subfile_with_a_display (WRITE of a subfile, READC, UPDATE of a subfile record);
- rpg_read_of_a_display_record (WRITE and READ).

The database CHAINs and READs in those programs, which were interpreted before, are native too.

## S2.115: MOVE of dates, times and timestamps

MOVE and MOVEL with a date, time or timestamp field on either side are lowered (`ir::Lowered::MoveTemporal`). Factor 2 is a field or a literal, and the result a field with no index.

`rpgrt::native::move_temporal` writes again interp.rs move_temporal as a pure function:
- from temporal to temporal, with no factor 1, the timestamp keeps its other part;
- from temporal to character or numeric, in factor 1's format, else the field's or DATFMT or TIMFMT;
- from character or numeric to temporal, the rightmost or leftmost characters the format needs.

`Runtime::move_temporal` loads the result's current value, passes the program's DATFMT and TIMFMT, and stores with Fit::Truncate. native_tripwire.rs watches move_temporal.

**Verified by** the native unit NMOVDAT, under both engines, as captured on PUB400 (tools/golden/native115.txt n115_movedate). It moves:
- a date to character (*ISO, and factor 1 *MDY) and to numeric;
- character and numeric (*YMD) to a date;
- a time to character (MOVEL);
- a timestamp to a date, and a date into a timestamp.

## S2.116: OPTIONS(*TRIM)

A parameter with OPTIONS(*TRIM) is lowered as args_for takes it:
- the value path, so a CONST field goes by value, not by reference;
- the character value with its leading and trailing blanks removed (`CharExpr::Trim` on its own path);
- a figurative constant, which args_for leaves as it is, stays with the interpreter.

**Verified by** the native unit NOPTTRM, under both engines, as captured on PUB400 (tools/golden/native116.txt n116_trim). It passes, to *TRIM parameters of local procedures:
- a literal with blanks on both sides to a VALUE varying parameter;
- a field with blanks to a CONST character parameter;
- a %TRIMR argument.

## S2.117: calls of procedures with files of their own

A call of a procedure that declares files of its own is lowered. As call_proc does, the runtime's call_frame:
- opens them before the parameters are stored (errors.rs open_local_files; its error is the call's, and ON-EXIT still runs);
- closes them, except STATIC ones, after ON-EXIT (close_local_files).

These are the same Machine functions that open and close a program's files when it is activated, and that native code already relies on.

The file operations of such a program still stay with the interpreter. Lowering would first have to resolve a file name as file_idx does, the procedure's files before the program's.

**Verified by** the native unit NLOCFIL, under both engines, as captured on PUB400 (tools/golden/native117.txt n117_localfile):
- COUNT's file opens at each call and is read from the start (6, 6);
- NEXT's STATIC file stays open, so each call reads the next record (7, 8, 9).

## S2.118: file operations in programs with procedures' own files

Lowering now resolves a file name as errors.rs file_idx does (`file_of`): the routine's procedure's own file by name, then by format, then the program's, then file_index. `Cx` carries the procedure.

Before, a file of any procedure kept every file operation of the program with the interpreter. Now database and display file operations take a procedure's own files, which the runtime's file_idx_n finds the same way at run time.

**Still with the interpreter:**
- a program with a LIKEFILE parameter, since file_idx looks at the frame's files first;
- a procedure's own printer file, since printers are kept by file name.

**Verified by** S2.117's native unit NLOCFIL (tools/golden/native117 n117_localfile), whose READs of COUNT's and NEXT's own files are now native.

## S2.119: reads into a result data structure

CHAIN, READ, READE, READP and READPE of a database file into a result data structure are lowered (`FileRead::ds`, `Runtime::file_read_ds`), as bifs.rs record_to_ds takes them:
- each field of the record goes to the subfield named like its program field or its file field (ds_members, the data structure's name before it when qualified);
- the moves are the ones of the program fields, `field_value` and a store with Fit::Strict;
- there are no null checks, since fill_fields' ALWNULL handling is not record_to_ds's.

**Stays with the interpreter:**
- hexadecimal fields;
- program-described files;
- WRITE and UPDATE from a data structure (ds_to_record).

A procedure's own file is implicitly qualified, so every I/O on one takes a result data structure (RNF7274 on PUB400 otherwise). With S2.118, a procedure's own files are now native.

**Verified by** S2.117's native unit NLOCFIL (tools/golden/native117 n117_localfile). Its READs into LIKEREC data structures, of COUNT's file and of NEXT's STATIC file, are now native.

## S2.120: WRITE and UPDATE from a result data structure

WRITE and UPDATE of a database file from a result data structure are lowered (`FileChange::ds`, `Runtime::file_change_ds`), as bifs.rs ds_to_record makes the record:
- each field comes from the subfield named like its program field or its file field;
- a field with no such subfield is left out of the record;
- numbers are rescaled to the field's decimal places and characters are taken as they are (`field_text`);
- the record carries no null values, since ds_to_record does not add record_of's ALWNULL(*USRCTL) ones.

**Stays with the interpreter:**
- DELETE;
- %FIELDS;
- hexadecimal, date, time and timestamp subfields;
- a subfield whose type is not the field's kind;
- program-described files.

A qualified file's format named as `file.format` is not found by the interpreter (fidelity.md), so lowering leaves it to the interpreter too.

**Verified by** the native unit NDSIO, under both engines (tools/golden/native120.txt): two WRITEs from a LIKEREC(*OUTPUT) data structure, a READ and an UPDATE from a LIKEREC(*INPUT) one, then the reads. IBM i gives `50:zz 6:de` with the file qualified (n120_dsio) and not qualified (n120_dsiou).

## S2.121: native-diff's runs with the same library names

The 23 programs that `rust` mode refused whole (native-diff level 2, "none generated") had no native code because their source was different in every run. Tests such as rla_test.rs's put their library name in the RPG text (`qlichgll('{lib}': ...)` in change_and_retrieve_library_list). setup makes that name from the process ID and a sequence number ("T<pid><seq>"), so it changed between the generation run and the `rust` run.

scripts/native-diff.sh now makes its three runs of the suite (generation, auto, rust) the same way:
- Each test binary runs as process 1 of a PID namespace of its own. Cargo's target runner (`CARGO_TARGET_<host>_RUNNER="unshare -cpf"`) does this and keeps the current user, so no root privileges are involved.
- Its tests run one at a time (`--test-threads=1`).

The names, and the source that has them, are then the same in every run. Two runs of rla_test's library_list tests gave the same fingerprints (2026-11-19).

The tests themselves are unchanged. A machine where `unshare -cpf` fails makes the script stop with a message rather than run differently. The cost is time: rla_test one test at a time, about 270 s with two at a time on the 2-CPU machines.

The first validation with it (jsr-test-2, s7f-s2121) failed in ways that came from the harness, not from the engine:
- **Leftover rows.** With the same names in every run, the auto and rust runs met rows the reference run had left on the same database: duplicate keys in logical_files and display_files, since setup removes a library's objects and files but not every catalog table's rows. Each of the three runs now has a new database of its own (`suitedb`).
- **Split result lines.** A test that writes to standard error while it runs splits its result line when tests run one at a time (`test NAME ... <text>`, then `ok`), and `results` missed it. It now reads the result from the next line.
- **A broken premise.** native_engine.rs's NATSW1 must have no generated code: its text carried the process number, which is now always 1. It now carries the number and the time the process first asked (`run_tag`), so its premise holds as before.

Upstream's tests are still not touched.

## S2.122: %SUBST with expressions as its start and length

%SUBST's start and length are now numeric expressions (`NumExpr`), not only fields and literals, as interp.rs's "SUBST" arm evaluates them:
- the string first, then the start, checked against it (RNQ0100), then the length.

Programs that call APIs take fields out of a receiver variable this way (`%subst(j: 81 + (j.nsys + j.nprd) * 11: 11)`, rla_test.rs CHGLL, RTVJOBD, MSGQL, NETA), and many of the programs S2.121 makes generatable did.

**Verified by** the native unit NSUBSTX, under both engines, as captured on PUB400 (tools/golden/native122.txt n122_subst: `ghij qr 567890 err100`):
- expressions of a field for the start and the length;
- a %LEN expression;
- %SUBST of a whole data structure;
- a start of 0 under MONITOR (status 00100).

## S2.123: %CHAR of a condition

%CHAR of a comparison, of AND or OR, or of NOT is lowered (`CharExpr::CondChar` of the condition, as IF takes it). eval gives the condition's Value::Ind, and interp.rs bif's CHAR arm makes it '1' or '0'. API programs show a flag this way (`%char(apierr.avail > 0)`, rla_test.rs MSGAPI; MIWAIT, FTOKT).

**Verified by** the native unit NCHARCND, under both engines, as captured on PUB400 (tools/golden/native123.txt n123_charcond: `100110 1`). It covers numeric and character comparisons, OR, AND, NOT, and an arithmetic comparison.

A condition inside an expression is named by its place in the statement (`cc{pc}_{path}`), and its sub-conditions' helpers after that name (rpgcg `cond_fns_in`). A statement's own condition keeps `c{pc}_{n}`. The unit's first generated code had several %CHAR conditions in one statement, which defined `c0_1` twice and did not compile; generated-code clippy caught it.

## S2.124: a based data structure as a whole

A based data structure named as a whole is now a variable of lowered code, as its based subfields have been since S2.48. It is a character field of the structure's size at the place its pointer gives now (`ir::Based`, `Runtime::based_at`, the interpreter's based_place). This holds for a global structure and for a procedure's own, and the pointer must not be based itself.

So it can be assigned, taken by %SUBST and passed by reference, the way API programs pass a receiver variable (rla_test.rs RCVKEY: `qmhrcvpm(hdr: 2048: ...)` with `hdr` based on an allocated pointer). An array or multiple-occurrence structure still stays with the interpreter.

**Verified by** the native unit NBASEDDS, under both engines, as captured on PUB400 (tools/golden/native124.txt n124_basedds: `7abcd wxyz EFGH`):
- subfields set through the pointer;
- the whole structure passed to a CHAR(100) OPTIONS(*VARSIZE) parameter that a procedure changes;
- %SUBST of the structure;
- the whole structure assigned.

## S2.125: a whole data structure for a local procedure's character parameter

A whole data structure passed to a character parameter of this module's procedure is lowered, when that parameter is a field and not LIKEDS. args_for treats it as it does for any call (S2.60), with its bytes as loc_of gives them:
- by reference when the types match, or with OPTIONS(*VARSIZE);
- for CONST, by reference when the types match or with OPTIONS(*VARSIZE), and as its value otherwise;
- for VALUE, as its value.

call_frame then takes these arguments as call_proc does. A LIKEDS parameter still takes only S2.97's by-reference form. A data structure array (LIKEDS DIM CONST) stays with the interpreter.

**Verified by** the native unit NDSCHRP, under both engines, as captured on PUB400 (tools/golden/native125.txt n125_dsparm: `XYZ999 QYZ QYZ999 6 QYZ`). The procedures change the caller's subfields by reference and through OPTIONS(*VARSIZE), return a CONST copy of the same length and one of another length padded with blanks, and change a VALUE copy without touching the caller's.

## S2.126: indicators compared with *ON and *OFF

An indicator (`*INnn`, `*INLR` and the like) compared with `*ON` or `*OFF` by `=` or `<>` is lowered, on either side. data::compare orders an indicator against the other side's truth (`a.cmp(&b.truth())`), so the comparison is the indicator itself or its negation. This is fixed form's `*IN99 DOUEQ *ON` and `*IN50 IFEQ *ON` (samples/apps/contacts CONTCNT, CONTLST), and free form's `if *in50 = *on`. Other comparisons, and two indicators compared, stay with the interpreter.

AS4TAINR_NATIVE_WHY now also notes each IF, DOW, DOU or WHEN condition that is not lowered (`cond=` and its nodes).

**Verified by** the native unit NINDFIG, under both engines, as captured on PUB400 (tools/golden/native126.txt n126_indfig: `acde3`). It covers = and <> with *ON and *OFF on either side, and a DOU until an indicator is on.

## S2.127: a call compared in a condition

A comparison of two names took the operand path. A call is a name with arguments, as in `if fact(6) = 720`, but it is not an operand, so lowering gave up on the condition. A side with a call now takes the expression path (S2.13), where calls in numeric and character expressions are already lowered. Both sides are evaluated in order, the left first, as eval does.

**Verified by** native units that were already there. Their IF on a call's result now runs natively:
- NPROC: the recursive `fact(6) = 720`, expected values from IBM Docs, "Passing by Reference" and "FOR";
- NCALLS;
- NCFUNC: C functions compared, captured on PUB400 (tools/golden/native57.txt n57_c).

The WHY pass gave these as `cond=bin:Eq,call:...`.

## S2.128: OPEN and CLOSE of a database file

OPEN and CLOSE of a database file are lowered (`Lowered::DiskOpenClose`, `Runtime::disk_open_close`) as errors.rs file_op runs them:
- the error flag off and the savepoint under commitment control, as file_read_n starts;
- then file_op_inner's arms: OPEN of an open file is CPF4161 (status 01215), else `open_file`; CLOSE closes the open file if there is one (`close_open_file`), and a closed file is no error;
- then file_op's end (file_end_n).

**Stays with the interpreter:**
- a file with an INFDS, which open_file writes and file_end_n does not write again;
- the E extender and resulting indicators;
- printer files, which keep S2.79's path.

**Verified by** the native unit NOPNCLS, under both engines, as captured on PUB400 (tools/golden/native128.txt n128_openclose: `50 1215 closed 50`):
- a USROPN file opened and read;
- opened again while open under MONITOR (01215);
- closed twice (no error);
- opened and read again from the start.

NDSIO's OPEN and CLOSE (S2.120) are native now too.

## S2.129: %PARMS

%PARMS is lowered (`NumExpr::Parms`, `Runtime::parms`) as interp.rs bif's PARMS arm reads it: the number of arguments passed to the running procedure, which is its frame's `passed`, or the program's `parms_passed` in the main procedure. The runtime's call_frame records `passed` as call_proc does, so an *OMIT argument counts and an argument not passed does not.

AS4TAINR_NATIVE_WHY now also notes each RETURN that is not lowered (`return=`). It gave `return %parms` four times and `if %parms >= n` once.

**Verified by** the native unit NCALL, captured on PUB400 (tools/golden/native44.txt n44_call: `PARMS 1 1`, `PARMS 2 2` for a procedure with an OPTIONS(*NOPASS) parameter). Its `return %parms` is now native.

## S2.130: a field based on a based pointer

A based field or data structure may now be based on a pointer that is itself based, once (`base_pointer_ok`). This is the pattern of a subfield of a based structure: `dcl-s s char(5) based(h.p)` with `h` based on a pointer parameter, as in rla_test.rs RDR's callback. Its own pointer must then not be based.

The generated code reads the pointer as `value` reads any based field, through `Runtime::based_at`. That is where the interpreter's based_place finds it through `sym`.

**Verified by** the native unit NBASED2, under both engines, as captured on PUB400 (tools/golden/native130.txt n130_based2: `Jello Jello Jel XYZlo`). A procedure changes and returns such a field over the caller's storage, and a global one is read and assigned.

## S2.131: IN %RANGE and IN %LIST

`x IN %RANGE(a:b)` and `x IN %LIST(v1:...)` in a condition are lowered (`Cond::In`), as bifs.rs eval_in runs them:
- x first, then the range's two values, or every value of the list (bif LIST evaluates them all);
- then data::compare. For a range that is a <= x <= b, the second comparison only when the first holds, as `&&` does. For a list, x must equal one of the values, in order.

All the sides are of one class, numeric or character. IN an array or an enumeration stays with the interpreter. As for any comparison (S2.13, `side`), arithmetic on a side stays with it too, because a condition has no target to take the precision of its intermediate results from: the unit's `n * 2 in %list(3 : 10)` runs in the interpreter.

**Verified by** the native unit NINLIST, under both engines, as captured on PUB400 (tools/golden/native131.txt n131_inlist: `acdefg`). It covers numeric and character ranges and lists, NOT of IN, an expression on the left, and a range of one value.

## S2.132: RETURN of a figurative constant

A RETURN of a figurative constant is lowered (`*ON`, `*OFF`, `*BLANKS`, `*ZEROS`, `*HIVAL`, `*LOVAL`, `*ALL'x'`, `*NULL`). Its value is eval's `Value::Fig`, the same value the generated code makes for that operand, and the caller's return field takes it as the interpreter's does. This is the usual `return *on` and `return *off` of a procedure returning an indicator (rla_test.rs RUNITF).

**Verified by** the native unit NRETFIG, under both engines, as captured on PUB400 (tools/golden/native132.txt n132_retfig: `10[   ].00 999 abab`):
- *ON and *OFF from an indicator procedure;
- *BLANKS into CHAR(3), *ZEROS into PACKED(5:2) (whose %CHAR is `.00`), *HIVAL into PACKED(3:0), and *ALL'ab' into CHAR(4).

## S2.133: a call returning an indicator as a condition

A call of a procedure that returns an indicator is a condition now (`Cond::Call`): eval's call, then its value's truth, as IF takes it. So `if isbig(x)`, `not isbig(x)`, AND and OR of such calls, and DOW and DOU on one are lowered. %CHAR of such a call is the indicator's '1' or '0' (`CondChar` of the call).

**Verified by** the native unit NINDCALL, under both engines, as captured on PUB400 (tools/golden/native133.txt n133_indcall: `ab61`). It covers IF, NOT, AND, a DOW loop until the call is true, and %CHAR. NRETFIG's `%char(isbig(7)) + %char(isbig(2))` (S2.132) is native now too.

A RETURN of a comparison, AND, OR or NOT is lowered too (`Lowered::ReturnInd`, NINDCALL's `return x > 5`). It gives eval's Value::Ind of the condition, the value the interpreter's RETURN passes back, not a character.

## S2.134: %CHAR of a date, time or timestamp expression

%CHAR of a date, time or timestamp expression is lowered (`CharExpr::TemporalCharOf`). The expression is %DATE, %TIME or %TIMESTAMP, or + or - of a duration, whose kind is its base's (`temporal_kind`, bifs.rs temporal_type).

It follows bif_override. The value is temporal_value's, and the format is the explicit special value, else the control specification's DATFMT or TIMFMT. temporal_type gives an expression the default type, so a field's own format does not count. A timestamp is *ISO0 or not.

That default format is the interpreter's, and IBM i differs from it under DATFMT or TIMFMT other than *ISO: IBM i gives *ISO (fidelity.md). Native code follows the interpreter.

**Verified by** the native unit NTCHREX, under both engines, as captured on PUB400 with the default formats (tools/golden/native134.txt n134_tchariso: `10.00.00`, `2026-02-01-11.59.59.000000`, `2026-10-20`, `02/01/2026`, `2025-12-31`). It covers a time and a timestamp plus a duration, %DATE of characters with *MDY, a date plus days with *USA, and a date minus months.

## S2.135: OPTIONS(*VARSIZE) for this module's procedures

A character argument of any length for an OPTIONS(*VARSIZE) parameter of this module's procedure is lowered by reference. Since S2.51 that held only for programs and bound procedures. The call passes the argument as loc_of gives it, with its own size and type.

call_frame now makes a reference argument's operational descriptor from that type, as call_proc does from the Loc's:
- a varying one's current length (4, or 11 for UCS-2);
- UCS-2's characters (9);
- else its size (2).

Before, it used the parameter's type, which a *VARSIZE parameter does not share. With the types the same, as for any other reference argument, nothing changes.

**Verified by** the native units NDSCHRP (S2.125, `vsize(d)`) and NBASEDDS (S2.124, `fill(hdr: %size(hdr))`), as captured on PUB400. Their *VARSIZE calls to local procedures, which the WHY pass gave as `callp=local why=options`, are now native.

## S2.136: a figurative constant assigned to %SUBST

An EVAL of a figurative constant to %SUBST is lowered (*BLANKS, *ZEROS, *HIVAL, *LOVAL, *ALL'x', *NULL; not *ON or *OFF). The value is eval's Value::Fig. assign's SUBST arm fills the part with it by data::store, and so does `subst_target`. Examples: `%subst(x: 1: 3) = *blanks`, rla_test.rs's `%subst(jid: 49: 8) = *allx'00'`.

**Verified by** the native unit NSUBFIG, under both engines, as captured on PUB400 (tools/golden/native136.txt n136_substfig: `[a   efgh][--  efgh][--  0000]h`). It covers *BLANKS, *ALL'-', *ZEROS to the end of the field, and *HIVAL.

## S2.137: %NULLIND

%NULLIND(field) is lowered in three places:
- as a condition, `Cond::NullInd` and `Runtime::nullind`;
- in %CHAR, as its '1' or '0';
- as an EVAL target, `Lowered::EvalNullInd` and `Runtime::set_nullind`, with *ON, *OFF or a condition.

The runtime calls the interpreter's own nullind_field:
- ALWNULL(*USRCTL) is required;
- the field must be a null-capable file field.

Then the field is looked up in, or put in or taken out of, the machine's null indicators, which the native file reads set (fields_read_n) as fill_fields does. As in Stmt::Eval, the value is evaluated before the target's checks.

**Verified by** the native unit NNULLIND, under both engines, as captured on PUB400 (tools/golden/native137.txt n137_nullind: `11n0 20 2026-01-31`):
- a null date read, then tested, in %CHAR, and set off;
- then a record whose date is not null.

## S2.138: TEST(D), TEST(T), TEST(Z)

TEST(D), TEST(T) and TEST(Z) of a character or numeric field are lowered (`Lowered::TestTemporal`, `Runtime::test_temporal`), as ops.rs string_op's TEST arm runs them:
- the field's value as text; a number is its whole digits, in the format with a 0;
- checked against factor 1's format, or *ISO with none;
- then %ERROR with E, the indicator in positions 73-74, or RNX0112.

The tripwire already watches string_op.

**Verified by** the native unit NTESTDT, under both engines, as captured on PUB400 (tools/golden/native138.txt n138_test: `10100`):
- a date that is not valid, and a valid zoned *ISO one;
- a time that is not valid, and a valid timestamp;
- a *USA date.

A TEST with neither E nor an indicator does not compile on IBM i (RNF5056), and the interpreter takes it (fidelity.md).

## S2.139: database and printer files with an INFDS

The runtime's file_end_n now ends with the interpreter's own `infds_update`, as errors.rs file_op ends every operation. It writes the file information data structure from the status just made. That status includes the routine and the statement, which the generated code keeps up to date with `rt.line` and the calls.

So the operations of a database or printer file with an INFDS are lowered: reads, WRITE, UPDATE and DELETE, OPEN and CLOSE, and printer WRITE, OPEN, CLOSE and FEOD. A display file's INFDS, with its attention key, stays with the interpreter.

AS4TAINR_NATIVE_WHY also notes each file operation that is not lowered:
- its device, and whether the file is a procedure's own;
- whether it has an INFDS;
- its key, result data structure and %FIELDS;
- its extenders and resulting indicators.

It found nine operations kept in the interpreter by an INFDS alone.

**Verified by** the native unit NINFDS, under both engines, as captured on PUB400 (tools/golden/native139.txt n139_infds). It checks the file name, the open and end-of-file flags, the status, the operation, and the record format and relative record number after two READs, then the READ at the end of file.

The read count and the end-of-file record format and relative record number differ between IBM i and the interpreter (fidelity.md), so the unit leaves them out.

## S2.140: DSPLY with a response field

DSPLY with a response field is lowered (`Lowered::DsplyReply`, `Runtime::dsply_reply`). The message is made as for a DSPLY with none. Then Stmt::Dsply's match decides:
- **an interactive job:** the message and a reply line on its screen (dsply_screen), with the reply assigned to the field;
- **a batch call:** the message alone, and the field stays as it was.

A date or time field as the message stays with the interpreter.

**Verified by parity only**, against rla_test.rs rpg_data_area_in_lock_out's DTALOCK under native-diff. Its `dsply 'Next order?' '' r` runs with a simulated operator, who answers on the DSPLYR screen while another job waits on the data area's lock. That is the interactive case, and the test expects `1005` after it.

There is no IBM i value for the batch case. On IBM i a DSPLY with a response in batch sends an inquiry to QSYSOPR and waits, and as4tainr gives a batch call no operator, by design. Native code runs the interpreter's own functions for both cases.

## S2.141: %ADDR of a whole array, RETURN of a pointer

**%ADDR of a whole array**, or of a whole data structure array, is lowered (`whole_array_place`). Its place is the one interp.rs loc_of gives a name with no index: the variable's own buffer and offset, that is, its first element. The operand carries the element's type, but %ADDR takes only its place. Based and multiple-occurrence ones stay with the interpreter, and a procedure's own only in a procedure.

**A RETURN in a procedure that returns a pointer** is lowered (`Lowered::ReturnPtr`). The value is eval's pointer, as ptr_val makes it:
- an address;
- pointer arithmetic;
- a pointer field;
- a call;
- *NULL.

**Verified by** native units already captured on PUB400, whose statements the WHY pass gave as not lowered and which are native now:
- NBASARR (S2.96): `p = %addr(real)` and `pc = %addr(creal)`;
- NRETPTR (S2.99): `return %addr(buf) + n`.

## S2.142: calls of procedures with their own *PSSR

A call of a procedure with its own *PSSR is lowered. For such a procedure, the runtime's call_body runs the calculations as call_proc's guarded does:
- from `start`, an error goes to the interpreter's own errors.rs `recover`, which notes it and runs the subroutine;
- the subroutine's RETURN ends the call with its value;
- its GOTO goes on at the TAG;
- anything else lets the error go on.

A procedure with no *PSSR keeps the path it had. There recover would do the same as call_body already does, note the error and pass it on.

**Verified by** the native unit NPSSRPR, under both engines, as captured on PUB400 (tools/golden/native142.txt n142_pssr: `5 -1 102 3`):
- a division by zero caught by the procedure's *PSSR, which returns -1;
- one whose *PSSR returns %STATUS, 00102;
- calls with no error.

## S2.143: %DEC of a date, time or timestamp

%DEC of a date, time or timestamp is lowered (`NumExpr::TextDec`, `rpgrt::native::text_dec`). The argument is a field, as in S2.53, or an expression, as in S2.134, with a special value as its format or none.

It follows bifs.rs bif_override's DEC arm, which comes before the numeric %DEC:
- %CHAR's text in that format with no separators, and a timestamp always *ISO0;
- then Dec::parse.

**Verified by** the native unit NDECTMP, under both engines, as captured on PUB400 (tools/golden/native143.txt n143_dectmp: `20260131 1312026 91500 20260131235959123456 20260201`):
- a date into a packed field, and a date with *USA;
- a time, a timestamp, and a date plus days.

## S2.144: indicator subfields by their path

An indicator subfield of a data structure array element, by its path (`l.ev(1).expected.isNull`), is an indicator field of lowered code (`ind_field_ok`, `ind_field`). It is placed as ds_array_field places any path subfield (`ds_array_field_of` with indicators allowed), with its literal or field indexes. So it can be used:
- in a concatenation, as its '1' or '0';
- as a condition;
- as an EVAL target with *ON, *OFF or a condition;
- compared.

ds_array_field still gives no indicator to the other contexts.

**Verified by** the native unit NINDPATH, under both engines, as captured on PUB400 (tools/golden/native144.txt n144_indpath: `110yn`). It covers a concatenation, an assignment, and IF and NOT, with literal and field indexes. rla_test.rs INZLDS's `l.ev(1).expected.isNull + ...` is native now too.

## S2.145: arithmetic in conditions

A side of a comparison may now hold arithmetic, + - * /, in IF, DOW, DOU, WHEN, IN %RANGE and IN %LIST, and in %CHAR of a condition.

S2.13 had kept arithmetic out because it took precision to come from an EVAL's target. It does not: interp.rs eval gives each operation the precision of its own node (to_rule_scale and divide, num_precision of the operation). The runtime's `precision` finds that node from the condition at the instruction, as it does from an EVAL's value. FOR limits follow in S2.146.

**Verified by** the native unit NCMPAR, under both engines, as captured on PUB400 (tools/golden/native145.txt n145_cmparith: `abdef5`). It covers a product, a quotient against fewer places, a sum and difference, IN %LIST of a product, a product of a difference, and a DOW on a difference.

The capture also shows a quotient compared with a 28-place literal, where IBM i and the interpreter differ (fidelity.md), so the unit leaves that test out. NINLIST's `n * 2 in %list(3 : 14)` (S2.131) is native now.

## S2.146: arithmetic in FOR limits

A FOR limit is now a numeric expression (`LoopTest::limit`), as in `for i = 0 to len - 1`. The interpreter's LoopTest arm evaluates it after the index, on every turn, and compares the two as decimals. Its operations take their own nodes' precision, as in S2.145, and the runtime's `precision` now also starts from a LoopTest's limit. BY values are still operands.

**Verified by** the native unit NFORLIM, under both engines, as captured on PUB400 (tools/golden/native146.txt n146_forlim: `abcd876543012`):
- %LEN minus one;
- a product down to a difference;
- a difference of a product.

A limit that is not whole (`10 / 4`) does not compile on IBM i (RNF7569), while the interpreter takes it (fidelity.md).

## S2.147: S2.138's TEST(DE), lowered as it was meant to be

S2.138 lowered TEST only when the operation's other extenders held nothing but D, T or Z. The parser keeps E there too, as well as setting ext.error, so a TEST(DE) never matched. It went on running in the interpreter, and so did every TEST in the suite. NTESTDT passed under both engines because the interpreter ran its TESTs. S2.138's commit says the unit verifies the native path, and that was wrong.

The condition now allows E, and the five TESTs of NTESTDT are native: the generated code's map has `"what":"TEST","via":"native"`. The expected value is still the PUB400 capture's (tools/golden/native138.txt n138_test: `10100`).

The WHY pass did not show it, because it noted no `Stmt::Op`. AS4TAINR_NATIVE_WHY now notes every operation that is not lowered: its code, extenders, resulting indicators and factors. A block is verified only when the map shows its statements as native.

## S2.148: the SQRT operation

The fixed-form SQRT operation is lowered (`Lowered::SqrtOp`, `rpgrt::native::sqrt_op`), as ops.rs string_op's SQRT arm runs it:
- factor 2's value;
- for a negative value, RNQ0101;
- else dec_sqrt to one decimal place more than the result field has;
- assigned with Truncate, or rounded with (H).

The result is a packed, zoned, binary or integer field. The tripwire already watches string_op and dec_sqrt.

**Verified by** the native unit NSQRTOP, under both engines, as captured on PUB400 (tools/golden/native148.txt n148_sqrtop: `3.16 1.4142 4 4`). It covers (H) into packed(5:2), no (H) into packed(7:4), and (H) and not into an integer. The four SQRTs are native in the generated map.

## S2.149: tables and their current elements

Tables (TABxxx arrays) are lowered in two places:
- **A table named with no index** is its current element, as interp.rs table_index takes it: LOOKUP's, or 1 at first. `ir::Index::table` makes the index `Runtime::table_cur` instead of an operand. This holds for a global table that is neither varying, based, nor of multiple occurrences.
- **LOOKUP of a table**, with an alternate table or array in the result or none, follows ops.rs lookup's table arm. When an element is found, the table's current element becomes it, and so does the alternate's (the machine's table_cur). There is no index to set.

**Verified by** the native unit NTABLES, under both engines, as captured on PUB400 (tools/golden/native149.txt n149_tables: `two1two0Ctwo`). It uses a compile-time table with its ALT table:
- a LOOKUP found and one not found;
- a LOOKUP of the table alone;
- each table's current element by its name.

## S2.150: *OMIT arguments

An *OMIT argument for an OPTIONS(*OMIT) parameter is lowered as args_for's `Arg::Omit` (`CallArg::Omit`). It applies to a call of a bound procedure, a CEE API, a C function or a procedure pointer, and call_proto passes the argument on as it is. A program (EXTPGM) and a procedure of this module keep the interpreter for now, so call_frame never sees one. Any other argument for such a parameter goes as it would with no *OMIT.

**Verified by** the native unit NOMITCE, under both engines, as captured on PUB400 (tools/golden/native150.txt n150_omit: `161912 7 2026-02-01`). It calls CEEDAYS and CEEDYWK with an omitted feedback code, and CEEDATE with one passed.

## S2.151: *ON and *OFF for an indicator parameter passed by value

*ON or *OFF as the argument for an indicator parameter passed by value is lowered as a constant condition, an empty `Cond::Chain` (an AND of none is true, an OR of none is false). args_for's eval of the argument gives Value::Fig, which data::store writes into the indicator as '1' or '0', the same bytes a true or false condition gives.

**Verified by** the native unit NINDFGA, under both engines, as captured on PUB400 (tools/golden/native151.txt n151_figind: `yesno if 21`). It makes the calls in an assignment, in a condition and in arithmetic.

Not lowered: a by-reference parameter given a field of another type without OPTIONS(*VARSIZE). args_for passes it as the argument's own storage, but IBM i rejects such a call when it compiles it, so no value can be captured (SYSAPI's QDCXLATE call, a CHAR(2) for CHAR(10)).

DSPLY of a float field was not lowered here; S2.183 lowers it. A second capture was tried on 2026-11-20: the program was submitted as a batch job (SBMJOB LOG(4 00 *SECLVL) LOGOUTPUT(*JOBEND)), and its QPJOBLOG was to be copied to a stream file. It left neither a job log nor its stream file within 45 seconds (CPF3303). IBM i's text is therefore still unknown, and the interpreter keeps the statement (S2.110).

## S2.152: COMMIT and ROLBK

COMMIT and ROLBK with no boundary, extender or indicator are lowered (`Lowered::Commit`, `Runtime::commit_op`), as the op arm does them:
- the commitment definition of the job and activation group is looked up, and RNQ0802 is raised when there is none;
- then its commit or its rollback runs, and RNQ0804 or RNQ0805 is raised if that fails.

**Verified by** the native unit NCOMMIT, under both engines, as captured on PUB400 (tools/golden/native152.txt n152_commit: `commit 802 rolbk 802`). It runs both operations with no commitment control active, each in a MONITOR group. The path with commitment control active, where a CL caller starts it, is the native-diff suite's CMTRPG under upstream's commitment_control test.

## S2.153: bit built-in functions of numbers

%BITAND, %BITOR, %BITXOR and %BITNOT of numbers are lowered (`NumExpr::Bit`, `rpgrt::native::bit_num`), as bif_more's numeric arm:
- the first argument is a numeric expression, taken as an integer and negated for %BITNOT;
- each further argument is combined with it as int_arg gives it. That is a number, or a hexadecimal literal or constant of 1 to 8 bytes as its unsigned value (data::hex_unsigned, worked out from its bytes when lowering).

A comparison of a number with such a hexadecimal literal or constant keeps its character side, and data::compare reads it the same way. That is iRPGUnit's `%bitand(flags : SRCBIT) = SRCBIT`, rla_test.rs FDTEST.

**Verified by** the native unit NBITNUM, under both engines, as captured on PUB400 (tools/golden/native153.txt n153_bitnum: `src low 8 15 9 28 -13`). IBM i accepts two arguments for %BITXOR and no more (RNF0317 for three, first run of the capture); the interpreter takes more.

## S2.154: MOVEA with a literal index

A MOVEA side that is an array element named by a literal index (`MOVEA 'XY' CH(2)`, `MOVEA CH(4) A3`) is lowered, with `MoveaSide::Array`'s `at`. movea_parts takes the elements from that index on, as it does for an index field. Only a whole number from 1 to the dimension is lowered, because IBM i refuses a literal index out of range when it compiles, so there is no range error to reproduce.

**Verified by** the native unit NMOVEAL, under both engines, as captured on PUB400 (tools/golden/native154.txt n154_moveal: `AXYDEF DEF... ..EF`, then two blanks, which the unit's DSPLY comparison trims). It moves into an element, out of an element, and with P from an element into an element.

## S2.155: %UNS and %UNSH

%UNS and %UNSH are lowered (`NumExpr::Uns`, `rpgrt::native::uns_bif`), as bif_more's "UNS" arm:
- a character value is read by Dec::parse of its trimmed text, and a number is taken as it is;
- the value is then rescaled to no places, rounded for %UNSH;
- RNQ0103 is raised when the result is negative.

**Verified by** the native unit NUNSBIF, under both engines, as captured on PUB400 (tools/golden/native155.txt n155_uns: `42 8 9 10 13 103 103`). It covers character and numeric arguments, a sum of a %UNS, a negative value, and a value too large for UNS(3), each in a MONITOR group. rla_test.rs IMQINT's `u3 = %uns('300')` is native now.

## S2.156: WRITE of a program-described printer file from a data structure

WRITE of a program-described printer file from a global data structure is lowered (`Lowered::PrinterWriteDs`, `Runtime::printer_write_ds`). It follows errors.rs file_op around printer.rs printer_write's program-described arm: the structure's bytes are printed as one line, an output record with that one literal field and no spacing (print_line_n). IBM i accepts only a data structure there (RNF5191 for a field, first run of the capture); the interpreter also takes a field.

**Verified by** the native unit NPRTDS, under both engines, as captured on PUB400 (tools/golden/native156.txt n156_prtds: PASE system's output of the spooled file, `first line           00042` and `second line          00007`, then PUB400's banner). rla_test.rs WRTPRT's two WRITEs are native now.

## S2.157: a date plus a time

A date plus a time is lowered as a timestamp value (`CharExpr::DatePlusTime`, `rpgrt::native::date_plus_time`), as eval's arm builds it: the text `date-time.000000`, with the time's colons as periods, checked by parse_ts. Both sides are evaluated in their written order. Only the date-first form is lowered: IBM i refuses a time plus a date (RNF7421 and RNF7416, first run of the capture), which the interpreter takes (fidelity.md).

**Verified by** the native unit NDATTIM, under both engines, as captured on PUB400 (tools/golden/native157.txt n157_datetime: `2024-09-16-11.22.33.000000 2026-01-31-23.59.58.000000 2026-01-31-23.59.58`). It adds %DATE to %TIME and a date field to a time field, into timestamps of 6 and 0 fractional digits. rla_test.rs TSPREC's `%date(...) + %time(...)` is native now.

## S2.158: %STR as an assignment's target

`%STR(p : max) = value` is lowered (`Lowered::EvalStr`, `Runtime::store_str`). As eval and assign_value's "STR" arm do it:
- the value is evaluated first, then the pointer;
- then ptrs.rs store_str runs: the value is cut to max - 1 characters, and a null goes after them at the place the pointer addresses, whose room for them is checked.

Only a constant maximum of at least 1 is lowered: a literal, a named constant or %SIZE. The arm converts the maximum `as usize`, which would wrap a negative one.

**Verified by** the native unit NSTRSET, under both engines, as captured on PUB400 (tools/golden/native158.txt n158_strset: `Hello 5 xxxxxx Heab zzz`). It covers a literal maximum and a %SIZE one, a pointer moved on, and the bytes left after the null. rla_test.rs PTRS's `%str(...) = ...` is native now.

## S2.159: %ADDR of a based field

%ADDR of a field based on a pointer, a subfield of a based data structure included, is lowered. It takes loc_of's place, which Machine::sym gives through based_place: the pointer's storage checked for the field's size. Generated code gets it from `based_at`, as it does for a based field's value (S2.48). The address is ptr_to of that place. An element of a based array stays out.

**Verified by** the native unit NADDRBS, under both engines, as captured on PUB400 (tools/golden/native159.txt n159_addrbased: `same ..........hello     ..........world`, then five blanks, which the unit's DSPLY comparison trims):
- the address is written through by another based field;
- it is compared with the basing pointer plus the subfield's offset;
- it is taken again after the basing pointer moves.

rla_test.rs SVAL's `q = %addr(data)` is native now.

## S2.160: comparisons of pointer expressions

A comparison with a pointer expression on a side is lowered (`Cond::PtrCmp`): pointer arithmetic, %ADDR, %ALLOC, %REALLOC or %PADDR, as `ptr_val` gives them. Eval's values of both sides, the left first, are compared by data::compare: in address order, with *NULL before any address. A comparison of two pointer fields was already lowered as operands.

**Verified by** the native unit NPTRCMP, under both engines, as captured on PUB400 (tools/golden/native160.txt n160_ptrcmp: `lt gt eq nn`). It covers <, >, = (true and false) and <> *NULL with arithmetic and %ADDR on either side. NADDRBS's `if q = p + 10` (S2.159, the one statement of it the interpreter still ran) is native now.

## S2.161: OPEN, CLOSE and FEOD with E

OPEN, CLOSE and FEOD of a printer file, and OPEN and CLOSE of a database file, are lowered with the E extender too. errors.rs file_op clears %ERROR and %STATUS at its start when E is given, and the runtime's printer_op_n and disk_open_close_n now do the same. An error of these operations still goes on as the interpreter's does, because stmt_e does not take a file statement.

**Verified by** the native unit NOPENE, under both engines, against the PUB400 capture (tools/golden/native161.txt n161_opene: `1 1215 0 0 0 0`). Two differences between IBM i and the interpreter, which the native engine reproduces, are in fidelity.md:
- the second OPEN(E) of a printer file gives `1 1215` on IBM i but `0 0` in the interpreter, so the unit checks only the four fields where both agree;
- OPEN(E) of a database file that is already open gives `1 1215` on IBM i (n161_dblopen), where the interpreter raises CPF4161.

## S2.162: %ABS of a float expression

%ABS of a float expression is lowered as a float value (`FloatExpr::Abs`, `rpgrt::native::float_abs`). It is bif's ABS arm on eval's value of the argument, the float arm's: a float stays a float, and a decimal's absolute value is a decimal. is_float does not take %ABS as float, so float_expr tries it before its float check.

**Verified by** the native unit NFLTABS, under both engines, against the PUB400 capture (tools/golden/native162.txt n162_fltabs: `+2.500000000000000E+000 +8.500000000000000E+000 +6.2500000E-01 huge`). It covers a field, an expression, a float(4) target, and a value too large for a 63-digit decimal (rla_test.rs FLOATS's `d = %abs(f - f / 2)`). The third field is %CHAR of a float(4), a known interpreter difference (fidelity.md, "%CHAR of float(4)"), so the unit checks the other three.

## S2.163: a table's current element as a string operation's factor

A character factor of a fixed-form string operation (SCAN, CHECK, CHECKR, XLATE, CAT) may now be a table named with no index. That is its current element, as eval's table_index gives it, with S2.149's index. RPG3TEST's and RPG3FREE's `MSG CAT TABN:1 OUT` after a LOOKUP is native now.

**Verified by** the native unit NTABCAT, under both engines, as captured on PUB400 (tools/golden/native163.txt n163_tabcat: `IS: oneBt|AoneBt`). It uses a table as factor 2 and as factor 1, with (P).

## S2.164: file operations beside a LIKEFILE parameter

A LIKEFILE parameter no longer keeps every file operation of the program in the interpreter. errors.rs file_idx, and the runtime's file_idx_n, look first at the frame's files, and a frame holds only its own procedure's file parameters. So only a name that is a file parameter of the routine's own procedure stays with the interpreter. Every other name is resolved as file_idx resolves it after that step.

**Verified by** the native unit NLIKEF, under both engines, as captured on PUB400 (tools/golden/native164.txt n164_likef: `3 3cc`). In the main procedure it runs OPEN, SETLL *HIVAL, READP into a data structure and CLOSE on a LIKEFILE file with EXTFILE, while a procedure reads the file through a LIKEFILE parameter. That procedure's own operations, and the call that passes the file, stay with the interpreter. IBM i requires BLOCK on the template file (RNF2590) and refuses SETLL *LOVAL on a file with no key (RNF7055); both were found on the capture's first run.

## S2.165: XML-INTO

XML-INTO into a receiver named with no index is lowered (`Lowered::XmlInto`, `Runtime::xml_into`), with %XML(document [: options]) and without %HANDLER or E. The document and the options are character expressions, which generated code evaluates as the op arm's eval does. The rest is shared with the interpreter: parsing the document, the options, and filling the receiver by its names. It runs in xmlinto.rs `xml_into_doc`, which xml_into now calls after its evaluation. The user authorized that split on 2026-11-20 as a touch point, with no behavior change (CLAUDE.native.md section 3).

The fill assigns through the machine's names, as record_to_ds does for a result data structure (S2.119). It does not evaluate user expressions. Removing the interpreter would still need that fill, and the assignment it uses, in the runtime.

**Verified by** the native unit NXMLIN, under both engines, as captured on PUB400 (tools/golden/native165.txt n165_xmlinto: `7 Ann A1=3 B2=12 [   ]`). It fills a data structure, and a data structure array with path and case=any, whose third element stays blank.

## S2.166: SET RESULT SETS ARRAY

`SET RESULT SETS [WITH RETURN ...] ARRAY :host FOR n ROWS` is lowered (`Lowered::SqlSetResult`, `Runtime::sql_set_result`), as exec_sql.rs sql_set_result_sets does it:
- The count is taken first: a number, or a numeric field, at least 0 and at most the dimension.
- The first n elements become rows of their subfields' text: characters as they are, numbers as their text.
- Those rows are the result sets RESULT_SETS keeps for the procedure's caller, and SQLCODE is 0.

The user authorized `pub(super)` on RESULT_SETS on 2026-11-20 (CLAUDE.native.md section 3). The host structure array may now be a procedure's own local, placed in its frame. INSERT ... ROWS (S2.74) shares the placement and still takes globals only. NONE, a cursor's result set and a structure of multiple occurrences stay with the interpreter.

**Verified by** the native unit NRSLTST, under both engines, as captured on PUB400 (tools/golden/native166.txt n166_resultset). The module is called through an SQL procedure (CREATE PROCEDURE ... EXTERNAL NAME), as rla_test.rs LISTMOD does. Qshell db2's listing shows the columns NAME, QTY and NOTE, and two rows: `ab`, 7.50, `one` and `cd`, -8.00, an empty note. rla_test.rs LISTMOD's LISTARR is native now; its LISTOCC, an OCCURS structure, is not.

## S2.167: the review of S2.140 to S2.166

A read-only codex review of S2.140 to S2.166 (2026-11-20) found one difference, now fixed. In S2.153, generated code evaluated all of a numeric bit function's arguments before making them integers. bif_more evaluates each argument and makes it an integer before it evaluates the next. So `%BITAND(big : 1 / zero)`, with `big` past an i128, raised the division by zero where the interpreter raises the conversion error. Each argument is now taken in turn: `bit_start`, then `bit_step` for each further one, then `bit_end`. The review said it was incomplete for the other stages, and was run again in four groups of blocks. Each finding was checked against the code:
- **S2.143, fixed in S2.168.** %DEC of a date, time or timestamp field whose value bifs.rs `temporal` does not recognize, such as blanks, is the generic %DEC in bif_override: char_number of its characters, RNQ0105 for blanks. The native runtime parsed the text as it was. `rpgrt::native::temporal_dec` now does as the interpreter does. IBM i gives such a field status 00112 (fidelity.md).
- **S2.150, fixed in S2.168.** *OMIT for a parameter that is both LIKEFILE and OPTIONS(*OMIT): args_for checks LIKEFILE first and raises an error. Lowering passed the *OMIT on, and it now refuses the call.
- **S2.154 and S2.166, recorded.** These are two more numbers that the interpreter converts `as usize`: a MOVEA index below 1 in RNQ0121's message, and a SET RESULT SETS row count of 2^64 or more (fidelity.md, "Numbers past 2^64").
- **S2.159 and S2.160, not differences.** The review said the interpreter defers the fault of a *NULL basing pointer in %ADDR to the end of the statement. It does not: loc_of calls check_buf, which raises the fault of the BAD_BUF place at once, as based_at does.

**Verified by** the native unit NDECBAD, under both engines, against the PUB400 capture (tools/golden/native168.txt n168_decbad: `112 20260131`), on the field where IBM i and the interpreter agree.

## S2.169: data structure arrays as CONST parameters

A whole data structure array, with DIM above 1, passed to a CONST parameter of this module's procedure is lowered as args_for's Arg::Ref: all its elements, at the array's place. For a LIKEDS DIM(n) parameter, that is the callee's view of the same storage. call_proc takes the descriptor's length from the Loc's size, which is the whole array's; call_frame takes it from the reference's type. So the native reference is typed as characters of the array's size. The array may be global or the calling procedure's own.

In the callee, the elements' subfields of such a parameter (`m(i).field`) are now native: ds_array_field_of no longer refuses a data structure array that is a parameter. rt.local places it where Machine::sym does, which for one passed by reference is the caller's storage. path_sym checks only the index's range for a data structure array, so there is no MCH0601 `reach` check to reproduce. elem_off makes that check for a scalar array, which stays refused.

**Verified by** the native unit NDSAPRM, under both engines, as captured on PUB400 (tools/golden/native169.txt n169_dsaparm: `1 2two 3 |9x 0 0 4four`, then a blank). It passes a global array and a procedure's local one. The capture's first version had arrays with no INZ, whose varying subfields began with blanks: IBM i ended in RNX9998, and the interpreter read the blanks as lengths. The arrays now have INZ. rla_test.rs IMQDSARR's call and DSPLY are native now.

## S2.170: named constants as string operations' factors

A character factor of a fixed-form string operation (SCAN, CHECK, CHECKR, XLATE, CAT) may be a named constant of character data. It takes eval's value of it, as of the literal it stands for, through any_operand, which puts the value in the layout lines. RPG3TEST's and RPG3FREE's `MSG CAT TABN:1 OUT`, with MSG a constant, is native now.

**Verified by** the native unit NSTRCON, under both engines, as captured on PUB400 (tools/golden/native170.txt n170_strconst: `MONTH IS MAY|X-|2|CABBAge`). It covers CAT's factor 1 and factor 2, CAT(P), SCAN's argument, and XLATE's from and to strings.

## S2.171: E and the error indicator on file reads

READ, READE, READP, READPE, CHAIN, SETLL and SETGT of a database file are lowered with the E extender and with an error indicator in positions 73-74, as errors.rs file_op and file_op_inner take them:
- With E, %ERROR and %STATUS start over, before the file's error flag is cleared.
- An error of the operation itself (rla's read, positioning or lock) ends the operation: file_failed, the indicator on, then file_op's end, where the status keeps file_failed's. The read then gives no record, and file_done_n sees it as ended.
- An error of moving the record into the program (a data mapping error) is taken the same way, before the status and the indicators.
- The check that the file is open comes before the part E takes. So its CPF4135 is still raised, as in the interpreter.

**Verified by** the native unit NREADE, under both engines, against the PUB400 capture (tools/golden/native171.txt n171_reade: `3 0 1 1 1211`), on the fields where IBM i and the interpreter agree: a READ(E) loop with %ERROR and %EOF. For READ(E) of a file that is not open, IBM i sets %ERROR and 01211, where the interpreter raises CPF4135 (fidelity.md). IBM i requires an OPEN in the program for a USROPN file (RNF7062), and EXTDESC for a file named by EXTFILE (SQL1001). Both were found on the capture's first runs.

%CHAR of %EOF, %FOUND or %EQUAL, of a file or not, is lowered too, as bif's CHAR of its Value::Ind (CondChar of the file status condition). It is the unit's last statement.

## S2.172: E and the error indicator on WRITE, UPDATE and DELETE

S2.171's extender check also let E through on WRITE, UPDATE and DELETE, because file_read lowers changes as well, but their runtime did not take E yet. This is now done as file_op_inner's change arm does it:
- With E, %ERROR and %STATUS start over at the change's start.
- An error of record_change (the change itself, a duplicate key for example) is taken: file_failed, the indicator on, then file_op's end, where the status keeps file_failed's.
- An error of making the record (record_of, ds_to_record), the authority check and the open check are still raised.

Snapshot s2171 had the hole. S2.171 and S2.172 are therefore committed after a validation of a snapshot with both.

**Verified by** the native unit NWRITEE, under both engines, as captured on PUB400 (tools/golden/native172.txt n172_writee: `0 0 1 1021`). It covers a WRITE(E) of a new key, then of a key the primary key already has.

The capture's first version also had CHAIN and UPDATE(E) by the primary key. IBM i takes the table's primary key as its keyed access path (`keyed` compiles, and CHAIN finds the record). as4tainr does not: "file GN172F has 0 key fields" (fidelity.md).

## S2.173: CALLP(E)

CALLP(E), or CALLP with an error indicator in positions 73-74, of a call native code lowers is lowered (`Lowered::CallE`, `Runtime::call_e_start` and `call_e_end`), as errors.rs stmt_e runs it:
- With E, %ERROR and %STATUS start over, and the error indicator is turned off.
- Then the call runs, and stmt's pointer fault check follows: a fault while it ran is its error.
- An error that is not a signal and does not pass this program by (passes_by) is noted (note_error), and the error indicator is turned on. The program goes on.

**Verified by** the native unit NCALLPE, under both engines, as captured on PUB400 (tools/golden/native173.txt n173_callpe: `1 202 0 0 0 5`). A procedure that divides by zero leaves status 00202, the called procedure failed. A call that does not fail gives 0 0.

## S2.174: UPDATE with %FIELDS

UPDATE with %FIELDS is lowered as file_read's change, and the runtime keeps only the fields named, as file_op_inner's io_fields does. The record is made, then the fields are kept whose program or database name is in the list. That also drops the null values' entry, as `retain` drops it. Then record_change runs.

**Verified by** the native unit NUPDFLD, under both engines, as captured on PUB400 (tools/golden/native174.txt n174_updfld: `1 aa yy`). A record is read, two fields are changed, and only one is named in %FIELDS. rla_test.rs DSIO's `update custrec %fields(cusbal)` is native now.

## S2.175: SND-MSG

SND-MSG is lowered (`Lowered::SndMsg`, `Runtime::snd_msg`), as the op arm does it:
- The message type is factor 1, *INFO when there is none.
- The message is a character expression, or %MSG(id : msgf [: data]) with each argument a character expression, evaluated in order.
- `id: text` is built from the trimmed id and the data with blanks off the end, or the text alone.
- *ESCAPE raises it as an error, CPF9898 with no id; the other types go to the job log.

**Verified by** the native unit NSNDMSG, under both engines, as captured on PUB400 (tools/golden/native175.txt n175_sndmsg: `start info`). *INFO goes to the job log, and the program goes on.

The capture's first version also sent *ESCAPE inside MONITOR groups. IBM i sends it to the caller by default, so the program ended with CPF9898 unmonitored. The interpreter catches it in the procedure's own group (fidelity.md), and native code does the same.

## S2.176: XML-INTO(E)

XML-INTO with the E extender is lowered. It runs inside errors.rs stmt_e's start and catch, which S2.173's call_e_start and call_e_end now give any operation. With E, %ERROR and %STATUS start over. An error of the operation, the document's evaluation included, is noted, and the program goes on. As stmt_e does, the error indicator counts only for IN, OUT and CALLP.

**Verified by** the native unit NXMLINE, under both engines, as captured on PUB400 (tools/golden/native176.txt n176_xmlintoe: `1 351 1 353 0 9Cy`):
- a document that is not well formed gives 00351;
- a value that does not fit its subfield's type gives 00353;
- a good document gives no error.

## S2.177: EVAL-CORR

EVAL-CORR of two data structures named with no index is lowered (`Lowered::EvalCorr`): ops.rs eval_corr's walk is made when lowering. The target's subfields are taken in order, each with the source's subfield of its short name and its type class (eval_corr's `class`):
- data structures in both: their own subfields, recursively;
- arrays in both: element by element, up to the smaller dimension;
- scalars: eval's value assigned with Strict, or half adjust with (H).

The pairs run in that order. A structure on one side only, a based or multiple-occurrence one, and a field `variable` does not place stay with the interpreter.

**Verified by** the native unit NEVCORR, under both engines, as captured on PUB400 (tools/golden/native177.txt n177_evalcorr: `[Alice     ] 30 7 Paris     75001 T1T2 O`). It covers character into varying, packed into zoned, a LIKEDS subfield in both, arrays of 3 and 2 elements, character against packed (kept), and a target-only subfield. rla_test.rs EVALCORR is native now.

## S2.178: SET RESULT SETS ARRAY of a structure of multiple occurrences

SET RESULT SETS ARRAY of a global data structure of multiple occurrences is lowered with S2.166's runtime. sql_set_result_sets sets each occurrence from the first, reads each subfield (`HOST.SF`, else `SF`), and gives the current occurrence back. Machine::sym places a subfield of occurrence i at its place plus (i - 1) times the occurrence's size. So the rows are read as an array's: the occurrence's size is the stride, and the number of occurrences is the dimension. The current occurrence is not changed.

**Verified by** the native unit NRSLTOC, under both engines, as captured on PUB400 (tools/golden/native178.txt n178_resultocc). Qshell db2 lists the columns LINE and QTY and the rows `first` 5 and `second` -7, three occurrences set and a count of 2. rla_test.rs LISTMOD's LISTOCC is native now.

## S2.179: an array assigned %SPLIT

EVAL of a global character or varying array, not based nor of multiple occurrences, with a %SPLIT value is lowered (`Lowered::EvalSplit`, `rpgrt::native::split_pieces`, `Runtime::assign_list`):
- First, bifs.rs's "SPLIT" arm, which takes the string, the separators unless the argument is a special value, and *ALLSEP.
- Then assign_value's list into an array, as the interpreter does it:
  - each piece goes into its element;
  - the elements after the last piece are initialized;
  - more pieces than elements is RNQ0121;
  - a varying-dimension array's elements in use become the pieces.

**Verified by** the native unit NSPLITA, under both engines, against the PUB400 capture (tools/golden/native179.txt n179_split: `the/fox/old|a   b   c   [    ]|a   [    ]b   |2y`), on the parts where IBM i and the interpreter agree. IBM i leaves a fixed array's elements after the pieces as they were, which the interpreter initializes (fidelity.md). rla_test.rs FREEFORM's `words = %split(...)` is native now.

## S2.180: XML-INTO %HANDLER

XML-INTO %HANDLER(procedure : area) %XML(document [: options]) is lowered, with or without E. Lowering first checks the handler as handler_items will at run time, so that its checks cannot fail when the operation runs:
- it is a procedure of the module;
- it has at least three parameters;
- its second parameter is an array passed by reference;
- the area is a name.

Generated code evaluates the document and the options in order. The runtime then takes handler_items, and `xml_into_handler_doc` does the rest of xml_into_handler unchanged. That function is upstream's xml_into_handler split after its checks and evaluation, under the same authorization as S2.165's split; CLAUDE.native.md section 3 records it.

**Verified by** the native unit NXMLHND, under both engines, against the PUB400 capture (tools/golden/native180.txt n180_handler: `9 2 abc|5 1 ab 1 351`), on the parts where IBM i and the interpreter agree. IBM i ends the operation with error 00351 when the handler returns non-zero; the interpreter ends it without an error (fidelity.md). rla_test.rs XMLHND is native now.

## S2.181: XML-SAX %HANDLER

XML-SAX %HANDLER(procedure : area) %XML(document [: options]) is lowered with S2.165's `Lowered::XmlInto`, with or without E. Lowering checks xml_sax's conditions before the document (a %HANDLER of a procedure name and an area), so they cannot fail at run time. Generated code evaluates the document and the options in order. The runtime then calls `xml_sax_doc`, which is upstream's xml_sax split after its checks and evaluation with no behavior change (authorized by the user on 2026-11-21; CLAUDE.native.md, section 3). The handler's calls go through the program's ordinary procedure calls, so a native handler runs natively.

**Verified by** the native unit NXMLSAX, under both engines, against the PUB400 capture (tools/golden/native181.txt n181_sax: `<order<itempen<itemink & nib|id=7 14 1 351`), on the parts where IBM i and the interpreter agree. When the handler returns non-zero, IBM i ends the parsing with error 00351 and the interpreter ends it without an error (fidelity.md). rla_test.rs SAXT is native now.

## S2.182: x IN an enumeration, %CONCAT

`x IN enumeration` is lowered as `x IN %LIST(...)` (S2.131). An enumeration is a named constant whose value is the list of its constants' values (program.rs), and that list is what bifs.rs eval_in's list_of takes. Each value becomes a literal, of character or numeric class, compared with data::compare.

%CONCAT(separator : a : b ...) is lowered as a chain, following bif's CONCAT arm. The separator is a literal, or *NONE or *BLANK(S) written as names. A figurative *BLANK, which is what the parser gives, is not lowered; on that form the interpreter differs from IBM i (fidelity.md). It is evaluated once there, which is the same as once per gap because it has no effect of its own. The values are character literals or character fields. A numeric operand is not lowered: IBM i rejects it with RNF0203, while the interpreter accepts it (fidelity.md).

**Verified by** the native unit NINCONC, under both engines, as captured on PUB400 (tools/golden/native182.txt n182_inconcat: `G-in X-out 9-in|x, y, z|ab|a b|ab  -q-ab  |`), on the parts where IBM i and the interpreter agree (not the *BLANK one). rla_test.rs FREEFORM's `'G' in color` and `%concat(', ' : ...)` are native now.

## S2.183: DSPLY of a float field

DSPLY of a float field with no response field is lowered (`CharExpr::FloatDsply`, `rpgrt::native::float_dsply`). It follows Stmt::Dsply's float arm:
1. the field's value through num();
2. its text parsed back to f64, or 0 when the text does not parse;
3. data::float_text of the field's size.

That differs from %CHAR's path, float_value. A float expression other than a field, and DSPLY with a response field, still stay with the interpreter.

IBM i's text was captured at last. DSPLY with a message queue operand sends the value to a queue made for the capture, GN183Q, and the program reads it back through QSYS2.MESSAGE_QUEUE_INFO. A plain DSPLY in a PASE job ends with status 00333. A batch job did not run, because the user's job queue is not attached to an active subsystem.

**Verified by** the native unit NFLTDSP, under both engines, against the PUB400 capture (tools/golden/native183.txt n183_fltdsply), on its 8-byte values. IBM i shows a 4-byte float with eight digits and a two-digit exponent; the interpreter shows seven and three (fidelity.md). rla_test.rs TYPES's `dsply rate` and `dsply score` are native now.

## S2.184: timestamp host variables in EXEC SQL

A timestamp field is lowered as a host variable wherever exec_sql.rs takes the host through host_literal or assign_hosts (`sql_host_field`). That covers statement inputs, INTO, EXECUTE USING, OPEN USING, FETCH and the arguments of CALL. GET DIAGNOSTICS and the other places that take a field keep sql_field.

- **Input:** generated code calls the runtime's `sql_ts_literal`. Like host_literal's timestamp arm, it makes `CAST('<iso_timestamp of the characters>' AS timestamp)`.
- **Output:** assign_hosts_n gives a timestamp target db2_timestamp's text, as assign_hosts does.

Both functions are exec_sql.rs's own, made `pub(super)` with the user's authorization of 2026-11-21 (CLAUDE.native.md, section 3).

**Verified by** the native unit NSQLTS, under both engines, as captured on PUB400 (tools/golden/native184.txt n184_tshost: `2 2026-11-01-20.43.10.123456 0|2026-10-31-20.43.10.123456`): a parameter marker of EXECUTE USING, `:ts + 1 day` in VALUES, a comparison in WHERE, and MAX and MIN read back INTO. rla_test.rs SQLOVR's timestamp statements are native now.

## S2.185: LIKEFILE parameters inside procedures

A procedure's own LIKEFILE parameter no longer keeps its file operations in the interpreter.
- **At run time,** the runtime's file_idx_n looks first at the frame's files, as errors.rs file_idx does, and finds the caller's file there.
- **At lowering,** file_of takes the record format of LIKEFILE's file. That is the program's own file of that name, which must be a DISK file, and the caller's file has the same format. The name comes from the new `ParamInfo.likefile`, authorized by the user on 2026-11-21 (CLAUDE.native.md, section 3). An INFDS of the caller's file is file_end_n's, by the file the operation runs on.

A call that passes a file to a LIKEFILE parameter of a procedure of the module is lowered as `CallArg::File(name)`, following args_for:
- the parameter's p.file is checked first;
- the argument is a name with no index;
- the runtime resolves it in the caller with file_idx_n, giving the same error when it is not a file;
- call_frame binds it in the new frame's files, with no operational descriptor.

A file for a program, a bound procedure or a procedure pointer stays with the interpreter.

**Verified by** the native unit NLIKEF, under both engines, as captured on PUB400 (tools/golden/native164.txt n164_likef: `3 3cc`). Its countAll(t1) call and the procedure's SETLL *START, READ into a data structure and %EOF on the parameter all run natively now (the WHY pass lists none of NLIKEF's statements). rla_test.rs LIKEF is native now.

## S2.186: whole-array operations on based arrays

These operations of a global array based on a pointer, not one of multiple occurrences, are lowered:
- fixed-form XFOOT and LOOKUP;
- SORTA;
- MOVEA with the based array as factor 2.

The array's place is resolved once, first, as each operation's array_ref resolves it: interp.rs sym's based_place of its pointer for the size of one element (`Runtime::based_at`). The elements are then read and written from there, with the operations' existing runtime.

An element past the end of the storage the pointer addresses is handled as interp.rs load_var and store_var handle it (`load_pad_n`, `store_pad_n`):
- a read gets what is there, then blanks;
- a write changes only the part there is.

An array that is not based never reaches past its own storage, so this changes nothing for it.

A based array as MOVEA's result stays with the interpreter. Its writes resolve the place element by element, and one of them could change the pointer.

**Verified by** the native unit NBASARR, under both engines, as captured on PUB400 (tools/golden/native96.txt n96_based): XFOOT, LOOKUP, SORTA, FOR-EACH and MOVEA of arrays based on pointers to other arrays.

Note on the generator version: rpgcg's VERSION stayed at `+stage2.171` from S2.172 to S2.185. Each block's change of it looked for a text that was no longer there, so it never applied. The routines' fingerprints still changed with their compiled code, and every validation generates its code afresh, so no stale code ran. VERSION is `+stage2.186` from this block on.

## S2.187: a procedure's own named constant as an output edit word

An EXCEPT whose output field's edit word is a named constant of the procedure it runs in is lowered. The native runtime's out_text_n now takes the constant as the interpreter's out_text does, through `Machine::sym`: the procedure's own first, as the frame finds it, then the module's. Before, it took the module's constant only, and lowering left a procedure's own constant with the interpreter (the review of 2026-11-18). Lowering still requires the name to be a named constant where the EXCEPT runs.

**Verified by** the native unit NEXCPT2, under both engines, as captured on PUB400 (tools/golden/native92). Its EXCEPT in the procedure `local`, which declares the edit word `ew` itself, runs natively now (the WHY pass lists none of NEXCPT2's statements).

## S2.188: calls of an OVERLOAD prototype

A call of an OVERLOAD prototype is lowered as a call of the candidate bifs.rs overload_target chooses. The choice is made when lowering, from each argument's kinds as arg_kind gives them:
- *ON and *OFF are an indicator or character;
- a numeric literal is a number or a float;
- a field with no index goes by its type: an indicator is also character, a float also a number, a number also a float.

The first candidate whose parameters each take an argument's first kind wins; failing that, the first that takes one of its kinds; both count the arguments, with those not *NOPASS required. Other arguments are not lowered:
- character literals, because arg_kind looks at their text as a possible date;
- expressions, because arg_kind evaluates them.

Nor is a call with no candidate, since that is the interpreter's error.

**Verified by** the native unit NOVERLD, under both engines, as captured on PUB400 (tools/golden/native188.txt n188_overload: `N12.50|D04/03/26|Cabc|N-7.00`): a numeric literal, a date field, a character field and an integer field. The calls whose candidate is fmtNum or fmtChr run natively (the WHY pass lists only line 20). `fmt(d)`'s candidate fmtDate takes a CONST date parameter, which call_of does not lower yet, so that statement stays with the interpreter, as does rla_test.rs FREEFORM's `fmt(d)`; FREEFORM's `fmt(12.5)` is native now.

## S2.189: date, time and timestamp parameters

A call's date, time or timestamp parameter is lowered as args_for passes one when the argument is a field of the parameter's own type (format and separator included). The field goes by reference, for a parameter passed by reference, or CONST (same_type_variable): the field's bytes, which call_frame aliases. A value for such a parameter is not lowered. That covers a CONST parameter given a field of another format, an expression or a literal: args_for evaluates it and stores it in the parameter's type, and value_arg gives nothing for these types.

**Verified by** the native unit NDTPARM, under both engines, as captured on PUB400 (tools/golden/native189.txt n189_dtparm: `2026-02-28|01:45 PM|2026-02-03-04.06.36.000007|2026-02-28 2026-12-25`):
- a date by reference that the procedure changes;
- a time CONST;
- a timestamp by reference.

These run natively. The statement whose call passes a *DMY date for an *ISO CONST parameter, line 18, stays with the interpreter (the WHY pass lists only it). NOVERLD's `fmt(d)` (S2.188) and rla_test.rs FREEFORM's `fmt(d)` are native now.

## S2.190: reading a based float field

A global float field based on a pointer, not of multiple occurrences, is lowered where a float field is read (`float_read`):
- in a float expression (`FloatExpr::Field`);
- in %CHAR;
- in DSPLY.

`value` reads it where its pointer points: interp.rs sym's based_place for the field's size, which `Runtime::based_at` checks the same way. A based float as a target still stays with the interpreter, because the generated stores take no based place.

**Verified by** the native unit NFLTCHR, under both engines, as captured on PUB400 (tools/golden/native110). Its `%char(bf)` of a float based on `%addr(f8)` runs natively now (the WHY pass lists none of NFLTCHR's statements).

## S2.191: reads of program-described files and of files with input specifications

CHAIN, READ, READE, READP and READPE of a DISK file that is program described, or that has input specifications, are lowered when there is no result data structure. The record is moved as interp.rs file_op moves it, through move_input: the record image, the input fields by their positions (blanks in a zoned field read as zeros), and the record identifying and field indicators.

- **Lowering** generates no field moves for such a file.
- **file_read_n** keeps the record read (`PendingRead.input`).
- **file_done_n** calls move_input as the record's move, so an error in it is the operation's, as a field's is, before the status and the indicators.

A program-described file read with no key needs no database description when the program is compiled; the runtime's open finds the file. Other operations on such files still stay with the interpreter: writes, updates, deletes, and reads into a data structure.

**Verified by** the native unit NPDREAD, under both engines, as captured on PUB400 (tools/golden/native191.txt n191_pdread: `Habc D12/35.00 Hxyz D999/.10 1`): two record types told apart by position 1, a character field and zoned fields with decimals, read until end of file (the WHY pass lists none of NPDREAD's statements). rla_test.rs QTMPRD's READ is native now.

## S2.192: UNLOCK of data areas; IN and OUT with E

UNLOCK of a data area's field, or of *DTAARA, with no extender or indicator, is lowered (`Lowered::DtaaraUnlock`, `Runtime::dtaara_unlock`). The runtime does what interp.rs stmt's File arm does before any file operation, through the Machine's own unlock_dtaara and release_dtaaras: the IN *LOCK lock goes.

IN and OUT of data areas with E, or with an error indicator in positions 73-74, are lowered inside errors.rs stmt_e's start and catch (call_e_start, call_e_end), as CALLP(E) is (S2.173).

**Verified by** the native unit NDTAUNL, under both engines, against the PUB400 capture (tools/golden/native192.txt n192_unlock), on the values read before any OUT. On IBM i, OUT(E) of a data area that is no longer locked is an error, status 00412; the interpreter writes the data area (fidelity.md). The capture's second UNLOCK names the data area, not *DTAARA, because as4tainr's free-form parser takes *DTAARA as the name DTAARA (fidelity.md). rla_test.rs LCKU's UNLOCK is native now.

## S2.193: %KDS as a file operation's key

A key `%KDS(ds [: n])` of a database file operation is lowered as interp.rs key_values takes it: the data structure's subfields, by bifs.rs ds_members' names (the structure's name before each when qualified), the first n when n is given, then as a list of key fields (S2.58). n must be a literal, because key_values evaluates it.

**Verified by** the native unit NKDS, under both engines, as captured on PUB400 (tools/golden/native193.txt n193_kds: `Gamma 1 Delta 1 1 Delta 0 []`). It makes three CHAIN calls into a LIKEREC data structure: with both keys, with the first key only, and with a key not found; the program fields stay as they were. The WHY pass lists none of NKDS's statements. rla_test.rs DSIO's `chain %kds(key) custmast cust` is native now. A record format with a hexadecimal field still keeps a read into a data structure with the interpreter (S2.119); that is why the unit's file is not S2.58's GN58PF, whose format has one.

## S2.194: OPTIONS(*CONVERT)

An argument for a VALUE or CONST character parameter with OPTIONS(*CONVERT) is lowered as args_for's value path gives it, when the argument is a field whose type is known when lowering:
- a numeric, date, time or timestamp field goes as `%CHAR` of it (bif CHAR, as char_expr lowers %CHAR);
- a character field goes as it is.

Anything else stays with the interpreter. That covers an expression or a literal, whose type args_for asks type_of for at run time, and a parameter with *VARSIZE, *TRIM or *STRING too.

**Verified by** the native unit NCONVRT, under both engines, as captured on PUB400 (tools/golden/native194.txt n194_convert: `[-42][1234.50][2026-07-04][ab    ]`): an integer, a packed field, a date and a character field. The WHY pass lists none of NCONVRT's statements. rla_test.rs SQLOVR's `show(n)` is native now.

## S2.195: *OMIT for a procedure of the module

*OMIT for an OPTIONS(*OMIT) parameter of a procedure of the module is lowered. Before, S2.150 lowered *OMIT only for bound procedures, CEE APIs and procedure pointers. call_frame now does what call_proc does with Arg::Omit:
- the parameter is noted in the frame's omitted list;
- it is counted in %PARMS;
- it has no operational descriptor.

A *OMIT for a parameter without OPTIONS(*OMIT) is args_for's error, which lowering never passes on. In the procedure, %ADDR of an OPTIONS(*OMIT) parameter named alone is lowered as bif ADDR takes it (`PtrVal::AddrOmit`): *NULL when the frame notes the parameter omitted (bifs.rs omitted, `Runtime::omitted`), the parameter's address otherwise.

**Verified by** the native unit NOMITLC, under both engines, as captured on PUB400 (tools/golden/native195.txt n195_omit: `1 2N 2x`). It calls a procedure with a *NOPASS and *OMIT parameter three ways: with the argument not passed, with *OMIT (%PARMS 2, its %ADDR *NULL) and with a value. The WHY pass lists none of NOMITLC's statements. rla_test.rs IMQNOPASS is native now.

## S2.196: %LOOKUP of DSARR(*).SUBFIELD

%LOOKUP of `dsarr(*).subfield` is lowered with S2.27's Lookup, as bifs.rs list_of takes the subfield of every element. The array is a global array of data structures, not based nor of multiple occurrences. The subfield's place in the first element (program.rs's DSARR.SUBFIELD) is the base, the structure's size the stride, and its dimension the number of elements. %LOOKUPLT, LE, GE and GT are not lowered for this form: IBM i asks for a sequenced array there, and a data structure array's subfield is not one (RNF0592).

**Verified by** the native unit NLOOKDS, under both engines, as captured on PUB400 (tools/golden/native196.txt n196_lookds: `2 0 4 3`): a character and a numeric subfield, a start element, and a value not found. rla_test.rs LOOKDS is native now.

## S2.197: EXCEPT to a database file

An EXCEPT whose exception records are written to a DISK file, without fetch overflow, is lowered. The runtime's except_n gives each such record to interp.rs disk_output, as print_line's DISK arm does. disk_output reads the program fields where they are, and for a program-described file it uses the record image of the last read (S2.191's move_input). It is one more of the Machine's own functions the runtime calls as a system service (ADR 0007, phase B). Printer records keep S2.78's runtime.

**Verified by** the native unit NEXCDSK, under both engines, as captured on PUB400 (tools/golden/native197.txt n197_exceptdisk: `[Tabc     00042      ][XY                 7]`). It adds two exception records with a constant, a character field, and zoned fields, one with edit code Z, at their end positions, and reads them back with SQL. The WHY pass lists none of NEXCDSK's statements. Its file is named on the F specification and made before the compile: the interpreter takes a file's description from the compile, by that name only (fidelity.md). rla_test.rs TRANSPD's EXCEPT is native now.

## S2.198: reading a program-described file into a data structure

A read of a program-described DISK file into a result data structure is lowered, as file_op gives it to bifs.rs record_to_ds: the record image goes into the structure byte for byte, and is kept as the file's image. Following S2.191, lowering generates no field moves. The runtime's file_done_n calls record_to_ds with the statement's data structure, as the record's move. An externally described file with input specifications still stays with the interpreter when it is read into a data structure.

**Verified by** the native unit NPDDS, under both engines, as captured on PUB400 (tools/golden/native198.txt n198_pdds: `[HELLO world         |HELLO][second   xy         |secon]`). A procedure with its own `disk(20)` file reads two records into a 20-byte structure with a subfield over part of it. The WHY pass lists none of NPDDS's statements. rla_test.rs CEEPGM's `read nosuch rec` is native now.

## S2.199: pointer differences beyond two pointer fields

A pointer minus a pointer where either side is not a pointer field, such as %ADDR of a field or of an array element, or a pointer expression, is lowered (`NumExpr::PtrDiffOf`). It is lowered as the whole value, as S2.98's PtrDiff is, or as the argument of %CHAR; neither gives the difference a precision of its own to matter. eval's operands come in order, then ptr_arith's Sub of two pointers, through S2.98's runtime ptr_diff: the offsets' difference within the same storage, or its error.

**Verified by** the native unit NPTRDF2, under both engines, as captured on PUB400 (tools/golden/native199.txt n199_ptrdiff: `2 8 -2`). rla_test.rs PTRS's `%char(p - %addr(name))` is native now.

## S2.200: EXTPGM naming a variable

A call through a prototype whose EXTPGM names a scalar character field is lowered. The runtime's call_proto reads the field's value, as eval gives it, and calls the program of that name in upper case, as call_named_inner does. call_named_inner reads the name before args_for evaluates the arguments, and generated code evaluates them first. So the call is lowered only when its arguments change nothing (fields, literals, figurative constants).

**Verified by** the native unit NPGMVAR, under both engines, against the PUB400 capture (tools/golden/native200.txt n200_pgmvar), on its first two calls (`1 0 hello 0`: to ASCII with QTCPASC and back with QTCPEBC). IBM i calls the program as the variable spells it, and the interpreter in upper case (fidelity.md). rla_test.rs HEAPA's call through such a prototype is native now.

## S2.201: C functions with float parameters; a call in a float comparison

A float parameter of a C function passed by value or CONST is lowered. args_for evaluates the argument, a number as it is, and the C function takes it as float_value. A side of a float comparison that is a call alone is lowered as the call's returned value (`FloatExpr::Call`). eval's comparison under is_float, like float_cmp, takes float_value of each side whatever type the call gives it. Calls inside float arithmetic still stay with the interpreter (S2.108): there eval picks the operation by the value's type.

**Verified by** the native unit NCFLOAT, under both engines, against the PUB400 capture (tools/golden/native201.txt n201_pow: `ABC`), on its first two comparisons. IBM i's pow(9, 0.5) is not exactly 3, while the interpreter's powf gives 3 (fidelity.md). The WHY pass lists none of NCFLOAT's statements. rla_test.rs HEAPA's `if pow(2 : 10) = 1024` is native now.
