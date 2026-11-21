**FREE
// Native unit NPDDS (S2.198): READ of a program-described file into a
// data structure, the program captured on PUB400 as GN198PD with DSPLY
// for its output.
// Source: golden capture tools/golden/native198 n198_pdds (PUB400,
// IBM i 7.5, 2026-11-21).
ctl-opt dftactgrp(*no);
dcl-s line varchar(200);

exec sql set option commit = *none;
exec sql drop table QTEMP/GN198F;
exec sql create table QTEMP/GN198F (r char(20) not null);
exec sql insert into QTEMP/GN198F values ('HELLO world'), ('second   xy');
line = readAll();
dsply (line);
*inlr = *on;

dcl-proc readAll;
  dcl-pi *n varchar(200) end-pi;
  dcl-f gn198f disk(20) extfile('QTEMP/GN198F') usropn;
  dcl-ds rec len(20);
    head char(5) pos(1);
  end-ds;
  dcl-s res varchar(200);
  open gn198f;
  read gn198f rec;
  dow not %eof(gn198f);
    res += '[' + rec + '|' + head + ']';
    read gn198f rec;
  enddo;
  close gn198f;
  return res;
end-proc;
