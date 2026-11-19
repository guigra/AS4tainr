**FREE
// Native unit NDSPARM (S2.97): LIKEDS parameters by reference to local
// procedures, the program captured on PUB400 as GN98DSP with DSPLY for
// its output.
// Source: golden capture tools/golden/native98 n98_dsparm (PUB400, IBM i
// 7.5, 2026-11-19).
ctl-opt dftactgrp(*no);
dcl-ds t qualified template;
  a char(3);
  n packed(5:0);
end-ds;
dcl-ds g likeds(t) inz;
dcl-ds h qualified;
  a char(3) inz('hhh');
  n packed(5:0) inz(10);
end-ds;
dcl-s line varchar(80);
g.a = 'ggg';
g.n = 1;
bump(g);
bump(g);
bump(h);
line = g.a + %char(g.n) + ' ' + h.a + %char(h.n) + ' ' + inner();
dsply (line);
*inlr = *on;

dcl-proc bump;
  dcl-pi *n;
    d likeds(t);
  end-pi;
  d.n += 5;
  d.a = %subst(d.a: 2: 2) + %char(d.n / 5);
end-proc;

dcl-proc inner;
  dcl-pi *n varchar(20);
  end-pi;
  dcl-ds loc likeds(t);
  loc.a = 'LLL';
  loc.n = 100;
  bump(loc);
  return loc.a + %char(loc.n);
end-proc;
