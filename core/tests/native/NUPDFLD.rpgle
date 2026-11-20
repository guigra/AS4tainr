**FREE
// Native unit NUPDFLD (S2.174): UPDATE with %FIELDS, the program captured
// on PUB400 as GN174UF with DSPLY for its output.
// Source: golden capture tools/golden/native174 n174_updfld (PUB400,
// IBM i 7.5, 2026-11-20).
ctl-opt dftactgrp(*no);
dcl-f gn174f usage(*update) rename(gn174f : gn174r);
dcl-s line varchar(80);

read gn174f;
s = 'zz';
t = 'yy';
update gn174r %fields(t);
setll *start gn174f;
read gn174f;
line = %char(n) + ' ' + s + ' ' + t;
dsply (line);
*inlr = *on;
