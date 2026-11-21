**FREE
// Native unit NCFLTEX (S2.202): float expressions for C functions' float
// parameters, the program captured on PUB400 as GN202CF with DSPLY for
// its output.
// Source: golden capture tools/golden/native202 n202_cfloat (PUB400,
// IBM i 7.5, 2026-11-21).
ctl-opt dftactgrp(*no);
dcl-pr QXXDTOZ extproc('QXXDTOZ');
  target pointer value;
  digits int(10) value;
  frac int(10) value;
  value float(8) value;
end-pr;
dcl-pr pow float(8) extproc('pow');
  x float(8) value;
  y float(8) value;
end-pr;
dcl-s zoned zoned(7:2);
dcl-s f float(8) inz(-3.25);
dcl-s g float(8) inz(3);
dcl-s line varchar(200);

QXXDTOZ(%addr(zoned) : 7 : 2 : 12.5e0);
line = %char(zoned);
QXXDTOZ(%addr(zoned) : 7 : 2 : f);
line += ' ' + %char(zoned);
if pow(g : 2) = 9;
  line += ' P';
endif;
dsply (line);
*inlr = *on;
