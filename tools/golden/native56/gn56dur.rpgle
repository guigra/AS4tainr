**FREE
// Stage 2, S2.55: %DIFF, %SUBDT, and dates, times and timestamps plus or
// minus durations; comparisons of temporal expressions; a date past
// 9999-12-31 under MONITOR. Captured on PUB400 (tools/golden/native56.txt
// n56_dur) for the native unit NTDUR. A duration on the left of + is not
// valid on IBM i (gn56first.rpgle, n56_crt_first: RNF7421; fidelity.md).
ctl-opt dftactgrp(*no);
dcl-s d1 date(*iso);
dcl-s d2 date(*iso);
dcl-s t1 time(*iso);
dcl-s z1 timestamp;
dcl-s z2 timestamp;
dcl-s n int(10);
d1 = %date('2026-03-31' : *iso);
d2 = %date('2025-01-15' : *iso);
t1 = %time('23.30.00' : *iso);
z1 = %timestamp('2026-11-16-10.00.00.000000');
z2 = %timestamp('2026-11-15-08.30.15.500000');
n = %diff(d1 : d2 : *days);
say('DAYS ' + %char(n));
n = %diff(d1 : d2 : *months);
say('MONTHS ' + %char(n));
n = %diff(d1 : d2 : *years);
say('YEARS ' + %char(n));
n = %diff(z1 : z2 : *hours);
say('HOURS ' + %char(n));
n = %diff(z1 : z2 : *seconds);
say('SECONDS ' + %char(n));
n = %diff(d2 : d1 : *days);
say('NEG DAYS ' + %char(n));
n = %subdt(d1 : *months);
say('SUBDT M ' + %char(n));
n = %subdt(z2 : *mseconds);
say('SUBDT MS ' + %char(n));
d2 = d1 - %months(1);
say('MINUS MONTH ' + %char(d2));
d2 = d1 + %days(1);
say('PLUS DAY ' + %char(d2));
d2 = d1 + %days(30) - %months(1);
say('CHAIN ' + %char(d2));
t1 = t1 + %hours(1);
say('TIME WRAP ' + %char(t1));
z2 = z1 + %mseconds(1500);
say('TS MS ' + %char(z2));
z2 = z1 - %minutes(90);
say('TS MIN ' + %char(z2));
if d1 > %date('2026-01-01' : *iso);
  say('D1 AFTER');
endif;
if d1 + %days(1) = %date('2026-04-01' : *iso);
  say('NEXT DAY APRIL');
endif;
d1 = %date('9999-12-31' : *iso);
monitor;
  d2 = d1 + %days(1);
  say('OVER OK');
on-error;
  say('OVER ERROR ' + %char(%status));
endmon;
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
