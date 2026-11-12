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
