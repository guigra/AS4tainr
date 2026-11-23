**FREE
// Phase 2: an indicator used in a condition and set nowhere in the
// program (compile only; fidelity.md, RNF7503 seen on 2026-11-15 but
// not kept).
ctl-opt dftactgrp(*no);
dcl-s x int(10);
if *in60;
  x = 1;
endif;
*inlr = *on;
