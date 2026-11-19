**FREE
// Native unit NINDFIG (S2.126): indicators compared with *ON and *OFF,
// the program captured on PUB400 as GN126IF with DSPLY for its output.
// Source: golden capture tools/golden/native126 n126_indfig (PUB400,
// IBM i 7.5, 2026-11-19).
ctl-opt dftactgrp(*no);
dcl-s n int(10);
dcl-s line varchar(80);

*in50 = *on;
*in51 = *off;
if *in50 = *on;
  line += 'a';
endif;
if *in51 = *on;
  line += 'b';
endif;
if *in51 = *off;
  line += 'c';
endif;
if *off <> *in50;
  line += 'd';
endif;
if *in50 <> *off;
  line += 'e';
endif;
dou *in99 = *on;
  n += 1;
  if n = 3;
    *in99 = *on;
  endif;
enddo;
line += %char(n);
dsply (line);
*inlr = *on;
