**FREE
// Native unit NFLTEVL (S2.106, ADR 0008): EVAL of float fields to
// decimal values, the program captured on PUB400 as GN107FEV with DSPLY
// for its output.
// Source: golden capture tools/golden/native107 n107_floateval (PUB400,
// IBM i 7.5, 2026-11-19).
ctl-opt dftactgrp(*no);
dcl-s f8 float(8);
dcl-s f4 float(4);
dcl-s n int(10) inz(7);
dcl-s p packed(7:2) inz(12.25);
dcl-s line varchar(200);
f8 = p * 3;
line = %char(f8);
f8 = 10 / 4;
line += ' ' + %char(f8);
f4 = n;
line += ' ' + %char(f4);
f4 = -2.5;
line += ' ' + %char(f4);
line += ' ' + local();
dsply (line);
*inlr = *on;

dcl-proc local;
  dcl-pi *n varchar(40);
  end-pi;
  dcl-s lf float(8);
  lf = 1 / 3;
  return %char(lf);
end-proc;
