**FREE
// Native unit NLOCDS (S2.93): a procedure's own data structures as a
// whole, the program captured on PUB400 as GN93LDS with DSPLY for its
// output.
// Source: golden capture tools/golden/native93 n93_localds (PUB400, IBM i
// 7.5, 2026-11-18).
ctl-opt dftactgrp(*no);
dcl-ds t qualified template;
  a char(3);
  n zoned(3:0);
end-ds;
dcl-ds g likeds(t);
dcl-s line varchar(80);
line = take('ABC012XYZ');
fill(g);
line += ' ' + g.a + '/' + %char(g.n);
dsply line;
*inlr = *on;

dcl-proc take;
  dcl-pi *n varchar(40);
    buf char(20) const;
  end-pi;
  dcl-ds h qualified;
    a char(3);
    n zoned(3:0);
  end-ds;
  dcl-s c char(6);
  h = %subst(buf: 1: %size(h));
  c = h;
  return h.a + '|' + %char(h.n + 1) + '|' + c;
end-proc;

dcl-proc fill;
  dcl-pi *n;
    d likeds(t);
  end-pi;
  d = 'QRS042';
end-proc;
