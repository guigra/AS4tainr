**FREE
// Stage 2, S2.49: *NULL plus a number, for parity between the engines only
// (native_diff.rs). IBM i moves it with no error (golden capture
// tools/golden/native50.txt n50_pa, PUB400, IBM i 7.5, 2026-11-16); the
// interpreter raises MCH3601 (status 222), and the native engine does the
// same (fidelity.md).
ctl-opt dftactgrp(*no);
dcl-s p pointer;
p = *null;
monitor;
  p = p + 1;
  dsply ('NULL MOVED');
on-error;
  dsply ('NULL ERROR ' + %char(%status));
endmon;
*inlr = *on;
