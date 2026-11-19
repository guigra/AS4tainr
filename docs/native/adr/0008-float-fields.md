# ADR 0008: float fields in native code

Status: **Accepted** by the user on 2026-11-19. Implemented in parts (S2.106 on).

## Context

Float fields (`float(4)`, `float(8)`) are now the largest group of statements still run by the interpreter: about 10 in the native-diff suite (S2.91's WHY pass). They are EVAL targets, %FLOAT, division of floats, %ABS of a float expression, and DSPLY of a float.

CLAUDE.md rule 2 forbids floating point for business values: packed and zoned stay exact decimals. That rule does not cover RPG float fields themselves, which are IEEE binary floating point on IBM i by definition (ILE RPG Reference, "Float format").

The interpreter already represents them with `f64` (`Value::Float`, `DType::Float(4|8)`). Its float rules are particular:
- **An operation is done in float when one operand is a float.** interp.rs eval converts the other operand with `float_value`. This covers +, -, *, / and the comparisons, but not character or indicator operands.
- **The result goes back to `Value::Num` when it fits a decimal** (`data::float_dec`), and stays `Value::Float` otherwise.
- **Division by 0.0 is MCH1211.**
- **%CHAR of a float, and DSPLY of one, have their own text** (`data::float_text`).

## Decision (proposed)

1. **Same model, same functions.** Native code takes the interpreter's float model unchanged and calls the same rpgrt functions (`float_value`, `float_dec`, `float_text`, and data::store and decode for DType::Float). No new float arithmetic is written, and no decimal value is ever turned into a float, except where the interpreter does so.
2. **A type and an expression of their own.** A new `ir::Ty::Float(n)` comes with a float expression in the IR, apart from `NumExpr`. It is lowered only where the interpreter's choice is decided by types known at lowering, which is the interpreter's own `is_float` test on the operand expressions. Each operation's result is the interpreter's `Value`, Num or Float, so an enclosing operation sees what eval would see.
3. **Results identical to the interpreter's.** Both engines run on the same machine with the same Rust `f64` operations in the same order. The differential suite must stay green, and the native units' expected values come from PUB400 captures, as always.
4. **Not in this ADR:** float host variables in embedded SQL, floats in data structure I/O formats beyond load and store, and %EDITC or %EDITW of floats. They stay with the interpreter.

## Consequences

- About 10 more statements become native, and so do the programs that use float(8) for ratios and scientific values.
- The rule "no f64 for business values" is unchanged. Floats only ever come from float fields or %FLOAT, as in the interpreter.
- A new risk is a native float path that rounds differently from the interpreter's. The differential units are the guard: each operation and conversion gets a unit, with expected values captured on PUB400.
