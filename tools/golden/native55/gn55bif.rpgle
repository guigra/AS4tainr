**FREE
// Stage 2, S2.54: %DATE, %TIME and %TIMESTAMP of character values with a
// format, of numbers, of dates and timestamps, %DATE(), and an invalid
// date under MONITOR. With no format the value is taken in *ISO form, not
// DATFMT's (gn55dft.rpgle, n55_crt_dft: RNF7510; fidelity.md). Captured on PUB400
// (tools/golden/native55.txt n55_bif) for the native unit NTBIF.
ctl-opt dftactgrp(*no) datfmt(*mdy) timfmt(*hms);
dcl-s d date(*iso);
dcl-s t time(*iso);
dcl-s z timestamp;
dcl-s c char(10) inz('2026-11-16');
dcl-s n packed(6:0) inz(261116);
dcl-s m packed(6:0) inz(111626);
dcl-s h packed(6:0) inz(93007);
d = %date(c : *iso);
say('ISO ' + %char(d));
d = %date('11/17/26' : *mdy);
say('MDY ' + %char(d));
d = %date(n : *ymd);
say('YMD NUM ' + %char(d));
d = %date(m : *mdy);
say('MDY NUM ' + %char(d));
t = %time('10.11.12');
say('TIME DFT ' + %char(t));
t = %time(h : *hms);
say('TIME NUM ' + %char(t));
z = %timestamp('2026-11-16-10.11.12.000001');
say('TS ' + %char(z));
d = %date(z);
say('DATE OF TS ' + %char(d));
t = %time(z);
say('TIME OF TS ' + %char(t));
z = %timestamp(d);
say('TS OF DATE ' + %char(z));
monitor;
  d = %date('2026-13-45' : *iso);
  say('BAD OK');
on-error;
  say('BAD ERROR ' + %char(%status));
endmon;
d = %date();
if d > %date('2026-01-01' : *iso);
  say('SYS AFTER 2026-01-01');
endif;
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
