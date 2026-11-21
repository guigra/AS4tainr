**FREE
// Native unit NSQLTS (S2.184): timestamp host variables in EXEC SQL, the
// program captured on PUB400 as GN184TS (its table there GN184T, made
// before the call; here the program makes it in QTEMP), with DSPLY for
// its output.
// Source: golden capture tools/golden/native184 n184_tshost (PUB400,
// IBM i 7.5, 2026-11-21).
dcl-s stmt varchar(200);
dcl-s n int(10);
dcl-s ts timestamp;
dcl-s t2 timestamp;
dcl-s line varchar(200);

exec sql set option commit = *none;
exec sql drop table QTEMP/NSQLTST;
exec sql create table QTEMP/NSQLTST (t timestamp not null);
ts = z'2026-10-31-20.43.10.123456';
stmt = 'INSERT INTO QTEMP/NSQLTST VALUES (?)';
exec sql prepare s2 from :stmt;
exec sql execute s2 using :ts;
exec sql insert into QTEMP/NSQLTST values (:ts + 1 day);
exec sql select count(*), max(t) into :n, :t2 from QTEMP/NSQLTST where t >= :ts;
line = %char(n) + ' ' + %char(t2) + ' ' + %char(sqlcode);
exec sql select min(t) into :t2 from QTEMP/NSQLTST;
line += '|' + %char(t2);
dsply (line);
*inlr = *on;
