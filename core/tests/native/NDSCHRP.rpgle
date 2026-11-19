**FREE
// Native unit NDSCHRP (S2.125): a whole data structure for character
// parameters of local procedures, the program captured on PUB400 as
// GN125DP with DSPLY for its output.
// Source: golden capture tools/golden/native125 n125_dsparm (PUB400,
// IBM i 7.5, 2026-11-19).
ctl-opt dftactgrp(*no);
dcl-ds d qualified;
  a char(3) inz('abc');
  n zoned(3:0) inz(42);
end-ds;
dcl-s line varchar(80);

byref(d);
line = d.a + %char(d.n);
vsize(d);
line += ' ' + d.a;
line += ' ' + cst(d);
line += ' ' + %char(%len(%trimr(cstlong(d))));
val(d);
line += ' ' + d.a;
dsply (line);
*inlr = *on;

dcl-proc byref;
  dcl-pi *n;
    x char(6);
  end-pi;
  x = 'XYZ999';
end-proc;

dcl-proc vsize;
  dcl-pi *n;
    x char(100) options(*varsize);
  end-pi;
  %subst(x: 1: 1) = 'Q';
end-proc;

dcl-proc cst;
  dcl-pi *n char(6);
    x char(6) const;
  end-pi;
  return x;
end-proc;

dcl-proc cstlong;
  dcl-pi *n char(10);
    x char(10) const;
  end-pi;
  return x;
end-proc;

dcl-proc val;
  dcl-pi *n;
    x char(6) value;
  end-pi;
  x = 'ZZZZZZ';
end-proc;
