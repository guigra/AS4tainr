**FREE
// Native unit NBASEDDS (S2.124): a based data structure as a whole, the
// program captured on PUB400 as GN124BD with DSPLY for its output.
// Source: golden capture tools/golden/native124 n124_basedds (PUB400,
// IBM i 7.5, 2026-11-19).
ctl-opt dftactgrp(*no);
dcl-ds hdr qualified based(p);
  a int(10);
  b char(4);
end-ds;
dcl-s p pointer;
dcl-s line varchar(80);
dcl-pr fill;
  buf char(100) options(*varsize);
  n int(10) const;
end-pr;

p = %alloc(8);
hdr.a = 7;
hdr.b = 'abcd';
line = %char(hdr.a) + hdr.b;
fill(hdr: %size(hdr));
line += ' ' + %subst(hdr: 5: 4);
hdr = 'ABCDEFGH';
line += ' ' + hdr.b;
dealloc p;
dsply (line);
*inlr = *on;

dcl-proc fill;
  dcl-pi *n;
    buf char(100) options(*varsize);
    n int(10) const;
  end-pi;
  %subst(buf: 5: 4) = 'wxyz';
end-proc;
