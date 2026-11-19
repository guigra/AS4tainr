**FREE
// Native unit NCHARCND (S2.123): %CHAR of conditions, the program
// captured on PUB400 as GN123CC with DSPLY for its output.
// Source: golden capture tools/golden/native123 n123_charcond (PUB400,
// IBM i 7.5, 2026-11-19).
ctl-opt dftactgrp(*no);
dcl-s n int(10) inz(5);
dcl-s c char(3) inz('abc');
dcl-s line varchar(80);

line = %char(n > 3) + %char(n = 3 or n < 0) + %char(not (n > 3))
     + %char(n >= 5 and n < 10) + %char(c < 'abd') + %char(c <> 'abc');
line += ' ' + %char(n * 2 = 10);
dsply (line);
*inlr = *on;
