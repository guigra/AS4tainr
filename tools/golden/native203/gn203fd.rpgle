**FREE
// Stage 2, S2.203: %DEC, %DECH, %INT and %INTH of float values: a C
// function's float result (QXXZTOD, pow) and float fields. Captured on
// PUB400 (tools/golden/native203.txt n203_floatdec) for the native unit
// NFLTDEC.
ctl-opt dftactgrp(*no);
dcl-pr QXXZTOD float(8) extproc('QXXZTOD');
  source pointer value;
  digits int(10) value;
  frac int(10) value;
end-pr;
dcl-pr pow float(8) extproc('pow');
  x float(8) value;
  y float(8) value;
end-pr;
dcl-s zoned zoned(7:2) inz(-12.55);
dcl-s f float(8) inz(2.675);
dcl-s g float(8) inz(-7.5);
dcl-s line varchar(200);

line = %char(%dech(QXXZTOD(%addr(zoned) : 7 : 2) : 7 : 1));
line += ' ' + %char(%dec(f : 7 : 2));
line += ' ' + %char(%dech(g : 5 : 0));
line += ' ' + %char(%int(g));
line += ' ' + %char(%inth(pow(2 : 0.5)));
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
