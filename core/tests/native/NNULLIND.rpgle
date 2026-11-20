**FREE
// Native unit NNULLIND (S2.137): %NULLIND, the program captured on
// PUB400 as GN137NI with DSPLY for its output; its file is QGPL/GN137N
// here with the capture's two records.
// Source: golden capture tools/golden/native137 n137_nullind (PUB400,
// IBM i 7.5, 2026-11-19).
ctl-opt dftactgrp(*no) alwnull(*usrctl);
dcl-f gn137n usage(*input) rename(gn137n:rec137)
             extfile('QGPL/GN137N');
dcl-s line varchar(80);

read gn137n;
line = %char(id) + %char(%nullind(d));
if %nullind(d);
  line += 'n';
endif;
%nullind(d) = *off;
line += %char(%nullind(d));
read gn137n;
line += ' ' + %char(id) + %char(%nullind(d)) + ' ' + %char(d);
dsply (line);
*inlr = *on;
