**FREE
// Native unit NARRFN (S2.68): %XFOOT, %MAXARR and %MINARR of whole
// arrays. A packed array with decimals and negative values, ties (the
// first one counts), an integer array, a character array. The program
// captured on PUB400, with DSPLY for its output.
// Source: golden capture tools/golden/native68 n68_arr (PUB400, IBM i
// 7.5, 2026-11-17).
ctl-opt dftactgrp(*no);
dcl-s p packed(7 : 2) dim(5);
dcl-s q int(10) dim(4);
dcl-s c char(3) dim(4);
dcl-s t packed(11 : 2);
p(1) = 10.50;
p(2) = -3.25;
p(3) = 99.99;
p(4) = 99.99;
p(5) = -3.25;
q(1) = 7;
q(2) = 7;
q(3) = -2;
q(4) = 5;
c(1) = 'bob';
c(2) = 'amy';
c(3) = 'zed';
c(4) = 'amy';
t = %xfoot(p);
dsply ('XFOOT P ' + %char(t));
dsply ('XFOOT Q ' + %char(%xfoot(q)));
dsply ('MAX P ' + %char(%maxarr(p)) + ' MIN P ' + %char(%minarr(p)));
dsply ('MAX Q ' + %char(%maxarr(q)) + ' MIN Q ' + %char(%minarr(q)));
dsply ('MAX C ' + %char(%maxarr(c)) + ' MIN C ' + %char(%minarr(c)));
*inlr = *on;
