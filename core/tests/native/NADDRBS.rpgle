**FREE
// Native unit NADDRBS (S2.159): %ADDR of a based subfield, the program
// captured on PUB400 as GN159AB with DSPLY for its output.
// Source: golden capture tools/golden/native159 n159_addrbased (PUB400,
// IBM i 7.5, 2026-11-20).
ctl-opt dftactgrp(*no);
dcl-s buf char(40) inz(*all'.');
dcl-ds ent based(p);
  name char(10);
  data char(10);
end-ds;
dcl-s v char(10) based(q);
dcl-s p pointer;
dcl-s q pointer;
dcl-s line varchar(80);

p = %addr(buf);
q = %addr(data);
v = 'hello';
if q = p + 10;
  line = 'same';
endif;
p = %addr(buf) + 20;
q = %addr(data);
v = 'world';
line += ' ' + buf;
dsply (line);
*inlr = *on;
