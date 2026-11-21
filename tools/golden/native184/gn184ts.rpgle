**FREE
// Stage 2, S2.184: timestamp host variables in EXEC SQL: given to a
// parameter marker of EXECUTE USING, in an expression of INSERT's VALUES,
// compared in WHERE and read back INTO. Captured on PUB400
// (tools/golden/native184.txt n184_tshost) for the native unit NSQLTS.
ctl-opt dftactgrp(*no);
dcl-s stmt varchar(200);
dcl-s n int(10);
dcl-s ts timestamp;
dcl-s t2 timestamp;
dcl-s line varchar(200);

exec sql delete from gn184t;
ts = z'2026-10-31-20.43.10.123456';
stmt = 'INSERT INTO GN184T VALUES (?)';
exec sql prepare s2 from :stmt;
exec sql execute s2 using :ts;
exec sql insert into gn184t values (:ts + 1 day);
exec sql select count(*), max(t) into :n, :t2 from gn184t where t >= :ts;
line = %char(n) + ' ' + %char(t2) + ' ' + %char(sqlcode);
exec sql select min(t) into :t2 from gn184t;
line += '|' + %char(t2);
say(line);
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
