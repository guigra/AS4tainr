**FREE
// Native unit NSQRT (S2.113): %SQRT, the program captured on PUB400 as
// GN113SQR with DSPLY for its output.
// Source: golden capture tools/golden/native113 n113_sqrt (PUB400, IBM i
// 7.5, 2026-11-19).
ctl-opt dftactgrp(*no);
dcl-s p packed(15:13);
dcl-s q packed(7:2) inz(6.25);
dcl-s neg packed(3:0) inz(-1);
dcl-s line varchar(200);
p = %sqrt(16);
line = %char(p);
p = %sqrt(2);
line += ' ' + %char(p);
p = %sqrt(q);
line += ' ' + %char(p) + ' ' + %char(%sqrt(10));
monitor;
  p = %sqrt(neg);
  line += ' D';
on-error;
  line += ' E' + %char(%status);
endmon;
dsply (line);
*inlr = *on;
