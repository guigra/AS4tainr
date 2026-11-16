**FREE
// Stage 2, S2.49: pointer arithmetic, lowered to Rust. Expected: what IBM i
// 7.5 gave (golden capture tools/golden/native50.txt n50_pa, cited in
// native_units.rs). *NULL plus a number is NPNULL: the interpreter differs
// from IBM i there (fidelity.md).
ctl-opt dftactgrp(*no);
dcl-s buf char(10) inz('ABCDEFGHIJ');
dcl-s p pointer;
dcl-s q pointer;
dcl-s c char(1) based(p);
dcl-s k int(10) inz(3);
dcl-s d int(10);
p = %addr(buf) + 1;
dsply ('PLUS [' + c + ']');
p = p + k;
dsply ('FIELD [' + c + ']');
p = p - 2;
dsply ('LESS [' + c + ']');
p = %addr(buf) + 9 - 4 + 1;
dsply ('CHAIN [' + c + ']');
q = %addr(buf);
d = p - q;
dsply ('DIFF ' + %char(d));
*inlr = *on;
