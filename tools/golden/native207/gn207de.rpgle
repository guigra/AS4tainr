**FREE
// Stage 2, S2.207: an element of a data structure array as a whole: a
// LIKEDS DIM subfield's element passed by reference to a LIKEDS
// parameter and changed there, an element assigned another, an element
// read as character data, and an index out of range. Captured on PUB400
// (tools/golden/native207.txt n207_dselem) for the native unit NDSELEM.
ctl-opt dftactgrp(*no);
dcl-ds user_t qualified template;
  name char(5);
  days zoned(3:0);
end-ds;
dcl-ds list qualified;
  count int(10);
  users likeds(user_t) dim(3);
end-ds;
dcl-ds pts qualified dim(3);
  x zoned(3:0);
  y zoned(3:0);
end-ds;
dcl-s i int(10) inz(2);
dcl-s k int(10) inz(4);
dcl-s line varchar(200);

list.users(2).name = 'GRACE';
list.users(2).days = 12;
bump(list.users(i));
line = %trim(list.users(2).name) + %char(list.users(2).days);
pts(1).x = 7;
pts(1).y = 3;
pts(3) = pts(1);
line += ' ' + %char(pts(3).x) + %char(pts(3).y) + ' [' + pts(1) + ']';
monitor;
  pts(k) = pts(1);
on-error;
  line += ' E' + %char(%status);
endmon;
say(line);
*inlr = *on;

dcl-proc bump;
  dcl-pi *n;
    u likeds(user_t);
  end-pi;
  u.days += 1;
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
