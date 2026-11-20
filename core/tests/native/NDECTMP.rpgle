**FREE
// Native unit NDECTMP (S2.143): %DEC of dates, times and timestamps, the
// program captured on PUB400 as GN143DT with DSPLY for its output.
// Source: golden capture tools/golden/native143 n143_dectmp (PUB400,
// IBM i 7.5, 2026-11-20).
ctl-opt dftactgrp(*no);
dcl-s d date inz(d'2026-01-31');
dcl-s t time inz(t'09.15.00');
dcl-s ts timestamp inz(z'2026-01-31-23.59.59.123456');
dcl-s n packed(8:0);
dcl-s line varchar(100);

n = %dec(d);
line = %char(n) + ' ' + %char(%dec(d : *usa)) + ' ' + %char(%dec(t))
     + ' ' + %char(%dec(ts)) + ' ' + %char(%dec(d + %days(1)));
dsply (line);
*inlr = *on;
