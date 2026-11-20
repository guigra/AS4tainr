**FREE
// Stage 2, S2.157: a date plus a time is a timestamp: %DATE + %TIME,
// a date field plus a time field (date first: IBM i refuses a time plus a date, RNF7421), into timestamps of
// 6 and 0 fractional digits. Captured on PUB400 (tools/golden/native157.txt
// n157_datetime) for the native unit NDATTIM.
ctl-opt dftactgrp(*no);
dcl-s t6 timestamp;
dcl-s t0 timestamp(0);
dcl-s d date inz(d'2026-01-31');
dcl-s t time inz(t'23.59.58');
dcl-s line varchar(120);

t6 = %date('2024-09-16': *iso) + %time('11.22.33': *iso);
line = %char(t6);
t6 = d + t;
line += ' ' + %char(t6);
t0 = d + t;
line += ' ' + %char(t0);
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
