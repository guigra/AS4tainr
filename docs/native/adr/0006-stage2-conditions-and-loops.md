# ADR 0006: Stage 2, step 1: conditions and loop control in Rust

Status: accepted under ADR 0001 (stage 2 is lowering one construct at a time); written 2026-11-12

## Decision

The first constructs lowered to direct Rust (rpgcg 0.1.0+stage2.1):

- **Conditions** of jump-if-not instructions (IF, ELSEIF, DOW, DOU, WHEN): comparisons (`=`, `<>`, `<`, `<=`, `>`, `>=`) between eligible variables and numeric literals, and AND/OR of them.
- **FOR/DO control**: the loop test (index against its limit) and the step (index plus or minus the increment), when the index and the limit/increment are eligible.

An eligible variable is a global standalone or subfield of the main program with all of these properties:

- numeric: packed, zoned, binary, integer or unsigned (no float);
- not an array, not BASED, not in a multiple-occurrence data structure;
- not varying-dimension, not IMPORT or EXPORT;
- not a field of one of the program's files. Their buffer numbers are not stable across compiles; see fidelity.md.

Only routines of the main program are lowered; procedures keep stage 1. A main-program subroutine can be run from inside a procedure, where local names could hide a global one, so the generated code asks the runtime first (`in_procedure`). If it is inside a procedure, the instruction goes through the interpreter as in stage 1.

## Why it is the same as the interpreter

For such a variable, with no procedure frame:

- **Reading.** The interpreter's `eval` of the name is `load_var` with no index: `data::load(dtype, &buf[off..off + size])`. The generated code calls the same `rpgrt::rpg::data::load` on the same bytes (`rpgrt::native::load`).
- **Comparing.** Two numeric non-float values reach `data::compare` after the date, pointer and float paths, none of which applies. The generated code calls `data::compare` and maps the operator as `eval` does.
- **AND/OR** short-circuit as in `eval`.
- **Literals.** `Expr::Num(d)` is `Value::Num(d)`. The generated code builds the same `Dec` with `Dec::new(mantissa, scale)`; `Dec::new` does not normalize. A literal whose mantissa does not fit an i128 is not lowered.
- **Loop test and step.** These are `run_code_at`'s arms with `eval`/`assign` replaced as above. `assign` to such a target is `store_var` with no index: `data::store(dtype, value, &mut buf[off..off + size], Fit::Strict, false)`. The `*VARSIZE` short-storage branch cannot happen for a global, and `room_for_based` acts only for pointer values.

The fingerprint of a lowered routine also covers each lowered operand's name, buffer, offset and type (ADR 0002). A program whose layout changes gets new code, never stale code.

## Consequences

- The tripwire test watches `load_var`, `store_var`, the comparison part of `eval`, and the generated helpers' counterparts.
- Expected speed-up: loop control and conditions no longer resolve names, walk the AST or allocate. Statements still go through the interpreter. Measured with `scripts/native-bench.sh`.

## S2.2: numeric EVAL (rpgcg 0.1.0+stage2.2)

**What is lowered.** `EVAL` (no E extender; H is fine) where all of these hold:

- the target is an eligible field;
- the value is a tree of `+`, `-` and `*` over eligible fields and numeric literals, or a single operand.

**Why it is the same as the interpreter:**

- `stmt_e` without E is `stmt`, which is `stmt_inner` plus the pointer-fault check. `settle` makes that check after every instruction anyway.
- `stmt_inner`'s EVAL is `eval(value)`, then `assign(target, v, Fit::Strict { half_adjust: H }, right)`. The hexadecimal-literal case cannot happen, because the value is never `Expr::Hex`.
- In `eval`, `+`, `-` and `*` of two numeric non-float values skip the date, pointer, timestamp and float paths. They are `to_rule_scale(node, x.add/sub/mul(y))`.
- `to_rule_scale` is `num_precision(node)` followed by a rescale with no rounding when there are 37 decimal places or fewer. `rpgrt::native::rule_scale` is that rescale.
- The precision itself is not copied. The generated code asks the interpreter's own `num_precision`, through `Runtime::precision(pc, path)`, once per operation, and keeps it in a `OnceLock`. It depends only on the operands' types and the literals' digits, all of which the fingerprint covers, and it is asked only when no procedure is running.

**Expected values.** The truncation, EVAL(H) rounding, the decimals of the intermediate results, and overflow come from PUB400: tools/golden/native2.txt n2_eval (IBM i 7.5, 2026-11-12). as4tainr gives the same values.

## S2.3: fixed-form ADD, SUB, MULT, Z-ADD, Z-SUB (rpgcg 0.1.0+stage2.3)

**What is lowered.** These operations with no E extender (H is fine), when factor 1, factor 2 and the result field are eligible. DIV and MVR are not lowered: they use the machine's remainder (`Machine::remainder`).

**Why it is the same as the interpreter.** `stmt_inner`'s arithmetic arm does this, in this order:

1. Z-ADD takes factor 2's value; Z-SUB takes its negation.
2. ADD, SUB and MULT evaluate factor 1 (or the result field when there is none), then factor 2, and combine them with exact `Dec` operations. No precision rules apply.
3. The value is assigned with `Fit::Truncate { half_adjust: H }`.
4. The result field is read back, and `num_inds` sets the resulting indicators from it.

The generated code does the same steps, with the same rpgrt functions. Steps 1 to 3 work directly on the field's bytes. The indicators are set by the interpreter's own `num_inds`, called through `Runtime::num_inds`.

**Expected values.** The sums, high-order truncation (998 + 5 into a 3,0 field gives 3), MULT and MULT(H), and the resulting indicators come from PUB400: tools/golden/native3.txt n3_arith (IBM i 7.5, 2026-11-12). as4tainr gives the same ten lines.

## S2.4: character fields (rpgcg 0.1.0+stage2.4)

**What is lowered.** Eligible fixed-length character fields (`CHAR(n)`) and character literals, with the same exclusions as numeric fields:

- in conditions: comparisons where both operands are character, or both numeric;
- in EVAL/EVALR (no E extender): a character target and a tree of `+` (concatenation) over character operands.

**Why it is the same as the interpreter:**

- A character name reads as `data::load` of its bytes (`Value::Char`); a literal is `Value::Char` of its bytes.
- `+` with a character operand is concatenation, `chars()` of each joined, after the date, pointer, timestamp and float paths, none of which applies. Comparisons are `data::compare`.
- The assignment is `data::store` with `Fit::Strict` and `right` (EVALR), which pads and truncates as the interpreter does.

**Expected values.** Concatenation keeping trailing blanks, truncation and padding with EVAL and EVALR, blank-padded comparison, and EBCDIC order (lower case before upper case before digits) come from PUB400: tools/golden/native4.txt n4_char (IBM i 7.5, 2026-11-12). as4tainr gives the same nine lines.

## S2.5a and S2.6

- **S2.5a** (no behavior change): the dispatcher finds a routine's native code by comparing addresses, with no allocation, and keeps the program's first missing routine. Verified with native-diff. NBENCH1 then runs 20.8% faster than the interpreter on aarch64.
- **S2.6:** file fields become eligible. Upstream's `bind` turned file fields that are not defined otherwise into standalone fields in the iteration order of a `HashMap`, so their buffer numbers changed from one compile to the next. With the user's approval (2026-11-12), program.rs sorts them by name first, one line. Each field has a buffer of its own, so only the numbering changes, and it is now the same in every compile. Level 1 of the harness compiles each corpus program again before each engine's run; with file fields lowered, a numbering that still changed would show up there as refused native code.

## S2.7: fixed-form DIV and MVR (rpgcg 0.1.0+stage2.7)

**What the interpreter does, and the generated code does the same:**

- **DIV** computes `q = a1.div_scale(a2, decs + H)`. `decs` is the result field's decimal places (`type_of(result).precision()`), worked out at lowering from the same type. Then `qt = q.rescale(decs, false)`, and the remainder `a1 - qt * a2` is kept before the assignment.
- **MVR** assigns that remainder.

The remainder is the machine's (`Machine::remainder`), reached through `Runtime::remainder` and `set_remainder`, so an MVR finds it whichever engine ran the DIV. Division by zero is `div_scale`'s own error (status 00102).

**Expected values.** The quotient's decimals, DIV(H), the remainder and its sign, the indicators, and status 00102 come from PUB400: tools/golden/native5.txt n5_div (IBM i 7.5, 2026-11-12). as4tainr gives the same six lines.

## S2.8: procedures (rpgcg 0.1.0+stage2.8)

**What changes.** Lowering now covers a procedure's routines too: its body, its subroutines and ON-EXIT. Names are looked up as the procedure sees them: its own `syms`, which hold its locals and the globals bound into it.

**Locals.** A local (not based, not an array) is lowered with its frame buffer and offset. The generated code asks `Runtime::local(buf, off)` where it is in this call. That is `Machine::sym`'s rule: a parameter passed by reference (`Frame::alias`) is the caller's storage, anything else is `base + buf`, and `check_buf` applies. The tripwire now watches `sym`.

**Storage.** `rpgrt::native::load` and `store` now also copy `load_var`'s and `store_var`'s handling of a parameter longer than the storage passed for it (OPTIONS(*VARSIZE)): blanks after what is there, and only the argument's storage written.

**When lowered code runs.** The guard is now `Runtime::native_ok`, worked out once at the routine's entry:

- a main-program routine runs lowered code only with no procedure frame;
- a procedure's routine runs it only when the top frame is that procedure's own (`ProcInfo` compared by address).

Anywhere else the interpreter runs the instruction, as in stage 1.

**Expected values.** `core/tests/native/NPROC.rpgle`: a FOR loop and EVAL on locals, a parameter passed by reference and changed, a CONST parameter read, recursion. The values are the arithmetic, under the parameter rules of IBM Docs "Passing by Reference" and "Passing by Value".

## S2.9: %TRIM, %TRIML, %TRIMR and %LEN (rpgcg 0.1.0+stage2.9)

**What changes.** A character expression can now hold %TRIM, %TRIML or %TRIMR of one argument, itself a character expression. The generated helper `t<pc>_<path>` calls `rpgrt::native::trim`, which drops leading and trailing blanks (X'40') as the interpreter's `"TRIM" | "TRIML" | "TRIMR"` arm does when there is no second argument. %LEN of an eligible fixed-length character field becomes a numeric literal: its length, which is what the interpreter's `type_of(arg).len()` gives. A varying field is not eligible, so its %LEN, which depends on the current length, stays with the interpreter.

**Precision.** A numeric expression with %LEN in it still asks `Runtime::precision` for the precision of its operations, from the original expression. So the interpreter's precision rules for %LEN apply unchanged.

**Not lowered.** A second argument of %TRIM (the characters to trim), %TRIM in a condition, and %LEN of anything other than a fixed-length character field.

**Expected values.** `core/tests/native/NBIF.rpgle`, which covers:

- each of the three functions;
- a trim in a concatenation and in EVALR;
- a field that is all blanks;
- %LEN in arithmetic.

The expected values were captured on PUB400: tools/golden/native6.txt n6_bif (IBM i 7.5, 2026-11-12).

## S2.10: %SUBST (rpgcg 0.1.0+stage2.10)

**What changes.** A character expression can now hold %SUBST of a character expression, with a start and an optional length. Each of them is a numeric literal or an eligible field. The generated helper `u<pc>_<path>` evaluates in the interpreter's order: the string, then the start, which `rpgrt::native::subst_start` checks, then the length, which `subst_take` checks. Both functions raise the interpreter's RNQ0100 errors, with the same message texts (as4tainr's own), so status 00100, MONITOR and *PSSR see the same thing.

**One difference, for absurd values only.** A start or a length beyond `usize` is out of range in the native code. The interpreter truncates it with `as usize`.

**Not lowered.** A start or a length that is an expression, and %SUBST as the target of an assignment.

**Expected values.** `core/tests/native/NSUB.rpgle`, captured on PUB400: tools/golden/native7.txt n7_subst (IBM i 7.5, 2026-11-12). That capture found two interpreter defects (fidelity.md): a start one past the end with length 0 is status 00100 on IBM i, and constant arguments out of range do not compile. Both are left out of the test.

## S2.11: fixed-form MOVE and MOVEL (rpgcg 0.1.0+stage2.11)

**What changes.** MOVE and MOVEL, with or without P, are now lowered when:

- they have no date format in factor 1 and no result indicators;
- factor 2 is a literal or an eligible field;
- the result is an eligible field.

Eligible fields are never dates, times or timestamps, so the interpreter's `move_temporal` would do nothing for them. The generated helper `mv<pc>` uses `rpgrt::native::move_image` and `move_into`, which are the interpreter's `move_image` and `do_move` written again under section 5's lints:

- a number is moved as zoned digits of its type's precision (a literal's own digits);
- a numeric result takes the digit of each byte and the sign from the zone of the last;
- the value is assigned with truncation.

**Not lowered.** Result indicators, figurative constants, data structures, arrays, and dates.

**Expected values.** `core/tests/native/NMOVE.rpgle`, captured on PUB400: tools/golden/native8.txt n8_move (IBM i 7.5, 2026-11-12). The interpreter gives the same values.

## S2.12: %SCAN and %XLATE (rpgcg 0.1.0+stage2.12)

**What changes.**

- %SCAN of two character expressions, with a start that is a numeric literal or an eligible field, is now a numeric expression of lowered code. Its helper is `n<pc>_<path>`.
- %XLATE of three character expressions, with an optional start of the same kind, is a character expression. Its helper is `x<pc>_<path>`.

The helpers evaluate the arguments in the interpreter's order. `rpgrt::native::scan` and `xlate` compute what the interpreter's `"SCAN"` and `"XLATE"` arms compute, with the same RNQ0100 error for an %XLATE start outside the string.

Helper names now spell a third operand `x` (`path_id`). The names of existing helpers, which have only first and second operands, are unchanged.

**Not lowered.** %SCAN in a condition (IF %SCAN(...) > 0), %SCANRPL, %CHECK and %REPLACE.

**Expected values.** `core/tests/native/NSCAN.rpgle`, captured on PUB400: tools/golden/native9.txt n9_scan (IBM i 7.5, 2026-11-12). The capture found an interpreter defect (fidelity.md): %SCAN with a start outside the string is status 00100 on IBM i. It is left out of the test.

## S2.13: conditions with functions (rpgcg 0.1.0+stage2.13)

**What changes.** A comparison in a condition (IF, ELSEIF, DOW, DOU, WHEN) can now have an expression on either side.

- A character side is any lowered character expression: %SUBST, %TRIM, %TRIML, %TRIMR, %XLATE, concatenation.
- A numeric side is a lowered numeric expression with no `+`, `-` or `*`: %SCAN, or %LEN of a fixed-length field.

Both sides must be of the same class. Comparisons of two literals or names stay `Cond::Cmp`. The sides' helpers are named by their place in the condition (0 left, 1 right), and the comparison is `rpgrt::native::cmp`, as for operands.

**Why no arithmetic.** The precision of `+`, `-` and `*` comes from `Runtime::precision`, which reads it from an EVAL's expression. A condition's arithmetic would need its own lookup. Until then it stays with the interpreter.

**Expected values.** `core/tests/native/NCOND.rpgle`, captured on PUB400: tools/golden/native11.txt n11_cond (IBM i 7.5, 2026-11-12).

## S2.14: %CHAR of numbers, %INT, %INTH, %DEC, %DECH (rpgcg 0.1.0+stage2.14)

**What changes.**

- %INT and %INTH of a numeric expression are numeric expressions of lowered code: `rescale(0, half)`, as the interpreter.
- %DEC and %DECH are numeric expressions too, with no other argument or with literal digits and decimal places. `rpgrt::native::dec_to` gives the interpreter's RNQ0103 error when the value does not fit.
- %CHAR of a numeric expression is a character expression. `rpgrt::native::char_of_num` is the interpreter's `char_of_num_edit`, written again under section 5's lints.

**DECEDIT.** The decimal point and the zero before it come from the program's DECEDIT (`Program::decedit`), resolved when the code is generated. They are therefore part of the code, and the lowering adds them to the fingerprint's layout lines (`decedit <point> <zero>`).

**Precision inside functions and conditions.** `Runtime::precision` now finds its node from a condition (`JumpIfNot`) as well as from an EVAL. A step of its path is 0 or 1 into an operation, or the index of a built-in function's argument; every lowered helper's path already follows that numbering. So `%INT(z * 10)` and `IF %INT(d * 2) = -3` take their precision from the interpreter's `num_precision` of the same node.

**Not lowered.** %CHAR of dates, times, indicators and character values, %CHAR with a format, %DEC of character values, and %DEC with digits that are not literals.

**Expected values.** `core/tests/native/NFUN.rpgle` and `NFUN2.rpgle` (DECEDIT('0,')), captured on PUB400: tools/golden/native12.txt n12_fun and n12_decedit (IBM i 7.5, 2026-11-12).

## S2.15: array elements with literal indexes (rpgcg 0.1.0+stage2.15)

**What changes.** `ARR(k)` with a literal integer `k` from 1 to the array's DIM is an eligible field. It applies to global arrays that are not based, varying (`DIM(*AUTO)`), in a multiple-occurrence data structure, imported, exported, or a table (`TAB...`). The element is a field at a fixed place, `off + (k - 1) * stride`: that is the interpreter's `elem_off`, whose index check cannot fail for such a `k`. Its layout line names the element (`layout ARR(3) ...`), so the fingerprint covers the offset. Scalar fields keep the layout line they had.

**Not lowered.** An index out of range (the interpreter's RNQ0121 path stays the interpreter's), a variable or computed index, local arrays, and whole arrays.

**Expected values.** `core/tests/native/NARR.rpgle`, captured on PUB400: tools/golden/native16.txt n16_arr (IBM i 7.5, 2026-11-13).

## S2.16: qualified subfields (rpgcg 0.1.0+stage2.16)

**What changes.** A qualified name (`ds.field`, a QUALIFIED or LIKEDS data structure's subfield) is looked up under its parts joined with periods, as the interpreter's `sym(&joined(parts))` does. An eligible one is lowered as any field. Names with an index on a part before the last (`list(3).name`, `Expr::Path`) are not lowered.

**Expected values.** `core/tests/native/NDSQ.rpgle`, captured on PUB400: tools/golden/native17.txt n17_dsq (IBM i 7.5, 2026-11-13).

## S2.17: / in expressions (rpgcg 0.1.0+stage2.17)

**What changes.** `/` is an operation of lowered numeric expressions. `rpgrt::native::divide` is the interpreter's `divide`, written again under section 5's lints:

- the quotient's decimal places are `M - ((L1 - D1) + D2)`, with M = 30, or 63 when an operand has more than 30 digits, and at most 37;
- fewer places are used when the quotient does not fit;
- a zero divisor is MCH1211, which is status 00102.

Each operand's precision is that of its own node, `Runtime::precision(pc, path + [0])` and `[1]`, kept in a `OnceLock` per helper. When it is not known, the operand's own digits are used, as in the interpreter. The quotient is not rescaled further (no `rule_scale`), which also matches the interpreter.

**Not lowered.** `/` at the top of a comparison's side (as for `+`, `-`, `*`), and `**`.

**Expected values.** `core/tests/native/NDIVE.rpgle`, captured on PUB400: tools/golden/native18.txt n18_div (IBM i 7.5, 2026-11-13). It agrees with upstream's own capture tools/golden/rpg-prec.txt: 28, 27, 25 and 20 decimal places.

## S2.18: %REM and %DIV (rpgcg 0.1.0+stage2.18)

**What changes.** %DIV and %REM of two numeric expressions are numeric expressions of lowered code. They compute `x.div_scale(&y, 0)` and `x.rem(&y)`, as the interpreter's `"DIV"` and `"REM"` arms, and a zero divisor gives the same MCH1211 (status 00102).

**Expected values.** `core/tests/native/NREM.rpgle`, captured on PUB400: tools/golden/native19.txt n19_rem (IBM i 7.5, 2026-11-13).

## S2.19: varying-length character fields (rpgcg 0.1.0+stage2.19)

**What changes.** A VARCHAR field is eligible (`Ty::Varchar`), and it is a character operand. It is loaded and stored with `data::load` and `data::store` over its whole storage, the length prefix included, as the interpreter's `load_var` and `store_var`. So its current length, truncation at the maximum and comparisons behave as there. Varying-dimension arrays (`Program::varying`) stay excluded. %LEN of a varying field depends on its current length, so it stays with the interpreter, as does %LEN(v) as a target.

**Expected values.** `core/tests/native/NVAR.rpgle`, captured on PUB400: tools/golden/native20.txt n20_var (IBM i 7.5, 2026-11-13).

## S2.20: %LEN of varying-length fields (rpgcg 0.1.0+stage2.20)

**What changes.** %LEN of an eligible VARCHAR field is a numeric expression of lowered code: the length of its current value, as the interpreter's `"LEN"` arm gives it for a varying field. %LEN with *MAX, and %LEN as a target, stay with the interpreter.

**Expected values.** `core/tests/native/NVLEN.rpgle`, captured on PUB400: tools/golden/native21.txt n21_vlen (IBM i 7.5, 2026-11-13).

## S2.21: array elements with an index that is a field (rpgcg 0.1.0+stage2.21)

**What changes.** `ARR(i)`, with `i` an eligible numeric field or literal, is an operand of lowered code. It applies to the same arrays as S2.15. The operand carries its index (`ir::Index`: the index operand, the stride and DIM). Each load computes its place at run time with `rpgrt::native::elem_at`, and so does each store, in its prelude. `elem_at` is the interpreter's `elem_off`: the index's number as an integer, from 1 to DIM, else RNQ0121 with the same text (status 00121). As in the interpreter, a target's index is evaluated after the value. The layout line names the index, the stride and DIM.

**Not lowered.** Computed indexes (`arr(i - 1)`), local arrays, tables, varying-dimension arrays.

**Expected values.** `core/tests/native/NARRV.rpgle`, captured on PUB400: tools/golden/native27.txt n27_arrv (IBM i 7.5, 2026-11-14).

## S2.22: figurative constants (rpgcg 0.1.0+stage2.22)

**What changes.**

- `*BLANKS`, `*ZEROS`, `*HIVAL`, `*LOVAL` and `*ALL'x'` are operands of comparisons (`Operand::Fig`). They compare with an operand of either class, through `data::compare`, as in the interpreter.
- EVAL of one into an eligible field is `Lowered::EvalFig`. It stores `Value::Fig` with `data::store`, which fills the field by its type, as the interpreter's `assign` does.

A figurative constant is never a numeric operand of lowered arithmetic, nor factor 2 of a lowered MOVE: there the interpreter fills it to the result's length.

**Expected values.** `core/tests/native/NFIG.rpgle`, captured on PUB400: tools/golden/native28.txt n28_fig (IBM i 7.5, 2026-11-14). The capture found two interpreter defects (fidelity.md): a numeric field compared with *HIVAL, and *BLANKS into a varying field. Both are left out of the test.

## S2.23: %REPLACE (rpgcg 0.1.0+stage2.23)

**What changes.** %REPLACE of two character expressions, with a start and a length that are numeric literals or eligible fields, is a character expression of lowered code. Its helper is `w<pc>_<path>`. `rpgrt::native::replace` computes what the interpreter's `"REPLACE"` arm computes, the start and length cut to the string, in the same order of evaluation.

**Expected values.** `core/tests/native/NREPL.rpgle`, captured on PUB400: tools/golden/native30.txt n30_repl (IBM i 7.5, 2026-11-14). The capture found an interpreter defect (fidelity.md): a start outside 1 to the length plus 1 is status 00100 on IBM i. It is left out of the test.

## S2.24: a whole data structure as a character operand (rpgcg 0.1.0+stage2.24)

**What changes.** The name of a data structure is an eligible operand when the data structure is global, not an array, not based, not of multiple occurrences, and not imported or exported. It is `Ty::Char(size)` at the data structure's buffer and offset, because that is how the interpreter treats it: `eval` gives its bytes as `Value::Char`, and `assign` stores with `data::store` as `DType::Char(size)`. Its subfields share the bytes, lowered or not. The layout line marks it `ds`.

**Expected values.** `core/tests/native/NDSW.rpgle`, captured on PUB400: tools/golden/native31.txt n31_dsw (IBM i 7.5, 2026-11-14).

## S2.25: %SIZE and %ELEM (rpgcg 0.1.0+stage2.25)

**What changes.** %SIZE of a field, an array element (and of the whole array with a second argument), a data structure or a named constant, and %ELEM of an array that is not of varying dimension or of a data structure array, are numeric literals of lowered code. Their values are the interpreter's `"SIZE"` and `"ELEM"` arms, worked out from the same definitions. The value goes into the fingerprint's layout lines.

**Expected values.** `core/tests/native/NSIZE.rpgle`, captured on PUB400: tools/golden/native32.txt n32_size (IBM i 7.5, 2026-11-14). The capture found an interpreter defect (fidelity.md): %SIZE of a numeric named constant is its digits as written, not its size as packed decimal. It is left out of the test.

## S2.26: %EDITC (rpgcg 0.1.0+stage2.26)

**What changes.** %EDITC of a numeric expression with a literal edit code is a character expression of lowered code. Its helper is `k<pc>_<path>`. `rpgrt::native::edit_code_with` is the interpreter's `edit_code_with`, written again under section 5's lints. `editc` chooses the digits and decimal places as the interpreter's `"EDITC"` arm does:
- for a field or an expression, its node's precision (`Runtime::precision`);
- for a literal, its own digits.

The program's DECEDIT is resolved when the code is generated, and is in the fingerprint's layout. The tripwire watches `edit_code_with`.

**Not lowered.** The third argument (*ASTFILL, *CURSYM, a currency symbol), an edit code that is not a literal, and %EDITW.

**Expected values.** `core/tests/native/NEDITC.rpgle`, captured on PUB400: tools/golden/native34.txt n34_editc (IBM i 7.5, 2026-11-14), 33 cases. The interpreter gives the same 33 values.

## S2.27: %LOOKUP and %LOOKUPxx (rpgcg 0.1.0+stage2.27)

**What changes.** %LOOKUP, %LOOKUPLT, %LOOKUPLE, %LOOKUPGE and %LOOKUPGT are numeric expressions of lowered code when:
- the argument is a literal or an eligible field;
- the array is global and not a table, of varying dimension, based, of multiple occurrences, imported or exported;
- the start and the number of elements are literals or fields.

The generated code keeps the interpreter's order of evaluation: the argument, then the array, then the start, then the number. `rpgrt::native::lookup_list` loads the DIM elements, as `list_of` does. `search` is the interpreter's `search`. The array's place and type go into the fingerprint's layout.

**Not lowered.** %TLOOKUP (it moves the table's current element), arrays of data structures (`ds(*).field`), local arrays.

**Expected values.** `core/tests/native/NLOOK.rpgle`, captured on PUB400: tools/golden/native35.txt n35_look (IBM i 7.5, 2026-11-14). The interpreter gives the same 13 values.

## S2.28: %SCANRPL (rpgcg 0.1.0+stage2.28)

**What changes.** %SCANRPL of three character expressions, with a start and a length that are literals or fields, is a character expression of lowered code. Its helper is `v<pc>_<path>`. `rpgrt::native::scanrpl` is the interpreter's `"SCANRPL"` arm, in the same order of evaluation and with the same RNQ0100 check. One difference: where the interpreter's default length underflows (a start more than one past the end) and panics in a debug build, the native code computes it with saturation and gives the RNQ0100 error, which is what a release build of the interpreter gives.

**Expected values.** `core/tests/native/NSRPL.rpgle`, captured on PUB400: tools/golden/native36.txt n36_srpl (IBM i 7.5, 2026-11-14). Two cases differ in the interpreter (fidelity.md) and are left out of the test.

## S2.29: %EDITW (rpgcg 0.1.0+stage2.29)

**What changes.** %EDITW of a numeric expression with a literal edit word is a character expression of lowered code. Its helper is `o<pc>_<path>`. `rpgrt::native::edit_word` is the interpreter's `edit_word`, written again under section 5's lints. `editw` takes the digits and decimal places as the interpreter's `"EDITW"` arm does: a field's type, or for any other expression the value's own digits, at least one. That differs from %EDITC, which takes an expression's precision. The tripwire watches `edit_word`.

**Expected values.** `core/tests/native/NEDITW.rpgle`, captured on PUB400: tools/golden/native37.txt n37_editw (IBM i 7.5, 2026-11-14). The capture found an interpreter defect (fidelity.md): with a floating currency symbol, the thousands separator lands one position off. It is left out of the test.

## S2.30: subfields of a data structure array (rpgcg 0.1.0+stage2.30)

**What changes.** `ds(i).field` is an eligible operand when:
- the data structure array is global, not based and not of multiple occurrences;
- the subfield is scalar and of an eligible type;
- the index is a literal or an eligible numeric field.

The interpreter's `path_sym` adds (i - 1) times the data structure's size to the subfield's place, after the same RNQ0121 check as an array element's, with the same text. So the operand is S2.21's indexed operand, with the data structure's size as the stride and its DIM.

**Not lowered.** Deeper paths (`a(i).b(j).c`), array subfields, local data structure arrays, `ds(*)`.

**Expected values.** `core/tests/native/NDSA.rpgle`, captured on PUB400: tools/golden/native38.txt n38_dsa (IBM i 7.5, 2026-11-14). The interpreter gives the same values.

## S2.31: %STATUS with no file (rpgcg 0.1.0+stage2.31)

**What changes.** `%STATUS` with no argument is a numeric expression that is eligible anywhere a numeric expression is: an EVAL's value, arithmetic, a condition. The interpreter gives `Dec::new(code, 0)` from the machine's last status (`bifs.rs` "STATUS"). Generated code asks the runtime for that same code (`Runtime::status`, S2.31), so the value comes from where the interpreter keeps it: errors are settled by the interpreter's own path (`settle`), so `last` is set the same way in both engines.

**Not lowered.** `%STATUS(file)`: a file's status.

**Expected values.** `core/tests/native/NSTAT.rpgle`, captured on PUB400: tools/golden/native40.txt n40_stat (IBM i 7.5, 2026-11-14). The program keeps the values and writes them at the end. A first version wrote each one as it went, through a procedure that runs SQL, and on IBM i %STATUS was 0 again after each such call. That case is not in the test; it is noted in HANDOFF.md to look at. The interpreter gives the captured values.
