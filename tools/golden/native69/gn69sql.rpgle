**FREE
// Stage 2, S2.69: embedded SQL with no host variables. CREATE TABLE,
// INSERT of literal rows, an UPDATE that changes no rows (SQLCODE 100), a
// DELETE of two rows (SQLERRD(3)), a missing table (SQLCODE -204), COMMIT
// and DROP TABLE, each followed by the SQLCA's SQLCODE, SQLSTATE and
// SQLERRD(3). Captured on PUB400 (tools/golden/native69.txt n69_sql) for
// the native unit NSQL1.
ctl-opt dftactgrp(*no);
exec sql set option commit = *none;
exec sql drop table GUIGRA261/GN69T;
exec sql create table GUIGRA261/GN69T (id int not null, name char(10));
show('CREATE');
exec sql insert into GUIGRA261/GN69T values (1, 'one'), (2, 'two'), (3, 'three');
show('INSERT');
exec sql update GUIGRA261/GN69T set name = 'none' where id > 10;
show('UPDATE0');
exec sql delete from GUIGRA261/GN69T where id < 3;
show('DELETE');
exec sql delete from GUIGRA261/GN69NONE;
show('MISSING');
exec sql commit;
show('COMMIT');
exec sql drop table GUIGRA261/GN69T;
show('DROP');
*inlr = *on;

dcl-proc show;
  dcl-pi *n;
    what varchar(10) const;
  end-pi;
  say(what + ' ' + %char(sqlcode) + ' ' + sqlstate + ' ' + %char(sqlerrd(3)));
end-proc;

dcl-proc say;
  dcl-pi *n;
    text varchar(200) const;
  end-pi;
  dcl-s line varchar(200);
  line = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :line,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
