**FREE
// Stage 2, S2.17: / in expressions, lowered to Rust. Expected: what IBM i
// 7.5 gave (golden capture tools/golden/native18.txt n18_div, cited in
// native_units.rs).
ctl-opt dftactgrp(*no);
dcl-s p72 packed(7:2) inz(10);
dcl-s i10 int(10) inz(10);
dcl-s q packed(9:2);
dcl-s z zoned(5:0) inz(0);
dcl-s c char(40);
c = %char(10 / 3);
dsply ('DIV LIT [' + c + ']');
c = %char(100 / 3);
dsply ('DIV LIT100 [' + c + ']');
c = %char(p72 / 3);
dsply ('DIV PACKED [' + c + ']');
c = %char(i10 / 3);
dsply ('DIV INT [' + c + ']');
q = p72 / 3;
dsply ('DIV ASSIGN ' + %char(q));
eval(h) q = 2 / 3;
dsply ('DIV HALF ' + %char(q));
q = (p72 + 5) / 4 * 2;
dsply ('DIV EXPR ' + %char(q));
monitor;
  q = p72 / z;
  dsply ('DIV ZERO ' + %char(q));
on-error 00102;
  dsply ('DIV ZERO 00102');
endmon;
*inlr = *on;
