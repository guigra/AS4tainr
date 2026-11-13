**FREE
// Stage 2, S2.20: %LEN of varying-length fields, lowered to Rust. Expected:
// what IBM i 7.5 gave (golden capture tools/golden/native21.txt n21_vlen,
// cited in native_units.rs).
ctl-opt dftactgrp(*no);
dcl-s v varchar(20);
dcl-s e varchar(8);
dcl-s n int(10);
v = 'ABC';
n = %len(v);
dsply ('VLEN ' + %char(n));
v = v + '  ';
n = %len(v) * 2;
dsply ('VLEN EXPR ' + %char(n));
n = %len(e);
dsply ('VLEN EMPTY ' + %char(n));
if %len(v) > 3;
  dsply ('VLEN COND GT');
else;
  dsply ('VLEN COND LE');
endif;
v = %trimr(v);
dsply ('VLEN TRIMR ' + %char(%len(v)));
*inlr = *on;
