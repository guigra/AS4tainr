**FREE
// Stage 2, S2.98: a pointer minus a pointer: two pointers into the same
// character field, both ways; into the same %ALLOC storage; and into two
// different fields, under MONITOR. Captured on PUB400
// (tools/golden/native99.txt n99_ptrdiff) for the native unit NPTRDIF.
ctl-opt dftactgrp(*no);
dcl-s buf char(20);
dcl-s other char(4);
dcl-s p1 pointer;
dcl-s p2 pointer;
dcl-s h1 pointer;
dcl-s h2 pointer;
dcl-s n int(10);
dcl-s line varchar(80);
p1 = %addr(buf);
p2 = %addr(buf) + 5;
n = p2 - p1;
line = %char(n);
n = p1 - p2;
line += ' ' + %char(n);
h1 = %alloc(16);
h2 = h1 + 7;
n = h2 - h1;
line += ' ' + %char(n);
dealloc h1;
p2 = %addr(other);
monitor;
  n = p2 - p1;
  line += ' D';
on-error;
  line += ' E ' + %char(%status);
endmon;
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
