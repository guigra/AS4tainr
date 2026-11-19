**FREE
// Stage 2, S2.122: %SUBST with expressions as its start and length, of a
// field and of a whole data structure, and a start of 0 under MONITOR.
// Captured on PUB400 (tools/golden/native122.txt n122_subst) for the
// native unit NSUBSTX.
ctl-opt dftactgrp(*no);
dcl-s s char(20) inz('abcdefghijklmnopqrst');
dcl-s n int(10) inz(3);
dcl-s r varchar(20);
dcl-ds d qualified;
  x char(5) inz('12345');
  y char(5) inz('67890');
end-ds;
dcl-s line varchar(80);

line = %subst(s: n * 2 + 1: n + 1);
line += ' ' + %subst(s: %len(%trimr(s)) - n: 2);
line += ' ' + %subst(d: n + 2: n * 2);
monitor;
  r = %subst(s: n - 3: 1);
on-error;
  r = 'err' + %char(%status);
endmon;
line += ' ' + r;
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
