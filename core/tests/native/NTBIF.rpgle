**FREE
// Stage 2, S2.54: %DATE, %TIME and %TIMESTAMP, lowered to Rust.
// Expected: what IBM i 7.5 gave (golden capture tools/golden/native55.txt
// n55_bif, cited in native_units.rs).
ctl-opt dftactgrp(*no) datfmt(*mdy) timfmt(*hms);
dcl-s d date(*iso);
dcl-s t time(*iso);
dcl-s z timestamp;
dcl-s c char(10) inz('2026-11-16');
dcl-s n packed(6:0) inz(261116);
dcl-s m packed(6:0) inz(111626);
dcl-s h packed(6:0) inz(93007);
d = %date(c : *iso);
dsply ('ISO ' + %char(d));
d = %date('11/17/26' : *mdy);
dsply ('MDY ' + %char(d));
d = %date(n : *ymd);
dsply ('YMD NUM ' + %char(d));
d = %date(m : *mdy);
dsply ('MDY NUM ' + %char(d));
t = %time('10.11.12');
dsply ('TIME DFT ' + %char(t));
t = %time(h : *hms);
dsply ('TIME NUM ' + %char(t));
z = %timestamp('2026-11-16-10.11.12.000001');
dsply ('TS ' + %char(z));
d = %date(z);
dsply ('DATE OF TS ' + %char(d));
t = %time(z);
dsply ('TIME OF TS ' + %char(t));
z = %timestamp(d);
dsply ('TS OF DATE ' + %char(z));
monitor;
  d = %date('2026-13-45' : *iso);
  dsply ('BAD OK');
on-error;
  dsply ('BAD ERROR ' + %char(%status));
endmon;
d = %date();
if d > %date('2026-01-01' : *iso);
  dsply ('SYS AFTER 2026-01-01');
endif;
*inlr = *on;
