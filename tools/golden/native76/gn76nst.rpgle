**FREE
// Stage 2, S2.76: subfields of a LIKEDS subfield with DIM, a path with
// one index on its middle part (list.users(i).name), assigned and read
// with a literal and a field index, and one past the elements caught by
// MONITOR (status 121). Captured on PUB400 (tools/golden/native76.txt
// n76_nested) for the native unit NNEST.
ctl-opt dftactgrp(*no);
dcl-ds user_t qualified template;
  name char(10);
  days zoned(3:0);
end-ds;
dcl-ds list qualified;
  count int(10);
  users likeds(user_t) dim(3);
end-ds;
dcl-s i int(10);
dcl-s line varchar(52);
list.count = 3;
list.users(1).name = 'ADA';
list.users(1).days = 5;
i = 2;
list.users(i).name = 'GRACE';
list.users(i).days = list.users(1).days + 7;
list.users(3).name = 'LIN';
list.users(3).days = list.users(i).days * 2;
line = 'U';
for i = 1 to list.count;
  line += ' ' + %trim(list.users(i).name) + '=' + %char(list.users(i).days);
endfor;
say(line);
i = 4;
monitor;
  list.users(i).days = 1;
  say('NO ERROR');
on-error;
  say('ERROR ' + %char(%status));
endmon;
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
