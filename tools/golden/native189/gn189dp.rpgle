**FREE
// Stage 2, S2.189: date, time and timestamp parameters of procedures:
// a date by reference changed by the procedure, a time CONST, a
// timestamp by reference, and a date CONST given a field of another
// format. Captured on PUB400 (tools/golden/native189.txt n189_dtparm)
// for the native unit NDTPARM.
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
say(line);
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

dcl-proc say;
  dcl-pi *n;
    text varchar(200) const;
  end-pi;
  dcl-s wbuf varchar(200);
  wbuf = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :wbuf,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
