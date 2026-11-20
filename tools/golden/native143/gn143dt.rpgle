**FREE
// Stage 2, S2.143: %DEC of a date (its own format, and *USA), a time, a
// timestamp, and a date expression, assigned and in %CHAR. Captured on
// PUB400 (tools/golden/native143.txt n143_dectmp) for the native unit
// NDECTMP.
ctl-opt dftactgrp(*no);
dcl-s d date inz(d'2026-01-31');
dcl-s t time inz(t'09.15.00');
dcl-s ts timestamp inz(z'2026-01-31-23.59.59.123456');
dcl-s n packed(8:0);
dcl-s line varchar(100);

n = %dec(d);
line = %char(n) + ' ' + %char(%dec(d : *usa)) + ' ' + %char(%dec(t))
     + ' ' + %char(%dec(ts)) + ' ' + %char(%dec(d + %days(1)));
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
