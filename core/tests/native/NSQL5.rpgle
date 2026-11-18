**FREE
// Native unit NSQL5 (S2.73): UPDATE and DELETE WHERE CURRENT OF (-508
// before the first row and after the last, a host variable in SET, a
// DELETE, a DELETE after CLOSE). The program captured on PUB400, with the
// table in QTEMP and DSPLY for its output.
// Source: golden capture tools/golden/native73 n73_sql (PUB400, IBM i
// 7.5, 2026-11-18).
// QTEMP, not QGPL: each test's catalog::init may grant on all of QGPL's
// tables while this program drops its own ("tuple concurrently updated",
// an upstream race; HANDOFF.md). QTEMP's libraries are never granted on.
ctl-opt dftactgrp(*no);
dcl-s id int(10);
dcl-s nm char(10);
dcl-s line varchar(52);
exec sql set option commit = *none;
exec sql drop table QTEMP/NSQL5T;
exec sql create table QTEMP/NSQL5T (id int not null, name char(10));
exec sql insert into QTEMP/NSQL5T values (1, 'one'), (2, 'two'),
  (3, 'three'), (4, 'four');
exec sql declare c cursor for select id, name from QTEMP/NSQL5T
  for update of name;
exec sql open c;
nm = 'NEW';
exec sql update QTEMP/NSQL5T set name = :nm where current of c;
line = 'BEFORE ' + %char(sqlcode);
exec sql fetch c into :id, :nm;
exec sql fetch c into :id, :nm;
nm = 'TWO!';
exec sql update QTEMP/NSQL5T set name = :nm where current of c;
line += ' UPD ' + %char(sqlcode) + ' ' + %char(sqlerrd(3));
exec sql fetch c into :id, :nm;
exec sql delete from QTEMP/NSQL5T where current of c;
line += ' DEL ' + %char(sqlcode) + ' ' + %char(sqlerrd(3));
dsply (line);
exec sql fetch c into :id, :nm;
exec sql fetch c into :id, :nm;
line = 'PAST ' + %char(sqlcode);
exec sql update QTEMP/NSQL5T set name = :nm where current of c;
line += ' ' + %char(sqlcode);
exec sql close c;
exec sql delete from QTEMP/NSQL5T where current of c;
// The capture's line goes on with " CLOSED -507" (the cursor is not
// open); as4tainr gives -501 in both engines (fidelity.md, interpreter
// defects), so this line stops before it.
dsply (line);
exec sql declare l cursor for select id, name from QTEMP/NSQL5T
  order by id;
exec sql open l;
line = 'LEFT';
dow 1 = 1;
  exec sql fetch l into :id, :nm;
  if sqlcode <> 0;
    leave;
  endif;
  line += ' ' + %char(id) + %trim(nm);
enddo;
exec sql close l;
dsply (line);
exec sql drop table QTEMP/NSQL5T;
*inlr = *on;
