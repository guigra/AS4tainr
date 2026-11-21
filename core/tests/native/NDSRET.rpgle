**FREE
// Native unit NDSRET (S2.205): a data structure array returned, the
// program captured on PUB400 as GN205DR with DSPLY for its output.
// Source: golden capture tools/golden/native205 n205_dsret (PUB400,
// IBM i 7.5, 2026-11-21).
ctl-opt dftactgrp(*no);
dcl-ds ent_t qualified template;
  pgm char(4);
  n int(10);
end-ds;
dcl-pr mk likeds(ent_t) dim(2) end-pr;
dcl-ds e likeds(ent_t) dim(2);
dcl-s line varchar(200);

e(2).pgm = 'old';
e = mk();
line = e(1).pgm + %char(e(1).n) + '|' + e(2).pgm + %char(e(2).n);
dsply (line);
*inlr = *on;

dcl-proc mk;
  dcl-pi *n likeds(ent_t) dim(2) end-pi;
  dcl-ds res likeds(ent_t) dim(2);
  res(1).pgm = 'ONE';
  res(1).n = 1;
  res(2).pgm = 'TWO';
  res(2).n = 2;
  return res;
end-proc;
