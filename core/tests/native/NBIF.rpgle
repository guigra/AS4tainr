**FREE
// Stage 2, S2.9: %TRIM, %TRIML, %TRIMR and %LEN of character fields,
// lowered to Rust. Expected: what IBM i 7.5 gave (golden capture
// tools/golden/native6.txt n6_bif, cited in native_units.rs).
ctl-opt dftactgrp(*no);
dcl-s a char(6) inz('  AB  ');
dcl-s b char(3) inz('XY');
dcl-s c char(10);
dcl-s z char(4);
dcl-s n packed(5:0);
c = %trim(a);
dsply ('TRIM [' + c + ']');
c = %triml(a);
dsply ('TRIML [' + c + ']');
c = %trimr(a);
dsply ('TRIMR [' + c + ']');
c = %trimr(b) + %trim(a);
dsply ('TRIM CAT [' + c + ']');
c = %trim(z) + 'Q';
dsply ('TRIM BLANKS [' + c + ']');
evalr c = %trim(a);
dsply ('TRIM EVALR [' + c + ']');
n = %len(a);
dsply ('LEN ' + %char(n));
n = %len(a) + %len(b) * 2;
dsply ('LEN EXPR ' + %char(n));
*inlr = *on;
