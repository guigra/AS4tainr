**FREE
// Stage 2, S2.72: cursors. A cursor declared with a host variable in its
// query (bound at OPEN) and FOR READ ONLY, fetched into a data structure
// to the end (SQLCODE 100); OPEN of an open cursor (-502); FETCH and CLOSE
// of a closed one (-501); a scroll cursor (FIRST, LAST, PRIOR, RELATIVE,
// BEFORE, FETCH with no INTO; the precompiler took no ABSOLUTE, SQL0104); a cursor over a prepared
// statement opened with USING; a null fetched with an indicator.
// Captured on PUB400 (tools/golden/native72.txt n72_sql) for the native
// unit NSQL4.
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
exec sql drop table GUIGRA261/GN72T;
exec sql create table GUIGRA261/GN72T (id int not null, name char(10),
                                       amt decimal(7, 2));
exec sql insert into GUIGRA261/GN72T values (1, 'one', 1.50),
  (2, 'two', null), (3, 'three', 3.00), (4, 'four', 4.25);
exec sql declare c1 cursor for select id, name from GUIGRA261/GN72T
  where id > :lim order by id for read only;
lim = 1;
exec sql open c1;
line = 'OPEN ' + %char(sqlcode);
lim = 3;
exec sql open c1;
say(line + ' AGAIN ' + %char(sqlcode));
line = 'ROWS';
dow 1 = 1;
  exec sql fetch next from c1 into :r;
  if sqlcode <> 0;
    leave;
  endif;
  line += ' ' + %char(r.id) + %trim(r.name);
enddo;
say(line + ' END ' + %char(sqlcode) + ' ' + sqlstate);
exec sql close c1;
line = 'CLOSE ' + %char(sqlcode);
exec sql close c1;
line += ' ' + %char(sqlcode);
exec sql fetch c1 into :id;
say(line + ' FETCH ' + %char(sqlcode));
exec sql declare c2 scroll cursor for select id from GUIGRA261/GN72T
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
say(line);
exec sql fetch before from c2;
line = 'BEFORE ' + %char(sqlcode);
exec sql fetch next from c2;
line += ' NOINTO ' + %char(sqlcode);
exec sql fetch next from c2 into :id;
line += ' ' + %char(id);
exec sql fetch relative 9 from c2 into :id;
say(line + ' PAST ' + %char(sqlcode));
exec sql close c2;
stmt = 'select name, amt from GUIGRA261/GN72T where id >= ? order by id';
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
say(line + ' END ' + %char(sqlcode));
exec sql close c3;
exec sql drop table GUIGRA261/GN72T;
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
