**FREE
// Native unit NSPLITA (S2.179): arrays assigned %SPLIT, the program
// captured on PUB400 as GN179SP with DSPLY for its output.
// Source: golden capture tools/golden/native179 n179_split (PUB400,
// IBM i 7.5, 2026-11-20).
ctl-opt dftactgrp(*no);
dcl-s w varchar(10) dim(5);
dcl-s c char(4) dim(4);
dcl-s v varchar(10) dim(*auto : 5);
dcl-s line varchar(120);

w(5) = 'old';
w = %split('the quick  brown fox');
line = w(1) + '/' + w(4) + '/' + w(5) + '|';
c = %split('a,,b,c' : ',');
line += c(1) + c(2) + c(3) + '[' + c(4) + ']|';
c = %split('a,,b' : ',' : *allsep);
line += c(1) + '[' + c(2) + ']' + c(3) + '|';
v = %split('x y');
line += %char(%elem(v)) + v(2);
dsply (line);
*inlr = *on;
