**FREE
// Native unit NFORLIM (S2.146): FOR limits with arithmetic, the program
// captured on PUB400 as GN146FL with DSPLY for its output.
// Source: golden capture tools/golden/native146 n146_forlim (PUB400,
// IBM i 7.5, 2026-11-20).
ctl-opt dftactgrp(*no);
dcl-s s varchar(10) inz('abcde');
dcl-s i int(10);
dcl-s n packed(3:0) inz(4);
dcl-s line varchar(80);

for i = 1 to %len(s) - 1;
  line += %subst(s: i: 1);
endfor;
for i = n * 2 downto n - 1;
  line += %char(i);
endfor;
for i = 0 to 10 - n * 2;
  line += %char(i);
endfor;
dsply (line);
*inlr = *on;
