**FREE
// Native unit NINLIST (S2.131): IN %RANGE and IN %LIST, the program
// captured on PUB400 as GN131IN with DSPLY for its output.
// Source: golden capture tools/golden/native131 n131_inlist (PUB400,
// IBM i 7.5, 2026-11-19).
ctl-opt dftactgrp(*no);
dcl-s n int(10) inz(5);
dcl-s c char(1) inz('G');
dcl-s line varchar(80);

if n in %range(1 : 10);
  line += 'a';
endif;
if n in %range(6 : 10);
  line += 'b';
endif;
if c in %list('A' : 'G' : 'Z');
  line += 'c';
endif;
if not (c in %list('A' : 'B'));
  line += 'd';
endif;
if n * 2 in %list(3 : 10);
  line += 'e';
endif;
if 'M' in %range('A' : 'Z');
  line += 'f';
endif;
if n in %range(5 : 5);
  line += 'g';
endif;
dsply (line);
*inlr = *on;
