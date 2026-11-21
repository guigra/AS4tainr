**FREE
// Native unit NWHOLEA (S2.208): a whole array for a CONST array
// parameter, the program captured on PUB400 as GN208WA with DSPLY for its
// output.
// Source: golden capture tools/golden/native208 n208_wholearr (PUB400,
// IBM i 7.5, 2026-11-21).
ctl-opt dftactgrp(*no);
dcl-pr sumk int(10);
  n int(10) const;
  keys int(10) const dim(5) options(*varsize);
end-pr;
dcl-s two int(10) dim(2);
dcl-s five int(10) dim(5);
dcl-s line varchar(200);

two(1) = 3;
two(2) = 4;
five(1) = 1;
five(2) = 2;
five(3) = 3;
five(4) = 4;
five(5) = 5;
line = %char(sumk(2 : two)) + ' ' + %char(sumk(5 : five)) + ' ' + %char(local());
dsply (line);
*inlr = *on;

dcl-proc sumk;
  dcl-pi *n int(10);
    n int(10) const;
    keys int(10) const dim(5) options(*varsize);
  end-pi;
  dcl-s i int(10);
  dcl-s t int(10);
  for i = 1 to n;
    t += keys(i);
  endfor;
  return t;
end-proc;

dcl-proc local;
  dcl-pi *n int(10) end-pi;
  dcl-s k int(10) dim(1);
  k(1) = 40;
  return sumk(1 : k);
end-proc;
