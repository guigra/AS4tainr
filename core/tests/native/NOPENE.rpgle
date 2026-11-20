**FREE
// Native unit NOPENE (S2.161): OPEN(E), FEOD(E) and CLOSE(E) of a printer
// file and CLOSE(E) of a closed database file, the program captured on
// PUB400 as GN161OE with DSPLY for its output.
// Source: golden capture tools/golden/native161 n161_opene (PUB400,
// IBM i 7.5, 2026-11-20).
ctl-opt dftactgrp(*no);
dcl-f gn120f usage(*input) usropn rename(gn120f : gn120r);
dcl-f qprint printer(132) usropn;
dcl-s line varchar(80);

open qprint;
open(e) qprint;
line = %char(%error) + ' ' + %char(%status(qprint));
feod(e) qprint;
line += ' ' + %char(%error);
close(e) qprint;
line += ' ' + %char(%error);
open gn120f;
close gn120f;
close(e) gn120f;
line += ' ' + %char(%error) + ' ' + %char(%status(gn120f));
dsply (line);
*inlr = *on;
