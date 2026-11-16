**FREE
// Stage 2, S2.54, compile only: %DATE with no format under DATFMT(*MDY)
// and %DATE(*SYS); IBM i takes a value with no format in *ISO form
// (RNF7510) and rejects *SYS (RNF0605). The source as first written for:
// %DATE, %TIME and %TIMESTAMP of character values with a
// format and with the control specification's, of numbers, of dates and
// timestamps, *SYS, and an invalid date under MONITOR. Captured on PUB400
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
d = %date('11/17/26');
say('DFT ' + %char(d));
d = %date(n : *ymd);
say('YMD NUM ' + %char(d));
d = %date(m);
say('DFT NUM ' + %char(d));
t = %time('10:11:12');
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
d = %date(*sys);
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
