**FREE
// Stage 2, S2.156: WRITE of a program-described printer file from a data
// structure. Captured on PUB400 as PASE system shows the spooled file
// (the program's own read of it, SYSTOOLS.SPOOLED_FILE_DATA, finds none
// under system, which keeps no spooled files);
// (tools/golden/native156.txt n156_prtds) for the native unit NPRTDS.
ctl-opt dftactgrp(*no);
dcl-f QSYSPRT printer(80) usropn;
dcl-ds line len(80);
  text char(20);
  num zoned(5:0) pos(22);
end-ds;
dcl-s sdata varchar(200);

open QSYSPRT;
text = 'first line';
num = 42;
write QSYSPRT line;
text = 'second line';
num = 7;
write QSYSPRT line;
close QSYSPRT;
exec sql declare c1 cursor for
  select spooled_data from table(systools.spooled_file_data(
    job_name => '*', spooled_file_name => 'QSYSPRT',
    spooled_file_number => '*LAST')) x;
exec sql open c1;
dow 1 = 1;
  exec sql fetch c1 into :sdata;
  if sqlcode <> 0;
    leave;
  endif;
  if %trim(sdata) <> '';
    say(%trimr(sdata) + '|');
  endif;
enddo;
exec sql close c1;
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
