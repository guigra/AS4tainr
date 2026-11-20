**FREE
// Native unit NTESTDT (S2.138): TEST(D/T/Z), the program captured on
// PUB400 as GN138TS with DSPLY for its output.
// Source: golden capture tools/golden/native138 n138_test (PUB400,
// IBM i 7.5, 2026-11-20).
ctl-opt dftactgrp(*no);
dcl-s c10 char(10) inz('2026-02-30');
dcl-s n8 zoned(8:0) inz(20260131);
dcl-s tm char(8) inz('25.00.00');
dcl-s tz char(26) inz('2026-01-31-23.59.59.000000');
dcl-s line varchar(80);

test(de) *iso c10;
line = %char(%error);
test(de) *iso n8;
line += %char(%error);
test(te) *iso tm;
line += %char(%error);
test(ze) tz;
line += %char(%error);
c10 = '01/31/2026';
test(de) *usa c10;
line += %char(%error);
dsply (line);
*inlr = *on;
