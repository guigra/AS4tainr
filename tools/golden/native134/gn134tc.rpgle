**FREE
// Stage 2, S2.134: %CHAR of date, time and timestamp expressions (a
// duration added, %DATE of characters) under DATFMT(*EUR)
// TIMFMT(*USA), and with an explicit format. Captured on PUB400
// (tools/golden/native134.txt n134_tchar; the native unit NTCHREX takes gn134ti.rpgle).
ctl-opt dftactgrp(*no) datfmt(*eur) timfmt(*usa);
dcl-s d date inz(d'31.01.2026');
dcl-s t time inz(t'09:15 AM');
dcl-s ts timestamp inz(z'2026-01-31-23.59.59.000000');

say(%char(t + %minutes(45)));
say(%char(ts + %hours(12)));
say(%char(%date('10/20/26' : *mdy)));
say(%char(d + %days(1) : *usa));
say(%char(d - %months(1)));
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
