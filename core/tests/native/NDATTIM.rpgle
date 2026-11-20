**FREE
// Native unit NDATTIM (S2.157): a date plus a time, the program captured
// on PUB400 as GN157DT with DSPLY for its output.
// Source: golden capture tools/golden/native157 n157_datetime (PUB400,
// IBM i 7.5, 2026-11-20).
ctl-opt dftactgrp(*no);
dcl-s t6 timestamp;
dcl-s t0 timestamp(0);
dcl-s d date inz(d'2026-01-31');
dcl-s t time inz(t'23.59.58');
dcl-s line varchar(120);

t6 = %date('2024-09-16': *iso) + %time('11.22.33': *iso);
line = %char(t6);
t6 = d + t;
line += ' ' + %char(t6);
t0 = d + t;
line += ' ' + %char(t0);
dsply (line);
*inlr = *on;
