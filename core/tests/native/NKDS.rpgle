**FREE
// Native unit NKDS (S2.193): CHAIN with %KDS into a LIKEREC data
// structure, the program captured on PUB400 as GN193KD with DSPLY for its
// output; the file is NKDSPF, S2.58's NKDSPF made in QGPL from the same
// DDS and rows.
// Source: golden capture tools/golden/native193 n193_kds (PUB400,
// IBM i 7.5, 2026-11-21).
ctl-opt dftactgrp(*no);
dcl-f NKDSPF keyed;
dcl-ds rec likerec(GN193FMT);
dcl-ds k qualified;
  o zoned(3 : 0);
  l packed(3 : 0);
end-ds;
dcl-s line varchar(200);

k.o = 1;
k.l = 3;
chain %kds(k) NKDSPF rec;
line = %trim(rec.name) + ' ' + %char(%found(NKDSPF));
k.o = 2;
k.l = 7;
chain %kds(k : 1) NKDSPF rec;
line += ' ' + %trim(rec.name) + ' ' + %char(rec.lin) + ' ' + %char(%found(NKDSPF));
k.o = 9;
chain %kds(k) NKDSPF rec;
line += ' ' + %trim(rec.name) + ' ' + %char(%found(NKDSPF));
line += ' [' + %trim(name) + ']';
dsply (line);
*inlr = *on;
