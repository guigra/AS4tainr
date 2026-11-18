**FREE
// Stage 2, S2.79: WRITE of an externally described printer file's record
// formats (GN79P from tools/golden/native79/gn79p.prtf): HDR with a
// constant and PAGNBR, DTL with a character field, edit codes J and Z
// and a constant conditioned by indicator 50 (on, then off), TOT spaced
// before with an edit word, then CLOSE. Captured on PUB400
// (tools/golden/native79.txt n79_write) for the native unit NPRTW.
ctl-opt dftactgrp(*no);
dcl-f gn79p printer;
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
close gn79p;
*inlr = *on;
