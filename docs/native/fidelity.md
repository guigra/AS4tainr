# Native RPG backend: fidelity

This file covers how the native engine (generated code) differs from the interpreter, and the interpreter defects found while building it. The backend reproduces the interpreter; it never fixes IBM i behavior on its own (CLAUDE.native.md section 4).

## Differences between the native engine and the interpreter

Stage 1 (ADR 0001). Generated code compiles each routine's control flow and calls the interpreter's own code for every statement and expression. Its error routing is a copy of `run_code_at`'s, watched by `core/tests/native_tripwire.rs`.

| Area | Native engine | Interpreter | Why |
| --- | --- | --- | --- |
| Source line in generated code | A comment per instruction with the RPG line number. The source map (`<fingerprint>.map.json`) gives each instruction's pc, line, kind and line of generated Rust | — | The compiled code (`code::I`) keeps the line number only, not the member and sequence number that section 3 asks for |
| RETURN of a whole data structure array past its storage | An error (`native: data structure array past its storage`) | Panics on the slice | Generated code's runtime may not panic (section 5). Unreachable when the layout is right |
| FOR-EACH position counter | Saturating add | `+= 1` | The same for every list that fits in memory |
| A program whose routines are not all native, under `rust` | Ends before any calculation, with an error naming `AS4TAINR_RPG_ENGINE`, the program, the routine and its fingerprint; its *PSSR does not run, and the caller sees a failed call | — | ADR 0004 |
| MONITOR groups re-entered by a GOTO backwards | The stack grows as the interpreter's does | The same | Same semantics, kept on purpose |

## Interpreter defects found (upstream; not fixed here)

Each was observed on the interpreter. The backend reproduces it, as section 4 requires, until upstream fixes it. Reporting it upstream is the user's decision. A PUB400 capture would confirm each one (`tools/golden/`); none has been made yet.

| Defect | IBM i (documented) | as4tainr | Source |
| --- | --- | --- | --- |
| Nested MONITOR: an error the inner group has no ON-ERROR for | The enclosing groups get it in turn, from the innermost outwards | The routine ends with the error (`MCH1211` for a division by zero), and the outer `ON-ERROR 00102` never runs. `run_code_at` stops at the first group whose body holds the instruction, even when none of its handlers takes the error | IBM Docs, "MONITOR (Begin a Monitor Group)", IBM i 7.5, https://www.ibm.com/docs/en/ssw_ibm_i_75/rzasd/zzmonit.htm, read 2026-11-11. Probe: inner `ON-ERROR 00103`, outer `ON-ERROR 00102`, `x = 10 / 0` in the inner body |
| Assigning to a CONST parameter inside the procedure | Not allowed: the parameter is read-only, and the compiler checks it | Compiles and runs; the assignment takes effect | IBM Docs, "CONST{(constant)}", IBM i 7.5, https://www.ibm.com/docs/en/ssw_ibm_i_75/rzasd/dconst.htm, read 2026-11-11 |
| `OVERLAY(field : *NEXT)` | The subfield goes after the earlier overlays of the same field. The page's example: PartNumber 10A; Family 1-3, Sequence 4-9, Language 10 | Sequence and Language go after the whole of PartNumber (the data structure is 17 bytes, not 10) | IBM Docs, "OVERLAY(name{:start_pos \| *NEXT})", IBM i 7.5, https://www.ibm.com/docs/en/ssw_ibm_i_75/rzasd/doverly.htm, read 2026-11-11 |

## No public source (parity only)

These are compared between engines but have no IBM documentation that fixes the expected value. Each needs a PUB400 capture before it can be tested as IBM i behavior:

- RNX message IDs for status codes 00102 and 00103;
- two reference parameters aliasing one variable;
- the diagnostic for passing a CONST parameter on as a writable one;
- the text DSPLY shows for a numeric field.

## Notes for stage 2

- **Buffer numbers of external fields are not stable across compiles.** Two compiles of the same source, against files made again with the same DDS, gave CUSTLIST's scalar fields CUSNO and a zero packed field each other's buffer numbers. The values were the same. The likely cause is the order in which the catalog returns a file's fields; that is not confirmed. Stage 1 does not depend on buffer numbers. The level-1 diff matches buffers by the names of what is in them. A lowering that uses storage layout must put that layout into the fingerprint (ADR 0002), so a different numbering means different code, never wrong code. If the numbering keeps changing between runs, native code would be missed: the cause must be found first.
