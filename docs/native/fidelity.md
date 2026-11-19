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

- **CREATE TABLE in embedded SQL gives no warning.** IBM i ends `create table LIB/T (...)` in a program with SQLCODE +7905 and SQLSTATE 01567: the table was created but is not journaled (tools/golden/native69.txt n69_sql, PUB400 IBM i 7.5, 2026-11-18). as4tainr gives 0 and 00000 in both engines. NSQL1 does not check that line.
- **A null fetched into a host variable with an indicator leaves the variable as it was.** IBM i sets the indicator to -1 and the variable to its default too: a packed field that held 9 shows `.00` (tools/golden/native70.txt n70_sql, PUB400 IBM i 7.5, 2026-11-18). exec_sql.rs assign_hosts sets only the indicator, in both engines. NSQL2 does not check that line.
- **A CHAR host variable loses its trailing blanks in a string function.** On IBM i, `values upper(:key) concat '!' into :res` with KEY CHAR(10) 'two' gives `TWO       !` (same capture). as4tainr binds `CAST('two' AS char(10))`, and PostgreSQL's upper of a char gives text without the blanks: `TWO!`, in both engines. NSQL2 does not check that line.
- **A positioned change through a closed cursor gives -501, not -507.** On IBM i, `delete from T where current of c` after `close c` ends with SQLCODE -507, the cursor not open (tools/golden/native73.txt n73_sql, PUB400 IBM i 7.5, 2026-11-18). exec_sql.rs positioned_change gives -501, in both engines. NSQL5 does not check that part of its line.
- **A multiple-row INSERT of 0 rows is not an error.** On IBM i, `insert into T :n rows values(:ds)` with n = 0 ends with SQLCODE -221 (tools/golden/native74.txt n74_sql, PUB400 IBM i 7.5, 2026-11-18). exec_sql.rs sql_insert_rows answers 0, in both engines. NSQL6 does not check that part of its line.
- **EXEC SQL CREATE PROCEDURE with a compound body does not compile.** IBM i's precompiler takes `exec sql create or replace procedure P (...) language sql begin set b = a * 2; set c = c concat '!'; end;` in a free-form RPG program (tools/golden/native75.txt n75_crt_sql, PUB400 IBM i 7.5, 2026-11-18). as4tainr's RPG compiler ends the EXEC SQL at the first `;` inside BEGIN ... END ("the target of an assignment is not valid: set c = ..."). NSQL7 makes its procedures with EXECUTE IMMEDIATE instead.
- **CALL of a missing procedure gives -440, not -204.** On IBM i, `call LIB/NONE(:a)` in embedded SQL ends with SQLCODE -204, SQLSTATE 42704 (tools/golden/native75.txt n75_sql). as4tainr gives -440, 42884 in both engines (sqlpl.rs). NSQL7 does not check that part of its line.
- **IN *DTAARA in free form reads no data area.** `in *dtaara;` on IBM i reads every data area of the program (tools/golden/native80.txt n80_dtaara, PUB400 IBM i 7.5, 2026-11-18). as4tainr's free-form parser gives the name DTAARA, and the interpreter ends with "DTAARA is not defined as a data area". NDTAARA reads the two data areas one by one instead.
- **DTAARA(name) in free form names a variable on IBM i.** IBM's compiler rejects `dtaara(*var:name)` in free form (RNF3603: specify DTAARA(VARNAME)). An unquoted name there is the field that holds the data area's name; program.rs dtaara_name takes it as the data area's own name. NDTAARA uses a fixed-form DTAARA(*VAR:name), as the capture does.
- **OUT with no lock held is not an error.** On IBM i, the capture's first version ran OUT of a data area read with IN (not IN *LOCK), and the program ended in RNX9998. as4tainr writes it. The capture was changed to lock first; nothing checks the unlocked case.
- **%EDITC's floating symbol on an edited value that is all blanks.** On IBM i, `%editc(x: 'K': '+')` of zero is all blanks, with no '+' (tools/golden/native81.txt n81_editc, PUB400 IBM i 7.5, 2026-11-18). interp.rs's EDITC arm puts the symbol after the blanks, in both engines. NEDITC3 does not check that part.
- **A call through a procedure pointer set to *NULL is status 00222, not 00202.** On IBM i, the call ends with status 202, the called procedure failed, which MONITOR catches (tools/golden/native87.txt n87_procptr, PUB400 IBM i 7.5, 2026-11-18). interp.rs call_named_inner's MCH3601 gives 222 (a pointer not set), in both engines. NPROCPTR does not check the status.
- **COMMIT under COMMIT(*NONE) is not an error.** IBM i ends `commit` in a program compiled COMMIT(*NONE) with SQLCODE -7007 and SQLSTATE 51009 (same capture). as4tainr gives 0 in both engines (exec_sql.rs: with no commitment definition, nothing to do). NSQL1 does not check that line.

- **%SCANR with a start past the end and no length.** The interpreter's default length is `s.len() + 1 - start`. With a start more than one past the end of the string it subtracts past zero, which panics in a debug build. In a release build the length wraps, and the range check after it decides by accident. The native engine gives RNQ0100, as for any start or length outside the string, and as upstream's %SCANRPL fix (#23) does. No capture of IBM i for that form yet. The cases captured (tools/golden/native67.txt n67_str) agree in both engines.

Each was observed on the interpreter and confirmed on a real IBM i: PUB400, IBM i 7.5, 2026-11-12, with `tools/golden/native1.txt` (the captures are in `target/ibmi/golden`, not committed). The backend reproduces each one, as section 4 requires, until upstream fixes it. They were reported upstream on 2026-11-12, after a clean build of upstream main (f20febb) reproduced each one: kyleturley-cyber/AS4tainr issues #3 (nested MONITOR), #4 (OVERLAY *NEXT), #5 (CONST parameters), #6 (%SUBST and %SCAN start), #7 (constant %SUBST checks) and #8 (MOVE with a float field).

| Defect | IBM i | as4tainr | Source |
| --- | --- | --- | --- |
| Assigning to a CONST parameter inside the procedure | Does not compile: RNF5346, severity 30 | Compiles and runs; the assignment takes effect | Golden capture tools/golden/native1.txt n1_crt_cst (PUB400, IBM i 7.5, 2026-11-12). IBM Docs, "CONST{(constant)}", IBM i 7.5, https://www.ibm.com/docs/en/ssw_ibm_i_75/rzasd/dconst.htm, read 2026-11-11 |
| Passing a CONST parameter on to a procedure that changes its parameter | Does not compile: RNF7542, severity 30 | Compiles and runs | Golden capture tools/golden/native1.txt n1_crt_fwd (PUB400, IBM i 7.5, 2026-11-12) |
| `OVERLAY(field : *NEXT)` | Family 1-3, Sequence 4-9, Language 10; the data structure is 10 bytes | Sequence and Language go after the whole of PartNumber; the data structure is 17 bytes | Golden capture tools/golden/native1.txt n1_ovl (PUB400, IBM i 7.5, 2026-11-12). IBM Docs, "OVERLAY(name{:start_pos \| *NEXT})", IBM i 7.5, https://www.ibm.com/docs/en/ssw_ibm_i_75/rzasd/doverly.htm, read 2026-11-11 |
| %SUBST with constant arguments out of range (start 11 or 12 of a 10-character field, or a length past the end) | Does not compile: RNF0364 and RNF0365, severity 20 | No such check (no RNF0364 or RNF0365 in core/src); the statement compiles and is checked when it runs | Golden capture tools/golden/native7.txt n7_crt_const, source gn7const.rpgle (PUB400, IBM i 7.5, 2026-11-12) |
| `%SCAN(pat : v : 0)` of an empty varying field | 0: an empty string gives 0 whatever the start (start 0, -5, 2 and 99) | Status 00100 for a start below 1 since upstream #9; the follow-up pull request is open | Golden capture tools/golden/native22.txt n22_scan (PUB400, IBM i 7.5, 2026-11-13) |
| A numeric field compared with `*HIVAL` (or `*LOVAL`) after `z = *hival` (a 3-digit zoned field) | Equal: the field holds the highest value its type can, which is `*HIVAL` for it | Not equal: `data::compare` gives `*HIVAL` the value +infinity and `*LOVAL` -infinity, whatever the other operand's type | Golden capture tools/golden/native28.txt n28_fig, case FIG CMP HIVAL (PUB400, IBM i 7.5, 2026-11-14); as4tainr at 0b9d585 |
| `v = *blanks` with `v` a VARCHAR(5) holding 'XY' | Five blanks: the current length becomes the maximum, 5 | An empty string, length 0 (`data::store` of `Fig::Blanks` into a varying field) | Golden capture tools/golden/native28.txt n28_fig, case FIG VAR (PUB400, IBM i 7.5, 2026-11-14); as4tainr at 0b9d585 |
| `%REPLACE(rep : s : start)` with `start` outside 1 to the length of `s` plus 1 (0, or 9 for a 7-character string) | Status 00100 | No error: a start of 0 replaces from the first character, a start past the end appends | Golden capture tools/golden/native30.txt n30_repl, cases REPL PAST and REPL ZERO (PUB400, IBM i 7.5, 2026-11-14); `interp.rs`'s `"REPLACE"` arm by reading its code |
| `%SIZE` of a numeric named constant (`dcl-c KNUM 12345`) | Its digits as written, its size as zoned: 12345 is 5, 12.5 is 3, -5 is 1, 100.00 is 5 | Its size as packed decimal: 12345 is 3 (`interp.rs` "SIZE": digits / 2 + 1) | Golden captures tools/golden/native32.txt n32_size and native33.txt n33_ksize (PUB400, IBM i 7.5, 2026-11-14); as4tainr at 0b9d585 |
| `%SCANRPL(pat : rep : s : start)` with `start` one past the end (8 for a 7-character string) | Status 00100 | Accepted: the string unchanged (`bifs.rs` "SCANRPL": the default length is 0) | Golden capture tools/golden/native36.txt n36_srpl, case SRPL END (PUB400, IBM i 7.5, 2026-11-14); as4tainr at 0b9d585 |
| `%SCANRPL` with `start` more than one past the end (9 for a 7-character string), no length | Status 00100 | A panic, "attempt to subtract with overflow" at bifs.rs:573, in a debug build. A release build wraps around and gives RNQ0100, as the native engine does. OpenRPG's test396_scanrpl_portion (start 20 of a 9-character string) hits it in native-corpus.sh | Golden capture tools/golden/native36.txt n36_srpl, case SRPL PAST (PUB400, IBM i 7.5, 2026-11-14); as4tainr at 0b9d585 |
| `%EDITW` with a floating currency symbol (`$` just before the `0`), e.g. `p` -1234.56 in a 9,2 field with `'   ,   , $0.  '`, or 1000.00 in a 6,2 field with `'  ,  $0.  '` | `     $1,234.56`; `  $1000.00`: the `$` position takes a digit, the first digit position none, and the `$` prints just before the first character printed | `     $12,34.56`; ` $1,000.00` (`interp.rs` `edit_word`, whose unit test `edit_words` expects the second). Reported to upstream as an issue, no PR: the user's decision, 2026-11-14, because the fix changes that test's expected value | Golden captures tools/golden/native37.txt n37_editw, case EW DOLLAR, and tools/golden/native39.txt n39_a, n39_b, n39_c (PUB400, IBM i 7.5, 2026-11-14); as4tainr at 0b9d585 |
| `%CHAR(%MAX(a:b))` and `%CHAR(%MIN(a:b))` of numeric arguments with different decimal places, e.g. `a` packed(5:2) 1.50 and `b` packed(7:3) 2.125, or `a` and `d` zoned(3:0) 2 | `%MIN(a:b)` is `1.500`, `%MAX(a:d)` is `2.00`: the result has the arguments' greatest number of decimal places | `1.50`, `2`: the chosen argument keeps its own scale (`bifs.rs` "MAX" \| "MIN"). Assigned to a field, both give the same value | Golden capture tools/golden/native42.txt n42_mm, cases CHAR MIN AB, CHAR MAX AD, CHAR MAX EA (PUB400, IBM i 7.5, 2026-11-14); as4tainr at 0b9d585 |
| A subprocedure that runs EXSR on a subroutine of the main procedure | Does not compile: RNF7030 and RNF7018 at severity 30, compilation stopped (RNS9308, RNS9310) | Compiles and runs it, under the subprocedure's frame (`interp.rs` `Ins::ExSr` looks in the main procedure's subroutines after the procedure's own). The native engine refuses such a routine (ADR 0007, S2.37): under `auto` the interpreter runs it, under `rust` it is an error | Golden capture tools/golden/native45.txt n45_crt_sr (PUB400, IBM i 7.5, 2026-11-15); as4tainr at dd47e28 |
| An indicator used in a condition but set nowhere in the program (`if *in60;`) | Does not compile: RNF7030 and RNF7503 at severity 30 | Compiles; the indicator is off. The native engine does the same as the interpreter | Golden capture tools/golden/native46.txt n46_crt_ind, first version of gn46ind.rpgle (PUB400, IBM i 7.5, 2026-11-15); as4tainr at dd47e28 |
| An expression nested more than 100 levels (`x = ((( ... 150 levels ... 1 )));`) | Not compiled: "Levels of nesting in expressions: Maximum of 100" (ILE RPG Reference, Appendix A. RPG IV Restrictions, IBM i 7.5, https://www.ibm.com/docs/ssw_ibm_i_75/rzasd/rpgrest.htm, read 2026-11-15); RNF0640 at severity 30 for 101 levels of parentheses, and 100 levels compile (golden capture tools/golden/native47.txt n47_crt_n100, n47_crt_n101, PUB400, IBM i 7.5, 2026-11-15) | Compiles and runs it. With 20 000 levels the parser overflows its stack and the process aborts (rc 134), even with the binary's 64 MiB stacks. The native engine refuses an instruction nested more than 100 levels (S2.41) | IBM documentation; as4tainr at dd47e28 (upstream main, built in the up22 worktree) |
| *NULL plus a number (`p = *null; p = p + 1;`) | Gives a pointer with no error: the statement after it runs (golden capture tools/golden/native50.txt n50_pa, PUB400, IBM i 7.5, 2026-11-16) | MCH3601 (status 222), ptrs.rs ptr_arith. The native engine does the same (S2.49, `rpgrt::native::ptr_move`); NPNULL in native_diff.rs checks the engines agree | PUB400 capture; as4tainr at dd47e28 (upstream main) |
| Fixed-form ALLOC and REALLOC (`C   ALLOC   5   R`, `C   REALLOC   N   R`) | Compiled and run: the storage is allocated, and resized with its bytes kept (golden capture tools/golden/native51.txt n51_heap, PUB400, IBM i 7.5, 2026-11-16) | The run fails with "operation ALLOC is not supported yet": parse.rs does not take the opcodes (ptrs.rs storage_op has the code for them). The native engine does not lower them either (S2.50) | PUB400 capture; as4tainr at dd47e28 (upstream main) |
| Date and time literals with CTL-OPT DATFMT/TIMFMT (`ctl-opt datfmt(*eur)`, `inz(d'16.11.2026')`) | Literals are in the DATFMT/TIMFMT form: *EUR and *HMS literals compile, *ISO ones get RNF0305/RNF0306 (golden capture tools/golden/native53.txt n53_crt_date, PUB400, IBM i 7.5, 2026-11-16, both forms) | Takes *ISO literals only: "D'16.11.2026' is not a valid date literal in *ISO form". The native engine compiles what the interpreter compiles | PUB400 capture; as4tainr at dd47e28 (upstream main) |
| *LOVAL and *HIVAL assigned to a date (`iso = *loval;`, `usa = *hival;`) | 0001-01-01 and 9999-12-31 (12/31/9999 in *USA) (golden capture tools/golden/native53.txt n53_date) | The field is filled with x'00' or x'FF' bytes (data::fill_fig). The native engine does the same (S2.52); NDATEFIG in native_diff.rs checks the engines agree | PUB400 capture; as4tainr at dd47e28 (upstream main) |
| %DATE and %TIME of a character or numeric value with no format, under CTL-OPT DATFMT(*MDY) (`%date('11/17/26')`, `%date(m)` of a 6-digit number) | The value is taken in *ISO form, not DATFMT's: RNF7510 "too small to contain a complete *ISO date" (golden capture tools/golden/native55.txt n55_crt_dft, PUB400, IBM i 7.5, 2026-11-16) | Takes it in the DATFMT (TIMFMT) form (bifs.rs bif_more: `fmt.unwrap_or_else(\|\| dft.code())`), so it compiles and runs. The native engine does the same (S2.54) | PUB400 capture; as4tainr at dd47e28 (upstream main) |
| %DATE(*SYS) | Not valid: RNF0605 "The first parameter *SYS for %DATE is not valid" (n55_crt_dft) | Takes it as the current date, as %DATE() and %DATE(*JOB). The native engine does the same | PUB400 capture; as4tainr at dd47e28 (upstream main) |
| A duration on the left of + (`d2 = %years(2) + d1;`) | Not valid: RNF7421 "Operands are not compatible with the type of operator" (golden capture tools/golden/native56.txt n56_crt_first, PUB400, IBM i 7.5, 2026-11-16) | Adds the duration (bifs.rs date_arith takes it on either side of +). The native engine does the same (S2.55); NTDURF in native_diff.rs checks the engines agree | PUB400 capture; as4tainr at dd47e28 (upstream main) |
| A date past 9999-12-31 by a duration (`d2 = d1 + %days(1)` with d1 9999-12-31), under MONITOR | Status 113 (golden capture tools/golden/native56.txt n56_dur, PUB400, IBM i 7.5, 2026-11-16) | Status 112 (the dates module's error). The native engine does the same (S2.55); NTDURF checks the engines agree | PUB400 capture; as4tainr at dd47e28 (upstream main) |
| A DIM(*AUTO:5) array written at index 6 (a variable index past the dimension), and `%ELEM(arr) = 5` for DIM(*VAR:4) | Status 00124 | Status 00121 (RNQ0121, `interp.rs` assign_value). The native engine follows the interpreter | Golden capture tools/golden/native47b.txt n47_var, cases AUTO6 and VARSET5 (PUB400, IBM i 7.5, 2026-11-15); as4tainr at dd47e28 |
| A constant index past an array's dimension (`au(6) = 1` for DIM(*AUTO:5)), and `%ELEM(va) = 5` with a constant for DIM(*VAR:4) | Not compiled: RNF0530 (severity 20) and RNF7563 (severity 30) | Compiled; status 00121 at run time | Golden capture tools/golden/native47.txt n47_crt_var, first version of gn47var.rpgle (PUB400, IBM i 7.5, 2026-11-15); as4tainr at dd47e28 |

- **READPE right after READE reached end of file reads a record.** After `reade kOrd GN58PF` returns end of file (the last record with key 1 read), IBM i's `readpe kOrd GN58PF` is end of file too (tools/golden/native58.txt n58_r, "RPE1 EOF", PUB400 IBM i 7.5, 2026-11-17). as4tainr's `rla` returns the key's last record again ("RPE1 3"), under both engines: the position after an end of file is not IBM i's. The native unit NFREAD leaves that READPE out; to be fixed upstream in `rla` with this capture.

## Interpreter defects fixed upstream

The backend follows each fix since the rebase on upstream dd47e28 (2026-11-13). The native unit tests check the captured values again.

- **Nested MONITOR** (upstream #3, fixed by #10): an error the inner group does not take goes to the enclosing group. `native_rt.rs` `settle` follows the new routing of `run_code_at`.
- **%SUBST and %SCAN starts outside the string** (upstream #6, fixed by #9): status 00100. `rpgrt::native::subst_start` and `scan` follow the interpreter. `NSUB.rpgle` and `NSCAN.rpgle` have their cases again.
- **MOVE with a float field** (upstream #8, fixed by #13): the compiler now rejects it (RNF7547). Generated code is not involved.

## Settled by capture

- **Two reference parameters that are one variable:** they share its storage. After `p1 = 1; p2 = 2`, both are 2, and so is the caller's variable. as4tainr does the same. The capture is tools/golden/native1.txt n1_alias (PUB400, IBM i 7.5, 2026-11-12); `core/tests/native/ALIAS.rpgle` tests it.
- **An unmonitored division by zero or overflow, with the job's inquiry messages answered by default** (`CHGJOB INQMSGRPY(*DFT)`): the only exception message the caller receives is CPF9999. The captures are n1_div and n1_ovf (PUB400, IBM i 7.5, 2026-11-12). Not yet compared with as4tainr, which signals RNX escape messages (CLAUDE.md conventions). It is to be checked with a CL caller under `*DFT`.

## No public source yet (parity only)

- The text DSPLY shows for a numeric field.

## Notes for stage 2

- **Buffer numbers of external fields are not stable across compiles.** Two compiles of the same source, against files made again with the same DDS, gave CUSTLIST's scalar fields CUSNO and a zero packed field each other's buffer numbers. The values were the same. Cause, confirmed: `bind` (core/src/rpg/program.rs:2682) turns file fields that are not defined otherwise into standalone fields by iterating `file_fields`, a `std::collections::HashMap`, whose iteration order is random per instance. The catalog query itself is ordered (`ORDER BY seq`). A one-line upstream fix, sorting `names` before the loop, would make the numbering stable. It is outside the touch points and needs the user's approval when stage 2 needs it. Stage 1 does not depend on buffer numbers. The level-1 diff matches buffers by the names of what is in them. A lowering that uses storage layout must put that layout into the fingerprint (ADR 0002), so a different numbering means different code, never wrong code. Without the fix, the fingerprints of such routines would change with every compile, and their native code would never be found.

## Other upstream findings (to report)

- **%OCCUR set to a literal out of range compiles.** IBM i's compiler refuses `%occur(rec) = 4` for a data structure of OCCURS(3): RNF7571 severity 30 (tools/golden/native64.txt n64_crt_occ, first run, PUB400 IBM i 7.5, 2026-11-17). as4tainr's compiler accepts it, and both engines raise RNQ0122 when it runs. The check belongs in the front end, upstream.

- **A key literal of another length than the key field compiles.** IBM i's compiler refuses `setll 1 GN58PF` when the first key field is zoned 3,0: RNF7078 severity 30, "Length of Factor 1 is not the same as the first key field" (tools/golden/native58.txt n58_crt_r, first run, PUB400 IBM i 7.5, 2026-11-17). as4tainr's compiler accepts it, and both engines then search with the literal's value. The native engine reproduces the interpreter; the check belongs in the front end, upstream.

- **A panic in `rpgrt/src/rpg/data.rs:184`** (code moved unchanged from upstream). Fixed-form MOVE with a float field (OpenRPG's test corpus, `tests/test194_fixed_cspec_err_move_float.rpgle`) panics with "zero fits" in both engines, where the RPG error path is expected. Found by `scripts/native-corpus.sh` on 2026-11-12. On IBM i the program does not compile: RNF7547, severity 30, on the MOVE (golden capture tools/golden/native10.txt n10_crt_mvf, PUB400, IBM i 7.5, 2026-11-12).
- **`rla_test::verify22_joblog_info_in_a_system_job` is intermittent.** It reads `QSYS2.JOBLOG_INFO('*')` of the process's own job, which the tests running in parallel share. In the aarch64 evidence run of 769f0d5 it failed once under `auto`: a CPF9898 row's sender was `*N` where `GLD22Q` line 20 was expected. The same run passed it under `rust`. Run alone, it passed 3 times under `auto`, 2 under `rust` and once under `interp`. Upstream test, not touched.

## Numbers past 2^64 in length, start and count operands (found 2026-11-18)

When a number gives a length, start position or count, the two engines differ only for values of 2^64 and up:
- The interpreter casts it to `usize` with `as`, which keeps the low 64 bits:
  - ops.rs string_op: SCAN's length and start, CAT's blanks;
  - interp.rs bif's `%STR` maximum.
  So 18446744073709551617 becomes 1.
- The native runtime converts with `try_from`, as section 5 of CLAUDE.native.md requires:
  - `str_op_n`: a SCAN length or start that does not fit is the operation's error (status 100), and CAT's blanks become 0;
  - `str_at`: a `%STR` maximum that does not fit is no limit.

**Examples.** SCAN `'A':18446744073709551617` of `'A'` finds 1 under the interpreter and is an error natively. CAT `'A':18446744073709551617` `'B'` gives `A B` under the interpreter and `AB` natively.

IBM i rejects such a length or start: the operation's error, as for a length past the string. No capture has been made of the CAT blanks or the `%STR` maximum yet.

These were found by an independent review (codex, read-only) of native_rt.rs against the interpreter. They are left as they are until a PUB400 capture says which behavior is right. Fields of such a size are not seen in practice.

## EXCEPT and a printer's INFDS (found 2026-11-18)

An independent review (codex, read-only) of the printer functions of native_rt.rs against the interpreter found two differences between the engines. Both were made to follow the interpreter:
- **A varying field printed with blank after.** The native runtime set only its length. It now writes the type's initial image over all of the field's storage, as interp.rs print_line's CLEAR does.
- **An edit word named by a constant, in a procedure that has its own constant of that name.** The native runtime took the module's constant. Lowering now leaves such an EXCEPT to the interpreter, which takes the procedure's.

The review also found that a printer file with an INFDS was lowered, although errors.rs file_op's infds_update is not written again natively. Its WRITE, OPEN, CLOSE and FEOD now stay with the interpreter (lower.rs `has_infds`), as database reads already did.

PUB400 then showed the interpreter itself wrong in five places (tools/golden/native92: n92_except and n92_infds, IBM i 7.5, 2026-11-18):

| Case | IBM i | Interpreter |
|---|---|---|
| INZ('ABC') of a varchar(5) subfield | blanks after the value (`ABC  `) | x'00' bytes (`ABC00`) |
| Blank after of a varying field | length 0, the bytes are kept (`ABC  `) | the initial image, x'00' bytes |
| A varying field in output specifications | its declared length: ABC in 6-8 for end position 10 | its current length: ABC in 8-10 |
| An edit word constant in a procedure with its own constant of that name | the module's | the procedure's |
| A printer's INFDS | *RECORD after WRITE is the format (HDR, DTL); the line after CLOSE is 3 | *RECORD is blank; the line after CLOSE is 1 |

The native units NEXCPT2 and NPRTINF check only what IBM i and the interpreter both do, and leave out these parts with a comment. These are upstream interpreter defects, held for Kyle with the captures.

Two more of the review's findings are numbers out of range and are left as they are:
- a spooled file number or page at i32::MAX: the interpreter's `+ 1` overflows, the native runtime saturates;
- PAGE named with blank after: the two engines give different error texts before printing.

**Also from the review of 2026-11-19, left as it is.** MOVEA with an array index of -1 gives RNQ0121 in both engines. The interpreter's text shows the index converted to usize (18446744073709551615); the native runtime's shows -1. Only the message text differs.

## EXECUTE ... USING before the statement is prepared (found 2026-11-19)

The codex audit of 2026-11-19 (f1) found that the two engines read EXECUTE's USING host variables at different times; it is checked against the code.
- **Interpreter.** exec_sql.rs sql_statement's EXECUTE looks up the prepared statement first, so SQL0518 (SQLCODE -518, SQLSTATE 07003) comes before the values are read.
- **Native code.** The generated code reads the USING values (sql_inputs) before `exec_sql_dyn` looks the statement up.

**What IBM i does** (tools/golden/native97 n97_execute, PUB400, IBM i 7.5, 2026-11-19). A statement never prepared, and a USING packed field holding x'FFFF', gives a decimal data error, status 907, which MONITOR catches. The host variable is read first, as the native code does. The interpreter's -518 is the defect: an upstream one, held for Kyle with the capture.

The native code is left as it is: it already does what IBM i does. Making it follow the interpreter would make both engines wrong, with no test of IBM's value possible. The difference shows only for a statement that was not prepared and a USING value that cannot be read.

## A pointer minus a pointer to another field (found 2026-11-19)

Source: tools/golden/native99 n99_ptrdiff (PUB400, IBM i 7.5, 2026-11-19).

On IBM i, two pointers into different fields of the same program can be subtracted: the program's static storage is one space, so `%addr(other) - %addr(buf)` gives the distance between the two fields, and the capture's MONITOR is not reached.

as4tainr keeps each field in its own buffer. ptrs.rs ptr_arith fails a subtraction of pointers to different buffers, which MONITOR sees as status 9999.

Both engines do this; native code uses `rpgrt::native::ptr_diff`, which follows ptr_arith. This is an upstream interpreter defect, held for Kyle with the capture. The unit NPTRDIF leaves that part out.
