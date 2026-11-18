**FREE
// Stage 2, S2.82: date, time and timestamp fields of a database file.
// GN82T (id int, d date, t time, ts timestamp; made with RUNSQL before
// the compile) written twice, read in arrival order, its first record
// updated with date and time arithmetic, then read again. Each line
// shows the fields with %CHAR. Captured on PUB400
// (tools/golden/native82.txt n82_datefile) for the native unit NDATEF.
ctl-opt dftactgrp(*no);
dcl-f gn82t usage(*update: *output) rename(gn82t: rec);
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
dow not %eof(gn82t);
  show();
  if id = 1;
    d += %months(1);
    t += %hours(2);
    ts += %seconds(1);
    update rec;
  endif;
  read rec;
enddo;
setll *start gn82t;
read rec;
dow not %eof(gn82t);
  show();
  read rec;
enddo;
*inlr = *on;

dcl-proc show;
  say(%char(id) + ' ' + %char(d) + ' ' + %char(t) + ' ' + %char(ts));
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
