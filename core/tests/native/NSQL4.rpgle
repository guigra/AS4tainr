**FREE
// Native unit NSQL4 (S2.72): cursors. A cursor declared with a host
// variable bound at OPEN, fetched into a data structure to the end, OPEN
// of an open cursor, FETCH and CLOSE of a closed one, a scroll cursor, a
// cursor over a prepared statement opened with USING, a null with an
// indicator. The program captured on PUB400, with the table in QTEMP and
// DSPLY for its output.
// Source: golden capture tools/golden/native72 n72_sql (PUB400, IBM i
// 7.5, 2026-11-18).
// QTEMP, not QGPL: each test's catalog::init may grant on all of QGPL's
// tables while this program drops its own ("tuple concurrently updated",
// an upstream race; HANDOFF.md). QTEMP's libraries are never granted on.
ctl-opt dftactgrp(*no);
dcl-s lim packed(5:0);
dcl-s nm char(10);
dcl-s id int(10);
dcl-s amt packed(7:2);
dcl-s amti int(5);
dcl-s stmt varchar(200);
dcl-s line varchar(52);
dcl-ds r qualified;
  id int(10);
  name char(10);
end-ds;
exec sql set option commit = *none;
exec sql drop table QTEMP/NSQL4T;
exec sql create table QTEMP/NSQL4T (id int not null, name char(10),
                                       amt decimal(7, 2));
exec sql insert into QTEMP/NSQL4T values (1, 'one', 1.50),
  (2, 'two', null), (3, 'three', 3.00), (4, 'four', 4.25);
exec sql declare c1 cursor for select id, name from QTEMP/NSQL4T
  where id > :lim order by id for read only;
lim = 1;
exec sql open c1;
line = 'OPEN ' + %char(sqlcode);
lim = 3;
exec sql open c1;
dsply (line + ' AGAIN ' + %char(sqlcode));
line = 'ROWS';
dow 1 = 1;
  exec sql fetch next from c1 into :r;
  if sqlcode <> 0;
    leave;
  endif;
  line += ' ' + %char(r.id) + %trim(r.name);
enddo;
dsply (line + ' END ' + %char(sqlcode) + ' ' + sqlstate);
exec sql close c1;
line = 'CLOSE ' + %char(sqlcode);
exec sql close c1;
line += ' ' + %char(sqlcode);
exec sql fetch c1 into :id;
dsply (line + ' FETCH ' + %char(sqlcode));
exec sql declare c2 scroll cursor for select id from QTEMP/NSQL4T
  order by id;
exec sql open c2;
exec sql fetch last from c2 into :id;
line = 'LAST ' + %char(id);
exec sql fetch prior from c2 into :id;
line += ' PRIOR ' + %char(id);
exec sql fetch first from c2 into :id;
line += ' FIRST ' + %char(id);
exec sql fetch relative 2 from c2 into :id;
line += ' REL ' + %char(id);
dsply (line);
exec sql fetch before from c2;
line = 'BEFORE ' + %char(sqlcode);
exec sql fetch next from c2;
line += ' NOINTO ' + %char(sqlcode);
exec sql fetch next from c2 into :id;
line += ' ' + %char(id);
exec sql fetch relative 9 from c2 into :id;
dsply (line + ' PAST ' + %char(sqlcode));
exec sql close c2;
stmt = 'select name, amt from QTEMP/NSQL4T where id >= ? order by id';
exec sql prepare s3 from :stmt;
exec sql declare c3 cursor for s3;
lim = 2;
exec sql open c3 using :lim;
line = 'PREP';
dow 1 = 1;
  amt = 9;
  exec sql fetch c3 into :nm, :amt :amti;
  if sqlcode <> 0;
    leave;
  endif;
  line += ' ' + %trim(nm) + ' ' + %char(amti);
enddo;
dsply (line + ' END ' + %char(sqlcode));
exec sql close c3;
exec sql drop table QTEMP/NSQL4T;
*inlr = *on;
