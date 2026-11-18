**FREE
// Native unit NSQL1 (S2.69): embedded SQL with no host variables. CREATE
// TABLE, INSERT of literal rows, an UPDATE that changes no rows, a DELETE
// of two rows, a missing table, COMMIT and DROP TABLE, each followed by
// the SQLCA's SQLCODE, SQLSTATE and SQLERRD(3). The program captured on
// PUB400, with the table in QGPL and DSPLY for its output, less two lines
// as4tainr does not give (below).
// Source: golden capture tools/golden/native69 n69_sql (PUB400, IBM i
// 7.5, 2026-11-18).
ctl-opt dftactgrp(*no);
exec sql set option commit = *none;
exec sql drop table QGPL/NSQL1T;
exec sql create table QGPL/NSQL1T (id int not null, name char(10));
// The capture's CREATE line (SQLCODE +7905, SQLSTATE 01567: the table is
// not journaled) is not checked: as4tainr gives 0 in both engines
// (fidelity.md, interpreter defects).
exec sql insert into QGPL/NSQL1T values (1, 'one'), (2, 'two'), (3, 'three');
show('INSERT');
exec sql update QGPL/NSQL1T set name = 'none' where id > 10;
show('UPDATE0');
exec sql delete from QGPL/NSQL1T where id < 3;
show('DELETE');
exec sql delete from QGPL/NSQL1NONE;
show('MISSING');
exec sql commit;
// The capture's COMMIT line (SQLCODE -7007, SQLSTATE 51009: COMMIT under
// COMMIT(*NONE)) is not checked either: as4tainr gives 0 in both engines.
exec sql drop table QGPL/NSQL1T;
show('DROP');
*inlr = *on;

dcl-proc show;
  dcl-pi *n;
    what varchar(10) const;
  end-pi;
  dsply (what + ' ' + %char(sqlcode) + ' ' + sqlstate + ' ' + %char(sqlerrd(3)));
end-proc;
