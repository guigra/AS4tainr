**FREE
// Phase 2: a constant index past a DIM(*AUTO) array's dimension
// (compile only; RNF0530/RNF7563 seen on 2026-11-15 but not kept).
ctl-opt dftactgrp(*no);
dcl-s au int(10) dim(*auto : 5);
au(6) = 1;
*inlr = *on;
