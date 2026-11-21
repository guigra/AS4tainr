**FREE
// Stage 2, S2.201: a C function with float parameters passed by value
// (pow from the C runtime) and its float result compared with a number:
// integer and decimal arguments. Captured on PUB400
// (tools/golden/native201.txt n201_pow) for the native unit NCFLOAT.
ctl-opt dftactgrp(*no);
dcl-pr pow float(8) extproc('pow');
  x float(8) value;
  y float(8) value;
end-pr;
dcl-s line varchar(200);

if pow(2 : 10) = 1024;
  line = 'A';
endif;
if pow(2.5 : 2) = 6.25;
  line += 'B';
endif;
if pow(9 : 0.5) <> 3;
  line += 'C';
else;
  line += 'c';
endif;
say(line);
*inlr = *on;

dcl-proc say;
  dcl-pi *n;
    text varchar(200) const;
  end-pi;
  dcl-s wbuf varchar(200);
  wbuf = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :wbuf,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
