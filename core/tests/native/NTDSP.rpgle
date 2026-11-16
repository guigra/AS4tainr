**FREE
// Stage 2, S2.53: DSPLY of date, time and timestamp fields, for parity
// between the engines (native_diff.rs): Stmt::Dsply shows a date or time
// field as %CHAR of it (NTCHAR has IBM i's %CHAR values) and a timestamp
// field as its value.
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
dsply usa;
dsply dft;
dsply hms;
dsply dftt;
dsply ts;
*inlr = *on;
