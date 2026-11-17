**FREE
// Stage 2, S2.55, for parity between the engines only (native_diff.rs):
// a duration on the left of + (%YEARS(2) + D1), and a date past
// 9999-12-31 (IBM i: status 113, golden capture n56_dur; the interpreter
// gives 112). IBM i does not compile it
// (RNF7421, golden capture tools/golden/native56.txt n56_crt_first,
// PUB400, IBM i 7.5, 2026-11-16); the interpreter adds the duration, and
// the native engine does the same (fidelity.md).
ctl-opt dftactgrp(*no);
dcl-s d1 date(*iso);
dcl-s d2 date(*iso);
d1 = %date('2026-03-31' : *iso);
d2 = %years(2) + d1;
dsply ('YEARS FIRST ' + %char(d2));
d1 = %date('9999-12-31' : *iso);
monitor;
  d2 = d1 + %days(1);
  dsply ('OVER OK');
on-error;
  dsply ('OVER ERROR ' + %char(%status));
endmon;
*inlr = *on;
