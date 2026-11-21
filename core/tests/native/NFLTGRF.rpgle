**FREE
// Native unit NFLTGRF (S2.210): READ and UPDATE of a record with float
// and graphic fields, the program captured on PUB400 as GN210FF with
// DSPLY for its output; its table QGPL/NFLTGRT, made by the test as the
// capture made GN210T.
// Source: golden capture tools/golden/native210 n210_floatgraphic
// (PUB400, IBM i 7.5, 2026-11-21).
ctl-opt dftactgrp(*no) extbinint(*yes);
dcl-f nfltgrt usage(*update);
dcl-s line varchar(200);

read nfltgrt;
line = %char(id) + ' ' + %char(rate) + ' ' + %trim(note);
rate = rate / 4;
note = 'Bern';
update nfltgrr;
setll *start nfltgrt;
read nfltgrt;
line += '|' + %char(id) + ' ' + %char(rate) + ' ' + %trim(note);
dsply (line);
*inlr = *on;
