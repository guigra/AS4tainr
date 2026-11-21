**FREE
// Native unit NCONVRT (S2.194): OPTIONS(*CONVERT), the program captured
// on PUB400 as GN194CV with DSPLY for its output.
// Source: golden capture tools/golden/native194 n194_convert (PUB400,
// IBM i 7.5, 2026-11-21).
ctl-opt dftactgrp(*no);
dcl-pr show varchar(40);
  v varchar(30) value options(*convert);
end-pr;
dcl-s n int(10) inz(-42);
dcl-s p packed(7:2) inz(1234.5);
dcl-s d date inz(d'2026-07-04');
dcl-s c char(6) inz('ab');
dcl-s line varchar(200);

line = show(n);
line += show(p);
line += show(d);
line += show(c);
dsply (line);
*inlr = *on;

dcl-proc show;
  dcl-pi *n varchar(40);
    v varchar(30) value options(*convert);
  end-pi;
  return '[' + v + ']';
end-proc;
