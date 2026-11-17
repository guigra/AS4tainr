**FREE
// Stage 2, S2.55: %DIFF, %SUBDT and durations, lowered to Rust.
// Expected: what IBM i 7.5 gave (golden capture tools/golden/native56.txt
// n56_dur, cited in native_units.rs). The date past 9999-12-31 is in
// NTDURF: the interpreter's status differs from IBM i's (fidelity.md).
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
dsply ('DAYS ' + %char(n));
n = %diff(d1 : d2 : *months);
dsply ('MONTHS ' + %char(n));
n = %diff(d1 : d2 : *years);
dsply ('YEARS ' + %char(n));
n = %diff(z1 : z2 : *hours);
dsply ('HOURS ' + %char(n));
n = %diff(z1 : z2 : *seconds);
dsply ('SECONDS ' + %char(n));
n = %diff(d2 : d1 : *days);
dsply ('NEG DAYS ' + %char(n));
n = %subdt(d1 : *months);
dsply ('SUBDT M ' + %char(n));
n = %subdt(z2 : *mseconds);
dsply ('SUBDT MS ' + %char(n));
d2 = d1 - %months(1);
dsply ('MINUS MONTH ' + %char(d2));
d2 = d1 + %days(1);
dsply ('PLUS DAY ' + %char(d2));
d2 = d1 + %days(30) - %months(1);
dsply ('CHAIN ' + %char(d2));
t1 = t1 + %hours(1);
dsply ('TIME WRAP ' + %char(t1));
z2 = z1 + %mseconds(1500);
dsply ('TS MS ' + %char(z2));
z2 = z1 - %minutes(90);
dsply ('TS MIN ' + %char(z2));
if d1 > %date('2026-01-01' : *iso);
  dsply ('D1 AFTER');
endif;
if d1 + %days(1) = %date('2026-04-01' : *iso);
  dsply ('NEXT DAY APRIL');
endif;
*inlr = *on;
