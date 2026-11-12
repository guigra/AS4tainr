**FREE
// Stage 2, S2.10: %SUBST with constant arguments out of range, to see
// what the compiler says (tools/golden/native7.txt n7_crt_const).
ctl-opt dftactgrp(*no);
dcl-s a char(10) inz('ABCDEFGHIJ');
dcl-s c char(5);
c = %subst(a:11);
c = %subst(a:12:1);
c = %subst(a:9:5);
*inlr = *on;
