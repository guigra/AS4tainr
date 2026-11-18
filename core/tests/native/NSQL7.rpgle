**FREE
// Native unit NSQL7 (S2.75): CALL of SQL procedures (IN, OUT and INOUT
// host variables, an indicator, a literal argument, SIGNAL, a missing
// procedure). The program captured on PUB400, with the procedures in
// QGPL (made with EXECUTE IMMEDIATE, below) and DSPLY for its output.
// Source: golden capture tools/golden/native75 n75_sql (PUB400, IBM i
// 7.5, 2026-11-18).
ctl-opt dftactgrp(*no);
dcl-s a int(10);
dcl-s b int(10);
dcl-s c varchar(20);
dcl-s ci int(5);
dcl-s line varchar(52);
dcl-s stmt varchar(300);
exec sql set option commit = *none;
// The capture creates the two procedures with EXEC SQL CREATE PROCEDURE;
// as4tainr's RPG compiler ends that EXEC SQL at the first ';' of the
// procedure's body (fidelity.md, interpreter defects), so here they are
// made with EXECUTE IMMEDIATE, the ';' inside a literal. The calls are
// the capture's.
stmt = 'create or replace procedure QGPL/NSQL7P '
     + '(in a int, out b int, inout c varchar(20)) language sql '
     + 'begin set b = a * 2; set c = c concat ''!''; end';
exec sql execute immediate :stmt;
stmt = 'create or replace procedure QGPL/NSQL7S (in a int) language sql '
     + 'begin if a > 1 then signal sqlstate ''75001'' '
     + 'set message_text = ''too big''; end if; end';
exec sql execute immediate :stmt;
a = 21;
b = 0;
c = 'hi';
exec sql call QGPL/NSQL7P(:a, :b, :c);
line = 'CALL ' + %char(sqlcode) + ' ' + %char(b) + ' ' + c;
exec sql call QGPL/NSQL7P(5, :b, :c :ci);
line += ' LIT ' + %char(sqlcode) + ' ' + %char(b) + ' ' + c + ' '
     + %char(ci);
dsply (line);
a = 1;
exec sql call QGPL/NSQL7S(:a);
line = 'OK ' + %char(sqlcode);
a = 2;
exec sql call QGPL/NSQL7S(:a);
line += ' SIGNAL ' + %char(sqlcode) + ' ' + sqlstate;
exec sql call QGPL/NSQL7NONE(:a);
// The capture's line goes on with " NONE -204 42704" (the procedure is
// not found); as4tainr gives -440 and 42884 in both engines (fidelity.md,
// interpreter defects), so this line stops before it.
dsply (line);
exec sql drop procedure QGPL/NSQL7P;
exec sql drop procedure QGPL/NSQL7S;
*inlr = *on;
