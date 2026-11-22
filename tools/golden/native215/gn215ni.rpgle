**FREE
// Interpreter fix (fork branch fix/sql-null-indicator-default): a null
// fetched with an indicator into host variables of several types that
// held values: what each holds after. Captured on PUB400
// (tools/golden/native215.txt n215_nullind).
ctl-opt dftactgrp(*no);
dcl-s c char(5) inz('abcde');
dcl-s v varchar(10) inz('xyz');
dcl-s d date inz(d'2026-01-31');
dcl-s i int(10) inz(7);
dcl-s z zoned(5:1) inz(12.5);
dcl-s ci int(5);
dcl-s vi int(5);
dcl-s di int(5);
dcl-s ii int(5);
dcl-s zi int(5);
dcl-s line varchar(200);

exec sql set option commit = *none;
exec sql drop table GUIGRA261/GN215T;
exec sql create table GUIGRA261/GN215T (k int not null, c char(5),
  v varchar(10), d date, i int, z decimal(5, 1));
exec sql insert into GUIGRA261/GN215T (k) values (1);
exec sql select c, v, d, i, z into :c :ci, :v :vi, :d :di, :i :ii, :z :zi
  from GUIGRA261/GN215T where k = 1;
line = %char(sqlcode) + ' ' + %char(ci) + %char(vi) + %char(di) + %char(ii)
  + %char(zi) + ' [' + c + '][' + v + '][' + %char(d) + '][' + %char(i)
  + '][' + %char(z) + ']';
say(line);
exec sql drop table GUIGRA261/GN215T;
*inlr = *on;

dcl-proc say;
  dcl-pi *n;
    text varchar(200) const;
  end-pi;
  dcl-s wbuf varchar(200);
  wbuf = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :wbuf,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
