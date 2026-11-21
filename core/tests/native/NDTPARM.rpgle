**FREE
// Native unit NDTPARM (S2.189): date, time and timestamp parameters, the
// program captured on PUB400 as GN189DP with DSPLY for its output.
// Source: golden capture tools/golden/native189 n189_dtparm (PUB400,
// IBM i 7.5, 2026-11-21).
ctl-opt dftactgrp(*no);
dcl-s d date inz(d'2026-01-31');
dcl-s e date(*dmy) inz(d'2026-12-25');
dcl-s t time inz(t'13.45.10');
dcl-s z timestamp inz(z'2026-02-03-04.05.06.000007');
dcl-s line varchar(200);

nextMonth(d);
line = %char(d);
line += '|' + showTime(t);
addSec(z);
line += '|' + %char(z);
line += '|' + showDate(d) + ' ' + showDate(e);
dsply (line);
*inlr = *on;

dcl-proc nextMonth;
  dcl-pi *n;
    x date;
  end-pi;
  x += %months(1);
end-proc;

dcl-proc showTime;
  dcl-pi *n varchar(20);
    x time const;
  end-pi;
  return %char(x : *usa);
end-proc;

dcl-proc addSec;
  dcl-pi *n;
    x timestamp;
  end-pi;
  x += %seconds(90);
end-proc;

dcl-proc showDate;
  dcl-pi *n varchar(20);
    x date const;
  end-pi;
  return %char(x : *iso);
end-proc;
