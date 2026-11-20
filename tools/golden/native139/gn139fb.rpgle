**FREE
// Stage 2, S2.139: a database file's INFDS after its operations: the
// file, open and end-of-file flags, status, operation, record format,
// read count and relative record number, after two READs and the READ at
// the end (GN120F of native120: 50 zz, 6 de). Captured on PUB400
// (tools/golden/native139.txt n139_infds) for the native unit NINFDS.
ctl-opt dftactgrp(*no);
dcl-f gn120f usage(*input) infds(fb) rename(gn120f:rec120)
             extfile('GUIGRA261/GN120F');
dcl-ds fb;
  fname char(8) pos(1);
  opn char(1) pos(9);
  eof char(1) pos(10);
  sts zoned(5:0) pos(11);
  opc char(6) pos(16);
  rcd char(8) pos(38);
  nrd int(10) pos(247);
  rrn int(10) pos(397);
end-ds;

read gn120f;
say(fb_text());
read gn120f;
say(fb_text());
read gn120f;
say(fb_text());
*inlr = *on;

dcl-proc fb_text;
  dcl-pi *n varchar(80);
  end-pi;
  return fname + '|' + opn + eof + '|' + %char(sts) + '|' + opc + '|'
       + rcd + '|' + %char(nrd) + '|' + %char(rrn);
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
