**FREE
// Stage 2, S2.32: %ABS lowered to Rust: packed, zoned and integer fields,
// zero, an expression, arithmetic, a condition, and the smallest int(10)
// into int(20) and into int(10) (status 00103). Expected: what IBM i 7.5
// gave (golden capture tools/golden/native41.txt n41_abs, cited in
// native_units.rs).
ctl-opt dftactgrp(*no);
dcl-s p packed(5:2) inz(-123.45);
dcl-s q packed(5:2) inz(67.89);
dcl-s z zoned(7:3) inz(-0.125);
dcl-s i int(10) inz(-42);
dcl-s m int(10) inz(-2147483648);
dcl-s r packed(9:3);
dcl-s k int(10);
dcl-s u uns(5);
dcl-s w int(20);
r = %abs(p);
dsply ('ABS P ' + %char(r));
r = %abs(q);
dsply ('ABS Q ' + %char(r));
r = %abs(z);
dsply ('ABS Z ' + %char(r));
k = %abs(i);
dsply ('ABS I ' + %char(k));
r = %abs(p - q);
dsply ('ABS DIFF ' + %char(r));
r = %abs(p) + %abs(z) * 2;
dsply ('ABS ARITH ' + %char(r));
r = %abs(0);
dsply ('ABS ZERO ' + %char(r));
u = %abs(i);
dsply ('ABS UNS ' + %char(u));
if %abs(p) > 100;
  dsply ('ABS COND TRUE');
else;
  dsply ('ABS COND FALSE');
endif;
w = %abs(m);
dsply ('ABS MIN INT20 ' + %char(w));
monitor;
  k = %abs(m);
  dsply ('ABS MIN INT10 ' + %char(k));
on-error;
  dsply ('ABS MIN INT10 ERR ' + %char(%status));
endmon;
*inlr = *on;
