**FREE
// Native unit NDTAUNL (S2.192): UNLOCK of data areas, the program
// captured on PUB400 as GN192UL with DSPLY for its output; its data areas
// QGPL/NDTAUA and QGPL/NDTAUB, made by the test (the capture's program
// made QTEMP/GN192A and GN192B itself with QCMDEXC).
// Source: golden capture tools/golden/native192 n192_unlock (PUB400,
// IBM i 7.5, 2026-11-21).
ctl-opt dftactgrp(*no);
dcl-s val char(10) dtaara('QGPL/NDTAUA');
dcl-s oth char(10) dtaara('QGPL/NDTAUB');
dcl-s line varchar(200);

in *lock val;
line = %trim(val);
unlock val;
val = 'changed';
out(e) val;
line += ' ' + %char(%error) + ' ' + %char(%status);
in *lock val;
in *lock oth;
line += ' ' + %trim(val) + ' ' + %trim(oth);
// One data area by name: as4tainr's free-form parser takes *DTAARA as
// the name DTAARA (fidelity.md, interpreter defects).
unlock oth;
oth = 'other';
out(e) oth;
line += ' ' + %char(%error) + ' ' + %char(%status);
in oth;
line += ' ' + %trim(oth);
dsply (line);
*inlr = *on;
