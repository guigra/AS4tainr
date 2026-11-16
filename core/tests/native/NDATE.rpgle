**FREE
// Stage 2, S2.52: date, time and timestamp fields, lowered to Rust.
// Expected: what IBM i 7.5 gave (golden capture tools/golden/native53.txt
// n53_date, cited in native_units.rs). *LOVAL and *HIVAL are NDATEFIG:
// the interpreter differs from IBM i there (fidelity.md).
ctl-opt dftactgrp(*no) datfmt(*eur) timfmt(*hms);
dcl-s iso date(*iso);
dcl-s usa date(*usa);
dcl-s eur date(*eur);
dcl-s mdy date(*mdy);
dcl-s dft date;
dcl-s t1 time(*iso);
dcl-s t2 time(*hms);
dcl-s ts timestamp inz(z'2026-11-16-13.45.10.123456');
dcl-s ts2 timestamp;
dcl-s early date(*iso);
iso = %date('2026-11-16' : *iso);
t1 = %time('13.45.10' : *iso);
early = %date('1999-12-31' : *iso);
usa = iso;
eur = iso;
mdy = iso;
dft = iso;
dsply ('USA ' + %char(usa));
dsply ('EUR ' + %char(eur));
dsply ('MDY ' + %char(mdy));
dsply ('DFT ' + %char(dft));
iso = usa;
dsply ('BACK ' + %char(iso));
t2 = t1;
dsply ('HMS ' + %char(t2));
ts2 = ts;
dsply ('TS ' + %char(ts2));
if early < iso;
  dsply ('EARLY BEFORE');
endif;
if usa = eur;
  dsply ('USA EQ EUR');
endif;
if t1 > t2;
  dsply ('T1 AFTER');
else;
  dsply ('T1 NOT AFTER');
endif;
*inlr = *on;
