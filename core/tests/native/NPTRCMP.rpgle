**FREE
// Native unit NPTRCMP (S2.160): comparisons of pointer expressions, the
// program captured on PUB400 as GN160PC with DSPLY for its output.
// Source: golden capture tools/golden/native160 n160_ptrcmp (PUB400,
// IBM i 7.5, 2026-11-20).
ctl-opt dftactgrp(*no);
dcl-s buf char(20);
dcl-s p pointer;
dcl-s line varchar(80);

p = %addr(buf);
if %addr(buf) + 1 < %addr(buf) + 2;
  line = 'lt';
endif;
if p + 5 > p;
  line += ' gt';
endif;
if p + 0 = %addr(buf);
  line += ' eq';
endif;
if %addr(buf) + 3 = p + 4;
  line += ' bad';
endif;
if %addr(buf) <> *null;
  line += ' nn';
endif;
dsply (line);
*inlr = *on;
