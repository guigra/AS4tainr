**FREE
// Stage 2, S2.14: %CHAR under DECEDIT('0,'), lowered to Rust. Expected:
// what IBM i 7.5 gave (golden capture tools/golden/native12.txt
// n12_decedit, cited in native_units.rs).
ctl-opt dftactgrp(*no) decedit('0,');
dcl-s z zoned(5:2) inz(0.25);
dcl-s d52 packed(5:2) inz(-12.50);
dcl-s c char(12);
c = %char(z);
dsply ('DECEDIT FRACTION [' + c + ']');
c = %char(d52);
dsply ('DECEDIT NEG [' + c + ']');
*inlr = *on;
