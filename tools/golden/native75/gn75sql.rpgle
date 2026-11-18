**FREE
// Stage 2, S2.75: CALL of SQL procedures from embedded SQL. A procedure
// with IN, OUT and INOUT parameters called with host variables (an
// INOUT with an indicator, a literal argument), one that ends with
// SIGNAL (SQLCODE -438 and its SQLSTATE), and a missing one. Captured on
// PUB400 (tools/golden/native75.txt n75_sql) for the native unit NSQL7.
ctl-opt dftactgrp(*no);
dcl-s a int(10);
dcl-s b int(10);
dcl-s c varchar(20);
dcl-s ci int(5);
dcl-s line varchar(52);
exec sql set option commit = *none;
exec sql create or replace procedure GUIGRA261/GN75P
  (in a int, out b int, inout c varchar(20))
  language sql
  begin
    set b = a * 2;
    set c = c concat '!';
  end;
exec sql create or replace procedure GUIGRA261/GN75S (in a int)
  language sql
  begin
    if a > 1 then
      signal sqlstate '75001' set message_text = 'too big';
    end if;
  end;
a = 21;
b = 0;
c = 'hi';
exec sql call GUIGRA261/GN75P(:a, :b, :c);
line = 'CALL ' + %char(sqlcode) + ' ' + %char(b) + ' ' + c;
exec sql call GUIGRA261/GN75P(5, :b, :c :ci);
line += ' LIT ' + %char(sqlcode) + ' ' + %char(b) + ' ' + c + ' '
     + %char(ci);
say(line);
a = 1;
exec sql call GUIGRA261/GN75S(:a);
line = 'OK ' + %char(sqlcode);
a = 2;
exec sql call GUIGRA261/GN75S(:a);
line += ' SIGNAL ' + %char(sqlcode) + ' ' + sqlstate;
exec sql call GUIGRA261/GN75NONE(:a);
say(line + ' NONE ' + %char(sqlcode) + ' ' + sqlstate);
exec sql drop procedure GUIGRA261/GN75P;
exec sql drop procedure GUIGRA261/GN75S;
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
