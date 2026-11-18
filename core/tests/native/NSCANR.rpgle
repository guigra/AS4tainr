**FREE
// Native unit NSCANR (S2.67): %SCANR, %LEFT and %RIGHT. %SCANR with and
// without a start and a length, not found, a pattern longer than the part
// searched; %LEFT and %RIGHT of a field and of an expression, with lengths
// 0 and the whole string; a length longer than the string, and a %SCANR
// length past the end, under MONITOR. The program captured on PUB400,
// with DSPLY for its output.
// Source: golden capture tools/golden/native67 n67_str (PUB400, IBM i
// 7.5, 2026-11-17).
ctl-opt dftactgrp(*no);
dcl-s s varchar(20) inz('abcabcabc');
dcl-s f char(8) inz('ABCDEFGH');
dcl-s k int(10);
dcl-s n int(10);
k = %scanr('abc' : s);
dsply ('R1 ' + %char(k));
k = %scanr('abc' : s : 2);
dsply ('R2 ' + %char(k));
k = %scanr('abc' : s : 1 : 6);
dsply ('R3 ' + %char(k));
k = %scanr('x' : s);
dsply ('R4 ' + %char(k));
k = %scanr('abcd' : s : 7);
dsply ('R5 ' + %char(k));
k = %scanr('c' : f + s);
dsply ('R6 ' + %char(k));
n = 3;
dsply ('L1 [' + %left(f : n) + '] [' + %right(f : n) + ']');
dsply ('L2 [' + %left(s : 0) + '] [' + %right(f : 8) + ']');
dsply ('L3 [' + %left(%trim(f) + 'xy' : 9) + ']');
monitor;
  n = 9;
  dsply ('L4 ' + %left(f : n));
on-error;
  dsply ('L4 ERR ' + %char(%status));
endmon;
monitor;
  k = %scanr('a' : s : 5 : 10);
  dsply ('R7 ' + %char(k));
on-error;
  dsply ('R7 ERR ' + %char(%status));
endmon;
*inlr = *on;
