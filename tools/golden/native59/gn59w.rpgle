**FREE
// Stage 2, S2.59: WRITE, UPDATE and DELETE of a database file. Records
// written from the program's fields (zoned, packed, character and
// hexadecimal), one updated after a CHAIN, one deleted by its key, one
// deleted after a CHAIN, then the file read back. Captured on PUB400
// (tools/golden/native59.txt n59_w) for the native unit NFWRITE.
ctl-opt dftactgrp(*no);
dcl-f GN59PF keyed usage(*update : *delete : *output);
dcl-s kOrd zoned(3 : 0);
dcl-s kLin packed(3 : 0);
ORD = 1;
LIN = 1;
NAME = 'Alpha';
AMT = 10.50;
HX = x'C1C2';
write GN59FMT;
ORD = 1;
LIN = 2;
NAME = 'Beta';
AMT = -3.25;
HX = x'0000';
write GN59FMT;
ORD = 2;
LIN = 1;
NAME = 'Gamma';
AMT = 99999.99;
HX = x'FFFF';
write GN59FMT;
ORD = 3;
LIN = 1;
NAME = 'Delta';
AMT = 0;
HX = x'4040';
write GN59FMT;
kOrd = 1;
kLin = 2;
chain (kOrd : kLin) GN59PF;
if %found;
  NAME = 'Beta2';
  AMT = AMT * 2;
  update GN59FMT;
endif;
kOrd = 2;
kLin = 1;
delete (kOrd : kLin) GN59FMT;
kOrd = 3;
kLin = 1;
chain (kOrd : kLin) GN59PF;
if %found;
  delete GN59FMT;
endif;
setll *loval GN59PF;
read GN59PF;
dow not %eof(GN59PF);
  say('ROW ' + %char(ORD) + ' ' + %char(LIN) + ' ' + %trim(NAME) + ' ' + %char(AMT));
  if HX = x'C1C2';
    say('HX C1C2');
  endif;
  read GN59PF;
enddo;
*inlr = *on;

dcl-proc say;
  dcl-pi *n;
    text varchar(200) const;
  end-pi;
  dcl-s line varchar(200);
  line = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :line,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
