**FREE
// Phase 2: a key literal of another length than the key field (ORD is
// zoned 3,0) (compile only; RNF7078 seen on 2026-11-17 but not kept).
ctl-opt dftactgrp(*no);
dcl-f GN58PF keyed;
setll 1 GN58PF;
*inlr = *on;
