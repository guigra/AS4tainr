**FREE
// Native unit NSUBFIG (S2.136): figurative constants assigned to %SUBST,
// the program captured on PUB400 as GN136SF with DSPLY for its output.
// Source: golden capture tools/golden/native136 n136_substfig (PUB400,
// IBM i 7.5, 2026-11-19).
ctl-opt dftactgrp(*no);
dcl-s x char(8) inz('abcdefgh');
dcl-s line varchar(80);

%subst(x: 2: 3) = *blanks;
line = '[' + x + ']';
%subst(x: 1: 2) = *all'-';
line += '[' + x + ']';
%subst(x: 5) = *zeros;
line += '[' + x + ']';
%subst(x: 3: 1) = *hival;
if %subst(x: 3: 1) = x'FF';
  line += 'h';
endif;
dsply (line);
*inlr = *on;
