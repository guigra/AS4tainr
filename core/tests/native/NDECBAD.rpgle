**FREE
// Native unit NDECBAD (S2.168): %DEC of a date field that holds blanks,
// the program captured on PUB400 as GN168DD with DSPLY for its output.
// Source: golden capture tools/golden/native168 n168_decbad (PUB400,
// IBM i 7.5, 2026-11-20).
ctl-opt dftactgrp(*no);
dcl-ds x;
  c char(10) pos(1) inz(*blanks);
  d date pos(1);
end-ds;
dcl-s n packed(9:0);
dcl-s line varchar(80);

monitor;
  n = %dec(d);
  line = 'no error ' + %char(n);
on-error;
  line = %char(%status);
endmon;
c = '2026-01-31';
n = %dec(d);
line += ' ' + %char(n);
dsply (line);
*inlr = *on;
