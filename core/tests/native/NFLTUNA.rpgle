**FREE
// Native unit NFLTUNA (the review of 2026-11-19, ADR 0008): decimal
// parts under unary operators in float expressions, the program captured
// on PUB400 as GN111FUN with DSPLY for its output.
// Source: golden capture tools/golden/native111 n111_floatunary (PUB400,
// IBM i 7.5, 2026-11-19).
ctl-opt dftactgrp(*no);
dcl-s f float(8);
dcl-s res float(8);
dcl-s p packed(30:0);
dcl-s q packed(2:2) inz(.99);
dcl-s line varchar(200);
res = -(f + (p + q));
line = %char(res);
f = 2;
res = +(f * (q + 1));
line += ' ' + %char(res);
dsply (line);
*inlr = *on;
