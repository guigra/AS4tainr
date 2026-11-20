**FREE
// Native unit NFLTABS (S2.162): %ABS of float expressions, the program
// captured on PUB400 as GN162FA with DSPLY for its output.
// Source: golden capture tools/golden/native162 n162_fltabs (PUB400,
// IBM i 7.5, 2026-11-20).
ctl-opt dftactgrp(*no);
dcl-s f float(8);
dcl-s d float(8);
dcl-s g float(4);
dcl-s line varchar(120);

f = -2.5E0;
d = %abs(f);
line = %char(d);
d = %abs(f * 3 - 1);
line += ' ' + %char(d);
g = %abs(f / 4);
line += ' ' + %char(g);
f = -1.797693134862315E+308;
d = %abs(f - f / 2);
if f < 0 and d > 0;
  line += ' huge';
endif;
dsply (line);
*inlr = *on;
