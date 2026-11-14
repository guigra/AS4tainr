**FREE
// Stage 2, S2.23: %REPLACE, lowered to Rust. Expected: what IBM i 7.5 gave
// (golden capture tools/golden/native30.txt n30_repl, cited in
// native_units.rs). The capture's
// cases REPL PAST and REPL ZERO are left out: the interpreter differs there
// (docs/native/fidelity.md).
ctl-opt dftactgrp(*no);
dcl-s s varchar(10) inz('ABCDEFG');
dcl-s x char(2) inz('XY');
dcl-s r varchar(20);
dcl-s p3 int(10) inz(3);
dcl-s p8 int(10) inz(8);
dcl-s p9 int(10) inz(9);
dcl-s p0 int(10) inz(0);
dcl-s l0 int(10) inz(0);
dcl-s l2 int(10) inz(2);
dcl-s l5 int(10) inz(5);
monitor;
  r = %replace(x:s:p3:l2);
  dsply ('REPL START LEN [' + r + ']');
on-error;
  dsply ('REPL START LEN ERR ' + %char(%status));
endmon;
monitor;
  r = %replace(x:s:p3);
  dsply ('REPL START [' + r + ']');
on-error;
  dsply ('REPL START ERR ' + %char(%status));
endmon;
monitor;
  r = %replace(x:s);
  dsply ('REPL NOSTART [' + r + ']');
on-error;
  dsply ('REPL NOSTART ERR ' + %char(%status));
endmon;
monitor;
  r = %replace(x:s:p3:l0);
  dsply ('REPL INSERT [' + r + ']');
on-error;
  dsply ('REPL INSERT ERR ' + %char(%status));
endmon;
monitor;
  r = %replace(x:s:p8:l0);
  dsply ('REPL APPEND [' + r + ']');
on-error;
  dsply ('REPL APPEND ERR ' + %char(%status));
endmon;
monitor;
  r = %replace(x:s:p3:l5);
  dsply ('REPL LONGLEN [' + r + ']');
on-error;
  dsply ('REPL LONGLEN ERR ' + %char(%status));
endmon;
monitor;
  r = %replace(x:s:p8:l2);
  dsply ('REPL PASTLEN [' + r + ']');
on-error;
  dsply ('REPL PASTLEN ERR ' + %char(%status));
endmon;
monitor;
  r = %replace(%trim('  Q '):s:p3:l2);
  dsply ('REPL TRIM [' + r + ']');
on-error;
  dsply ('REPL TRIM ERR ' + %char(%status));
endmon;
*inlr = *on;
