**FREE
// Native unit NPTRDIF (S2.98): a pointer minus a pointer, the program
// captured on PUB400 as GN99PDF with DSPLY for its output.
// Source: golden capture tools/golden/native99 n99_ptrdiff (PUB400, IBM i
// 7.5, 2026-11-19).
ctl-opt dftactgrp(*no);
dcl-s buf char(20);
dcl-s other char(4);
dcl-s p1 pointer;
dcl-s p2 pointer;
dcl-s h1 pointer;
dcl-s h2 pointer;
dcl-s n int(10);
dcl-s line varchar(80);
p1 = %addr(buf);
p2 = %addr(buf) + 5;
n = p2 - p1;
line = %char(n);
n = p1 - p2;
line += ' ' + %char(n);
h1 = %alloc(16);
h2 = h1 + 7;
n = h2 - h1;
line += ' ' + %char(n);
dealloc h1;
p2 = %addr(other);
monitor;
  n = p2 - p1;
  line += ' D';
on-error;
  line += ' E ' + %char(%status);
endmon;
dsply (line);
*inlr = *on;
