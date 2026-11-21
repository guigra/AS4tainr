**FREE
// Stage 2, S2.198: READ of a program-described file into a data
// structure, in a procedure with its own file: the record image into the
// structure byte for byte, a subfield over part of it. Captured on PUB400
// (tools/golden/native198.txt n198_pdds) for the native unit NPDDS.
ctl-opt dftactgrp(*no);
dcl-s line varchar(200);

exec sql set option commit = *none;
exec sql drop table QTEMP/GN198F;
exec sql create table QTEMP/GN198F (r char(20) not null);
exec sql insert into QTEMP/GN198F values ('HELLO world'), ('second   xy');
line = readAll();
say(line);
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

dcl-proc say;
  dcl-pi *n;
    text varchar(200) const;
  end-pi;
  dcl-s wbuf varchar(200);
  wbuf = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :wbuf,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
