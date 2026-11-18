**FREE
// Stage 2, S2.90: ON-EXIT. A procedure whose ON-EXIT records how it
// ended in a global field: a normal end (its indicator off), an ON-EXIT
// RETURN that replaces the value, and an abnormal end (a division by
// zero, caught by the caller's MONITOR, status 102) with the ON-EXIT run
// and its indicator on. Captured on PUB400 (tools/golden/native90.txt
// n90_onexit) for the native unit NONEXIT2.
ctl-opt dftactgrp(*no);
dcl-s trace varchar(40);
dcl-s r int(10);
dcl-s line varchar(52);
r = work(10: 2);
line = 'A ' + %char(r) + ' ' + trace;
r = work(7: 7);
line += ' B ' + %char(r) + ' ' + trace;
monitor;
  r = work(1: 0);
  line += ' C ' + %char(r);
on-error;
  line += ' ERR ' + %char(%status) + ' ' + trace;
endmon;
say(line);
*inlr = *on;

dcl-proc work;
  dcl-pi *n int(10);
    a int(10) value;
    b int(10) value;
  end-pi;
  dcl-s bad ind;
  trace = 'in';
  return a / b;
on-exit bad;
  if bad;
    trace = 'abnormal';
  else;
    trace = 'normal';
  endif;
  if a = b;
    return 99;
  endif;
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
