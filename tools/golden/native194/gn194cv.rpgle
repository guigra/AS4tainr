**FREE
// Stage 2, S2.194: OPTIONS(*CONVERT) on a VALUE character parameter: an
// integer, a packed field, a date and a character field passed to it
// (the first three as %CHAR gives them). Captured on PUB400
// (tools/golden/native194.txt n194_convert) for the native unit NCONVRT.
ctl-opt dftactgrp(*no);
dcl-pr show varchar(40);
  v varchar(30) value options(*convert);
end-pr;
dcl-s n int(10) inz(-42);
dcl-s p packed(7:2) inz(1234.5);
dcl-s d date inz(d'2026-07-04');
dcl-s c char(6) inz('ab');
dcl-s line varchar(200);

line = show(n);
line += show(p);
line += show(d);
line += show(c);
say(line);
*inlr = *on;

dcl-proc show;
  dcl-pi *n varchar(40);
    v varchar(30) value options(*convert);
  end-pi;
  return '[' + v + ']';
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
