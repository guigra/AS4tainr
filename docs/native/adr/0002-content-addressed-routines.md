# ADR 0002: Native routines found by the content of the compiled code

Status: accepted by the user (2026-11-11), after an independent review

## Context

CLAUDE.native.md defines the program key as a hash of the source, every /COPY and /INCLUDE, the compile options, the file and field definitions the program uses (as `compile_stamp` sees them), and the generator and runtime versions.

As upstream works today:

- Nothing compiled is kept. Every activation compiles the source again: every CALL from RPG goes through `compile_ovr` (interp.rs:8580).
- `compile_ovr` has an in-process cache keyed by name, options, library list, overrides and source (mod.rs:466-470) and checked against `compile_stamp`. A cache hit returns a full clone of the `Program` (mod.rs:474).
- `compile_stamp` is one global value: the count and latest change of `*FILE` objects, the count of file fields, and IFS changes for stream-file includes. It cannot tell which files or fields a program uses.
- Each clone's `Code`s are new allocations. A routine's address identifies it only while that `Program` lives, and a freed address can be reused.
- `run_code_at` is entered for every cycle pass, every EXSR, every procedure call and every *PSSR. Anything it does per entry must be O(1).
- `Program` is built in one place (`program::bind`, program.rs:3049) and derives `Clone` and `Debug`.

## Decision

**Identity: a fingerprint of each compiled routine.**

```
fingerprint = SHA-256(
    "as4tainr-native routine v1\n"
    rpgcg version, rpgrt version, rustc version\n
    for each I in the Code: format!("{:?}", I) + "\n"
    [stage 2: the layout of every storage item the lowered code uses]
)
```

- Stage 1 code depends only on the `Code`. Stage 2 also depends on the layout of the symbols it lowers. The fingerprint covers all of that, and more: the full `Debug` text of every statement.
- Every input the guide's key names reaches the compiled `Code`: the source, the includes, the options and defines, the overrides, and the file and field definitions that `bind_source` resolves. The fingerprint is therefore stricter than the guide's key.
- `Debug` is deterministic for everything reachable from `I`. The review checked `ast.rs`, `code.rs` and `dec.rs`: no `HashMap`, `HashSet`, `f64` or pointers. `Routines.subroutines` is a `HashMap`, but each `Code` is hashed on its own, so the map's order never enters a hash. `I.line` is part of the text, so a moved subroutine is a different routine.
- SHA-256 comes from `ring`, which core already depends on.

**Lookup: a native table carried by the `Program`.**

- `Program` gets one field, `native: native::Slot`, set to `Default::default()` in `bind`. That is two lines in program.rs, authorized by the user. A `Slot` is an `Arc<OnceLock<NativeTable>>` with `Clone` and a short `Debug`, and it is created when the program is compiled.
- The table is built the first time the dispatcher needs it, under an engine other than `interp`. Every later clone of the same compiled program (a `compile_ovr` cache hit, every CALL) shares the same `Arc`, so the fingerprints are computed once per compile, not once per CALL. With the engine `interp`, nothing is computed at all.
- *As implemented, 2026-11-11:* the first version of this ADR had `compile_ovr` fill the field. That is not needed with the lazy `OnceLock`, and `compile_ovr` is touched only for emission (ADR 0004).
- `NativeTable` maps a routine's **position** in the program to its generated function, or to the reason it has none. The position is: main detail or total; subroutine `name`; procedure `name` and its detail, a subroutine `name`, or ON-EXIT. Positions are the same in every clone.
- The dispatcher finds the position of the `&Code` it was given by comparing pointers with the routines of the `Program` the `Machine` is running (`self.prog`), then reads the table.
  - Pointer equality within the live `Program` is exact, so no address reuse can select the wrong code.
  - The cost is a handful of pointer comparisons per entry. A per-`Program` pointer → position map, built lazily, makes it O(1) if measurement says so.

**Without the `Program` field** (if the user refuses it), the fallback is a dispatcher cache keyed by heap address and length, plus fingerprints memoized by `compile_ovr`'s cache key.

- It stays safe only if every hit is validated by a deep `PartialEq` against a kept clone.
- That costs O(routine size) at every `run_code_at` entry, and it makes stage 1 measurably slower than the interpreter.
- It is recorded here as the rejected alternative.

## Consequences

- This deviates from the guide's wording, with a stricter key, and the guide should be updated with the user's agreement.
- Identical routines in different programs share one generated function. That is correct in stage 1, and stage 2 keeps it correct by adding layout to the fingerprint.
- A rustc upgrade can change `Debug` output, for example `str` escaping. Fingerprints then stop matching, and programs fall back (`auto`) or fail (`rust`); they never run the wrong code. The rustc version is in the fingerprint, and the toolchain is pinned.
