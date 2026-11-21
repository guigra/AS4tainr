**FREE
// Native unit NUCS2 (S2.206): UCS-2 fields with character data, the
// program captured on PUB400 as GN206UC with DSPLY for its output.
// Source: golden capture tools/golden/native206 n206_ucs2 (PUB400,
// IBM i 7.5, 2026-11-21).
ctl-opt dftactgrp(*no);
dcl-s u ucs2(6);
dcl-s v varucs2(10);
dcl-s line varchar(200);

u = 'abc';
v = 'Bern';
line = '[' + %char(u) + '][' + %trim(v) + '][' + %char(v) + ']';
v = 'Zurich';
line += ' ' + %trim(v);
if u = 'abc';
  line += ' EQ';
endif;
dsply (line);
*inlr = *on;
