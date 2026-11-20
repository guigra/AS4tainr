**FREE
// Native unit NCOMMIT (S2.152): COMMIT and ROLBK with no commitment
// control active, the program captured on PUB400 as GN152CM with DSPLY
// for its output.
// Source: golden capture tools/golden/native152 n152_commit (PUB400,
// IBM i 7.5, 2026-11-20).
ctl-opt dftactgrp(*no);
dcl-s line varchar(80);

monitor;
  commit;
  line = 'commit ok';
on-error;
  line = 'commit ' + %char(%status);
endmon;
monitor;
  rolbk;
  line += ' rolbk ok';
on-error;
  line += ' rolbk ' + %char(%status);
endmon;
dsply (line);
*inlr = *on;
