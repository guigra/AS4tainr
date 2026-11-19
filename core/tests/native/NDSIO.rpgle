**FREE
// Native unit NDSIO (S2.120): WRITE and UPDATE from a result data
// structure and reads into one, the program captured on PUB400 as
// GN120DU with DSPLY for its output; its file is QGPL/GN120F here.
// Source: golden capture tools/golden/native120 n120_dsiou (PUB400,
// IBM i 7.5, 2026-11-19).
ctl-opt dftactgrp(*no);
dcl-f gn120f usage(*update:*output) rename(gn120f:rec120)
             extfile('QGPL/GN120F');
dcl-ds o likerec(rec120:*output);
dcl-ds r likerec(rec120:*input);
dcl-s line varchar(80);

o.n = 5;
o.s = 'abc';
write rec120 o;
o.n = 6;
o.s = 'de';
write rec120 o;
close gn120f;
open gn120f;
read gn120f r;
r.n = r.n * 10;
r.s = 'zz';
update rec120 r;
close gn120f;
open gn120f;
read gn120f r;
dow not %eof(gn120f);
  line += %char(r.n) + ':' + %trimr(r.s) + ' ';
  read gn120f r;
enddo;
dsply (%trimr(line));
*inlr = *on;
