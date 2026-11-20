**FREE
// Native unit NUNSBIF (S2.155): %UNS and %UNSH, the program captured on
// PUB400 as GN155UN with DSPLY for its output.
// Source: golden capture tools/golden/native155 n155_uns (PUB400,
// IBM i 7.5, 2026-11-20).
ctl-opt dftactgrp(*no);
dcl-s u5 uns(5);
dcl-s u3 uns(3);
dcl-s p52 packed(5:2) inz(9.5);
dcl-s c char(6) inz(' 7.6  ');
dcl-s line varchar(80);

u5 = %uns(' 42 ');
line = %char(u5);
u5 = %unsh(c);
line += ' ' + %char(u5) + ' ' + %char(%uns(p52)) + ' ' + %char(%unsh(p52));
line += ' ' + %char(%uns('12.99') + 1);
monitor;
  u5 = %uns('-3');
  line += ' no error';
on-error;
  line += ' ' + %char(%status);
endmon;
monitor;
  u3 = %uns('300');
  line += ' no error';
on-error;
  line += ' ' + %char(%status);
endmon;
dsply (line);
*inlr = *on;
