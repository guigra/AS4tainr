**FREE
// Native unit NCFLOAT (S2.201): a C function's float parameters and
// result, the program captured on PUB400 as GN201PW with DSPLY for its
// output.
// Source: golden capture tools/golden/native201 n201_pow (PUB400,
// IBM i 7.5, 2026-11-21).
ctl-opt dftactgrp(*no);
dcl-pr pow float(8) extproc('pow');
  x float(8) value;
  y float(8) value;
end-pr;
dcl-s line varchar(200);

if pow(2 : 10) = 1024;
  line = 'A';
endif;
if pow(2.5 : 2) = 6.25;
  line += 'B';
endif;
if pow(9 : 0.5) <> 3;
  line += 'C';
else;
  line += 'c';
endif;
dsply (line);
*inlr = *on;
