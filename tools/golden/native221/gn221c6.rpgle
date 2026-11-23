**FREE
// Phase 2, RNF7078: a 6-character literal for a 5-character key
ctl-opt dftactgrp(*no);
dcl-f GN221PF keyed;
chain 'ABCDEF' GN221PF;
*inlr = *on;
