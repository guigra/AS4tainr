**FREE
// Stage 2, S2.74: multiple-row INSERT and FETCH with a host structure
// array. INSERT of 3 rows (a literal count) and of 2 more (a host
// variable), 0 rows, more rows than elements (-221); FETCH FOR 2 ROWS
// three times (SQLERRD(3) and SQLERRD(5), the last row), FETCH FOR more
// rows than elements (-221). Captured on PUB400 (tools/golden/native74.txt
// n74_sql) for the native unit NSQL6.
ctl-opt dftactgrp(*no);
dcl-ds rows qualified dim(4);
  id int(10);
  name char(8);
  amt packed(5:2);
end-ds;
dcl-ds got likeds(rows) dim(2);
dcl-s n int(10);
dcl-s i int(10);
dcl-s line varchar(52);
exec sql set option commit = *none;
exec sql drop table GUIGRA261/GN74T;
exec sql create table GUIGRA261/GN74T (id int not null, name char(8),
                                       amt decimal(5, 2));
for i = 1 to 4;
  rows(i).id = i;
  rows(i).name = 'n' + %char(i);
  rows(i).amt = i * 1.25;
endfor;
exec sql insert into GUIGRA261/GN74T 3 rows values(:rows);
line = 'INS3 ' + %char(sqlcode) + ' ' + %char(sqlerrd(3));
rows(1).id = 5;
rows(2).id = 6;
n = 2;
exec sql insert into GUIGRA261/GN74T :n rows values(:rows);
line += ' INS2 ' + %char(sqlcode) + ' ' + %char(sqlerrd(3));
n = 0;
exec sql insert into GUIGRA261/GN74T :n rows values(:rows);
line += ' INS0 ' + %char(sqlcode);
n = 5;
exec sql insert into GUIGRA261/GN74T :n rows values(:rows);
say(line + ' INS5 ' + %char(sqlcode));
exec sql declare c scroll cursor for select id, name, amt from GUIGRA261/GN74T
  order by id;
exec sql open c;
line = 'F';
for i = 1 to 3;
  exec sql fetch next from c for 2 rows into :got;
  line += ' ' + %char(sqlcode) + '/' + %char(sqlerrd(3)) + '/'
       + %char(sqlerrd(5)) + ':' + %char(got(1).id) + %trim(got(1).name)
       + ',' + %char(got(2).id);
endfor;
say(line);
exec sql fetch next from c for 2 rows into :got;
line = 'END ' + %char(sqlcode);
n = 3;
exec sql fetch first from c for :n rows into :got;
say(line + ' TOO ' + %char(sqlcode) + ' ' + %char(got(2).amt));
exec sql close c;
exec sql drop table GUIGRA261/GN74T;
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
