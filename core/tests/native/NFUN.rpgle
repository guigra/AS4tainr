**FREE
// Stage 2, S2.14: %CHAR of numbers, %INT, %INTH, %DEC and %DECH,
// lowered to Rust. Expected: what IBM i 7.5 gave (golden capture
// tools/golden/native12.txt n12_fun, cited in native_units.rs).
ctl-opt dftactgrp(*no);
dcl-s d52 packed(5:2) inz(-1.50);
dcl-s p70 packed(7:0) inz(1234);
dcl-s z zoned(5:2) inz(0.25);
dcl-s c char(12);
dcl-s n packed(5:0);
dcl-s x packed(5:1);
c = %char(d52);
dsply ('CHAR NEG [' + c + ']');
c = %char(p70);
dsply ('CHAR INT [' + c + ']');
c = %char(z);
dsply ('CHAR FRACTION [' + c + ']');
c = %char(p70 * 2);
dsply ('CHAR EXPR [' + c + ']');
c = %char(%int(d52));
dsply ('CHAR INT FN [' + c + ']');
c = %char(%inth(d52));
dsply ('CHAR INTH FN [' + c + ']');
c = 'X' + %char(3);
dsply ('CHAR LIT [' + c + ']');
n = %int(z * 10);
dsply ('INT EXPR ' + %char(n));
n = %inth(z * 10);
dsply ('INTH EXPR ' + %char(n));
x = %dec(d52 : 5 : 1);
dsply ('DEC ' + %char(x));
x = %dech(1.25 : 5 : 1);
dsply ('DECH ' + %char(x));
monitor;
  x = %dec(p70 : 3 : 0);
  dsply ('DEC SMALL ' + %char(x));
on-error;
  dsply ('DEC SMALL ERR ' + %char(%status));
endmon;
if %int(d52 * 2) = -3;
  dsply ('INT COND YES');
else;
  dsply ('INT COND NO');
endif;
*inlr = *on;
