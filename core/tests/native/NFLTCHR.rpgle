**FREE
// Native unit NFLTCHR (S2.110, ADR 0008): %FLOAT of character data and
// %ADDR of a float field, the program captured on PUB400 as GN110FCH with
// DSPLY for its output.
// Source: golden capture tools/golden/native110 n110_floatchar (PUB400,
// IBM i 7.5, 2026-11-19).
ctl-opt dftactgrp(*no);
dcl-s f8 float(8);
dcl-s txt varchar(10) inz('2,5');
dcl-s pp pointer;
dcl-s bf float(8) based(pp);
dcl-s line varchar(200);
f8 = %float('12.5');
line = %char(f8);
f8 = %float('1.5E3');
line += ' ' + %char(f8);
f8 = %float(txt);
line += ' ' + %char(f8);
monitor;
  f8 = %float('abc');
  line += ' D';
on-error;
  line += ' E ' + %char(%status);
endmon;
pp = %addr(f8);
line += ' ' + %char(bf);
dsply (line);
*inlr = *on;
