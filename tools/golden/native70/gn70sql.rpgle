**FREE
// Stage 2, S2.70: embedded SQL with host variables. SELECT INTO with a
// character and a numeric input, into a data structure, a null into a
// field with an indicator, no row (SQLCODE 100), more than one row
// (-811, the first row's values kept), VALUES INTO, SET :host, an UPDATE
// and a DELETE with host variables (SQLERRD(3)), and an input indicator
// of -1 (a null). Captured on PUB400 (tools/golden/native70.txt n70_sql)
// for the native unit NSQL2.
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
exec sql drop table GUIGRA261/GN70T;
exec sql create table GUIGRA261/GN70T (id int not null, name char(10),
                                       amt decimal(7, 2));
exec sql insert into GUIGRA261/GN70T values (1, 'one', 1.50),
  (2, 'two', null), (3, 'three', 30.25);
key = 'two';
exec sql select id into :cnt from GUIGRA261/GN70T where name = :key;
say('KEY ' + %char(sqlcode) + ' ' + %char(cnt));
lim = 2;
exec sql select count(*) into :cnt from GUIGRA261/GN70T where id >= :lim;
say('COUNT ' + %char(sqlcode) + ' ' + %char(cnt));
exec sql select id, name into :row from GUIGRA261/GN70T where id = 3;
say('DS ' + %char(sqlcode) + ' ' + %char(row.id) + ' ' + %trim(row.name));
amt = 9;
exec sql select amt into :amt :amti from GUIGRA261/GN70T where id = 2;
say('NULL ' + %char(sqlcode) + ' ' + %char(amti) + ' ' + %char(amt));
exec sql select amt into :amt indicator :amti from GUIGRA261/GN70T
  where id = 3;
say('VAL ' + %char(sqlcode) + ' ' + %char(amti) + ' ' + %char(amt));
nm = 'x';
exec sql select name into :nm from GUIGRA261/GN70T where id = 99;
say('NONE ' + %char(sqlcode) + ' ' + sqlstate + ' ' + %trim(nm));
exec sql select name into :nm from GUIGRA261/GN70T where id > :cnt - 2
  order by id;
say('MANY ' + %char(sqlcode) + ' ' + sqlstate + ' ' + %trim(nm));
exec sql values upper(:key) concat '!' into :res;
say('VALUES ' + %char(sqlcode) + ' ' + res);
exec sql set :cnt = :lim * 10 + 1;
say('SET ' + %char(sqlcode) + ' ' + %char(cnt));
amt = 2.5;
exec sql update GUIGRA261/GN70T set amt = amt + :amt where id <= :lim;
say('UPDATE ' + %char(sqlcode) + ' ' + %char(sqlerrd(3)));
exec sql select amt into :amt from GUIGRA261/GN70T where id = 1;
say('AFTER ' + %char(amt));
nmi = -1;
nm = 'gone';
exec sql update GUIGRA261/GN70T set name = :nm :nmi where id = 1;
exec sql select count(*) into :cnt from GUIGRA261/GN70T where name is null;
say('NULLIN ' + %char(sqlcode) + ' ' + %char(cnt));
exec sql delete from GUIGRA261/GN70T where id = :lim or name = :key;
say('DELETE ' + %char(sqlcode) + ' ' + %char(sqlerrd(3)));
exec sql drop table GUIGRA261/GN70T;
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
