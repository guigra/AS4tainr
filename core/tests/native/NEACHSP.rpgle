**FREE
// Native unit NEACHSP (S2.213): FOR-EACH over %SPLIT and over a DIM(*AUTO)
// array, the program captured on PUB400 as GN218FE with DSPLY for its
// output.
// Source: golden capture tools/golden/native218 n218_foreach (PUB400,
// IBM i 7.5, 2026-11-22).
ctl-opt dftactgrp(*no);
dcl-s item varchar(10);
dcl-s auto varchar(10) dim(*auto : 5);
dcl-s line varchar(200);

for-each item in %split('a,b,,c' : ',');
  line += '[' + item + ']';
endfor;
line += '|';
for-each item in %split('a,b,,c' : ',' : *allsep);
  line += '[' + item + ']';
endfor;
line += '|';
for-each item in %split('  the quick  fox ');
  line += '[' + item + ']';
endfor;
line += '|';
auto(*next) = 'x';
auto(*next) = 'y';
auto(*next) = 'z';
for-each item in auto;
  line += '[' + item + ']';
endfor;
line += '|' + %char(%elem(auto));
dsply (line);
*inlr = *on;
