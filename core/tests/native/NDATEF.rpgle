**FREE
// Native unit NDATEF (S2.82): date, time and timestamp fields of a
// database file written, read, updated with date and time arithmetic and
// read again. The program captured on PUB400, its table NDATEFT made by
// the test in QGPL as the capture made GN82T, and DSPLY for its output.
// Source: golden capture tools/golden/native82 n82_datefile (PUB400, IBM
// i 7.5, 2026-11-18).
ctl-opt dftactgrp(*no);
dcl-f ndateft usage(*update: *output) rename(ndateft: rec);
dcl-s line varchar(80);
id = 1;
d = d'2026-01-31';
t = t'09.15.00';
ts = z'2026-01-31-23.59.59.000000';
write rec;
id = 2;
d = d'1815-12-10';
t = t'23.00.00';
ts = z'2000-02-29-12.00.00.500000';
write rec;
read rec;
dow not %eof(ndateft);
  show();
  if id = 1;
    d += %months(1);
    t += %hours(2);
    ts += %seconds(1);
    update rec;
  endif;
  read rec;
enddo;
setll *start ndateft;
read rec;
dow not %eof(ndateft);
  show();
  read rec;
enddo;
*inlr = *on;

dcl-proc show;
  dsply (%char(id) + ' ' + %char(d) + ' ' + %char(t) + ' ' + %char(ts));
end-proc;
