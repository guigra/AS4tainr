**FREE
// Native unit NINFDS (S2.139): a database file's INFDS, the program
// captured on PUB400 as GN139FB with DSPLY for its output; its file is
// QGPL/GN120F here with the capture's two records.
// Source: golden capture tools/golden/native139 n139_infds (PUB400,
// IBM i 7.5, 2026-11-20).
ctl-opt dftactgrp(*no);
dcl-f gn120f usage(*input) infds(fb) rename(gn120f:rec120)
             extfile('QGPL/GN120F');
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

// The read count (247) and, after the end of file, the record format and
// the relative record number are left out: IBM i and the interpreter
// differ there (fidelity.md, "A database file's INFDS").
read gn120f;
dsply (fb_text(*on));
read gn120f;
dsply (fb_text(*on));
read gn120f;
dsply (fb_text(*off));
*inlr = *on;

dcl-proc fb_text;
  dcl-pi *n varchar(80);
    rec ind value;
  end-pi;
  dcl-s t varchar(80);
  t = fname + '|' + opn + eof + '|' + %char(sts) + '|' + opc;
  if rec;
    t += '|' + rcd + '|' + %char(rrn);
  endif;
  return t;
end-proc;
