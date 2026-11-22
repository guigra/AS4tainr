**FREE
// Interpreter fix (fork branch fix/test-needs-e): TEST(D) with neither
// E nor an error indicator. Expected not to compile (RNF5056). Captured
// on PUB400 (tools/golden/native216.txt n216_crt).
ctl-opt dftactgrp(*no);
dcl-s c char(10) inz('2026-02-30');

test(d) *iso c;
*inlr = *on;
