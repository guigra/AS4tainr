**FREE
// Stage 2, S2.53: %CHAR of date, time and timestamp fields, with no format
// (the field's, or the control specification's DATFMT and TIMFMT) and with
// an explicit one. Captured on PUB400 (tools/golden/native54.txt n54_chr)
// for the native unit NTCHAR.
ctl-opt dftactgrp(*no) datfmt(*dmy) timfmt(*usa);
dcl-s usa date(*usa);
dcl-s dft date;
dcl-s hms time(*hms);
dcl-s dftt time;
dcl-s ts timestamp;
usa = %date('2026-11-16' : *iso);
dft = usa;
hms = %time('09.05.07' : *iso);
dftt = hms;
ts = %timestamp('2026-11-16-09.05.07.123456' : *iso);
say('USA ' + %char(usa));
say('DFT ' + %char(dft));
say('HMS ' + %char(hms));
say('DFTT ' + %char(dftt));
say('TS ' + %char(ts));
say('YMD ' + %char(usa : *ymd));
say('JUL ' + %char(usa : *jul));
say('ISO0 ' + %char(usa : *iso0));
say('EUR ' + %char(hms : *eur));
say('TS0 ' + %char(ts : *iso0));
*inlr = *on;
// Write one line to gn1.out in the job's current directory (DSPLY under
// `system` goes to QSYSOPR, a queue PUB400 users share).
dcl-proc say;
  dcl-pi *n;
    text varchar(200) const;
  end-pi;
  dcl-s line varchar(200);
  line = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :line,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
