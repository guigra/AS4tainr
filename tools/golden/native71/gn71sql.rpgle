**FREE
// Stage 2, S2.71: dynamic SQL and GET DIAGNOSTICS. EXECUTE IMMEDIATE of a
// CREATE TABLE and of an INSERT, PREPARE from a varying field and from a
// fixed one, EXECUTE with USING (a character and a numeric value, an
// indicator of -1), EXECUTE of a prepared DELETE twice (rows, then none),
// GET DIAGNOSTICS ROW_COUNT, and CONDITION 1 items after a missing table
// (each right after its statement: say's CALL is an SQL statement too).
// Captured on PUB400 (tools/golden/native71.txt n71_sql) for the native
// unit NSQL3.
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
exec sql drop table GUIGRA261/GN71T;
stmt = 'create table GUIGRA261/GN71T (id dec(5, 0), name char(10))';
exec sql execute immediate :stmt;
say('CREATE ' + %char(sqlcode));
stmt = 'insert into GUIGRA261/GN71T values (1, ''one''), (2, ''two'')';
exec sql execute immediate :stmt;
cnt = sqlerrd(3);
exec sql get diagnostics :rc = row_count;
say('IMMED ' + %char(cnt) + ' ROWS ' + %char(sqlcode) + ' ' + %char(rc));
stmt = 'insert into GUIGRA261/GN71T values (?, ?)';
exec sql prepare ins from :stmt;
say('PREP ' + %char(sqlcode));
n = 3;
nm = 'three';
exec sql execute ins using :n, :nm;
say('EXEC ' + %char(sqlcode) + ' ' + %char(sqlerrd(3)));
n = 4;
ni = -1;
exec sql execute ins using :n, :nm :ni;
say('EXNULL ' + %char(sqlcode));
exec sql select count(*) into :cnt from GUIGRA261/GN71T where name is null;
say('NULLS ' + %char(cnt));
fixed = 'delete from GUIGRA261/GN71T where id > ?';
exec sql prepare del from :fixed;
n = 2;
exec sql execute del using :n;
say('DEL ' + %char(sqlcode) + ' ' + %char(sqlerrd(3)));
exec sql execute del using :n;
say('DEL2 ' + %char(sqlcode) + ' ' + sqlstate);
stmt = 'delete from GUIGRA261/GN71NONE';
exec sql execute immediate :stmt;
cnt = sqlcode;
exec sql get diagnostics condition 1 :st = returned_sqlstate,
  :code = db2_returned_sqlcode, :mid = db2_message_id;
say('MISS ' + %char(cnt) + ' DIAG ' + %char(sqlcode) + ' ' + st + ' '
    + %char(code) + ' ' + %trim(mid));
exec sql select count(*) into :cnt from GUIGRA261/GN71T;
say('LEFT ' + %char(cnt));
stmt = 'drop table GUIGRA261/GN71T';
exec sql execute immediate :stmt;
say('DROP ' + %char(sqlcode));
*inlr = *on;

dcl-proc say;
  dcl-pi *n;
    text varchar(200) const;
  end-pi;
  dcl-s line varchar(200);
  line = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :line,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
