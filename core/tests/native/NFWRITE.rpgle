**FREE
// Native unit NFWRITE (S2.59): WRITE, UPDATE and DELETE of a database
// file. Records written from the program's fields (zoned, packed,
// character and hexadecimal), one updated after a CHAIN, one deleted by
// its key, one deleted after a CHAIN, then the file read back. The
// program and the file are those captured on PUB400, with DSPLY for its
// output.
// Source: golden capture tools/golden/native59 n59_w (PUB400, IBM i 7.5,
// 2026-11-17).
ctl-opt dftactgrp(*no);
dcl-f NFW59PF keyed usage(*update : *delete : *output);
dcl-s kOrd zoned(3 : 0);
dcl-s kLin packed(3 : 0);
ORD = 1;
LIN = 1;
NAME = 'Alpha';
AMT = 10.50;
HX = x'C1C2';
write NFW59FMT;
ORD = 1;
LIN = 2;
NAME = 'Beta';
AMT = -3.25;
HX = x'0000';
write NFW59FMT;
ORD = 2;
LIN = 1;
NAME = 'Gamma';
AMT = 99999.99;
HX = x'FFFF';
write NFW59FMT;
ORD = 3;
LIN = 1;
NAME = 'Delta';
AMT = 0;
HX = x'4040';
write NFW59FMT;
kOrd = 1;
kLin = 2;
chain (kOrd : kLin) NFW59PF;
if %found;
  NAME = 'Beta2';
  AMT = AMT * 2;
  update NFW59FMT;
endif;
kOrd = 2;
kLin = 1;
delete (kOrd : kLin) NFW59FMT;
kOrd = 3;
kLin = 1;
chain (kOrd : kLin) NFW59PF;
if %found;
  delete NFW59FMT;
endif;
setll *loval NFW59PF;
read NFW59PF;
dow not %eof(NFW59PF);
  dsply ('ROW ' + %char(ORD) + ' ' + %char(LIN) + ' ' + %trim(NAME) + ' ' + %char(AMT));
  if HX = x'C1C2';
    dsply ('HX C1C2');
  endif;
  read NFW59PF;
enddo;
*inlr = *on;
