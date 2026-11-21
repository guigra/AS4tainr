**FREE
// Stage 2, S2.188: calls of an OVERLOAD prototype: the candidate by the
// arguments' kinds (a numeric literal, a date field, a character field,
// an integer field). Captured on PUB400 (tools/golden/native188.txt
// n188_overload) for the native unit NOVERLD.
ctl-opt dftactgrp(*no);
dcl-s d date inz(d'2026-03-04');
dcl-s c char(5) inz('abc');
dcl-s i int(10) inz(-7);
dcl-s line varchar(200);
// The OVERLOAD prototype has no END-PR (an END-PR is RNF3551 on IBM i);
// a prototype follows it (the interpreter ends it only there).
dcl-pr fmt varchar(30) overload(fmtNum : fmtDate : fmtChr);
dcl-pr fmtNum varchar(30);
  n packed(9:2) const;
end-pr;

// One statement per call: each runs natively or not on its own.
line = fmt(12.5);
line += '|' + fmt(d);
line += '|' + fmt(c);
line += '|' + fmt(i);
say(line);
*inlr = *on;

dcl-proc fmtNum;
  dcl-pi *n varchar(30);
    n packed(9:2) const;
  end-pi;
  return 'N' + %char(n);
end-proc;

dcl-proc fmtDate;
  dcl-pi *n varchar(30);
    x date const;
  end-pi;
  return 'D' + %char(x : *dmy);
end-proc;

dcl-proc fmtChr;
  dcl-pi *n varchar(30);
    s varchar(20) const;
  end-pi;
  return 'C' + %trim(s);
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
