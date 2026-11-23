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
- **A multiple-row INSERT of 0 rows is not an error.** On IBM i, `insert into T :n rows values(:ds)` with n = 0 ends with SQLCODE -221 (tools/golden/native74.txt n74_sql, PUB400 IBM i 7.5, 2026-11-18). exec_sql.rs sql_insert_rows answers 0, in both engines. NSQL6 does not check that part of its line. Fixed in the fork's interpreter (`fix/sql-insert-zero-rows`); the native engine follows it (native_rt.rs sql_insert_rows_n): rla_test `insert_of_zero_rows_is_sqlcode_221` passes under rust, every statement native (2026-11-23).
- **EXEC SQL CREATE PROCEDURE with a compound body does not compile.** IBM i's precompiler takes `exec sql create or replace procedure P (...) language sql begin set b = a * 2; set c = c concat '!'; end;` in a free-form RPG program (tools/golden/native75.txt n75_crt_sql, PUB400 IBM i 7.5, 2026-11-18). as4tainr's RPG compiler ends the EXEC SQL at the first `;` inside BEGIN ... END ("the target of an assignment is not valid: set c = ..."). NSQL7 makes its procedures with EXECUTE IMMEDIATE instead.
- **CALL of a missing procedure gives -440, not -204.** On IBM i, `call LIB/NONE(:a)` in embedded SQL ends with SQLCODE -204, SQLSTATE 42704 (tools/golden/native75.txt n75_sql). as4tainr gives -440, 42884 in both engines (sqlpl.rs). NSQL7 does not check that part of its line.
- **IN *DTAARA in free form reads no data area.** `in *dtaara;` on IBM i reads every data area of the program (tools/golden/native80.txt n80_dtaara, PUB400 IBM i 7.5, 2026-11-18). as4tainr's free-form parser gives the name DTAARA, and the interpreter ends with "DTAARA is not defined as a data area". NDTAARA reads the two data areas one by one instead.
- **DTAARA(name) in free form names a variable on IBM i.** IBM's compiler rejects `dtaara(*var:name)` in free form (RNF3603: specify DTAARA(VARNAME)). An unquoted name there is the field that holds the data area's name; program.rs dtaara_name takes it as the data area's own name. NDTAARA uses a fixed-form DTAARA(*VAR:name), as the capture does.
- **OUT with no lock held is not an error.** On IBM i, the capture's first version ran OUT of a data area read with IN (not IN *LOCK), and the program ended in RNX9998. as4tainr writes it. The capture was changed to lock first; nothing checks the unlocked case.
- **%EDITC's floating symbol on an edited value that is all blanks.** On IBM i, `%editc(x: 'K': '+')` of zero is all blanks, with no '+' (tools/golden/native81.txt n81_editc, PUB400 IBM i 7.5, 2026-11-18). interp.rs's EDITC arm puts the symbol after the blanks, in both engines. NEDITC3 does not check that part. Fixed in the fork's interpreter (`fix/editc-blank-floating`); the native engine follows it (rpgrt::native::editc_third): rla_test `editc_floating_symbol_on_an_all_blank_value` passes under rust (2026-11-23).
- **A call through a procedure pointer set to *NULL is status 00222, not 00202.** On IBM i, the call ends with status 202, the called procedure failed, which MONITOR catches (tools/golden/native87.txt n87_procptr, PUB400 IBM i 7.5, 2026-11-18). interp.rs call_named_inner's MCH3601 gives 222 (a pointer not set), in both engines. NPROCPTR does not check the status. Fixed in the fork's interpreter (`fix/null-procptr-status`, the MCH3601 raised as a called procedure's error); the native engine follows it (native_rt.rs procptr_target_n: the same error with errors::At, called): rla_test `a_call_through_a_null_procedure_pointer_is_status_202` passes under rust, every statement native (2026-11-23).
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
| *NULL plus a number (`p = *null; p = p + 1;`) | Gives a pointer with no error: the statement after it runs (golden capture tools/golden/native50.txt n50_pa, PUB400, IBM i 7.5, 2026-11-16) | MCH3601 (status 222), ptrs.rs ptr_arith. The native engine does the same (S2.49, `rpgrt::native::ptr_move`); NPNULL in native_diff.rs checks the engines agree. Fixed in the fork's interpreter (`fix/null-plus-number`, KIND_NULL_OFF); the native engine follows it (ptr_move, ptr_diff, addr_n): rla_test `null_plus_a_number_is_a_pointer` passes under rust (2026-11-23) | PUB400 capture; as4tainr at dd47e28 (upstream main) |
| Fixed-form ALLOC and REALLOC (`C   ALLOC   5   R`, `C   REALLOC   N   R`) | Compiled and run: the storage is allocated, and resized with its bytes kept (golden capture tools/golden/native51.txt n51_heap, PUB400, IBM i 7.5, 2026-11-16) | The run fails with "operation ALLOC is not supported yet": parse.rs does not take the opcodes (ptrs.rs storage_op has the code for them). The native engine does not lower them either (S2.50). Fixed in the fork's interpreter (`fix/fixed-alloc`); the native engine lowers them as EvalPtr of %ALLOC and %REALLOC, and CAT of a based character field (lower.rs cat_chars): rla_test `fixed_form_alloc_and_realloc_run` passes under rust, every statement native (2026-11-23) | PUB400 capture; as4tainr at dd47e28 (upstream main) |
| Date and time literals with CTL-OPT DATFMT/TIMFMT (`ctl-opt datfmt(*eur)`, `inz(d'16.11.2026')`) | Literals are in the DATFMT/TIMFMT form: *EUR and *HMS literals compile, *ISO ones get RNF0305/RNF0306 (golden capture tools/golden/native53.txt n53_crt_date, PUB400, IBM i 7.5, 2026-11-16, both forms) | Takes *ISO literals only: "D'16.11.2026' is not a valid date literal in *ISO form". The native engine compiles what the interpreter compiles | PUB400 capture; as4tainr at dd47e28 (upstream main) |
| *LOVAL and *HIVAL assigned to a date (`iso = *loval;`, `usa = *hival;`) | 0001-01-01 and 9999-12-31 (12/31/9999 in *USA) (golden capture tools/golden/native53.txt n53_date) | The field is filled with x'00' or x'FF' bytes (data::fill_fig). The native engine does the same (S2.52); NDATEFIG in native_diff.rs checks the engines agree | PUB400 capture; as4tainr at dd47e28 (upstream main) |
| %DATE and %TIME of a character or numeric value with no format, under CTL-OPT DATFMT(*MDY) (`%date('11/17/26')`, `%date(m)` of a 6-digit number) | The value is taken in *ISO form, not DATFMT's: RNF7510 "too small to contain a complete *ISO date" (golden capture tools/golden/native55.txt n55_crt_dft, PUB400, IBM i 7.5, 2026-11-16) | Takes it in the DATFMT (TIMFMT) form (bifs.rs bif_more: `fmt.unwrap_or_else(\|\| dft.code())`), so it compiles and runs. The native engine does the same (S2.54) | PUB400 capture; as4tainr at dd47e28 (upstream main) |
| %DATE(*SYS) | Not valid: RNF0605 "The first parameter *SYS for %DATE is not valid" (n55_crt_dft) | Takes it as the current date, as %DATE() and %DATE(*JOB). The native engine does the same | PUB400 capture; as4tainr at dd47e28 (upstream main) |
| A duration on the left of + (`d2 = %years(2) + d1;`) | Not valid: RNF7421 "Operands are not compatible with the type of operator" (golden capture tools/golden/native56.txt n56_crt_first, PUB400, IBM i 7.5, 2026-11-16) | Adds the duration (bifs.rs date_arith takes it on either side of +). The native engine does the same (S2.55); NTDURF in native_diff.rs checks the engines agree | PUB400 capture; as4tainr at dd47e28 (upstream main) |
| A date past 9999-12-31 by a duration (`d2 = d1 + %days(1)` with d1 9999-12-31), under MONITOR | Status 113 (golden capture tools/golden/native56.txt n56_dur, PUB400, IBM i 7.5, 2026-11-16) | Status 112 (the dates module's error). The native engine does the same (S2.55); NTDURF checks the engines agree | PUB400 capture; as4tainr at dd47e28 (upstream main) |
| A DIM(*AUTO:5) array written at index 6 (a variable index past the dimension), and `%ELEM(arr) = 5` for DIM(*VAR:4) | Status 00124 | Status 00121 (RNQ0121, `interp.rs` assign_value). Fixed in the fork's interpreter (`fix/varying-dim-124`); the native engine follows it (rpgrt::native::elem_grow, set_elems): rla_test `varying_dimension_arrays_past_their_dimension_are_status_124` passes under rust (2026-11-23) | Golden capture tools/golden/native47b.txt n47_var, cases AUTO6 and VARSET5 (PUB400, IBM i 7.5, 2026-11-15); as4tainr at dd47e28 |
| A constant index past an array's dimension (`au(6) = 1` for DIM(*AUTO:5)), and `%ELEM(va) = 5` with a constant for DIM(*VAR:4) | Not compiled: RNF0530 (severity 20) and RNF7563 (severity 30) | Compiled; status 00121 at run time | Golden capture tools/golden/native47.txt n47_crt_var, first version of gn47var.rpgle (PUB400, IBM i 7.5, 2026-11-15); as4tainr at dd47e28 |

- **READPE right after READE reached end of file reads a record.** After `reade kOrd GN58PF` returns end of file (the last record with key 1 read), IBM i's `readpe kOrd GN58PF` is end of file too (tools/golden/native58.txt n58_r, "RPE1 EOF", PUB400 IBM i 7.5, 2026-11-17). as4tainr's `rla` returns the key's last record again ("RPE1 3"), under both engines: the position after an end of file is not IBM i's. The native unit NFREAD leaves that READPE out; to be fixed upstream in `rla` with this capture.

## Interpreter defects fixed upstream

The backend follows each fix since the rebase on upstream dd47e28 (2026-11-13). The native unit tests check the captured values again.

- **Nested MONITOR** (upstream #3, fixed by #10): an error the inner group does not take goes to the enclosing group. `native_rt.rs` `settle` follows the new routing of `run_code_at`.
- **%SUBST and %SCAN starts outside the string** (upstream #6, fixed by #9): status 00100. `rpgrt::native::subst_start` and `scan` follow the interpreter. `NSUB.rpgle` and `NSCAN.rpgle` have their cases again.
- **MOVE with a float field** (upstream #8, fixed by #13): the compiler now rejects it (RNF7547). Generated code is not involved.


## Interpreter defects fixed in the fork (phase 2, not yet upstream)

Fixed on local branches `fix/*` from upstream dd47e28, one topic each, with a test whose expected value comes from the PUB400 capture or IBM documentation named in it; each test fails without its fix. The 37 branches pass together the upstream suite, `scripts/demo.sh` and clippy (combined validations p2-f and p2-g on jsr-test-2, 2026-11-23). Not pushed, not yet offered upstream; the native engine follows each fix after the rebase on them. Where a fix changes an upstream test's expectation (the user's decision, 2026-11-22 and 2026-11-23), the commit says so.

| Branch | IBM i behavior now reproduced |
| --- | --- |
| `fix/char-temporal-expr-iso` | %CHAR of a date or time expression with no format is *ISO |
| `fix/compile-checks` | %OCCUR set to a literal outside the occurrences does not compile (RNF7571) |
| `fix/compile-checks` | A numeric operand of %CONCAT does not compile (RNF0203) |
| `fix/compile-checks` | A time plus a date does not compile (RNF7421) |
| `fix/compile-checks` | A FOR limit that is not whole does not compile (RNF7569) |
| `fix/compile-checks` | A duration before the date it changes does not compile (RNF7421) |
| `fix/compile-checks` | %DATE(*SYS) does not compile (RNF0605) |
| `fix/compile-checks` | EXSR in a subprocedure of the main procedure's subroutine does not compile |
| `fix/compile-checks` | An expression nested more than 100 levels does not compile (RNF0640) |
| `fix/compile-checks` | %DATE of a value too short for an *ISO date does not compile (RNF7510) |
| `fix/concat-blank` | %CONCAT(*BLANK) puts one blank between the values |
| `fix/date-iso-default` | %DATE with no format takes its value in *ISO form, whatever DATFMT (the native engine follows it: lower.rs temporal_bif; rla_test `date_with_no_format_takes_iso_whatever_datfmt` passes under rust, 2026-11-23) |
| `fix/date-loval-hival` | *LOVAL and *HIVAL assigned to a date are its first and last dates |
| `fix/date-overflow-113` | A date past 9999-12-31 by a duration is status 00113 |
| `fix/dec-blank-date` | %DEC of a date field that holds no date is status 00112 |
| `fix/editc-blank-floating` | %EDITC's floating symbol: none on an edited value that is all blanks |
| `fix/extfile-program-described-output` | output to a program-described file named by EXTFILE |
| `fix/extpgm-var-case` | EXTPGM of a variable calls the program as the variable spells it |
| `fix/fixed-alloc` | fixed-form ALLOC and REALLOC |
| `fix/float4-text` | A 4-byte float's text has eight digits and a two-digit exponent |
| `fix/in-dtaara-free` | IN *DTAARA in free form reads every data area of the program |
| `fix/int-of-float` | %INT, %INTH, %DEC and %DECH of a float a procedure returns |
| `fix/list-array-rest` | An array assigned %SPLIT or %LIST keeps the elements after the values |
| `fix/list-array-rest` | a whole array assigned another array |
| `fix/max-min-places` | %MAX and %MIN give the arguments' greatest number of decimal places |
| `fix/null-procptr-status` | A call through a procedure pointer set to *NULL is status 00202 |
| `fix/open-already-open` | OPEN(E) of an open file and READ(E) of a closed one set their file status |
| `fix/out-unlocked-dtaara` | OUT of a data area that is not locked is status 00412 |
| `fix/overload-no-end-pr` | An OVERLOAD prototype has no END-PR |
| `fix/qualified-format-io` | WRITE and UPDATE of a qualified file's record format |
| `fix/quotient-compare` | A quotient compared keeps the decimal places of 63 digits |
| `fix/read-after-eof` | READP and READPE right after READE reached end of file are end of file |
| `fix/read-not-open-1211` | an operation on a file that is not open honors E and the error indicator |
| `fix/sql-call-missing-204` | CALL of a procedure that does not exist is SQL0204 |
| `fix/sql-char-host-blanks` | SQL: UPPER and LOWER of a CHAR value keep its length |
| `fix/sql-closed-cursor-507` | UPDATE and DELETE WHERE CURRENT OF a closed cursor is SQLCODE -507 |
| `fix/sql-commit-none-7007` | COMMIT in a program without commitment control is SQLCODE -7007 |
| `fix/sql-compound-create` | EXEC SQL CREATE PROCEDURE with a compound body is one statement |
| `fix/sql-create-table-7905` | Embedded CREATE TABLE in a library without QSQJRN warns +7905 |
| `fix/sql-insert-zero-rows` | A multiple-row INSERT of 0 rows is SQLCODE -221 |
| `fix/sql-null-indicator-default` | A null fetched with an indicator empties the host variable |
| `fix/sql-primary-key-keyed` | an SQL table declared keyed takes its primary key as its keys |
| `fix/sqrt-places` | %SQRT keeps 30 digits, truncated |
| `fix/test-needs-e` | TEST(D), TEST(T) and TEST(Z) need E or an error indicator (RNF5056) |
| `fix/varying-dim-124` | A varying-dimension array past its dimension is status 00124 |
| `fix/xml-handler-nonzero` | XML-SAX and XML-INTO %HANDLER: a handler that returns non-zero is status 00351 |

Still open, each waiting for a PUB400 capture (the earlier captures kept only the final version of their program) or with no public source:
- READ (not READP) right after READE reached end of file (native58; upstream's `setll_partial_key_then_reade_reads_one_order` expects a record).
- An indicator used but set nowhere (RNF7503, native46); a constant index past an array's dimension (RNF0530 and RNF7563, native47).
- Date and time literals in the DATFMT and TIMFMT forms, and RNF0305/RNF0306 for *ISO ones under them (native53, native134).
- SND-MSG *ESCAPE going to the procedure's caller (native175); a key literal of another length than the key field (RNF7078, native58).
- *NULL plus a number: no error on IBM i, but the pointer it gives was not captured (native50).
- pow(9 : 0.5) differing from 3 in the last bits (native201); a record with a CLOB column by native I/O (native210, not narrowed).

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

- **OPEN of a file that is already open.** On IBM i, a second OPEN(E) of a printer file, or of a database file, sets %ERROR and status 01215 (tools/golden/native161.txt n161_opene and n161_dblopen, PUB400 IBM i 7.5, 2026-11-20: `1 1215`). In the interpreter, the printer's second OPEN only writes the lines it keeps (%ERROR 0, %STATUS 0). The database file's raises CPF4161 even with E, because stmt_e does not take a file statement, so the program ends. The native engine (S2.161) does as the interpreter does. The fix belongs upstream, in printer.rs printer_op and in errors.rs (E for file statements). Fixed in the fork's interpreter (`fix/open-already-open`: printer_op's prt_open, and file_op's end takes an error out of file_op_inner with E or an error indicator); the native engine follows it (native_rt.rs printer_op_n's prt_open, file_takes_n in disk_open_close_n, printer_op_n, workstn_n, file_read_n and the change path): rla_test `open_e_of_an_open_file_and_read_e_of_a_closed_one_set_their_status` passes under rust, every statement native (2026-11-23).

- **%DEC of a date field that holds blanks.** On IBM i, `%dec(d)` of a date field whose bytes are blanks (overlaid in a data structure) ends with status 00112, a date that is not valid (tools/golden/native168.txt n168_decbad, PUB400 IBM i 7.5, 2026-11-20: `112 20260131`). In the interpreter, bif_override's DEC arm does not recognize the value and leaves it to the generic %DEC, whose char_number gives 00105. The native engine (S2.168) does as the interpreter does. Fixed in the fork's interpreter (`fix/dec-blank-date`); the native engine follows it (rpgrt::native::temporal_dec, RNX0112): rla_test `dec_of_a_date_field_holding_blanks_is_status_112` passes under rust (2026-11-23).

- **READ(E) of a file that is not open.** On IBM i, READ(E) of a USROPN file that is not open sets %ERROR and status 01211, and the program goes on (tools/golden/native171.txt n171_reade, PUB400 IBM i 7.5, 2026-11-20: `3 0 1 1 1211`). The interpreter's file_op_inner checks that the file is open before the part E takes, so CPF4135 is raised past E (a MONITOR group catches it as 1211). The native engine (S2.171) does as the interpreter does. The fix belongs upstream, in file_op_inner. Fixed in the fork's interpreter (`fix/read-not-open-1211`); the native engine follows it (native_rt.rs file_read_n and file_change_start_n end the operation there, the status as file_failed sets it): rla_test `read_e_of_a_file_not_open_is_status_1211` passes under rust, every statement native (2026-11-23).

- **An SQL table's primary key is not a keyed access path.** On IBM i, a program that declares a table created by SQL with a PRIMARY KEY as `keyed` compiles, and CHAIN by the key finds the record. That is the first version of tools/golden/native172's program, PUB400 IBM i 7.5, 2026-11-20, whose capture gave `0 0 1 1021 0 1 aa`. In as4tainr the CHAIN fails with "1 key values given, file GN172F has 0 key fields", in both engines. The fix belongs upstream, where the file's keys are taken from the catalog.

- **SND-MSG *ESCAPE goes to the procedure's caller.** On IBM i, `snd-msg *escape %msg('CPF9898' : 'QCPFMSG' : ...)`, and `snd-msg *escape 'text'`, inside a MONITOR group with ON-EXCP 'CPF9898' are not caught there. The message goes to the caller by default, and the main procedure's program ends with CPF9898 unmonitored (tools/golden/native175's first version, PUB400 IBM i 7.5, 2026-11-20). The interpreter's op arm raises the message in its own procedure, where the group catches it. The native engine (S2.175) does as the interpreter does. Fixed in the fork's interpreter (`fix/snd-msg-escape-caller`); the native engine follows it (native_rt.rs snd_msg: the message sent to the caller, pgmmsg::Escape with its target): rla_test `rpg_free_form_7x_features_and_builtins` passes under rust (2026-11-23).

- **A whole array assigned another array is not taken.** On IBM i, `w = s` with W VARCHAR(10) DIM(5) and S VARCHAR(10) DIM(3) assigns the three elements and leaves the other two as they were (tools/golden/native212.txt n212_listassign, PUB400 IBM i 7.5, 2026-11-21: `efg[four][five]`). The interpreter refuses it at run time ("S is an array: an index is needed here"). Fixed in the fork's interpreter afterwards (`fix/list-array-rest`, a2674ea); the native engine lowers it (EvalList with the source array's elements in use): rla_test `an_array_assigned_a_smaller_array_keeps_its_other_elements` passes under rust, every statement native (2026-11-23).

- **A record with a CLOB column, by native I/O (observed, not narrowed).** On IBM i, the program GN210FF failed with RNX9998 unmonitored when its file GN210T had a CLOB column besides INT, DOUBLE and VARGRAPHIC columns. Without the CLOB column, the same READ and UPDATE run (tools/golden/native210.txt, first version, PUB400 IBM i 7.5, 2026-11-21). The status at the failing statement was not captured. The interpreter reads and updates such records, leaving the large objects out (rla_test.rs TYPES). To narrow with a capture of %STATUS before deciding a fix.

- **%INTH of a float with many digits.** On IBM i, `%inth(pow(2 : 0.5))` is 1 (tools/golden/native203.txt n203_floatdec, PUB400 IBM i 7.5, 2026-11-21: `-12.6 2.67 -8 -7 1`). In the interpreter the bif arm's num() of the float, 1.414213562373095..., is RNQ0103 ("does not fit a decimal of 63 digits"). The same goes for %DEC, %DECH and %INT of such a float. The native engine (S2.203) takes the same num(). The unit NFLTDEC leaves that statement out. Fixed in the fork's interpreter (`fix/int-of-float`); the native engine follows it (rpgrt::native::float_arg_dec): rla_test `inth_of_a_float_with_many_digits_is_a_number` passes under rust (2026-11-23).

- **pow(9 : 0.5).** On IBM i the C runtime's pow(9, 0.5) is not exactly 3: `pow(9 : 0.5) <> 3` is true (tools/golden/native201.txt n201_pow, PUB400 IBM i 7.5, 2026-11-21: `ABC`). The interpreter's crt.rs pow is Rust's f64::powf, which gives 3. The native engine (S2.201) calls the same pow. Other floating-point results of the C functions may differ in the last bits too.

- **EXTPGM variable in lower case.** On IBM i, a prototype whose EXTPGM names a variable calls the program as the variable spells it: with 'qdcxlate' the call ends with status 00211, program not found (tools/golden/native200.txt n200_pgmvar, PUB400 IBM i 7.5, 2026-11-21: `1 0 hello 0 hello 211`). The interpreter's call_named_inner takes the name in upper case and calls QDCXLATE. The native engine (S2.200) does as the interpreter does. Fixed in the fork's interpreter (`fix/extpgm-var-case`); the native engine follows it (native_rt.rs call_proto: the variable's value as spelled): rla_test `extpgm_variable_names_the_program_as_spelled` passes under rust, every statement native (2026-11-23).

- **Output to a program-described file named by EXTFILE.** On IBM i, a program can create a program-described DISK file in QTEMP before it opens the file under EXTFILE('QTEMP/GN197F'), and then write it with EXCEPT (tools/golden/native197.txt n197_exceptdisk, PUB400 IBM i 7.5, 2026-11-21). The interpreter's disk_output needs the file's description from the compile, which it finds by the F specification's name and not by EXTFILE. The EXCEPT then fails: "GN197F is not a database file". The unit NEXCDSK names its file, made before the compile, on the F specification.

- **OUT of a data area that is not locked.** On IBM i, OUT(E) of a data area after UNLOCK (or never locked with IN *LOCK) is an error: %ERROR on, status 00412, and the data area keeps its value (tools/golden/native192.txt n192_unlock, PUB400 IBM i 7.5, 2026-11-21: `one 1 412 one two 1 412 two`). The interpreter's data_area writes it with no error. The native engine (S2.192) does as the interpreter does. Fixed in the fork's interpreter (`fix/out-unlocked-dtaara`); the native engine follows it (native_rt.rs data_area_n: OUT needs the job's lock, else RNX0412): rla_test `out_of_a_data_area_that_is_not_locked_is_status_412` passes under rust, every statement native (2026-11-23).

- **An OVERLOAD prototype not followed by a prototype.** On IBM i, `dcl-pr fmt varchar(30) overload(a : b : c);` has no END-PR: an END-PR after it is RNF3551. The declarations and calculations after it are not its parameters, and the program compiles with them following directly (tools/golden/native188.txt n188_crt, first versions, PUB400 IBM i 7.5, 2026-11-21). The interpreter's parser ends the prototype only at the next DCL-PR. It takes the `dcl-s` and even the calculation lines after it as parameters ("LINE has no data structure or prototype", "parameter LINE needs a type and length"). FREEFORM and the unit NOVERLD put another prototype after it.

- **DSPLY of a 4-byte float.** On IBM i, DSPLY of a `float(4)` field holding 0.5 is `+5.0000000E-01`: eight significant digits and a two-digit exponent (tools/golden/native183.txt n183_fltdsply, PUB400 IBM i 7.5, 2026-11-21). The interpreter's data::float_text gives `+5.000000E-001`: seven digits and three. 8-byte floats agree (`+1.230000000000000E+002`, `-1.250000000000000E-004`). The native engine (S2.183) does as the interpreter does.

- **%CONCAT(*BLANK).** On IBM i, `%concat(*blank : 'a' : 'b')` is `a b` (tools/golden/native182.txt n182_inconcat, PUB400 IBM i 7.5, 2026-11-21). The parser gives *BLANK as a figurative constant, not as the name special_arg looks for, so the interpreter's bif CONCAT takes its characters, none, as the separator and gives `ab`. Fixed in the fork's interpreter (`fix/concat-blank`); the native engine lowers the figurative separator as one blank (S2.220): rla_test `concat_separators_literal_none_and_blank` passes under rust (2026-11-23).

- **%CONCAT of a number compiles.** On IBM i a numeric operand of %CONCAT is RNF0203 (severity 30: the operand must be character, graphic or UCS-2; tools/golden/native182.txt n182_crt, first version, PUB400 IBM i 7.5, 2026-11-21). The interpreter's bif CONCAT accepts it and joins char_of_num's text. The native engine (S2.182) does not lower a numeric operand, so such a program stays with the interpreter.

- **XML-SAX handler returning non-zero.** On IBM i, XML-SAX whose handler returns a value other than zero ends with an error: under (E), %ERROR is on and %STATUS is 00351; with no E the program ends with RNX9998 unmonitored (tools/golden/native181.txt n181_sax, PUB400 IBM i 7.5, 2026-11-21: `<order<itempen<itemink & nib|id=7 14 1 351`). The interpreter's xml_sax stops parsing without an error. The native engine (S2.181) does as the interpreter does. Upstream's rla_test.rs xml_sax_handler_gets_events gets the same events as IBM i, but no error.

- **XML-INTO %HANDLER returning non-zero.** On IBM i a handler procedure that returns a value other than zero ends XML-INTO with an error: under (E), %ERROR is on and %STATUS is 00351; with no E the program ends with RNX9998 unmonitored (tools/golden/native180.txt n180_handler, PUB400 IBM i 7.5, 2026-11-20: `9 2 abc|5 1 ab 1 351`). The interpreter's xml_into_handler stops calling the handler without an error. The native engine (S2.180) does as the interpreter does.

- **%SPLIT into an array.** On IBM i, `w = %split(...)` into a fixed array of five elements, with four pieces, leaves the fifth element as it was (tools/golden/native179.txt n179_split, PUB400 IBM i 7.5, 2026-11-20: `the/fox/old|...`). The interpreter's assign_value initializes the elements after the pieces. The native engine (S2.179) does as the interpreter does. Fixed in the fork's interpreter (`fix/list-array-rest`); the native engine follows it (native_rt.rs assign_list), and lowers %LIST into an array too: rla_test `an_array_assigned_fewer_values_keeps_the_elements_after_them` passes under rust (2026-11-23).

- **A time plus a date compiles.** IBM i's compiler refuses `t0 = t + d` (a time plus a date): RNF7421 severity 30, "Operands are not compatible with the type of operator", and RNF7416 (tools/golden/native157.txt n157_crt, first run, PUB400 IBM i 7.5, 2026-11-20). as4tainr's compiler accepts it, and the interpreter makes the timestamp as for a date plus a time. The native engine lowers only the date-first form; the check belongs in the front end, upstream.

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

The review of S2.140 to S2.166 (codex, read-only, 2026-11-20) found two more of this kind:
- **SET RESULT SETS's row count** (S2.166). exec_sql.rs sql_set_result_sets converts the count `as usize`, so 2^64 rows wraps to 0, and no row is returned. The native runtime's sql_row_count gives a count that does not fit as the most there can be, so every element is returned.
- **A MOVEA index field below 1** (S2.86, reached by S2.154's literal source index too). ops.rs movea_parts converts the index `as usize` before its range check, so RNQ0121's message shows -1 as 18446744073709551615. The native runtime checks the signed value and shows -1. The status, 00121, is the same.

The review of S2.167 to S2.222 (codex, read-only, 2026-11-23) found more of this kind and one panic:
- **SET RESULT SETS ARRAY's row count** (S2.178), as SET RESULT SETS's above.
- **%LOOKUP of DSARR(*).SUBFIELD's start and count** (S2.196). bifs.rs converts them `as usize`: a start of 2^64 + 1 becomes 1, and a count of 2^64 becomes 0. The native runtime's `try_from` gives `usize::MAX` for both.
- **A start past the elements, with no count, panics in the interpreter.** bifs.rs's default count `dim + 1 - start` overflows: for a two-element array, `%LOOKUP(x : dsarr(*).sub : 4)` panics with overflow checks on (debug builds), and in release wraps. The native runtime saturates and gives 0. What IBM i gives (presumably status 00100, a start outside the array) is not captured yet. This is an interpreter defect for phase 2.

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

## %LEN as a target (found 2026-11-19)

Source: tools/golden/native102 n102_lenset (PUB400, IBM i 7.5, 2026-11-19).

The program cuts a varying field holding 'abcdef' to length 3, and then sets its length to 5.

| Case | IBM i | Interpreter |
|---|---|---|
| The field after the length goes from 3 to 5 | `abc  `: the new positions are blanks | `abcde`: the old characters come back. interp.rs assign_value's "LEN" arm writes only the length, and its comment says the storage shows through |
| A length past the field's maximum | status 115 | status 100 (RNQ0100) |

Native code follows the interpreter (rpgrt::native::set_var_len). Both are upstream interpreter defects, held for Kyle with the capture, and the unit NLENSET leaves them out.

## LIKEDS by reference through a prototype of another size (found 2026-11-19)

A codex review found that S2.97 lowered a call through a prototype whose LIKEDS parameter differs in size from the procedure interface's. One way is an EXTPROC naming a procedure of this module.

With OPDESC, the descriptor would differ: call_frame makes it from the interface's parameter, call_proc from the argument (CEEDOD's length 8 against 4). Lowering now requires the interface's parameter to be the prototype's own type. Such a call stays with the interpreter, so both engines do the same.

A PUB400 capture of the review's program failed in the bind step (RNS9301), so IBM i's value is not known. No unit checks it, and the change only gives the case back to the interpreter.

## %CHAR of float(4) (found 2026-11-19)

Source: tools/golden/native107 n107_floateval (PUB400, IBM i 7.5, 2026-11-19).

| Value | IBM i | Interpreter (data::float_text for 4 bytes) |
|---|---|---|
| 7 | +7.0000000E+00 | +7.000000E+000 |
| -2.5 | -2.5000000E+00 | -2.500000E+000 |

IBM i writes a float(4) with 8 significant digits (7 after the point) and a two-digit exponent; the interpreter writes 7 significant digits and three exponent digits. float(8) agrees with IBM i (+3.675000000000000E+001). Both engines go through the interpreter's text, so this is an upstream interpreter defect, held for Kyle with the capture. The unit NFLTEVL leaves the float(4) values out.

## Intermediate precision of 30 digits (found 2026-11-19)

Source: tools/golden/native111 n111_floatunary (PUB400, IBM i 7.5, 2026-11-19).

With f float(8) = 0, p packed(30:0) = 0 and q packed(2:2) = .99, `res = -(f + (p + q))` gives -9.900000000000000E-001 on IBM i: p + q keeps its two decimal places.

The interpreter's precision rules (interp.rs num_precision) give the result at most 30 digits when no operand has more than 30. p + q then needs 31 integer digits, keeps no decimals, and comes out as 0. The interpreter stores +0.000000000000000E+000.

This is the rule codex's audit pointed at for multiplication and division (its F4 and F5). It can change decimal expressions too, not only float ones. Native code follows the interpreter: a review of 2026-11-19 made the float path give decimal parts under + and - their precision, as eval does.

This is an upstream interpreter defect, held for Kyle with the capture. The unit NFLTUNA leaves that value out.

## OPEN of an open file (found 2026-11-19)

Source: tools/golden/native112 n112_charconv (PUB400, IBM i 7.5, 2026-11-19).

A second `OPEN(E)` of a printer file that is already open differs:
- **IBM i:** it is an error, with %ERROR on and %STATUS of the file 1215.
- **The interpreter:** it takes it with no error (0, 0).

Both engines run the OPEN(E) in the interpreter (OPEN with E stays with it), so this is an upstream interpreter defect, held for Kyle with the capture. Fixed in the fork's interpreter (`fix/open-already-open`); the native engine, which lowers OPEN(E) since S2.161, follows it (printer_op_n). The unit NCHRCNV leaves those two values out.

## %SQRT's decimal places (found 2026-11-19)

Source: tools/golden/native113 n113_sqrt (PUB400, IBM i 7.5, 2026-11-19).

%CHAR(%SQRT(10)) differs:
- **IBM i:** 3.16227766016837933199889354443, 29 decimal places.
- **The interpreter:** 3.162277660168379. bifs.rs's "SQRT" arm takes dec_sqrt with 15 places, normalized.

Assigned to a packed(15:13) field, the roots agree (4, 1.4142135623730, 2.5). Native code follows the interpreter (rpgrt::native::sqrt_bif).

This is an upstream interpreter defect, held for Kyle with the capture. The unit NSQRT leaves the %CHAR value out.

Fixed in the fork's interpreter (`fix/sqrt-places`: 30 digits, truncated, from the operand's precision). The native engine follows it (rpgrt::native::sqrt_bif with the argument's node precision, Dec::sqrt_trunc): rla_test `sqrt_keeps_thirty_digits` and `rpg_free_form_7x_features_and_builtins` pass under rust (2026-11-23). The SQRT operation is unchanged.

## A data structure shorter than declared, on a display (found 2026-11-19, codex)

This needs a procedure's LIKEDS parameter that has a display field's name and is passed by reference with OPTIONS(*VARSIZE) in a shorter variable at the end of its buffer. A WRITE of that format then behaves differently:
- **The interpreter:** it slices the whole data structure (eval, assign_value and ws_inds), and panics out of bounds.
- **Native code:** it reads the storage there is, with blanks or "off" for the rest.

The interpreter's panic is an upstream defect; the case is very contrived, and it is left as it is.

## WRITE and UPDATE of a qualified file's format (found 2026-11-19)

On a qualified file, `WRITE gn120f.gn120f o` and `UPDATE gn120f.gn120f r` name the file's record format the way it must be named.
- **IBM i:** the program compiles and runs (tools/golden/native120 n120_dsio: `50:zz 6:de`).
- **The interpreter:** it fails at run time with "GN120F.GN120F is not a file of this program". errors.rs file_idx compares the whole qualified name with the files' names and formats.

Native code follows the interpreter: lowering finds no file either, and leaves the statement to it. This is an upstream interpreter defect, held for Kyle with the capture. Fixed in the fork's interpreter (`fix/qualified-format-io`: file_idx takes FILE.FORMAT, file_op's record name is the format); the native engine follows it (lower.rs file_of and native_rt.rs file_idx_n take FILE.FORMAT, file_end_n names the format): rla_test `write_and_update_of_a_qualified_files_format` passes under rust, the WRITE and UPDATE now native (2026-11-23). The unit NDSIO takes the program with the file not qualified and its format renamed (n120_dsiou), which IBM i runs with the same result.

## %CHAR of a date or time expression under DATFMT and TIMFMT (found 2026-11-19)

Source: tools/golden/native134 n134_tchar (PUB400, IBM i 7.5, 2026-11-19), a program with `ctl-opt datfmt(*eur) timfmt(*usa)`.

%CHAR of a date or time expression with no format of its own differs (`%char(t + %minutes(45))`, `%char(%date('10/20/26' : *mdy))`, `%char(d - %months(1))`):
- **IBM i:** *ISO (`10.00.00`, `2026-10-20`, `2025-12-31`).
- **The interpreter:** the control specification's TIMFMT or DATFMT (`10:00 AM`, `20.10.2026`, `31.12.2025`). bifs.rs temporal_type gives an expression the default type, and bif_override takes that type's format.

Under the default formats (*ISO) the two agree (n134_tchariso).

Fixed in the fork's interpreter (`fix/char-temporal-expr-iso`): bif_override gives an expression *ISO. The native engine follows it (lower.rs temporal_char_of; %DEC of an expression keeps the control specification's format, as its arm does): rla_test `char_of_a_date_expression_is_iso_whatever_datfmt` and `rpg_date_and_time_formats` pass under `AS4TAINR_RPG_ENGINE=rust` (2026-11-23).

## Date and time literals under DATFMT and TIMFMT (found 2026-11-19)

Source: tools/golden/native134 n134_crt (PUB400, IBM i 7.5, 2026-11-19).

With `ctl-opt datfmt(*eur) timfmt(*usa)`, the literals differ:
- **IBM i:** they must be in those formats. `inz(d'2026-01-31')` and `inz(t'09.15.00')` are RNF0305 and RNF0306 (severity 20), and `d'31.01.2026'` and `t'09:15 AM'` compile.
- **The interpreter:** the other way round. It rejects `d'31.01.2026'` ("not a valid date literal in *ISO form") and takes the *ISO form.

Native code follows the interpreter for both, which are upstream interpreter defects held for Kyle with the captures. The unit NTCHREX uses the default formats.

## TEST with no E and no indicator (found 2026-11-20)

Source: tools/golden/native138 n138_crt (PUB400, IBM i 7.5, 2026-11-20), first version of gn138ts.rpgle.

A TEST(D), TEST(T) or TEST(Z) with neither the E extender nor an error indicator differs:
- **IBM i:** RNF5056 (severity 30), and the program does not compile.
- **The interpreter:** it compiles. ops.rs's TEST arm raises RNX0112 at run time for a value that is not valid.

This is an upstream compiler-check defect, held for Kyle. Native code follows the interpreter. The unit NTESTDT uses TEST with E only.

## A database file's INFDS (found 2026-11-20)

Source: tools/golden/native139 n139_infds (PUB400, IBM i 7.5, 2026-11-20). The program reads GN120F's two records, then reads at the end of file, printing the INFDS after each READ.

IBM i and the interpreter agree on the file name, the open and end-of-file flags, the status, the operation, and the record format and relative record number of a record read. Three fields differ:

| Field | IBM i | Interpreter |
| --- | --- | --- |
| Read count (247-250) | stays 1 over the three READs (blocked input) | counts every READ: 1, 2, 3 |
| Record format (38-45) after the end of file | blank | REC120 |
| Relative record number (397-400) after the end of file | keeps the last record's, 2 | 0 |

These are upstream interpreter defects, held for Kyle with the capture. Native code writes the INFDS with the interpreter's own infds_update (S2.139), so it follows the interpreter. The unit NINFDS leaves those fields out.

## A quotient compared in a condition (found 2026-11-20)

Source: tools/golden/native145 n145_cmparith (PUB400, IBM i 7.5, 2026-11-20).

`if 10 / 3 = 3.3333333333333333333333333333` (28 decimal places) differs:
- **IBM i:** false.
- **The interpreter:** true. interp.rs divide gives the quotient 28 decimal places by the precision rules with M = 30, as it does for %CHAR(10 / 3), whose 28 places come from tools/golden/rpg-prec.txt.

So in a comparison IBM i keeps a quotient to other decimal places than %CHAR shows. Which ones is not known yet; another capture would settle it. The other comparisons with arithmetic in the capture agree: a product, a quotient against fewer places, a sum, IN %LIST of a product, a product of a difference, and a DOW on a difference.

This is an upstream interpreter defect, held for Kyle with the capture. Native code follows the interpreter, and the unit NCMPAR leaves the test out.

Fixed in the fork's interpreter (`fix/quotient-compare`: a division that is itself a comparison's operand keeps the places of 63 digits, up to 62). The native engine follows it (ArOp::DivCmp, rpgrt::native::divide's `wide`), and lowers literals too long for an i128 mantissa (NumExpr::Wide): rla_test `a_quotient_compared_keeps_the_places_of_63_digits` passes under rust, every statement native (2026-11-23).

## A FOR limit that is not whole (found 2026-11-20)

Source: tools/golden/native146 n146_crt (PUB400, IBM i 7.5, 2026-11-20), first version of gn146fl.rpgle.

`for i = 0 to 10 / 4` differs:
- **IBM i:** RNF7569 (severity 20, "the limit value used in a FOR operation is not valid"), and the program does not compile.
- **The interpreter:** it compiles, and compares the index with 2.5 at each turn.

This is an upstream compiler-check defect, held for Kyle. Native code follows the interpreter. The unit NFORLIM uses whole limits.
