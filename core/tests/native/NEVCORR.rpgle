**FREE
// Native unit NEVCORR (S2.177): EVAL-CORR, the program captured on PUB400
// as GN177EC with DSPLY for its output.
// Source: golden capture tools/golden/native177 n177_evalcorr (PUB400,
// IBM i 7.5, 2026-11-20).
ctl-opt dftactgrp(*no);
dcl-ds addr_t qualified template;
  city char(10);
  zip packed(5:0);
end-ds;
dcl-ds a qualified;
  name char(10) inz('Alice');
  age packed(3:0) inz(30);
  code char(3) inz('ABC');
  where likeds(addr_t);
  tags char(2) dim(3);
end-ds;
dcl-ds b qualified;
  age zoned(5:0) inz(1);
  name varchar(20) inz('x');
  code packed(3:0) inz(7);
  where likeds(addr_t);
  tags char(2) dim(2);
  oth char(1) inz('O');
end-ds;
dcl-s line varchar(120);

a.where.city = 'Paris';
a.where.zip = 75001;
a.tags(1) = 'T1';
a.tags(2) = 'T2';
a.tags(3) = 'T3';
eval-corr b = a;
line = '[' + b.name + '] ' + %char(b.age) + ' ' + %char(b.code) + ' '
     + b.where.city + %char(b.where.zip) + ' ' + b.tags(1) + b.tags(2)
     + ' ' + b.oth;
dsply (line);
*inlr = *on;
