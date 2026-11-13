**FREE
// Stage 2, S2.12: %SCAN and %XLATE of character expressions, lowered to
// Rust. Expected: what IBM i 7.5 gave (golden capture
// tools/golden/native9.txt n9_scan, cited in native_units.rs). SCAN PAST
// and SCAN ZERO are back since the interpreter's fix (upstream #9).
ctl-opt dftactgrp(*no);
dcl-s a char(10) inz('ABCABCXYZ');
dcl-s b char(6) inz(' aXbYc');
dcl-s c char(10);
dcl-s n packed(3:0);
dcl-s s4 packed(3:0) inz(4);
dcl-s s11 packed(3:0) inz(11);
dcl-s s0 packed(3:0) inz(0);
n = %scan('BC':a);
dsply ('SCAN ' + %char(n));
n = %scan('BC':a:3);
dsply ('SCAN START ' + %char(n));
n = %scan('Q':a);
dsply ('SCAN NONE ' + %char(n));
n = %scan('XYZ ':a);
dsply ('SCAN BLANK ' + %char(n));
n = %scan('A':a) + 10;
dsply ('SCAN EXPR ' + %char(n));
n = %scan('A':a:s4);
dsply ('SCAN VAR ' + %char(n));
n = %scan('C':%trim(b) + a);
dsply ('SCAN CAT ' + %char(n));
monitor;
  n = %scan('A':a:s11);
  dsply ('SCAN PAST ' + %char(n));
on-error;
  dsply ('SCAN PAST ERR ' + %char(%status));
endmon;
monitor;
  n = %scan('A':a:s0);
  dsply ('SCAN ZERO ' + %char(n));
on-error;
  dsply ('SCAN ZERO ERR ' + %char(%status));
endmon;
c = %xlate('abc':'ABC':%trim(b));
dsply ('XLATE [' + c + ']');
c = %xlate('ABC':'xy':a);
dsply ('XLATE SHORT [' + c + ']');
c = %xlate('A':'z':a:2);
dsply ('XLATE START [' + c + ']');
monitor;
  c = %xlate('A':'z':a:s11);
  dsply ('XLATE PAST [' + c + ']');
on-error;
  dsply ('XLATE PAST ERR ' + %char(%status));
endmon;
*inlr = *on;
*inlr = *on;
