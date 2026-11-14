**FREE
// Stage 2, S2.31: %STATUS with no file, lowered to Rust: before any error,
// after program errors in MONITOR groups, after statements that end
// normally, in arithmetic and in a condition. Expected: what IBM i 7.5
// gave (golden capture tools/golden/native40.txt n40_stat, cited in
// native_units.rs).
ctl-opt dftactgrp(*no);
dcl-s n packed(5:0);
dcl-s z packed(5:0) inz(0);
dcl-s a packed(5:0) dim(3);
dcl-s i int(10) inz(4);
dcl-s s varchar(10) inz('abc');
dcl-s c varchar(10);
dcl-s k int(10) dim(9);
dcl-s t varchar(20);
k(1) = %status;
monitor;
  n = 10 / z;
on-error;
  k(2) = %status;
endmon;
k(3) = %status;
n = 5;
k(4) = %status;
monitor;
  a(i) = 1;
on-error;
  k(5) = %status + 1;
endmon;
monitor;
  c = %subst(s:5:1);
on-error;
  if %status = 100;
    t = 'IS 100';
  else;
    t = 'NOT 100';
  endif;
  k(6) = %status * 2;
endmon;
monitor;
  n = 1;
on-error;
  t = 'NONE';
endmon;
k(7) = %status;
monitor;
  n = 10 / z;
on-error;
endmon;
n = 5;
c = 'x';
k(8) = %status;
dsply ('ST START ' + %char(k(1)));
dsply ('ST DIV ' + %char(k(2)));
dsply ('ST AFTER DIV ' + %char(k(3)));
dsply ('ST AFTER EVAL ' + %char(k(4)));
dsply ('ST INDEX PLUS 1 ' + %char(k(5)));
dsply ('ST SUBST ' + t);
dsply ('ST SUBST TIMES 2 ' + %char(k(6)));
dsply ('ST AFTER MONITOR ' + %char(k(7)));
dsply ('ST AFTER TWO EVALS ' + %char(k(8)));
*inlr = *on;
