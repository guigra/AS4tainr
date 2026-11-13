**FREE
// Stage 2, S2.19: varying-length character fields, lowered to Rust.
// Expected: what IBM i 7.5 gave (golden capture tools/golden/native20.txt
// n20_var, cited in native_units.rs).
ctl-opt dftactgrp(*no);
dcl-s v varchar(10);
dcl-s w varchar(5) inz('AB');
dcl-s c char(6);
v = 'HELLO';
dsply ('VAR [' + v + '] ' + %char(%len(v)));
v = v + ' W';
dsply ('VAR CAT [' + v + '] ' + %char(%len(v)));
w = 'TOOLONGVALUE';
dsply ('VAR TRUNC [' + w + '] ' + %char(%len(w)));
v = %trim('  X  ');
dsply ('VAR TRIM [' + v + '] ' + %char(%len(v)));
c = w;
dsply ('VAR TO CHAR [' + c + ']');
v = c;
dsply ('VAR FROM CHAR [' + v + '] ' + %char(%len(v)));
if v = 'TOOLO';
  dsply ('VAR CMP PADDED EQ');
else;
  dsply ('VAR CMP PADDED NE');
endif;
v = %subst(c : 1 : 3);
dsply ('VAR SUBST [' + v + '] ' + %char(%len(v)));
if %scan('O' : v) = 2;
  dsply ('VAR SCAN 2');
else;
  dsply ('VAR SCAN OTHER');
endif;
*inlr = *on;
