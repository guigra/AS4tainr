**FREE
// Stage 2, S2.18: %REM and %DIV, lowered to Rust. Expected: what IBM i 7.5
// gave (golden capture tools/golden/native19.txt n19_rem, cited in
// native_units.rs).
ctl-opt dftactgrp(*no);
dcl-s i int(10) inz(123);
dcl-s m int(10) inz(-17);
dcl-s z int(10) inz(0);
dcl-s n int(10);
n = %rem(17 : 5);
dsply ('REM ' + %char(n));
n = %rem(m : 5);
dsply ('REM NEG ' + %char(n));
n = %div(17 : 5);
dsply ('DIVF ' + %char(n));
n = %div(m : 5);
dsply ('DIVF NEG ' + %char(n));
n = %rem(i : 7) + %div(i : 7) * 7;
dsply ('REM DIV BACK ' + %char(n));
if %rem(i : 2) = 1;
  dsply ('REM COND ODD');
else;
  dsply ('REM COND EVEN');
endif;
monitor;
  n = %rem(i : z);
  dsply ('REM ZERO ' + %char(n));
on-error;
  dsply ('REM ZERO ERR ' + %char(%status));
endmon;
monitor;
  n = %div(i : z);
  dsply ('DIVF ZERO ' + %char(n));
on-error;
  dsply ('DIVF ZERO ERR ' + %char(%status));
endmon;
*inlr = *on;
