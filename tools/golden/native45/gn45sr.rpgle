**FREE
// ADR 0007, the native_ok fallback: a subprocedure that runs EXSR on a
// subroutine of the main procedure. Captured on PUB400
// (tools/golden/native45.txt n45_crt_sr): does the compiler take it?
ctl-opt dftactgrp(*no);
dcl-s n int(10);
sub();
*inlr = *on;
begsr mainsr;
  n = n + 1;
endsr;

dcl-proc sub;
  exsr mainsr;
end-proc;
