**FREE
// Native unit NFREAD (S2.58): reads of a database file. SETLL, SETGT,
// CHAIN, READ, READE, READP, READPE with key fields and *LOVAL/*HIVAL;
// %EOF, %FOUND and %EQUAL; zoned, packed, character and hexadecimal
// fields. The program and the file's rows are those captured on PUB400
// with DSPLY for its output, less one READPE (below).
// Source: golden capture tools/golden/native58 n58_r (PUB400, IBM i 7.5,
// 2026-11-17).
ctl-opt dftactgrp(*no);
dcl-f NFR58PF keyed;
dcl-s kOrd zoned(3 : 0);
dcl-s kLin packed(3 : 0);
kOrd = 1;
setll kOrd NFR58PF;
if %equal;
  dsply ('SETLL1 EQUAL');
endif;
if %found;
  dsply ('SETLL1 FOUND');
endif;
reade kOrd NFR58PF;
dow not %eof(NFR58PF);
  dsply ('RE ' + %char(ORD) + ' ' + %char(LIN) + ' ' + %trim(NAME) + ' ' + %char(AMT));
  if HX = x'C1C2';
    dsply ('HX C1C2');
  endif;
  reade kOrd NFR58PF;
enddo;
// The capture's READPE here, right after READE reached end of file, is
// left out: IBM i gives end of file (n58_r "RPE1 EOF"), as4tainr's rla the
// key's last record, in both engines (fidelity.md, interpreter defects).
// The SETGT below positions the file again, so the lines after it are
// the capture's.
setgt kOrd NFR58PF;
read NFR58PF;
dsply ('GT1 READ ' + %char(ORD) + ' ' + %char(LIN));
readp NFR58PF;
dsply ('READP ' + %char(ORD) + ' ' + %char(LIN));
kOrd = 9;
kLin = 9;
chain (kOrd : kLin) NFR58PF;
if not %found(NFR58PF);
  dsply ('CH99 NOT FOUND');
endif;
kOrd = 2;
kLin = 1;
chain (kOrd : kLin) NFR58PF;
if %found;
  dsply ('CH21 ' + %trim(NAME) + ' ' + %char(AMT));
endif;
setll *loval NFR58PF;
read NFR58PF;
dsply ('LOVAL READ ' + %char(ORD) + ' ' + %char(LIN));
setll *hival NFR58PF;
readp NFR58PF;
dsply ('HIVAL READP ' + %char(ORD) + ' ' + %char(LIN));
read NFR58PF;
if %eof(NFR58PF);
  dsply ('HIVAL READ EOF');
endif;
kOrd = 5;
setll kOrd NFR58PF;
if not %equal and not %found;
  dsply ('SETLL5 NEITHER');
endif;
kOrd = 1;
kLin = 2;
setll (kOrd : kLin) NFR58PF;
if %equal;
  dsply ('SETLL12 EQUAL');
endif;
reade (kOrd : kLin) NFR58PF;
dsply ('RE12 ' + %trim(NAME));
reade (kOrd : kLin) NFR58PF;
if %eof;
  dsply ('RE12 AGAIN EOF');
endif;
kOrd = 2;
setll kOrd NFR58PF;
readpe kOrd NFR58PF;
if %eof;
  dsply ('RPE2 EOF');
endif;
setgt kOrd NFR58PF;
readpe kOrd NFR58PF;
dsply ('GT2 RPE2 ' + %trim(NAME));
*inlr = *on;
