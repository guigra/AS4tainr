**FREE
// Native unit NPRTDS (S2.156): WRITE of a program-described printer file
// from a data structure, the program captured on PUB400 as GN156PD
// without its read of the spooled file.
// Source: golden capture tools/golden/native156 n156_prtds (PUB400,
// IBM i 7.5, 2026-11-20).
ctl-opt dftactgrp(*no);
dcl-f QSYSPRT printer(80) usropn;
dcl-ds line len(80);
  text char(20);
  num zoned(5:0) pos(22);
end-ds;

open QSYSPRT;
text = 'first line';
num = 42;
write QSYSPRT line;
text = 'second line';
num = 7;
write QSYSPRT line;
close QSYSPRT;
*inlr = *on;
