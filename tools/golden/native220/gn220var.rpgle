**FREE
// Phase 2: %ELEM set to a constant past a DIM(*VAR) array's dimension
// (compile only).
ctl-opt dftactgrp(*no);
dcl-s va int(10) dim(*var : 4);
%elem(va) = 5;
*inlr = *on;
