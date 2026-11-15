**FREE
// Stage 2, S2.44 (ADR 0007): %SUBST as the target of an EVAL, lowered to
// Rust. Expected values: IBM Docs, "%SUBST Used as the Result of an
// Assignment", IBM i 7.5,
// https://www.ibm.com/docs/en/i/7.5?topic=substring-subst-used-as-result-assignment,
// read 2026-11-15: with A 'abcdefghijklmno', %SUBST(A:3:4) = '****' gives
// 'ab****ghijklmno'; "Any valid expressions are permitted for the second
// and third parameters"; "If the length is not specified then the string
// is referenced to its end"; "If the length refers to characters beyond
// the end of the string, then a run-time error is issued."
ctl-opt dftactgrp(*no);
dcl-s a char(15) inz('abcdefghijklmno');
dcl-s b char(15) inz('abcdefghijklmno');
dcl-s c char(15) inz('abcdefghijklmno');
dcl-s d char(15) inz('abcdefghijklmno');
dcl-s k int(10) inz(2);
%subst(a:3:4) = '****';
dsply ('SUBT DOC [' + a + ']');
%subst(b:k + 1:k * 2) = '****';
dsply ('SUBT EXPR [' + b + ']');
%subst(c:13) = 'XYZ';
dsply ('SUBT END [' + c + ']');
monitor;
  %subst(d:14:5) = 'QRSTU';
  dsply ('SUBT PAST [' + d + ']');
on-error;
  dsply ('SUBT PAST ERR');
endmon;
*inlr = *on;
