**FREE
// Phase 2, RNF7078: a named constant of 1 digit for a 3-digit key
ctl-opt dftactgrp(*no);
dcl-f GN58PF keyed;
dcl-c ONE 1;
setll ONE GN58PF;
*inlr = *on;
