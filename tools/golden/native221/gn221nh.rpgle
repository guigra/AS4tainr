**FREE
// Phase 2, RNF7078: CHAIN with a 1-digit literal for a 3-digit key
ctl-opt dftactgrp(*no);
dcl-f GN58PF keyed;
chain 1 GN58PF;
*inlr = *on;
