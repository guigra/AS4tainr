**FREE
// Native unit NRESET (S2.95): RESET of global fields, an array, data
// structures, and from a procedure a global field and a local, the
// program captured on PUB400 as GN95RST with DSPLY for its output.
// Source: golden capture tools/golden/native95 n95_reset (PUB400, IBM i
// 7.5, 2026-11-19).
ctl-opt dftactgrp(*no);
dcl-s c char(4) inz('abcd');
dcl-s p packed(5:2) inz(12.34);
dcl-s z zoned(3:0);
dcl-s arr int(5) dim(3) inz(7);
dcl-ds d;
  dc char(3) inz('QRS');
  dz zoned(3:0) inz(42);
end-ds;
dcl-ds q qualified;
  a char(2) inz('mn');
  n packed(3:0) inz(5);
end-ds;
dcl-s line varchar(120);
c = 'wxyz';
p = 99.5;
z = 8;
arr(2) = 1;
dc = 'TUV';
dz = 1;
q.a = 'op';
q.n = 6;
reset c;
reset p;
reset z;
reset arr;
reset d;
reset q;
line = '[' + c + '] ' + %char(p) + ' ' + %char(z) + ' '
       + %char(arr(1) + arr(2) + arr(3)) + ' [' + dc + '] ' + %char(dz)
       + ' [' + q.a + '] ' + %char(q.n);
dsply (line);
p = 1;
dsply (local());
*inlr = *on;

dcl-proc local;
  dcl-pi *n varchar(40);
  end-pi;
  dcl-s lc char(3) inz('zzz');
  lc = 'aaa';
  reset lc;
  reset p;
  return '[' + lc + '] ' + %char(p);
end-proc;
