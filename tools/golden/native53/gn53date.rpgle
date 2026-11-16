**FREE
// Stage 2, S2.52: date, time and timestamp fields assigned (between
// formats, from literals, *LOVAL and *HIVAL) and compared; the program's
// DATFMT for a field with no format. No date or time literals: with
// DATFMT(*EUR) IBM i takes them in *EUR form only, the interpreter in
// *ISO form only (fidelity.md); %DATE and %TIME of *ISO text instead. Captured on PUB400
// (tools/golden/native53.txt n53_date) for the native unit NDATE.
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
say('USA ' + %char(usa));
say('EUR ' + %char(eur));
say('MDY ' + %char(mdy));
say('DFT ' + %char(dft));
iso = usa;
say('BACK ' + %char(iso));
t2 = t1;
say('HMS ' + %char(t2));
ts2 = ts;
say('TS ' + %char(ts2));
if early < iso;
  say('EARLY BEFORE');
endif;
if usa = eur;
  say('USA EQ EUR');
endif;
if t1 > t2;
  say('T1 AFTER');
else;
  say('T1 NOT AFTER');
endif;
iso = *loval;
say('LOVAL ' + %char(iso));
iso = *hival;
say('HIVAL ' + %char(iso));
usa = *hival;
say('USA HIVAL ' + %char(usa));
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
