# Native RPG backend: handoff

Last update: 2026-11-23, night.

## Now (2026-11-23, night): phase 2 closed; native/engine-p2 validated, waiting for the push

**Interpreter (phase 2).** 40 local branches (`fix/*` plus 13 upstream-PR branches; list in /home/ctl/data/as4tainr-up-logs/p2g.branches) combined into the fork base by forkbase.py; the base is now c9f6bc9 (2bfbe12 plus RNF7078 from fix/compile-checks 6d88086). Combined validation p2-h on jsr-test-2 (2bfbe12): rla_test 301, lib 347, 3; demo PASS. RNF7078 (the user's decision, 2026-11-23: follow IBM i) is checked for CHAIN and SETLL with one search argument, after the capture native221; its full suite run passed locally (rla_test 264, lib 347, 3; demo PASS). Upstream test expectations changed by the user's decisions: `rpg_date_and_time_formats`, `setll_partial_key_then_reade_reads_one_order`, freeform.rpgle, dates.rpgle (the commits say so).

**Native engine.** native/engine-p2 (worktree /home/ctl/data/as4tainr-native-p2, head d0766f1) is native/engine rebased on c9f6bc9, plus the commits following each fix. s7f-np5 on jsr-test-2 (x86_64, d0766f1): fmt, clippy-ours, cargo test, build-release, demo PASS; cargo test --release 301/302, the known QTEMP flake `change_a_physical_file_with_a_logical_file_over_it` ("tuple concurrently updated"), PASS run again alone (rerun-test-release=0); native-diff PASS: coverage 100.0% (6855 instructions), settle 698/698, generated lints PASS, level 2 auto and rust 867/867, level 3 demo PASS. 5250 end-to-end under both engines (e2e-np5, scripts/native-e2e.sh, Playwright venv copied to jsr-test-2): interp 165 checks, rust 165 checks, same output, no refusals. clippy without `-A clippy::chunks_exact_to_as_chunks` fails on upstream's host-server code only. Manifests: docs/native/evidence/2026-11-23-*.json for the 31 commits since the S2.223 handoff. Logs in target/evidence/vm/jsr-test-2/{s7f-np5,e2e-np5}.

**The user's:** the force-push of native/engine-p2 to origin/native/engine (the user does it, 2026-11-24). It makes the fix branches' commits public in the fork; nothing goes to Kyle yet.

**Needs a human:** line-by-line review of the native commits for file errors with E (also display files), fixed-form ALLOC/REALLOC with CAT of based fields, and the SQL null indicator with DATE output host variables (section 7: database changes).

**Not run:** the aarch64 run (jsr-test-arm is lent out until the user says).

**Next:** phase B (the interpreter services still called through the Runtime trait); performance (memory performance-next); then Kyle.

## Now (2026-11-23, 06:00 UTC): S2.223 validated on both platforms; phase 2 continues

**S2.223** (e05d082) closes the codex review of S2.167-S2.222: 14 native fixes, the four interpreter findings in fidelity.md. s7f-s2223 on jsr-test-2 (x86_64): coverage 100.0%, settle 647/647, generated lints pass, level 2 rust 817/817. The ARM run s7f-s2222 on jsr-test-arm (aarch64) gives the same; its logs are under target/evidence/vm/jsr-test-arm. Manifests: 1582848.

**Phase 2 (interpreter fixes, local fix/* branches off upstream/main dd47e28)** now has 26 branches. New since the last update:
- fix/max-min-places: %MAX and %MIN have the arguments' greatest decimal places (native42);
- fix/date-overflow-113: a date past 9999-12-31 by a duration is 00113 (native56);
- fix/varying-dim-124: DIM(*AUTO) past its dimension and %ELEM set past it are 00124 (native47b);
- fix/date-loval-hival: *LOVAL and *HIVAL of a date are 0001-01-01 and 9999-12-31 (native53);
- fix/compile-checks has three more commits: RNF7421 for a duration before a date (native56), RNF0605 for %DATE(*SYS) (native55), RNF7030 and RNF7018 for EXSR of the main procedure's subroutine in a subprocedure (native45); a fourth, RNF0640 for more than 100 levels of parentheses (native47), is being validated.

**Held for the user:** %DATE with no format takes *ISO on IBM i (native55, and IBM Docs bbdate.htm), but the upstream test core/tests/rpg/dates.rpgle (H DATFMT(*USA), `%DATE('01/02/2027')`) expects DATFMT's. Not changed. *NULL plus a number (native50) is not changed either: the capture shows no error but not what pointer results.

**Next:** the combined validation of every fix branch (combo.py, vm-p2.sh), then the rebase of native/engine on them and native code following each fix (work state `phase2-interp-fixes`), then phase B.

## Earlier (2026-11-23, 02:10 UTC): 100% native coverage

**Committed up to S2.222** (8ff3db9). On the x86_64 run s7f-s2222, native-diff gives:
- coverage 100.0%: 6209 instructions native, none with the interpreter;
- settle: 647 of 647 routines route errors natively;
- generated lints: pass on 647 routines;
- level 2 rust: 817 of 817 the same as under the interpreter.

The last 18 instructions were tied to their programs through the source maps, a WHY pass and a dump of the sources rla_test compiles (AS4TAINR_DUMP_SOURCE). ADR 0007's S2.212-S2.222 lists each.

Two follow interpreter defects on purpose, as native code must:
- SYSAPI's shorter field by reference, which IBM i refuses;
- %CONCAT(*BLANK) with no separator, which IBM i gives one blank and the fork branch fix/concat-blank fixes.

**Still interpreter services (phase B).** Some instructions are native but call into the machine's state through the Runtime trait:
- program calls (call_program);
- clear_ds;
- the error notes (PSDS, status, exception message);
- XML-INTO and XML-SAX's parse.

Writing those again in rpgrt without Machine is phase B, the step that makes the interpreter removable.

**Next:**
1. Phase 2 continues. The 21 fix branches are local; 20 of them were validated together (p2-d).
2. After those are integrated, native code follows each fix; the work state `phase2-interp-fixes` has the list.
3. Phase B.

## Earlier (2026-11-22, 20:10 UTC)
: plan step 2, the interpreter fixed in fork branches

Each fix is a branch `fix/<topic>` off upstream/main (dd47e28). It has one commit, signed, with a new rla_test whose expected values come from a PUB400 capture; the test fails without the fix. The branches are local and not pushed; nothing goes to Kyle until the end (memory: kyle-notify-last). They are worked in /home/ctl/data/as4tainr-fix-open (one worktree, branches switched in place). The running list is claude-mem's work state `phase2-interp-fixes`.

**Done (20 branches; fix/compile-checks has four commits):**
- fix/open-already-open: OPEN(E) of an open file is 1215, READ(E) of a closed one is 1211, and a printer's second OPEN is an error;
- fix/concat-blank: %CONCAT(*BLANK);
- fix/list-array-rest: an array assigned %SPLIT or %LIST keeps its other elements (capture native212);
- fix/dec-blank-date: %DEC of a blank date is 00112;
- fix/out-unlocked-dtaara: OUT without a lock is 00412;
- fix/quotient-compare: a compared quotient has M = 63 (capture native214);
- fix/null-procptr-status: a call through *NULL is 00202;
- fix/editc-blank-floating;
- fix/qualified-format-io: WRITE and UPDATE of FILE.FORMAT;
- fix/sql-closed-cursor-507;
- fix/sql-insert-zero-rows: -221;
- fix/sql-null-indicator-default (capture native215);
- fix/sql-commit-none-7007;
- fix/test-needs-e: RNF5056 (capture native216);
- fix/compile-checks: RNF7571 for %OCCUR set to a literal, RNF0203 for %CONCAT of a number, RNF7421 and RNF7416 for a time plus a date, RNF7569 for a FOR limit that is not whole (capture native217);
- fix/extpgm-var-case: an EXTPGM variable names the program as it is spelled;
- the four the user decided on 2026-11-22, "follow IBM i", each changing the upstream tests that expected otherwise and citing the captures:
  - fix/float4-text: a float(4) is `+5.0000000E-01`;
  - fix/sql-call-missing-204;
  - fix/sqrt-places: 30 digits, truncated;
  - fix/xml-handler-nonzero: 00351.

**Validated together on jsr-test-2:**
- p2-a: the first 8 fixes;
- p2-b: the first 13 (commit c5d1a3d, built from the branches' diffs);
- p2-c: the first 15 (commit 2140611, rla_test 270 tests).

fix/extpgm-var-case is not in a combined run yet.

fmt, every test (rla_test 265) and demo pass. clippy stops at upstream's core/src/hostsrv.rs:105, a newer lint that fix/clippy-rust-1-100 covers. Local clippy is clean per branch.

Combined run p2-d (all 20 branches, commit f2e7e8c) is on jsr-test-2. Build a combined tree with /home/ctl/data/as4tainr-up-logs/combo.py (run in a worktree at upstream/main; vm-p2.sh is next to it): source diffs with `git apply --3way`; changes to existing tests with context; every test added at the end of rla_test.rs appended whole.

**Not started (bigger):**
- date and time literals in the DATFMT and TIMFMT forms, with %CHAR of a date expression as *ISO: the expression tokenizer needs the control specification;
- CREATE TABLE +7905 in a library with no journal: it may change many tests;
- a CHAR host variable's trailing blanks in a string function;
- EXEC SQL CREATE PROCEDURE with a compound body;
- EXTPGM in lower case, and the others in fidelity.md.

**After phase 2:** native/engine must follow each fix, because native code follows the interpreter. Each work-state item says what changes; the units NOPENE, NREADE, NINCONC, NSPLITA, NDECBAD, NDTAUNL, NCMPAR, NPROCPTR, NEDITC3, NDSIO and NSQL1-6 can then check the parts they leave out.

## Earlier (2026-11-21, 21:00 UTC)

**Committed up to S2.211** (3fe0658). The x86_64 run s7f-s2211 gives level 2 rust 816 of 816 as under the interpreter, coverage 99.7% (18 instructions left with the interpreter), and settle 646 of 646 routines route errors natively.

**ARM is still reserved by the user** (memory: arm-vm-busy).

**Plan step 1 (coverage) is closed.** After S2.210 nothing that can be lowered is left with the interpreter:
- NEST100 and SYSAPI's call are sources IBM i does not compile;
- NINCONC:38 (%CONCAT with *BLANK), NDTPARM:18 (a CONST date of another format) and NWHOLEA:35 (an element of a *VARSIZE array parameter, reach) are left on purpose (fidelity.md, ADR 0007).

**The user chose step 3 before step 2** (2026-11-21): S2.211 makes the error routing native. Each routine has its own table, and rpgrt::native::settle routes errors over it. The machine still gives the services on its state (status, PSDS, exception message): that is phase B.

**Next:**
1. Step 2: fix the interpreter in fork branches `fix/<topic>`, one per fidelity.md defect and codex finding, each with a PUB400 capture, tests, evidence and a codex review. Nothing to Kyle until the end (memory: kyle-notify-last).
2. Phase B: the machine's services written again in rpgrt.

## Earlier (2026-11-21, 14:20 UTC)

**Committed up to S2.197** (3dd0c7c). The x86_64 run s7f-s2197 gives level 2 rust 803 of 803 as under the interpreter, and coverage is 99.4% (34 instructions left with the interpreter).

**Validating on jsr-test-2:** s2201 (S2.198 to S2.201), then s2202.

**ARM is still reserved by the user** (memory: arm-vm-busy).

**Plan to the milestone**, approved by the user on 2026-11-21 (memory: native-100-percent):
1. coverage up to about 99.8–99.9%;
2. the interpreter fixed in the fork where it accepts what IBM i rejects;
3. the error routing (settle) made native.

Since 2026-11-20 the user has also authorized these upstream touches, recorded in CLAUDE.native.md section 3:
- xmlsax.rs's split;
- exec_sql.rs's timestamp functions made `pub(super)`;
- `ParamInfo.likefile`.

**Blocks of 2026-11-21:**
- XML-SAX;
- IN of an enumeration, %CONCAT;
- DSPLY of a float;
- SQL timestamp host variables;
- LIKEFILE in procedures;
- based arrays;
- an EXCEPT edit word from the procedure;
- OVERLOAD;
- date, time and timestamp parameters;
- a based float read;
- program-described reads, also into a data structure;
- UNLOCK of data areas, and IN/OUT with E;
- %KDS;
- OPTIONS(*CONVERT);
- local *OMIT;
- %LOOKUP of DSARR(*).SUB;
- EXCEPT to DISK;
- pointer differences;
- EXTPGM in a variable;
- C functions' float parameters and results.

**New interpreter defects**, all in fidelity.md: XML-SAX's non-zero return, %CONCAT with numbers and *BLANK, DSPLY of float(4), an OVERLOAD prototype without END-PR, OUT of a data area that is not locked, EXTFILE for output files, EXTPGM in lower case, pow(9 : 0.5).

**Left:**
- TYPES: UCS-2, graphic and LOB fields;
- CONSTS: a UCS-2 constant;
- RUNITF: a data structure returned, a *VARSIZE argument, a local array;
- NESTED and IMQPATH: an element of a nested data structure array passed by reference;
- LOCALTR: a C call returning a data structure;
- XSCRT:97: %DECH of a call that returns a float.

IBM i rejects these at compile time, so they cannot be made native:
- NEST100;
- SYSAPI's mismatched by-reference argument.

The unit cases left on purpose are NINCONC:38 (a figurative *BLANK) and NDTPARM:18 (a CONST date of another format).

**Lessons (2026-11-21):**
- A WHY pass writes nothing unless AS4TAINR_NATIVE_EMIT is set too.
- Check that a unit's name is free before writing its file: NPTRDIF was overwritten once and restored from git.
- rpgcg's VERSION had not changed since S2.172, because the edit looked for a text that was no longer there. Check `grep '"+stage2'` after changing it.

## Before (2026-11-20, 22:40 UTC)

**Committed up to S2.176** (89e464a). The x86_64 run s7f-s2176 gives level 2 rust 786 of 786 as under the interpreter, and coverage is 98.8%. About 69 instructions of 5820 are left with the interpreter.

S2.166 to S2.176 add:
- SET RESULT SETS ARRAY;
- the fixes from codex's review of S2.140-S2.166 (S2.167, S2.168);
- data structure arrays as CONST parameters;
- named constants in string operations;
- E and the error indicator on database file operations (S2.171 and S2.172, in one commit);
- CALLP(E);
- UPDATE %FIELDS;
- SND-MSG;
- XML-INTO(E).

**Validating:** s2179, on jsr-test-2. It holds S2.177 (EVAL-CORR), S2.178 (SET RESULT SETS of an OCCURS structure) and S2.179 (%SPLIT into an array).

**ARM (jsr-test-arm) is reserved by the user** from 2026-11-20 16:40 UTC, for at least 24 hours or until they say otherwise. Its PostgreSQL container `as4tainr-test-db` is stopped. Run nothing there, and start the container first when it is free (memory: arm-vm-busy).

**Interpreter defects found today**, now in fidelity.md (fix them in the fork after the native engine, then notify Kyle last; memory: kyle-notify-last):
- %DEC of a date field of blanks;
- READ(E) of a file that is not open;
- OPEN(E) of an open file;
- an SQL table's primary key, which is not a keyed access path;
- SND-MSG *ESCAPE, which should go to the caller;
- %SPLIT into an array, which clears the elements after the pieces;
- a time plus a date;
- %BITXOR with three arguments.

**Left:**
- XML-INTO with %HANDLER and XML-SAX;
- UCS-2 (TYPES);
- a data structure array returned (RUNITF);
- EXCEPT to a program-described DISK file (TRANSPD);
- a program-described DISK file read into a data structure (CEEPGM);
- UNLOCK of a data area;
- the parts the user chose to leave (LIKEFILE inside procedures, SQL timestamp host variables) and those left on purpose (ADR 0007).

Even at 100%, errors still go through the interpreter's settle path. Removing the interpreter is a phase of its own.

**Lessons (2026-11-20):**
- A WHY pass over the whole suite needs a new database of its own: under `unshare` the library names repeat.
- Stop a chain by its process group, and only one run may use dev41 at a time.
- When one snapshot contains the next, the queue can be shortened: stop the chains that are waiting, then point the newest one at the run in progress.

## Before (2026-11-20, 16:30 UTC)

**Committed up to S2.165** (8007b1a). The x86_64 run s7f-s2165 gives level 2 rust 776 of 776 as under the interpreter, and coverage is 98.5%.

The S2.165 commit message wrongly says it adds the evidence manifests for S2.149 to S2.164. They come with the S2.169 snapshot instead, and the next commit message must say so (as4tainr-up-logs/errata-pending.txt).

**Decisions of the user (2026-11-20):**
- **No upstream touch for LIKEFILE inside procedures, nor for SQL timestamp host variables.** Both stay with the interpreter.
- **Two behavior-preserving touches are authorized, listed in CLAUDE.native.md section 3:** splitting xmlinto.rs's xml_into (xml_into_doc), and `pub(super)` on exec_sql.rs RESULT_SETS.
- **The order of the work:**
  1. Finish the native engine.
  2. Fix in the fork every interpreter defect in fidelity.md and every codex finding, with evidence and an audit.
  3. Notify Kyle last (memory: kyle-notify-last).

**Validating:**

| Snapshot | What | Where |
| --- | --- | --- |
| s2166 | SET RESULT SETS ARRAY (a procedure's local structure array too) | ARM |
| s2168 | S2.167 (the bit functions' order, from the codex review) and S2.168 (%DEC of a temporal value that is not one; *OMIT for a LIKEFILE parameter) | jsr-test-2 |
| s2169 | a data structure array for a CONST LIKEDS DIM parameter, and its elements in the callee; the manifests for S2.149-S2.164 | jsr-test-2, after s2168 |

**The codex review of S2.140-S2.166** ran in four groups (target/evidence: as4tainr-ev-x86/codex-s2142-s2166), and each finding was checked against the code. ADR 0007's S2.167 heading records what was fixed, what was recorded, and what was not a difference.

**What is left:** about 85 instructions.
- OCCURS structures in SET RESULT SETS (LISTMOD's LISTOCC);
- XML-INTO with %HANDLER, and XML-SAX;
- EVAL-CORR;
- IMQPATH's path arguments and returns;
- RUNITF's data structure returns;
- UCS-2;
- LIKEFILE inside procedures, and SQL timestamp host variables, which the user chose to leave;
- the parts left on purpose (ADR 0007).

## Before (2026-11-20, 12:00 UTC)

**Committed up to S2.161** (7eba985). The aarch64 run s7f-s2161 gives level 2 rust 772 of 772 as under the interpreter, and coverage is 98.4%. The full WHY pass on the s2162 tree leaves 89 instructions in the interpreter, out of 5599.

S2.147 to S2.161 add:
- TEST(E);
- SQRT;
- tables (TABx);
- *OMIT;
- *ON/*OFF for indicator parameters;
- COMMIT/ROLBK;
- the bit functions of numbers;
- MOVEA with a literal index;
- %UNS;
- WRITE of a printer from a DS;
- date + time;
- %STR as a target;
- %ADDR of based fields;
- pointer comparisons;
- OPEN/CLOSE/FEOD with E.

**Validating:**

| Snapshot | What | Where |
| --- | --- | --- |
| s2162 | %ABS of float expressions | jsr-test-2 |
| s2163 | CAT and the other string operations with a table's current element | ARM |
| s2164 | file operations beside a LIKEFILE parameter | jsr-test-2, after s2162 |

Each later snapshot contains the earlier ones. Only one run may use dev41 on a machine. Stop a chain by its process group (memory: remote-kill-by-pid).

**What is left, and what each part needs:**
- **Needs an upstream touch point (ask the user):**
  - Operations inside a procedure on its LIKEFILE parameter, and calls that pass a file, need the parameter's template file. `ParamInfo` keeps only `file: bool`, in program.rs.
  - Timestamp host variables in EXEC SQL (SQLOVR) need exec_sql.rs's iso_timestamp and db2_timestamp. They would have to be moved into rpgrt, or copied there with a tripwire.
- **Large blocks:** XML-INTO (7 instructions) and XML-SAX; SET RESULT SETS with host variables (LISTMOD); EVAL-CORR; data structure array parameters (IMQPATH, IMQDSARR, NESTED); returning a data structure (RUNITF); UCS-2 (TYPES).
- **Left on purpose, with the reason in ADR 0007:**
  - DSPLY of a float, which has no capture;
  - SYSAPI's QDCXLATE call, which IBM i does not compile;
  - NEXCPT2's local edit word;
  - NBASARR's based-array operations;
  - NEST100's depth limit.

**Interpreter defects found today** (fidelity.md, held for Kyle with captures):
- a second OPEN(E) of a file;
- a time plus a date;
- %BITXOR with more than two arguments;
- %CHAR of a float(4) (already known).

**Lessons (2026-11-20):**
- A units run's WHY entries and emit map are the check, not the pass count. S2.160 and S2.162 each passed while one of their statements still ran in the interpreter.
- Db2 for i's `create or replace table` keeps the rows (ON REPLACE PRESERVE ROWS), so a capture's setup deletes before it inserts.

## Before (2026-11-20, 07:40 UTC)

**Committed up to S2.146** (63c40e0). The x86_64 run s7f-s2146 gives level 2 rust 758 of 758 as under the interpreter, and coverage is 97.6%.

S2.140 to S2.146 add:
- DSPLY with a response field;
- %ADDR of a whole array, RETURN of a pointer;
- procedures with their own *PSSR;
- %DEC of temporal values;
- indicator subfields by their path;
- arithmetic in conditions and in FOR limits.

The S2.142 commit message corrects the "752 of 752" of S2.140 and S2.141, which should read 753.

**Snapshots validating, chained** (`as4tainr-up-logs/s7-<tag>.commit`, each one's parent the snapshot before). Commit them in order as they pass.

| Snapshot | What | Where |
| --- | --- | --- |
| s2147 | TEST(E) of dates, times and timestamps (S2.138 claimed native but was not: correct that claim in this commit's message) | ARM |
| s2148 | SQRT | jsr-test-2 |
| s2149 | tables (TABx): LOOKUP with an alternate, the current element | ARM |
| s2150 | *OMIT arguments (CEE, C, bound procedures, procedure pointers) | jsr-test-2 |
| s2151 | *ON/*OFF for indicator parameters passed by value | ARM |
| s2152 | COMMIT and ROLBK | jsr-test-2 |
| s2153, s2154 | %BITAND/%BITOR/%BITXOR/%BITNOT of numbers and comparisons with hex constants; MOVEA with a literal index | not yet: units running locally, one snapshot |

The snapshots s2142 and s2144 on ARM are made redundant by the x86_64 runs, which already validated them.

**Left to the interpreter, with the reason (2026-11-20):**
- DSPLY of a float: two PUB400 captures could not get IBM i's text. Under PASE system it ends in RNX9998, and the batch job left no job log (S2.110, S2.151 note).
- A by-reference argument of another type without *VARSIZE: IBM i does not compile it (SYSAPI).
- EXCEPT whose edit word is a procedure's own constant (NEXCPT2, review of 2026-11-18).
- Data structure array parameters (IMQPATH, IMQDSARR), XML-INTO, SET RESULT SETS with host variables, printer WRITE from a data structure, UCS-2.

**Lessons (2026-11-20):**
- Check the emit map's `"via":"native"` before calling a block verified: S2.138's TEST never ran natively.
- Never edit rpgrt or rpgcg while a units run is going: S2.153's rust phase built against S2.154's runtime and failed. Run the units again together.
- Renaming identifiers with sed also renames inside strings (gn1.out became gn1.outdt).

## Before (2026-11-19, 22:10 UTC)

**Committed up to S2.128** (10e6dba). Native-diff level 2 rust is 744 of 744 as under the interpreter, and coverage is 96.7%.

S2.121 (ee603f9) made native-diff's three suite runs use the same library names:
- each test binary is process 1 of its own PID namespace (`unshare -cpf`);
- its tests run one at a time;
- each run has a new database.

The 23 programs `rust` mode had refused whole now run natively, with no difference. The only refusal left is NATSW1, on purpose.

**Evidence manifests** (4003e3b): `scripts/native-manifest.py` writes one per commit, and the manifests run from 2026-11-15 on.
- It finds the validating runs by git tree. They are copied into `target/evidence/vm/<machine>/<run>`, outside Git.
- It never rewrites a committed manifest (section 8).
- After each commit: copy the run, run the script for the new range, and commit its output in a later snapshot.
- docs/native/methodology.md (English, for Kyle) describes the process.

**Snapshots validating, chained** (`as4tainr-up-logs/s7-<tag>.commit`). Commit each as it passes; a later one validates the ones it contains.

| Snapshot | What | Where |
| --- | --- | --- |
| s2129 | %PARMS | ARM |
| s2130b, s2131b, s2132b | fields based on based pointers; IN %RANGE and IN %LIST; RETURN of figurative constants | s2131b on ARM, s2132b on jsr-test-2 |
| s2133 | calls returning indicators as conditions, RETURN of a condition | jsr-test-2 |
| s2134 | %CHAR of temporal expressions | ARM |

The b snapshots keep 2026-11-19-383f072.json as committed, because a working copy had rewritten it.

**Interpreter defects found today** (fidelity.md, held for Kyle with captures):
- WRITE and UPDATE of `file.format` on a qualified file (native120);
- %CHAR of a date or time expression under DATFMT or TIMFMT other than *ISO: IBM i gives *ISO (native134);
- date and time literals under DATFMT(*EUR) and TIMFMT(*USA) (native134).

**Left to the interpreter, with the reason:**
- arithmetic on a comparison side, which has no target precision (S2.13, S2.131);
- %BITAND with hexadecimal constants as numbers (FDTEST);
- a by-reference argument shorter than its parameter without *VARSIZE, which IBM i does not compile (SYSAPI's QDCXLATE);
- tables (TABx) as their current element.

**Next:** a WHY pass, `AS4TAINR_NATIVE_WHY` over the whole suite, which now also notes conditions and RETURNs. Pick the largest group left: DSPLY 28, EVAL 23 and CALLP 16 in the last coverage.

**Lessons:**
- Over ssh, never `pkill -f` or `pgrep -f` with a pattern in the same command, since it kills its own session. List the PIDs, then kill them by number.
- Check that a unit's name is new before writing it: NDSPARM's source was overwritten once and restored from git.
- Replace the CAPTURE placeholder with a check that it happened.

## Before (2026-11-19, 07:35 UTC)

**Committed up to S2.105** (1bc82c0; 94.6% native, level 2 rust 705 of 727 as under the interpreter):
- S2.100-S2.101 and S2.104-S2.105: indicator fields as targets, conditions, VALUE arguments, characters, and compared;
- S2.102: %LEN as a target;
- S2.103: *NEXT, as EVAL targets only.

**ADR 0008 (floats) was accepted by the user on 2026-11-19.** It is implemented in parts, each a snapshot that is validating:

| Snapshot | What | Where |
| --- | --- | --- |
| s2106 | float EVAL targets, and the S2.97 review fix (ds_ref type) | jsr-test-2 |
| s2107 | %CHAR of floats; the tripwire now also watches clear_ds, reset_to_initial, assign_value and float_value | ARM |
| s2108 | float arithmetic (FloatExpr, EvalValue) | jsr-test-2 |
| s2109 | float comparisons | ARM |
| s2110 | %FLOAT of characters, %ADDR of a float; the tripwire watches char_number | jsr-test-2 |
| s2111 | codex's float review fixes: unary precision, the return type order, no calls in float expressions | ARM |

Commit them in order as each passes. A later snapshot that contains an earlier one validates both.

**Flakes, not the backend's** (upstream, held for Kyle):
- duplicate_objects: CRTDUPOBJ "could not open relation with OID";
- change_a_physical_file_with_a_logical_file_over_it: CHGLP in QTEMP;
- cl_declared_files_hold_shared_read_locks.

**Interpreter defects found today** (fidelity.md, held for Kyle):
- the 30-digit intermediate precision cap (IBM i keeps p + q's decimals);
- %CHAR of float(4);
- %LEN set longer;
- pointers to different fields;
- the EXECUTE USING order;
- the S2.92 printer and EXCEPT items.

**Not done, with the reason:**
- DSPLY of a float: PUB400 through PASE system gave RNX9998, so there is no IBM value;
- ** of floats;
- based floats;
- floats in calls.

**Waiting for the user:** how to send the audit's security findings to Kyle. The proposal is a private advisory after 2026-11-24.

**After the native engine is done** (the user's plan): fix every codex audit finding in the fork, on fix/<topic> branches, then tell Kyle what affects him.

## Before (2026-11-19, 03:55 UTC)

**Committed up to S2.99** (8ed9ae3; 94.3% native, level 2 rust 701 of 723 as under the interpreter):
- S2.93: a procedure's data structures as a whole;
- S2.94: CLEAR;
- S2.95: RESET of globals;
- S2.96: based arrays found by codex;
- 6dea5c4: the units' catalog::init once per process, which ends the NSQL1 race;
- S2.97: LIKEDS parameters by reference;
- S2.98: pointer differences;
- S2.99: calls returning pointers.

**Snapshots chained one on the other, validating** (`as4tainr-up-logs/s7-<tag>.commit`):

| Snapshot | What | Where |
| --- | --- | --- |
| s2100 | indicator fields: EVAL targets and conditions | ARM |
| s2101 | indicator parameters passed by value | jsr-test-2 |
| s2102 | %LEN as a target | ARM |
| s2103b | *NEXT, EVAL targets only (codex found the first version too wide) | jsr-test-2 |
| s2104b | indicator fields as characters | ARM |
| s2105b | indicator fields compared | jsr-test-2 |

Commit them in order as each passes, checking each time that the snapshot's parent tree is HEAD's.

**In the working tree, not in any snapshot yet.** The S2.97 review fix: `ds_ref` requires the interface's parameter type to be the prototype's. It goes in the next block, S2.106.

**Waiting for the user:**
- **ADR 0008** (floats, Proposed), `docs/native/adr/0008-float-fields.md`. It is kept out of the snapshots until it is accepted.
- **The codex audit of the whole fork**, 2026-11-19: /home/ctl/data/codex-audit-2026-11-19/RESUMEN.md.
  - Security findings in upstream code, two of them confirmed: the SSH sessions' AS4TAINR_DATABASE_URL, and the quoted "set_config" past the SQL filter.
  - The proposal is to send them to Kyle as a private security advisory, after 2026-11-24.
- **Codex on ARM** is not logged in. jsr-test-2 has no codex.

**A new upstream flake, not the backend's.** duplicate_objects fails now and then: CRTDUPOBJ OBJ(DUP*) OBJTYPE(*DTAARA) gives "could not open relation with OID n". It has no RPG.
- It failed in S2.101's and S2.105's cargo test on jsr-test-2.
- It failed in S2.102's reference run, under the interpreter, on ARM.

This looks like a PostgreSQL race between catalog DDL in parallel tests. It is held for Kyle.

**Interpreter defects found by captures today** (fidelity.md, held for Kyle):
- the EXECUTE USING order (SQL0518 before the values; IBM reads the values first, 907);
- pointers to different fields cannot be subtracted (IBM i allows it);
- %LEN set longer shows the old characters (IBM: blanks), and its out-of-range status is 100 (IBM: 115);
- the five printer and EXCEPT defects of S2.92.

**What is still interpreted:** floats (ADR 0008); CONST and DIM data structure parameters; local arrays of data structures; UCS-2; %NULLIND; XML-INTO; record formats in CLEAR; local names in RESET.

## Before (2026-11-19, 00:10 UTC)

**Committed up to S2.92** (93c39cc; 93.9% native, level 2 rust 694 of 716 as under the interpreter):
- S2.82-S2.86: temporal file fields and the fixed-form string, bit, array, SORTA and MOVEA operations;
- S2.87: procedure pointers. Its first run found a native bug: call_frame checked the argument count for calls through a prototype, which call_proc allows (OSSILE's OSSPARMS). It was fixed before the commit;
- S2.88-S2.89: %STR, %LEN of expressions, FOR-EACH;
- S2.90: ON-EXIT;
- S2.91: imported and exported fields;
- S2.92: printer parity found by the codex review.

**Snapshots chained one on the other, validating** (`as4tainr-up-logs/s7-<tag>.commit`):

| Snapshot | What | Where |
| --- | --- | --- |
| S2.93 | a procedure's own data structures as a whole | jsr-test-2 `s7f-s293` |
| S2.94 | CLEAR | ARM `s7f-s294` |
| S2.95 | RESET of globals | jsr-test-2 after S2.93 |

Commit them in order as each passes, checking each time that the snapshot's parent tree is HEAD's.

**Codex.** The user offered codex, which is on this host and on ARM. It is used read-only: `codex exec -s read-only -C <repo> - < prompt`.
- Lists of about 6 functions work; longer lists end unfinished.
- Each finding is checked against the code and settled with a PUB400 capture.
- Reviewed so far: str_op_n and str_at (out-of-range numbers only, in fidelity.md), and every printer and data area function (S2.92).
- Not reviewed yet: bit_op_n, array_op_n, sorta_n, movea_n, paddr_n, procptr_target_n, each_array/each_take, call_frame's ON-EXIT and `checked`, and the SQL functions (S2.69-S2.75).
- When the engine is finished, the user wants a full codex audit of the fork: security, performance and IBM i fidelity.

**Known flake, not the backend's.** native_unit_lowered_static_sql (NSQL1) fails now and then with PostgreSQL's "tuple concurrently updated", under either engine. It holds the units' DDL mutex, so the race is with catalog::init run by the other units in parallel.
- In production, catalog::init runs once when a server starts. Each CLI command (system, batch, put, get, db) runs it again, and could meet a DDL of another job.
- A retry or a no-change init is a possible upstream proposal. The user may want it after 2026-11-24.

**What is still interpreted**, from S2.91's native-diff coverage: DSPLY 57, EVAL 52, IF 37, CALLP 20, WRITE 13, RETURN 12, READ 11 and smaller groups.
- The WHY pass over the whole suite needs AS4TAINR_NATIVE_EMIT as well as AS4TAINR_NATIVE_WHY: lowering runs only when code is generated.
- `jsr-test-2:~/data/as4tainr-native/why-s291/why.txt` has 140 lines.
- The main causes: floats (10 or so; they need an ADR first), DS parameters to local procedures, procedures returning pointers, pointer arithmetic, UCS-2, %NULLIND, local varying fields, and local DS arrays.

## Before (2026-11-18, 16:00 UTC)

**Committed up to S2.81** (e1a0c4e; 91.9% native, level 2 rust 682 of 704 as under the interpreter):
- embedded SQL in full: S2.69-S2.75, the visibility commits 104e992 and 558eff2, QTEMP units 8714d95;
- S2.76-S2.77, nested data structure paths;
- printer files: S2.78 EXCEPT, S2.79 WRITE, OPEN, CLOSE and FEOD;
- 15c4a43, the units that make or drop objects run one at a time;
- S2.80, data areas;
- S2.81, %EDITC's third argument.

**Snapshots chained one on the other**, validating (`as4tainr-up-logs/s7-<tag>.commit`):

| Snapshot | What | Where |
| --- | --- | --- |
| `830e8ab` S2.82 | date, time and timestamp fields of database files | ARM `s7f-s282` |
| `d78fd7e` S2.83 | fixed-form SCAN, CHECK, CHECKR, SUBST, XLATE, CAT | jsr-test-2 `s7f-s283` |
| `744d8aa` S2.84 | fixed-form TESTN, TESTZ, TESTB, BITON, BITOFF | ARM after S2.82 |
| `8d31651` S2.85 | fixed-form LOOKUP of arrays, XFOOT, %CHAR of indicators | jsr-test-2 after S2.83 |

Commit them in order as each passes, checking each time that the snapshot's parent tree is HEAD's.

**Known failures that are not the backend's** (an upstream fragility under parallel tests, held for Kyle after 2026-11-24):
- change_a_physical_file_with_a_logical_file_over_it: QTEMP's CHGLP, "tuple concurrently updated" or CPF3219;
- dspmod_output_files_of_a_cl_module: QTEMP's file_fields duplicate key;
- declared_temporary_table_and_default_user: deadlock;
- open_files_hold_shared_read_locks: CPF1002, seen once in a debug run on ARM.

Each fails in some runs, with the interpreter as well. A commit cites the run and the cause.

**Next:**
- the fixed-form operations still interpreted: MOVEA, SORTA, MOVE of fixed programs, CLEAR, RESET, TEST, MHHZO and the like;
- then the DSPLY, EVAL and IF refusals by operand (floats, procedure pointers, data imports, UCS-2, %NULLIND, %ADDR/%STR).

The WHY pass over the whole suite (`as4tainr-ev-x86/why-s280`) lists them with program and line.

**jsr-test-2** (x86, 10.0.1.1) had no `/QSYS.LIB` directory, and qsh_commands_see_qsys_lib fails without one. It was made with sudo on 2026-11-18, as on the ARM VM and the dev host.

change_a_physical_file_with_a_logical_file_over_it failed in all three of S2.68's native-diff suites on jsr-test-2, interpreter included. It passes alone 4 times out of 4, and in the plain section 7 runs. It looks like an upstream flake in that environment, under QTEMP. Watch it in S2.70's run.

**WHY pass over the whole suite (x86, 2026-11-18 04:30, with S2.71).** 197 refusals:
- DSPLY 84: its operand's parts, each a few times. Packed fields refused as "other" (data imports), calls of local procedures that are not native themselves (ON-EXIT), %STR/%ADDR of pointers, arrays, floats, UCS-2, %NULLIND, procedure pointers, %CONCAT;
- CALLP through prototypes 21;
- assignments 50 or so: data imports (DATAMAIN, PP, PTRMAIN), floats (8), nested data structure paths (12), %LEN as a target.

Each of those is a small feature of its own:
- **data imports** need the module-to-module storage that ile.rs binds (interp.rs `binding`);
- **floats** need the interpreter's type-dependent float rules (data.rs decode gives Num when the value fits; bifs.rs %CHAR by type), so they want an ADR-style plan first.

**Upstream finding (held, after 2026-11-24):** two tests running in parallel can collide in QTEMP. change_a_physical_file_with_a_logical_file_over_it gave "CPF5813: File CHGLP already exists in library QTEMP", and dspmod_output_files_of_a_cl_module a duplicate key (library, file, seq) = (QTEMP, C4IX, 0) in file_fields: the catalog holds the name QTEMP itself, not the job's library. This happens with the interpreter, in the full suite only; both pass alone.

**S2.74 on jsr-test-2:** level2-auto FAIL, from the reference run alone. Under the interpreter, NSQL1's final `drop table QGPL/NSQL1T` failed with "tuple concurrently updated", while the auto run passed and level 2 rust passed (674 of 697).

This is the upstream race sql.rs already retries elsewhere: each test process's catalog::init grants on all tables, while other tests run DDL. S2.74 is committed on S2.76's run, whose tree contains it.

Upstream finding (held): DROP TABLE in embedded SQL is not retried against that race.

**Next, after SQL:** the long tail the AS4TAINR_NATIVE_WHY pass lists, now with program and line:
- temporal forms (%CHAR of time + duration, %DATE with *MDY or a number, %DEC of a date);
- procedure pointers, %CONCAT, data imports;
- nested data structure subfields;
- floats and %SQRT;
- printer and display files.

**How a block goes:**
1. Code.
2. A PUB400 capture (tools/golden/nativeNN, one ssh session).
3. A native unit (core/tests/native) whose expected lines are the capture's.
4. The units under both engines on x86 (or the VM), with AS4TAINR_NATIVE_WHY and AS4TAINR_NATIVE_EMIT set. WHY needs EMIT, or lowering does not run.
5. A snapshot (a temporary index, `git commit-tree`) on top of the previous one.
6. A full section 7 on the VM (`vm-s7full.sh <tag> <commit>`). `vm-chain.sh` runs several in a row from `src-<tag>` copies.
7. `git commit-tree` of the validated tree, `update-ref`, `git reset`, push.

**Upstream:** PRs #19-#24 were published on 2026-11-17, along with re-check comments and a transparency note on #11, #12 and #14. Nothing more goes to Kyle until 2026-11-24 (the user's choice). See memory kyle-quiet-week and as4tainr-up-logs/revision-2026-11-17-publicado.md.

**WASM side project:** the sign-on worked in the browser (1114 s). Next steps are in the landing repo's README. A package for another computer is to be prepared on the user's request: the memory wasm-landing says how.

## Earlier state (2026-11-14)

## State

- **Stage 1 is complete.** Every program runs natively under `AS4TAINR_RPG_ENGINE=rust|auto`, with the interpreter as the reference.
- **Stage 2, S2.1 to S2.20, is committed** (ADR 0006). S2.21 (array elements with field indexes) is committed too. S2.22-S2.24 are committed and validated on x86_64. S2.25 (%SIZE, %ELEM) is validated on aarch64 (Mac M1, 2026-11-14: native-diff PASS, evidence/2026-11-14-s225-aarch64-mac.json), not yet on x86_64. Lowered:
  - conditions, loops, numeric and character EVAL;
  - fixed-form arithmetic, DIV/MVR, `/`, %REM, %DIV;
  - %SUBST, %SCAN, %TRIM, %XLATE, %CHAR, %INT, %DEC, %LEN;
  - MOVE/MOVEL;
  - array elements with literal indexes, qualified subfields, varying fields, procedures.
- **Tools:**
  - `as4tainr-native-emit` (native-tools), with docs/native/production.md;
  - `scripts/native-diff.sh`, `scripts/native-e2e.sh` (the 5250 tests through Playwright) and `scripts/native-bench.sh` (NBENCH1, NBENCH2).
- **Evidence:** docs/native/evidence/2026-11-14-rebased2.json (0b9d585, both platforms).
- **Upstream:** this fork's issues #3 to #8 and PRs #9 to #17 are in kyleturley-cyber/AS4tainr.
  - Merged: #9, #10 and #13.
  - #11, #12 and #14 were updated after review.
  - #15, #16 and #17 are new.
  - #12 must be rebased once #11 is merged; their check.rs changes conflict.
  - The PR branches are `fix/*`, in worktrees /home/ctl/data/as4tainr-up*. Validate and publish them with /home/ctl/data/as4tainr-up-logs/vp.sh.
- The PRs and issues are framed as collaboration only: no mention of this fork or the native backend (the user's decision).

## Environment

- **Machines:**
  - this host: x86_64, 3.8 GB, short of memory under parallel cargo work;
  - jsr-test-arm: aarch64, work under /home/ctl/data.
  - Both have PostgreSQL 16 in podman `as4tainr-test-db`, with `--shm-size=1g`.
  - PUB400 is reachable over ssh (`ssh pub400`) from both.
  - Playwright and headless Chromium are on the VM, in /home/ctl/data/pw-venv and /home/ctl/data/pw-browsers.
- **Pushing to the fork:** `git -c credential.helper= -c credential.helper='!gh auth git-credential' push origin <branch>`. There is no GitHub ssh key.
- **Syncing a tree to the VM:** use `rsync -rlp --checksum`, not `-a`. Kept mtimes make cargo keep stale release builds of rpgcg and rpgrt.

## Known failures (not the backend's)

- clippy `chunks_exact_to_as_chunks` in the host servers, with rustc 1.99.1.
- `save_changed_objects` is intermittent under parallel runs.
- `host_servers_sign_on_and_database_attributes` and `ifs_statistics_json_defaults_and_new_activation_groups` hung once under the parallel release run on x86_64. Both pass alone.
- A release build with 2 jobs can run out of memory on the VM. Use CARGO_BUILD_JOBS=1.

## Next steps

2. Block A of stage 2: %EDITC/%EDITW, %REPLACE, %CHECK, %SCANRPL, %LOOKUP, figurative constants, whole data structure assignment, CALLP. EXSR without run_code_at needs the user's approval, because it touches interp.rs.
3. Priority 3:
   - a production image built with native-tools;
   - OrderDesk under `rust`;
   - gate3 (three clerks at once) under `rust`.
4. Follow the upstream PRs: answer reviews, and rebase #12 after #11.

## Continuing on a Mac M1 (2026-11-14)

The user can go on with the most blocking parts on a Mac M1, which has more memory than jsr-test-arm.

- **Get the code.** Clone guigra/AS4tainr and check out `native/engine`. It stacks on `native/runtime-extraction` and is rebased on upstream dd47e28. Add the remote `upstream` = kyleturley-cyber/AS4tainr.
- **Where to run it.** The scripts and tests assume Linux:
  - podman;
  - `/QSYS.LIB` present;
  - mount namespaces for QSH (`qshfs.rs`);
  - /proc in places.

  The safest setup is a Debian aarch64 VM on the Mac (UTM, Lima or `podman machine`), set up like jsr-test-arm:
  - rustup with the toolchain of `rust-toolchain.toml` (1.99.1);
  - PostgreSQL 16 in podman: `podman run -d --name as4tainr-test-db --shm-size=1g -p 127.0.0.1:5432:5432 -e POSTGRES_USER=as4tainr -e POSTGRES_PASSWORD=as4tainr -e POSTGRES_DB=as4tainr docker.io/library/postgres:16`;
  - `sudo mkdir /QSYS.LIB` (empty, owned by the user).

  Native macOS may build, but it is not tested here.
- **PUB400.** A new ed25519 key without a passphrase, installed in /home/GUIGRA26/.ssh/authorized_keys as `docs/native/pub400.md` says, and a `Host pub400` entry. Never copy keys or passwords through Git.
- **Playwright** (for `scripts/native-e2e.sh`): a Python venv with `pip install playwright` and `python -m playwright install chromium`.
- **Checks.** The commands in CLAUDE.native.md section 7. Development runs: `AS4TAINR_RPG_ENGINE=rust scripts/native-diff.sh`, `scripts/native-bench.sh` (NATIVE_BENCH_PGM=NBENCH2 too), `scripts/native-e2e.sh`.
- **The upstream hang (issue #18).** Full suite runs, native-diff included, can hang in `catalog::init`'s lock chain. While a run hangs, `pg_stat_activity` shows an `ALTER TABLE as4tainr.objects ADD COLUMN IF NOT EXISTS ...` waiting behind an `idle in transaction` session. Stop the `rla_test` process and run again. The fix is the held branch `fix/init-lock-timeout`. Carrying it on `native/engine` needs the user's approval: it touches core/src/catalog.rs.
- **First things to do there:**
  1. Validate S2.25: clippy, the native units, native-diff.
  2. Stage 2 block A, in this order:
     - CALLP and procedure calls;
     - %EDITC and %EDITW;
     - %LOOKUP;
     - %SCANRPL;
     - data structure arrays.
  3. Priority 3:
     - a production image with native-tools;
     - OrderDesk under `rust`;
     - gate3 under `rust`.
- **Upstream branches on origin, held** (no PRs until the work in progress is done, the user's decision):
  - `fix/figurative-varying-hival`;
  - `fix/replace-start`;
  - `fix/init-lock-timeout` (fixes issue #18);
  - `fix/size-numeric-constant`.

  The open PRs #11, #12 and #14 to #17 were being re-validated on x86_64 with a clean build per branch. Their earlier validations ran another branch's build: see the note in native-test-environment memory. Kyle must get a short note about it.

## Mac M1 session (2026-11-14)

- **Policy change (the user's decision, 2026-11-14).** Work only with guigra/AS4tainr. Publish nothing to kyleturley-cyber/AS4tainr: no issues, no PRs, no comments, no `upstream` remote pushes. This replaces "Follow the upstream PRs" (next steps, item 4), the `upstream` remote of "Get the code" and the note to Kyle above. Fixes go to this fork's `main` by PRs inside the fork.
- **Environment.** No Linux VM of its own: the podman machine (applehv, raised to 8 GiB, 6 CPUs) runs two containers:
  - `as4tainr-test-db`: PostgreSQL 16, `--shm-size=1g`, published on the Mac at 127.0.0.1:55432 (5432 is taken by a local PostgreSQL);
  - `as4tainr-dev`: Debian bookworm aarch64 with rustc 1.99.1, `--privileged`, `/QSYS.LIB`, the repository at /work, `target` in the volume `as4tainr-target`, on the database's network so 127.0.0.1:5432 works as in the scripts. Built from ~/.local/share/as4tainr-dev/Containerfile (outside Git).
  - Git does not work inside the container for worktrees (their `.git` points to the Mac path); cargo and the scripts do not need it.
  - With 6 GiB and CARGO_BUILD_JOBS=4 the kernel killed rustc building core's tests. Use 8 GiB and CARGO_BUILD_JOBS=3.
- **S2.25 validated on aarch64.** fmt PASS; clippy FAIL only on the upstream lint, PASS without it; native units 34/34 under interp and under rust; native-diff PASS (385 routines, 654 tests the same under auto). Evidence: docs/native/evidence/2026-11-14-s225-aarch64-mac.json.
- **Issue #18 reproduced here** on the first native-diff that ran tests (53 min hung). native-diff was run with `fix/init-lock-timeout` (79d5c03) applied without a commit, the user's decision; `native/engine` does not carry it. The fix was checked here: its test passes with it (3.2 s) and fails without it on main (34.6 s, the test's 30 s limit). It is proposed to this fork's main as https://github.com/guigra/AS4tainr/pull/1.
- **Not run here:** the full suite and release outside native-diff, the release build, native-e2e (no Playwright yet), golden (PUB400 not set up yet), anything on x86_64.
- **Next:** once PR #1 is merged, decide how `native/engine` picks it up (it touches core/src/catalog.rs, outside section 3's touch points). Then stage 2 block A, as listed above.

## Mac M1, S2.30 validation (2026-11-14)

Tasks from jsr-test's mac-tareas-2026-11-14.md, run at 8e212bd (S2.29 %EDITW, S2.30 subfields of data structure arrays) with the catalog::init lock fix (`git diff origin/main origin/fix/init-lock-timeout -- core/src/catalog.rs`) applied without a commit and removed before committing. There is no `upstream` remote here: origin/main is the same base (dd47e28).

| Step | Status |
| --- | --- |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --locked --all-targets -- -D warnings` | FAIL: upstream `chunks_exact_to_as_chunks` only (hostcmd.rs, hostdb.rs, hostifs.rs); PASS with that lint allowed |
| native_tripwire, native_units, native_engine | PASS: 1, 39, 2 |
| `AS4TAINR_RPG_ENGINE=rust scripts/native-diff.sh` | PASS: reference 658 results, 390 routines, level 1 identical, 658 the same under auto, 636 of 658 under rust (23 programs refused, none generated), demo the same; golden NOT_RUN (no PUB400 here) |
| `cargo test --locked --release` | PASS: 657 passed, 0 failed, 1 ignored (CARGO_BUILD_JOBS=2) |
| `cargo build --locked --release` | PASS: target/release/as4tainr SHA-256 83e217bb…694e |

save_changed_objects failed under interp and auto in native-diff (the known intermittent upstream failure) and passed in the release run. Evidence: docs/native/evidence/2026-11-14-s230-aarch64-mac.json; logs in the podman volume as4tainr-target, target/evidence/2026-11-14-s230-aarch64-mac.

Item 6 of the task file (an upstream PR for #18 from the x86 machine, then closing this fork's PR #1) was not done here: this session works only against guigra/AS4tainr (the user's decision), and PR #1 stays open until the user says otherwise.

## To look at (2026-11-14, x86_64)

- %STATUS after a call: on PUB400, %STATUS was 0 again after a call to a procedure that runs EXEC SQL (the first version of tools/golden/native40, saved in target/ibmi/golden as n40_stat before it was captured again). It is not known yet what sets it to 0 (the call, or the SQL statement) or whether the interpreter does the same. Capture both cases before changing anything.

## 2026-11-14, x86_64: S2.31-S2.34 and the x86_64 validation

- New steps: S2.31 %STATUS with no file, S2.32 %ABS, S2.33 %MAX/%MIN, S2.34 %CHECK/%CHECKR with no start (ADR 0006). PUB400 captures native40-43. New interpreter defect: %MAX/%MIN keep the chosen argument's scale; IBM i uses the arguments' greatest (fidelity.md).
- x86_64 at f89d678 (evidence/2026-11-14-s234-x86_64.json): native-diff PASS; release tests 632 passed. The 3 failures were leftovers in the shared test database, and they pass on a new one. Use a new database for evidence runs.
- aarch64 VM: S2.23-S2.28 and S2.29 development runs. native-diff PASS, 5250 end-to-end the same, OpenRPG 384/385 (test396: the interpreter's %SCANRPL panic).
- Upstream: nothing published. Five fix branches are validated and held, and there is an issue draft for %EDITW. The user decides when to publish (as4tainr-up-logs/revision-2026-11-14.md).
- Next: wait for the user's decisions: when to publish, the branch and worktree clean-up, and the next block (CALLP, then block B).

## 2026-11-15: the goal is a fully native engine (ADR 0007)

- The user's requirement: the `rust` engine must become 100% native, so the interpreter could be removed. Option A: behaviour that exists only in interp.rs is written again in our code (native_rt.rs and rpgrt), checked by the differential suite. Upstream code is not moved.
- Phase A: lower every instruction, with a dispatch count down to 0. Phase B: a runtime in rpgrt with no `Machine`.
- Next step: S2.35, native procedure calls within the module (ADR 0007, "S2.35"). First the PUB400 captures for its tests, then `enter_proc`/`leave_proc` in native_rt.rs, the IR for a call, and lowering.

## 2026-11-15 (later): S2.35-S2.43, the rules audit, coverage

- Done:
  - S2.35 native calls;
  - S2.36 DSPLY;
  - S2.37 no interpreter fallback;
  - S2.38 RETURN;
  - S2.39 indicators;
  - S2.40 EXSR;
  - S2.41 chains with no recursion, and IBM i's nesting limit of 100 with a refusal;
  - S2.42 named constants;
  - S2.43 the error path for program errors.
  
  See ADR 0007.
- Section 5 audit (d3ab6d7):
  - the generated code is now linted (native-diff's generated-lints step);
  - eprintln!, `as` casts and Index::index are fixed;
  - the refusals set is bounded.
  
  The user's decisions: errors stay anyhow (documented exception), and unwrap is allowed in tests.
- Section 7 before every commit: as4tainr-up-logs/section7.sh <snapshot> <tag> runs the full list on a `git stash create` snapshot, in its own worktree, on a new database, with the #18 fix applied without a commit.
- core/tests/native_coverage.rs:
  - each unit's count of interpreter instructions may not rise (a parity test cannot see a lowering that stops applying);
  - 100 nesting levels are generated, 101 are refused.
- Coverage of the OpenRPG corpus: 79.3% native at S2.39.
- PUB400 is unreachable for a few days (the IP is blocked, maybe by today's many captures). Space the captures out when it is back.
- Held for Kyle (publish at the end):
  - the five validated branches;
  - the %EDITW issue;
  - an issue and PR on expression nesting: over 100 levels compiles, and 20 000 aborts the process. The PR needs a PUB400 capture of the RNF ID.
- Next:
  - commit the chains block once s7-chain passes;
  - run section 7 on S2.42-S2.43 and commit;
  - rerun the VM bench with CARGO_BUILD_JOBS=1 (it was OOM-killed);
  - then %SUBST and %ELEM as targets, the remaining EVAL forms, and file I/O.
- Correction: f402885's message says AS4TAINR_NATIVE_WHY also writes CALLP and DSPLY shapes. That change was not in f402885; it is in the S2.46 commit.

## 2026-11-15 (night): state after S2.46, next is pointers

- Validated and pushed today, each with its full section 7 split between
  x86_64 and the VM:
  - 5b6f0c1 S2.41;
  - e2a1fb3 S2.42-S2.44;
  - f402885 S2.45;
  - d87059c S2.46.
  
  Corpus coverage is 72.8% native.
- AS4TAINR_NATIVE_WHY on the full suite: what is left is mostly pointers (EVAL targets of type pointer, %ADDR, pointer fields), calls through procedure pointers and C/CEE functions, file I/O and SQL.
- Plan for pointers:
  - S2.47:
    - pointer fields as operands (assignment, *NULL, comparisons; FigOp::Null);
    - %ADDR of global variables, and %ADDR(v:*DATA), with ptr_to written again in native_rt.rs and watched by the tripwire;
    - pointers kept out of numeric operands.
  - S2.48: BASED variables, read and written through `addr` with its checks.
  - Then pointer arithmetic and %ALLOC/%REALLOC/DEALLOC.
- PUB400: one connection at a time, captures batched (memory pub400-usage).

## 2026-11-16 (early): S2.47 done, next is BASED (S2.48)

- b0e8963 S2.47 (pointer fields, *NULL, %ADDR, room_for_based) was validated by a full section 7, split between x86_64 and the VM on snapshot 437b6f9, and pushed. Corpus coverage is 73.3% native.
- Two native-diff findings during S2.47, both fixed in b0e8963:
  - the pointer owner (an activation number) differs between runs, so the observation now clears it;
  - room_for_based was missing in the native store of a pointer.
- Lesson: never launch section 7 while a pre-check fails. The first S2.47 launch went out with a clippy error and had to be stopped.
- Next, S2.48: BASED variables read and written through the pointer (`addr_n` with its MCH3601/MCH0601 checks), with a PUB400 capture in one batch. Redo the nested-%ABS capture in the same batch.
