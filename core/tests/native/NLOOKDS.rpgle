**FREE
// Native unit NLOOKDS (S2.196): %LOOKUP of DSARR(*).SUBFIELD, the
// program captured on PUB400 as GN196LK with DSPLY for its output.
// Source: golden capture tools/golden/native196 n196_lookds (PUB400,
// IBM i 7.5, 2026-11-21).
ctl-opt dftactgrp(*no);
dcl-ds item qualified dim(4);
  name char(10);
  qty int(10);
end-ds;
dcl-s line varchar(200);

item(1).name = 'APPLE';
item(1).qty = 5;
item(2).name = 'PEAR';
item(2).qty = 9;
item(3).name = 'PLUM';
item(3).qty = 12;
item(4).name = 'PEAR';
item(4).qty = 20;
line = %char(%lookup('PEAR' : item(*).name)) + ' '
     + %char(%lookup('KIWI' : item(*).name)) + ' '
     + %char(%lookup('PEAR' : item(*).name : 3)) + ' '
     + %char(%lookup(12 : item(*).qty));
dsply (line);
*inlr = *on;
