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
