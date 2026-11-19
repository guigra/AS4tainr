**FREE
// Native unit NFLTARI (S2.108, ADR 0008): arithmetic with float
// operands, the program captured on PUB400 as GN108FAR with DSPLY for its
// output.
// Source: golden capture tools/golden/native108 n108_floatarith (PUB400,
// IBM i 7.5, 2026-11-19).
ctl-opt dftactgrp(*no);
dcl-s f8 float(8) inz(1.5);
dcl-s z8 float(8);
dcl-s f4 float(4);
dcl-s p packed(9:3);
dcl-s n int(10);
dcl-s d packed(5:2) inz(10.5);
dcl-s line varchar(200);
f8 = f8 * 2;
line = %char(f8);
f8 = d / f8;
line += ' ' + %char(f8);
f8 = -f8;
line += ' ' + %char(f8);
f4 = %float(3) / 7;
p = f8 * 10;
n = %float(7) / 2;
line += ' ' + %char(p) + ' ' + %char(n) + ' ' + local();
monitor;
  f8 = f8 / z8;
  line += ' D';
on-error;
  line += ' E ' + %char(%status);
endmon;
dsply (line);
*inlr = *on;

dcl-proc local;
  dcl-pi *n varchar(40);
  end-pi;
  dcl-s lf float(8) inz(4);
  lf = lf / 8 + 1;
  return %char(lf);
end-proc;
