**FREE
// Phase 2, RNF7078: a packed(5:0) field for a 3-digit key
ctl-opt dftactgrp(*no);
dcl-f GN58PF keyed;
dcl-s k packed(5:0) inz(1);
setll k GN58PF;
*inlr = *on;
