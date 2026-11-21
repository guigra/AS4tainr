**FREE
// Stage 2, S2.202: float expressions as a C function's float parameters
// passed by value: a float literal (12.5e0) and a float field to
// QXXDTOZ, a float field to pow. Captured on PUB400
// (tools/golden/native202.txt n202_cfloat) for the native unit NCFLTEX.
ctl-opt dftactgrp(*no);
dcl-pr QXXDTOZ extproc('QXXDTOZ');
  target pointer value;
  digits int(10) value;
  frac int(10) value;
  value float(8) value;
end-pr;
dcl-pr pow float(8) extproc('pow');
  x float(8) value;
  y float(8) value;
end-pr;
dcl-s zoned zoned(7:2);
dcl-s f float(8) inz(-3.25);
dcl-s g float(8) inz(3);
dcl-s line varchar(200);

QXXDTOZ(%addr(zoned) : 7 : 2 : 12.5e0);
line = %char(zoned);
QXXDTOZ(%addr(zoned) : 7 : 2 : f);
line += ' ' + %char(zoned);
if pow(g : 2) = 9;
  line += ' P';
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
