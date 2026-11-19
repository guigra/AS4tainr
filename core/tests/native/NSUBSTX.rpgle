**FREE
// Native unit NSUBSTX (S2.122): %SUBST with expressions as its start
// and length, the program captured on PUB400 as GN122SB with DSPLY for
// its output.
// Source: golden capture tools/golden/native122 n122_subst (PUB400,
// IBM i 7.5, 2026-11-19).
ctl-opt dftactgrp(*no);
dcl-s s char(20) inz('abcdefghijklmnopqrst');
dcl-s n int(10) inz(3);
dcl-s r varchar(20);
dcl-ds d qualified;
  x char(5) inz('12345');
  y char(5) inz('67890');
end-ds;
dcl-s line varchar(80);

line = %subst(s: n * 2 + 1: n + 1);
line += ' ' + %subst(s: %len(%trimr(s)) - n: 2);
line += ' ' + %subst(d: n + 2: n * 2);
monitor;
  r = %subst(s: n - 3: 1);
on-error;
  r = 'err' + %char(%status);
endmon;
line += ' ' + r;
dsply (line);
*inlr = *on;
