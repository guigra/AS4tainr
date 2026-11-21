**FREE
// Native unit NPTRDF2 (S2.199): pointer differences in %CHAR, the
// program captured on PUB400 as GN199PD with DSPLY for its output.
// Source: golden capture tools/golden/native199 n199_ptrdiff (PUB400,
// IBM i 7.5, 2026-11-21).
ctl-opt dftactgrp(*no);
dcl-s name char(10) inz('abcdefghij');
dcl-s p pointer;
dcl-s nums int(10) dim(3);
dcl-s line varchar(200);

p = %addr(name);
p += 2;
line = %char(p - %addr(name)) + ' ' + %char(%addr(nums(3)) - %addr(nums(1)))
     + ' ' + %char(%addr(name) - p);
dsply (line);
*inlr = *on;
