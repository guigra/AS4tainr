**FREE
// Stage 2, S2.106 (ADR 0008): EVAL of float fields to decimal values: a
// product, a division, an integer field, a negative literal, into
// float(8) and float(4), and a procedure's own float(8). The values are
// shown with %CHAR. Captured on PUB400 (tools/golden/native107.txt
// n107_floateval) for the native unit NFLTEVL.
ctl-opt dftactgrp(*no);
dcl-s f8 float(8);
dcl-s f4 float(4);
dcl-s n int(10) inz(7);
dcl-s p packed(7:2) inz(12.25);
dcl-s line varchar(200);
f8 = p * 3;
line = %char(f8);
f8 = 10 / 4;
line += ' ' + %char(f8);
f4 = n;
line += ' ' + %char(f4);
f4 = -2.5;
line += ' ' + %char(f4);
line += ' ' + local();
say(line);
*inlr = *on;

dcl-proc local;
  dcl-pi *n varchar(40);
  end-pi;
  dcl-s lf float(8);
  lf = 1 / 3;
  return %char(lf);
end-proc;

dcl-proc say;
  dcl-pi *n;
    text varchar(200) const;
  end-pi;
  dcl-s wbuf varchar(200);
  wbuf = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :wbuf,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
