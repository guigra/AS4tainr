**FREE
// Native unit NCHRCNV (S2.112): %INT and %DEC of character data, %ERROR,
// %STATUS of a file, the program captured on PUB400 as GN112CNV with
// DSPLY for its output.
// Source: golden capture tools/golden/native112 n112_charconv (PUB400, IBM
// i 7.5, 2026-11-19).
ctl-opt dftactgrp(*no);
dcl-f qprint printer(132) usropn;
dcl-s txt char(8) inz(' 42 ');
dcl-s bad char(10) inz('2026-13-40');
dcl-s n int(10);
dcl-s p packed(5:2);
dcl-s line varchar(200);
n = %int('123');
line = %char(n);
n = %inth('12.5');
line += ' ' + %char(n);
p = %dec('-3.25' : 5 : 2);
line += ' ' + %char(p);
n = %int(txt);
line += ' ' + %char(n);
monitor;
  n = %int('abc');
  line += ' D';
on-error;
  line += ' E' + %char(%status);
endmon;
test(de) *iso bad;
if %error;
  line += ' err';
endif;
line += ' ' + %char(%error);
open qprint;
open(e) qprint;
line += ' ' + %char(%error) + ' ' + %char(%status(qprint));
close qprint;
dsply (line);
*inlr = *on;
