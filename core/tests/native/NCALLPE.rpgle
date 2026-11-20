**FREE
// Native unit NCALLPE (S2.173): CALLP(E), the program captured on PUB400
// as GN173CE with DSPLY for its output.
// Source: golden capture tools/golden/native173 n173_callpe (PUB400,
// IBM i 7.5, 2026-11-20).
ctl-opt dftactgrp(*no);
dcl-s line varchar(80);
dcl-s r int(10);

callp(e) boom(0);
line = %char(%error) + ' ' + %char(%status) + ' ' + %char(r);
callp(e) boom(2);
line += ' ' + %char(%error) + ' ' + %char(%status) + ' ' + %char(r);
dsply (line);
*inlr = *on;

dcl-proc boom;
  dcl-pi *n;
    d int(10) const;
  end-pi;
  r = 10 / d;
end-proc;
