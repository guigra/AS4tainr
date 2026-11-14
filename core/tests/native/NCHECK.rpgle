**FREE
// Stage 2, S2.34: %CHECK and %CHECKR with no start, lowered to Rust.
// Expected: what IBM i 7.5 gave (golden capture tools/golden/native43.txt
// n43_chk, cited in native_units.rs).
ctl-opt dftactgrp(*no);
dcl-c DIGITS '0123456789';
dcl-s c char(10) inz('0012345');
dcl-s v varchar(10) inz('12a45');
dcl-s e varchar(10) inz('');
dcl-s b char(8) inz('  ab  ');
dcl-s n int(10);
n = %check(DIGITS:c);
dsply ('CHK C ' + %char(n));
n = %checkr(DIGITS:c);
dsply ('CHKR C ' + %char(n));
n = %checkr(' ':c);
dsply ('CHKR BLANK C ' + %char(n));
n = %check(DIGITS:v);
dsply ('CHK V ' + %char(n));
n = %checkr(DIGITS:v);
dsply ('CHKR V ' + %char(n));
n = %check(DIGITS:'2024');
dsply ('CHK ALL ' + %char(n));
n = %check(DIGITS:e);
dsply ('CHK EMPTY ' + %char(n));
n = %checkr(DIGITS:e);
dsply ('CHKR EMPTY ' + %char(n));
n = %check(' ':b);
dsply ('CHK LEAD ' + %char(n));
n = %checkr(' ':b);
dsply ('CHKR TRAIL ' + %char(n));
n = %check('a':%trim(b) + 'xa');
dsply ('CHK EXPR ' + %char(n));
n = %checkr(' ':c) - %check(DIGITS:v) * 2;
dsply ('CHK ARITH ' + %char(n));
if %check(DIGITS:v) > 0;
  dsply ('CHK COND TRUE');
else;
  dsply ('CHK COND FALSE');
endif;
*inlr = *on;
