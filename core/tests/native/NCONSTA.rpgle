**FREE
// Stage 2, S2.42 (ADR 0007): a named constant's value is part of the
// routine's key. NCONSTA and NCONSTB have the same calculations and a
// constant of a different value; each must show its own (they would
// share generated code if the value were not in the fingerprint).
// Expected: a named constant stands for its value (ILE RPG Reference,
// "Named Constants", IBM i 7.5,
// https://www.ibm.com/docs/ssw_ibm_i_75/rzasd/namcon.htm, read 2026-11-15).
ctl-opt dftactgrp(*no);
dcl-c W 'AAA';
dsply ('CONST ' + W);
*inlr = *on;
