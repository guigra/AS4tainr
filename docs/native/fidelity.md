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

Each was observed on the interpreter and confirmed on a real IBM i: PUB400, IBM i 7.5, 2026-11-12, with `tools/golden/native1.txt` (the captures are in `target/ibmi/golden`, not committed). The backend reproduces each one, as section 4 requires, until upstream fixes it. They were reported upstream on 2026-11-12, after a clean build of upstream main (f20febb) reproduced each one: kyleturley-cyber/AS4tainr issues #3 (nested MONITOR), #4 (OVERLAY *NEXT), #5 (CONST parameters), #6 (%SUBST and %SCAN start), #7 (constant %SUBST checks) and #8 (MOVE with a float field).

| Defect | IBM i | as4tainr | Source |
| --- | --- | --- | --- |
| Nested MONITOR: an error the inner group has no ON-ERROR for | The enclosing group takes it: `MON OUTER` then `MON AFTER` | The routine ends with the error (`MCH1211`), and the outer `ON-ERROR 00102` never runs. `run_code_at` stops at the first group whose body holds the instruction, even when none of its handlers takes the error | Golden capture tools/golden/native1.txt n1_mon (PUB400, IBM i 7.5, 2026-11-12). IBM Docs, "MONITOR (Begin a Monitor Group)", IBM i 7.5, https://www.ibm.com/docs/en/ssw_ibm_i_75/rzasd/zzmonit.htm, read 2026-11-11 |
| Assigning to a CONST parameter inside the procedure | Does not compile: RNF5346, severity 30 | Compiles and runs; the assignment takes effect | Golden capture tools/golden/native1.txt n1_crt_cst (PUB400, IBM i 7.5, 2026-11-12). IBM Docs, "CONST{(constant)}", IBM i 7.5, https://www.ibm.com/docs/en/ssw_ibm_i_75/rzasd/dconst.htm, read 2026-11-11 |
| Passing a CONST parameter on to a procedure that changes its parameter | Does not compile: RNF7542, severity 30 | Compiles and runs | Golden capture tools/golden/native1.txt n1_crt_fwd (PUB400, IBM i 7.5, 2026-11-12) |
| `OVERLAY(field : *NEXT)` | Family 1-3, Sequence 4-9, Language 10; the data structure is 10 bytes | Sequence and Language go after the whole of PartNumber; the data structure is 17 bytes | Golden capture tools/golden/native1.txt n1_ovl (PUB400, IBM i 7.5, 2026-11-12). IBM Docs, "OVERLAY(name{:start_pos \| *NEXT})", IBM i 7.5, https://www.ibm.com/docs/en/ssw_ibm_i_75/rzasd/doverly.htm, read 2026-11-11 |
| `%SUBST(s : start : 0)` with `start` one past the end (a 10-character field, start 11 in a variable) | Status 00100 | Accepted: a string of length 0. `interp.rs`'s `"SUBST"` arm allows a start up to the length plus 1 | Golden capture tools/golden/native7.txt n7_subst, case SUBST END ZERO (PUB400, IBM i 7.5, 2026-11-12); the interpreter by reading its code |
| %SUBST with constant arguments out of range (start 11 or 12 of a 10-character field, or a length past the end) | Does not compile: RNF0364 and RNF0365, severity 20 | No such check (no RNF0364 or RNF0365 in core/src); the statement compiles and is checked when it runs | Golden capture tools/golden/native7.txt n7_crt_const, source gn7const.rpgle (PUB400, IBM i 7.5, 2026-11-12) |
| `%SCAN(pat : s : start)` with `start` outside 1 to the length of `s` (0, or 11 for a 10-character field, in a variable) | Status 00100 | 0, no error: `interp.rs`'s `"SCAN"` arm gives 0 for any start outside the string | Golden capture tools/golden/native9.txt n9_scan, cases SCAN PAST and SCAN ZERO (PUB400, IBM i 7.5, 2026-11-12); the interpreter by reading its code |

## Settled by capture

- **Two reference parameters that are one variable:** they share its storage. After `p1 = 1; p2 = 2`, both are 2, and so is the caller's variable. as4tainr does the same. The capture is tools/golden/native1.txt n1_alias (PUB400, IBM i 7.5, 2026-11-12); `core/tests/native/ALIAS.rpgle` tests it.
- **An unmonitored division by zero or overflow, with the job's inquiry messages answered by default** (`CHGJOB INQMSGRPY(*DFT)`): the only exception message the caller receives is CPF9999. The captures are n1_div and n1_ovf (PUB400, IBM i 7.5, 2026-11-12). Not yet compared with as4tainr, which signals RNX escape messages (CLAUDE.md conventions). It is to be checked with a CL caller under `*DFT`.

## No public source yet (parity only)

- The text DSPLY shows for a numeric field.

## Notes for stage 2

- **Buffer numbers of external fields are not stable across compiles.** Two compiles of the same source, against files made again with the same DDS, gave CUSTLIST's scalar fields CUSNO and a zero packed field each other's buffer numbers. The values were the same. Cause, confirmed: `bind` (core/src/rpg/program.rs:2682) turns file fields that are not defined otherwise into standalone fields by iterating `file_fields`, a `std::collections::HashMap`, whose iteration order is random per instance. The catalog query itself is ordered (`ORDER BY seq`). A one-line upstream fix, sorting `names` before the loop, would make the numbering stable. It is outside the touch points and needs the user's approval when stage 2 needs it. Stage 1 does not depend on buffer numbers. The level-1 diff matches buffers by the names of what is in them. A lowering that uses storage layout must put that layout into the fingerprint (ADR 0002), so a different numbering means different code, never wrong code. Without the fix, the fingerprints of such routines would change with every compile, and their native code would never be found.

## Other upstream findings (to report)

- **A panic in `rpgrt/src/rpg/data.rs:184`** (code moved unchanged from upstream). Fixed-form MOVE with a float field (OpenRPG's test corpus, `tests/test194_fixed_cspec_err_move_float.rpgle`) panics with "zero fits" in both engines, where the RPG error path is expected. Found by `scripts/native-corpus.sh` on 2026-11-12. On IBM i the program does not compile: RNF7547, severity 30, on the MOVE (golden capture tools/golden/native10.txt n10_crt_mvf, PUB400, IBM i 7.5, 2026-11-12).
- **`rla_test::verify22_joblog_info_in_a_system_job` is intermittent.** It reads `QSYS2.JOBLOG_INFO('*')` of the process's own job, which the tests running in parallel share. In the aarch64 evidence run of 769f0d5 it failed once under `auto`: a CPF9898 row's sender was `*N` where `GLD22Q` line 20 was expected. The same run passed it under `rust`. Run alone, it passed 3 times under `auto`, 2 under `rust` and once under `interp`. Upstream test, not touched.
