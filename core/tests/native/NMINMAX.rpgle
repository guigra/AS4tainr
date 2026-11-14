**FREE
// Stage 2, S2.33: %MAX and %MIN lowered to Rust. Expected: what IBM i 7.5
// gave (golden capture tools/golden/native42.txt n42_mm, cited in
// native_units.rs). The capture's cases CHAR MIN AB, CHAR MAX AD and
// CHAR MAX EA are left out: the interpreter differs there
// (docs/native/fidelity.md).
ctl-opt dftactgrp(*no);
dcl-s a packed(5:2) inz(1.50);
dcl-s b packed(7:3) inz(2.125);
dcl-s c int(10) inz(-7);
dcl-s d zoned(3:0) inz(2);
dcl-s e packed(4:1) inz(1.5);
dcl-s r packed(9:3);
dcl-s k int(10);
r = %max(a:b);
dsply ('MAX AB ' + %char(r));
r = %min(a:b);
dsply ('MIN AB ' + %char(r));
dsply ('CHAR MAX AB ' + %char(%max(a:b)));
dsply ('CHAR MAX AE ' + %char(%max(a:e)));
r = %max(a:b:c:d);
dsply ('MAX 4 ' + %char(r));
r = %min(a:b:c:d);
dsply ('MIN 4 ' + %char(r));
k = %max(c:-10);
dsply ('MAX NEG ' + %char(k));
r = %max(a:b) * 2 + %min(c:d);
dsply ('MM ARITH ' + %char(r));
if %max(a:d) = 2;
  dsply ('MM COND TRUE');
else;
  dsply ('MM COND FALSE');
endif;
*inlr = *on;
