**FREE
// Stage 2, S2.53: %CHAR of date, time and timestamp fields, lowered to
// Rust. Expected: what IBM i 7.5 gave (golden capture
// tools/golden/native54.txt n54_chr, cited in native_units.rs).
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
dsply ('USA ' + %char(usa));
dsply ('DFT ' + %char(dft));
dsply ('HMS ' + %char(hms));
dsply ('DFTT ' + %char(dftt));
dsply ('TS ' + %char(ts));
dsply ('YMD ' + %char(usa : *ymd));
dsply ('JUL ' + %char(usa : *jul));
dsply ('ISO0 ' + %char(usa : *iso0));
dsply ('EUR ' + %char(hms : *eur));
dsply ('TS0 ' + %char(ts : *iso0));
*inlr = *on;
