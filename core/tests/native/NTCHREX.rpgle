**FREE
// Native unit NTCHREX (S2.134): %CHAR of temporal expressions under the
// default formats, the program captured on PUB400 as GN134TI with DSPLY
// for its output.
// Source: golden capture tools/golden/native134 n134_tchariso (PUB400,
// IBM i 7.5, 2026-11-19).
ctl-opt dftactgrp(*no);
dcl-s d date inz(d'2026-01-31');
dcl-s t time inz(t'09.15.00');
dcl-s ts timestamp inz(z'2026-01-31-23.59.59.000000');

dsply (%char(t + %minutes(45)));
dsply (%char(ts + %hours(12)));
dsply (%char(%date('10/20/26' : *mdy)));
dsply (%char(d + %days(1) : *usa));
dsply (%char(d - %months(1)));
*inlr = *on;
