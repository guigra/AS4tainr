**FREE
// Native unit NOPNCLS (S2.128): OPEN and CLOSE of a database file, the
// program captured on PUB400 as GN128OC with DSPLY for its output; its
// file is QGPL/GN120F here with the capture's two records.
// Source: golden capture tools/golden/native128 n128_openclose (PUB400,
// IBM i 7.5, 2026-11-19).
ctl-opt dftactgrp(*no);
dcl-f gn120f usage(*input) usropn qualified extfile('QGPL/GN120F');
dcl-ds r likerec(gn120f.gn120f);
dcl-s line varchar(80);

open gn120f;
read gn120f r;
line = %char(r.n);
monitor;
  open gn120f;
  line += ' again';
on-error;
  line += ' ' + %char(%status);
endmon;
close gn120f;
monitor;
  close gn120f;
  line += ' closed';
on-error;
  line += ' ' + %char(%status);
endmon;
open gn120f;
read gn120f r;
line += ' ' + %char(r.n);
close gn120f;
dsply (line);
*inlr = *on;
