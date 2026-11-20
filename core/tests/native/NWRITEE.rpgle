**FREE
// Native unit NWRITEE (S2.172): WRITE(E) of database files, the program
// captured on PUB400 as GN172WE with DSPLY for its output.
// Source: golden capture tools/golden/native172 n172_writee (PUB400,
// IBM i 7.5, 2026-11-20).
ctl-opt dftactgrp(*no);
dcl-f gn172f usage(*output) rename(gn172f : gn172r);
dcl-s line varchar(80);

k = 2;
s = 'bb';
write(e) gn172r;
line = %char(%error) + ' ' + %char(%status(gn172f));
k = 1;
s = 'zz';
write(e) gn172r;
line += ' ' + %char(%error) + ' ' + %char(%status(gn172f));
dsply (line);
*inlr = *on;
