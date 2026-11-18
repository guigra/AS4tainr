**FREE
// Native unit NPRTW (S2.79): WRITE of an externally described printer
// file's record formats, then CLOSE. The program captured on PUB400, with
// the printer file NPRTWP made from the same DDS (NPRTWP.prtf, the
// capture's gn79p.prtf); the printed lines are compared with the lines
// PASE system printed from it.
// Source: golden capture tools/golden/native79 n79_write (PUB400, IBM i
// 7.5, 2026-11-18).
ctl-opt dftactgrp(*no);
dcl-f nprtwp printer;
amt = 1234.5;
nm = 'ALPHA';
qty = 7;
*in50 = *on;
write hdr;
write dtl;
amt = -42;
nm = 'BETA';
*in50 = *off;
write dtl;
write tot;
close nprtwp;
*inlr = *on;
