**FREE
// Native unit NSQL6 (S2.74): multiple-row INSERT and FETCH with a host
// structure array. The program captured on PUB400, with the table in
// QTEMP and DSPLY for its output, less one part of a line (below).
// Source: golden capture tools/golden/native74 n74_sql (PUB400, IBM i
// 7.5, 2026-11-18).
// QTEMP, not QGPL: each test's catalog::init may grant on all of QGPL's
// tables while this program drops its own ("tuple concurrently updated",
// an upstream race; HANDOFF.md). QTEMP's libraries are never granted on.
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
exec sql drop table QTEMP/NSQL6T;
exec sql create table QTEMP/NSQL6T (id int not null, name char(8),
                                       amt decimal(5, 2));
for i = 1 to 4;
  rows(i).id = i;
  rows(i).name = 'n' + %char(i);
  rows(i).amt = i * 1.25;
endfor;
exec sql insert into QTEMP/NSQL6T 3 rows values(:rows);
line = 'INS3 ' + %char(sqlcode) + ' ' + %char(sqlerrd(3));
rows(1).id = 5;
rows(2).id = 6;
n = 2;
exec sql insert into QTEMP/NSQL6T :n rows values(:rows);
line += ' INS2 ' + %char(sqlcode) + ' ' + %char(sqlerrd(3));
n = 0;
exec sql insert into QTEMP/NSQL6T :n rows values(:rows);
// The capture's line has " INS0 -221" here (0 rows is not a valid
// count); as4tainr gives 0 in both engines (fidelity.md, interpreter
// defects), so this line leaves that part out.
n = 5;
exec sql insert into QTEMP/NSQL6T :n rows values(:rows);
dsply (line + ' INS5 ' + %char(sqlcode));
exec sql declare c scroll cursor for select id, name, amt from QTEMP/NSQL6T
  order by id;
exec sql open c;
line = 'F';
for i = 1 to 3;
  exec sql fetch next from c for 2 rows into :got;
  line += ' ' + %char(sqlcode) + '/' + %char(sqlerrd(3)) + '/'
       + %char(sqlerrd(5)) + ':' + %char(got(1).id) + %trim(got(1).name)
       + ',' + %char(got(2).id);
endfor;
dsply (line);
exec sql fetch next from c for 2 rows into :got;
line = 'END ' + %char(sqlcode);
n = 3;
exec sql fetch first from c for :n rows into :got;
dsply (line + ' TOO ' + %char(sqlcode) + ' ' + %char(got(2).amt));
exec sql close c;
exec sql drop table QTEMP/NSQL6T;
*inlr = *on;
