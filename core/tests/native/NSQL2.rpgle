**FREE
// Native unit NSQL2 (S2.70): embedded SQL with host variables (SELECT
// INTO, a data structure, indicators in and out, no row, more than one
// row, VALUES INTO, SET :host, UPDATE and DELETE with host variables).
// The program captured on PUB400, with the table in QGPL and DSPLY for
// its output, less two lines as4tainr does not give (below).
// Source: golden capture tools/golden/native70 n70_sql (PUB400, IBM i
// 7.5, 2026-11-18).
ctl-opt dftactgrp(*no);
dcl-s cnt int(10);
dcl-s nm char(10);
dcl-s key char(10);
dcl-s lim packed(5:0);
dcl-s amt packed(7:2);
dcl-s nmi int(5);
dcl-s amti int(5);
dcl-s res varchar(30);
dcl-ds row qualified;
  id int(10);
  name char(10);
end-ds;
exec sql set option commit = *none;
exec sql drop table QGPL/NSQL2T;
exec sql create table QGPL/NSQL2T (id int not null, name char(10),
                                       amt decimal(7, 2));
exec sql insert into QGPL/NSQL2T values (1, 'one', 1.50),
  (2, 'two', null), (3, 'three', 30.25);
key = 'two';
exec sql select id into :cnt from QGPL/NSQL2T where name = :key;
dsply ('KEY ' + %char(sqlcode) + ' ' + %char(cnt));
lim = 2;
exec sql select count(*) into :cnt from QGPL/NSQL2T where id >= :lim;
dsply ('COUNT ' + %char(sqlcode) + ' ' + %char(cnt));
exec sql select id, name into :row from QGPL/NSQL2T where id = 3;
dsply ('DS ' + %char(sqlcode) + ' ' + %char(row.id) + ' ' + %trim(row.name));
amt = 9;
exec sql select amt into :amt :amti from QGPL/NSQL2T where id = 2;
// The capture's NULL line (amt set to 0 with the null, ".00") is not
// checked: as4tainr leaves the field as it was (fidelity.md, interpreter
// defects).
exec sql select amt into :amt indicator :amti from QGPL/NSQL2T
  where id = 3;
dsply ('VAL ' + %char(sqlcode) + ' ' + %char(amti) + ' ' + %char(amt));
nm = 'x';
exec sql select name into :nm from QGPL/NSQL2T where id = 99;
dsply ('NONE ' + %char(sqlcode) + ' ' + sqlstate + ' ' + %trim(nm));
exec sql select name into :nm from QGPL/NSQL2T where id > :cnt - 2
  order by id;
dsply ('MANY ' + %char(sqlcode) + ' ' + sqlstate + ' ' + %trim(nm));
exec sql values upper(:key) concat '!' into :res;
// The capture's VALUES line ("TWO       !": upper of a CHAR(10) keeps its
// blanks) is not checked: as4tainr gives "TWO!" (fidelity.md).
exec sql set :cnt = :lim * 10 + 1;
dsply ('SET ' + %char(sqlcode) + ' ' + %char(cnt));
amt = 2.5;
exec sql update QGPL/NSQL2T set amt = amt + :amt where id <= :lim;
dsply ('UPDATE ' + %char(sqlcode) + ' ' + %char(sqlerrd(3)));
exec sql select amt into :amt from QGPL/NSQL2T where id = 1;
dsply ('AFTER ' + %char(amt));
nmi = -1;
nm = 'gone';
exec sql update QGPL/NSQL2T set name = :nm :nmi where id = 1;
exec sql select count(*) into :cnt from QGPL/NSQL2T where name is null;
dsply ('NULLIN ' + %char(sqlcode) + ' ' + %char(cnt));
exec sql delete from QGPL/NSQL2T where id = :lim or name = :key;
dsply ('DELETE ' + %char(sqlcode) + ' ' + %char(sqlerrd(3)));
exec sql drop table QGPL/NSQL2T;
*inlr = *on;
