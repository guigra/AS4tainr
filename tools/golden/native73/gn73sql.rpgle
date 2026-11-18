**FREE
// Stage 2, S2.73: UPDATE and DELETE WHERE CURRENT OF. A cursor FOR UPDATE:
// an UPDATE before the first FETCH (-508), an UPDATE of the second row
// with a host variable, a DELETE of the third, an UPDATE after the last
// row (-508), and one after CLOSE (-501); then the rows left. Captured
// on PUB400 (tools/golden/native73.txt n73_sql) for the native unit NSQL5.
ctl-opt dftactgrp(*no);
dcl-s id int(10);
dcl-s nm char(10);
dcl-s line varchar(52);
exec sql set option commit = *none;
exec sql drop table GUIGRA261/GN73T;
exec sql create table GUIGRA261/GN73T (id int not null, name char(10));
exec sql insert into GUIGRA261/GN73T values (1, 'one'), (2, 'two'),
  (3, 'three'), (4, 'four');
exec sql declare c cursor for select id, name from GUIGRA261/GN73T
  for update of name;
exec sql open c;
nm = 'NEW';
exec sql update GUIGRA261/GN73T set name = :nm where current of c;
line = 'BEFORE ' + %char(sqlcode);
exec sql fetch c into :id, :nm;
exec sql fetch c into :id, :nm;
nm = 'TWO!';
exec sql update GUIGRA261/GN73T set name = :nm where current of c;
line += ' UPD ' + %char(sqlcode) + ' ' + %char(sqlerrd(3));
exec sql fetch c into :id, :nm;
exec sql delete from GUIGRA261/GN73T where current of c;
line += ' DEL ' + %char(sqlcode) + ' ' + %char(sqlerrd(3));
say(line);
exec sql fetch c into :id, :nm;
exec sql fetch c into :id, :nm;
line = 'PAST ' + %char(sqlcode);
exec sql update GUIGRA261/GN73T set name = :nm where current of c;
line += ' ' + %char(sqlcode);
exec sql close c;
exec sql delete from GUIGRA261/GN73T where current of c;
say(line + ' CLOSED ' + %char(sqlcode));
exec sql declare l cursor for select id, name from GUIGRA261/GN73T
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
say(line);
exec sql drop table GUIGRA261/GN73T;
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
