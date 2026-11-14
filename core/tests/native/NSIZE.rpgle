**FREE
// Stage 2, S2.25: %SIZE and %ELEM, lowered to Rust as constants. Expected:
// what IBM i 7.5 gave (golden capture tools/golden/native32.txt n32_size,
// cited in native_units.rs). The capture's
// case SIZE CONST NUM is left out: the interpreter differs there
// (docs/native/fidelity.md).
ctl-opt dftactgrp(*no);
dcl-s c char(7);
dcl-s p packed(7:2);
dcl-s i int(10);
dcl-s arr char(3) dim(5);
dcl-ds d;
  a char(2);
  b packed(5:0);
end-ds;
dcl-c KCHR 'ABCD';
dcl-s n int(10);
n = %size(c);
dsply ('SIZE CHAR ' + %char(n));
n = %size(p);
dsply ('SIZE PACKED ' + %char(n));
n = %size(i);
dsply ('SIZE INT ' + %char(n));
n = %size(arr);
dsply ('SIZE ELEMENT ' + %char(n));
n = %size(arr:*all);
dsply ('SIZE ALL ' + %char(n));
n = %size(d);
dsply ('SIZE DS ' + %char(n));
n = %size(KCHR);
dsply ('SIZE CONST CHAR ' + %char(n));
n = %elem(arr);
dsply ('ELEM ' + %char(n));
n = %size(arr:*all) - %elem(arr);
dsply ('SIZE EXPR ' + %char(n));
if %size(c) = 7;
  dsply ('SIZE COND YES');
else;
  dsply ('SIZE COND NO');
endif;
*inlr = *on;
