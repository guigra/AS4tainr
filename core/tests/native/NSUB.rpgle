**FREE
// Stage 2, S2.10: %SUBST of character expressions, lowered to Rust.
// Expected: what IBM i 7.5 gave (golden capture tools/golden/native7.txt
// n7_subst, cited in native_units.rs). SUBST END ZERO is back since the
// interpreter's fix (upstream #9).
ctl-opt dftactgrp(*no);
dcl-s a char(10) inz('ABCDEFGHIJ');
dcl-s b char(6) inz('  XYZ ');
dcl-s c char(5);
dcl-s i packed(3:0) inz(2);
dcl-s l int(10) inz(3);
dcl-s s11 packed(3:0) inz(11);
dcl-s s12 packed(3:0) inz(12);
dcl-s s9 packed(3:0) inz(9);
dcl-s l0 packed(3:0) inz(0);
dcl-s l1 packed(3:0) inz(1);
dcl-s l5 packed(3:0) inz(5);
c = %subst(a:3:4);
dsply ('SUBST [' + c + ']');
c = %subst(a:8);
dsply ('SUBST REST [' + c + ']');
c = %subst(a:i:l);
dsply ('SUBST VARS [' + c + ']');
c = %subst(%trim(b):2:2) + 'Q';
dsply ('SUBST TRIM [' + c + ']');
monitor;
  c = %subst(a:s11:l0);
  dsply ('SUBST END ZERO [' + c + ']');
on-error 00100;
  dsply ('SUBST END ZERO 00100');
on-error;
  dsply ('SUBST END ZERO OTHER ' + %char(%status));
endmon;
monitor;
  c = %subst(a:s12:l1);
  dsply ('SUBST START FAIL');
on-error 00100;
  dsply ('SUBST START 00100');
on-error;
  dsply ('SUBST START OTHER ' + %char(%status));
endmon;
monitor;
  c = %subst(a:s9:l5);
  dsply ('SUBST LEN FAIL');
on-error 00100;
  dsply ('SUBST LEN 00100');
on-error;
  dsply ('SUBST LEN OTHER ' + %char(%status));
endmon;
*inlr = *on;
*inlr = *on;
