**FREE
// Native unit NBASED2 (S2.130): fields based on based pointers, the
// program captured on PUB400 as GN130B2 with DSPLY for its output.
// Source: golden capture tools/golden/native130 n130_based2 (PUB400,
// IBM i 7.5, 2026-11-19).
ctl-opt dftactgrp(*no);
dcl-ds h_t qualified template;
  p pointer;
end-ds;
dcl-s buf char(20) inz('hello');
dcl-ds hh likeds(h_t);
dcl-s gp pointer;
dcl-ds g likeds(h_t) based(gp);
dcl-s gs char(3) based(g.p);
dcl-s line varchar(80);

hh.p = %addr(buf);
line = info(%addr(hh));
line += ' ' + %subst(buf: 1: 5);
gp = %addr(hh);
line += ' ' + gs;
gs = 'XYZ';
line += ' ' + %subst(buf: 1: 5);
dsply (line);
*inlr = *on;

dcl-proc info;
  dcl-pi *n char(5);
    hp pointer const;
  end-pi;
  dcl-ds h likeds(h_t) based(hp);
  dcl-s s char(5) based(h.p);
  %subst(s: 1: 1) = 'J';
  return s;
end-proc;
