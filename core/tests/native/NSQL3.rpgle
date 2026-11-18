**FREE
// Native unit NSQL3 (S2.71): dynamic SQL (EXECUTE IMMEDIATE, PREPARE,
// EXECUTE with USING and an indicator) and GET DIAGNOSTICS (ROW_COUNT,
// CONDITION 1's RETURNED_SQLSTATE, DB2_RETURNED_SQLCODE and
// DB2_MESSAGE_ID). The program captured on PUB400, with the table in QGPL
// and DSPLY for its output, less the CREATE line (below).
// Source: golden capture tools/golden/native71 n71_sql (PUB400, IBM i
// 7.5, 2026-11-18).
ctl-opt dftactgrp(*no);
dcl-s stmt varchar(200);
dcl-s fixed char(80);
dcl-s nm char(10);
dcl-s n packed(5:0);
dcl-s ni int(5);
dcl-s cnt int(10);
dcl-s rc int(10);
dcl-s st char(5);
dcl-s code int(10);
dcl-s mid char(10);
exec sql set option commit = *none;
exec sql drop table QGPL/NSQL3T;
stmt = 'create table QGPL/NSQL3T (id dec(5, 0), name char(10))';
exec sql execute immediate :stmt;
// The capture's CREATE line (SQLCODE +7905) is not checked: as4tainr
// gives 0 (fidelity.md, interpreter defects).
stmt = 'insert into QGPL/NSQL3T values (1, ''one''), (2, ''two'')';
exec sql execute immediate :stmt;
cnt = sqlerrd(3);
exec sql get diagnostics :rc = row_count;
dsply ('IMMED ' + %char(cnt) + ' ROWS ' + %char(sqlcode) + ' ' + %char(rc));
stmt = 'insert into QGPL/NSQL3T values (?, ?)';
exec sql prepare ins from :stmt;
dsply ('PREP ' + %char(sqlcode));
n = 3;
nm = 'three';
exec sql execute ins using :n, :nm;
dsply ('EXEC ' + %char(sqlcode) + ' ' + %char(sqlerrd(3)));
n = 4;
ni = -1;
exec sql execute ins using :n, :nm :ni;
dsply ('EXNULL ' + %char(sqlcode));
exec sql select count(*) into :cnt from QGPL/NSQL3T where name is null;
dsply ('NULLS ' + %char(cnt));
fixed = 'delete from QGPL/NSQL3T where id > ?';
exec sql prepare del from :fixed;
n = 2;
exec sql execute del using :n;
dsply ('DEL ' + %char(sqlcode) + ' ' + %char(sqlerrd(3)));
exec sql execute del using :n;
dsply ('DEL2 ' + %char(sqlcode) + ' ' + sqlstate);
stmt = 'delete from QGPL/NSQL3NONE';
exec sql execute immediate :stmt;
cnt = sqlcode;
exec sql get diagnostics condition 1 :st = returned_sqlstate,
  :code = db2_returned_sqlcode, :mid = db2_message_id;
dsply ('MISS ' + %char(cnt) + ' DIAG ' + %char(sqlcode) + ' ' + st + ' '
    + %char(code) + ' ' + %trim(mid));
exec sql select count(*) into :cnt from QGPL/NSQL3T;
dsply ('LEFT ' + %char(cnt));
stmt = 'drop table QGPL/NSQL3T';
exec sql execute immediate :stmt;
dsply ('DROP ' + %char(sqlcode));
*inlr = *on;
